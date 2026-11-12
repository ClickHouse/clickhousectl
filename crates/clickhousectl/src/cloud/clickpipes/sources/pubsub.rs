use crate::cloud::clickpipes::cli::{ClickPipeCreateRequestArgs, StreamingDestinationArgs};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_request_args, print_created,
};
use crate::cloud::clickpipes::sources::{
    AuthenticationValueParser, GcpAuthentication, read_gcp_service_account_file,
};
use crate::cloud::clickpipes::{ClickPipeSourceKind, parse_enum};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::shared::{parse_datetime, resolve_org_id};
use clap::Args;
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};

pub(in crate::cloud::clickpipes) const PUBSUB_FORMATS: &[&str] =
    &["JSONEachRow", "Avro", "Protobuf"];
pub(in crate::cloud::clickpipes) const PUBSUB_SEEK_TYPES: &[&str] =
    &["latest", "earliest", "timestamp"];
/// `maxLength` of the Pub/Sub subscription filter in the spec, so an over-long
/// CEL expression is a usage error instead of a rejected request.
const PUBSUB_FILTER_MAX_LENGTH: usize = 256;

fn parse_pubsub_authentication(value: &str) -> CloudResult<GcpAuthentication> {
    use clickhouse_cloud_api::models::{
        ClickPipePostPubSubSourceAuthentication as ServiceAccountAuth,
        ClickPipePostPubSubWorkloadIdentitySourceAuthentication as WorkloadIdentityAuth,
    };

    let json = serde_json::Value::String(value.to_string());
    match serde_json::from_value::<ServiceAccountAuth>(json.clone())? {
        ServiceAccountAuth::ServiceAccount => Ok(GcpAuthentication::ServiceAccount),
        ServiceAccountAuth::Unknown(_) => {
            match serde_json::from_value::<WorkloadIdentityAuth>(json)? {
                WorkloadIdentityAuth::ServiceAccountWorkloadIdentity => {
                    Ok(GcpAuthentication::WorkloadIdentity)
                }
                WorkloadIdentityAuth::Unknown(value) => Err(CloudError::usage(format!(
                    "unknown Pub/Sub authentication method '{value}'"
                ))),
            }
        }
    }
}

pub(in crate::cloud::clickpipes) fn parse_supported_pubsub_auth(
    value: &str,
) -> Result<String, String> {
    parse_pubsub_authentication(value)
        .map(|_| value.to_string())
        .map_err(|error| error.message)
}

#[derive(Args)]
pub struct PubSubSchemaDiscoverArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    #[command(flatten)]
    pub source: PubSubSourceFields,
}

/// Source-connection fields for a Google Cloud Pub/Sub ClickPipe source.
/// Flattened into both `PubSubCreateArgs` (pipe creation) and the
/// schema-discover pubsub subcommand so the source field set has a single
/// definition, the way `KafkaSourceFields` does for Kafka.
///
/// Requiredness follows the selected `ClickPipePostPubSubSource` union arm.
#[derive(Args, Debug)]
pub struct PubSubSourceFields {
    /// Pub/Sub topic name (not the fully-qualified path)
    #[arg(long)]
    pub topic: String,

    /// GCP project ID that owns the Pub/Sub topic
    #[arg(long)]
    pub project_id: String,

    /// Format of messages in the Pub/Sub topic
    #[arg(long, value_parser = PossibleValuesParser::new(PUBSUB_FORMATS))]
    pub format: String,

    /// Path to the GCP service account JSON key file, or - to read it from stdin
    ///
    /// Required with --auth SERVICE_ACCOUNT and invalid with workload identity.
    #[arg(long, value_name = "PATH")]
    pub service_account_file: Option<String>,

    /// Starting position for consuming the subscription
    #[arg(long, value_parser = PossibleValuesParser::new(PUBSUB_SEEK_TYPES))]
    pub seek_type: String,

    /// Timestamp to seek to (ISO 8601 / RFC 3339)
    ///
    /// Required with --seek-type timestamp, rejected with any other seek type.
    #[arg(
        long,
        value_name = "TIMESTAMP",
        value_parser = parse_datetime,
        required_if_eq("seek_type", "timestamp"),
    )]
    pub seek_timestamp: Option<String>,

    /// Authentication method
    #[arg(
        long,
        default_value = "SERVICE_ACCOUNT",
        value_parser = AuthenticationValueParser { parse: parse_supported_pubsub_auth, values: &["SERVICE_ACCOUNT", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"] },
    )]
    pub auth: String,

    /// Pub/Sub subscription filter expression (CEL, at most 256 characters)
    #[arg(long, value_parser = parse_pubsub_filter)]
    pub filter: Option<String>,

    /// Enable ordered delivery (needs messages published with ordering keys)
    #[arg(long)]
    pub enable_ordering: bool,

    /// Acknowledgement deadline for messages, in seconds (10-600)
    #[arg(
        long,
        value_name = "SECONDS",
        value_parser = clap::value_parser!(i64).range(10..=600),
    )]
    pub ack_deadline: Option<i64>,
}

#[derive(Args, Debug)]
pub struct PubSubCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub request: ClickPipeCreateRequestArgs,

    #[command(flatten)]
    pub source: PubSubSourceFields,

    #[command(flatten)]
    pub destination: StreamingDestinationArgs,
}

/// Validate a Pub/Sub subscription filter against the spec's 256-character
/// limit, so an over-long CEL expression is a clap usage error instead of a
/// request the API rejects. The message reports the length only, never the
/// expression, which can name topic attributes.
fn parse_pubsub_filter(value: &str) -> Result<String, String> {
    let length = value.chars().count();
    if length > PUBSUB_FILTER_MAX_LENGTH {
        return Err(format!(
            "filter is {length} characters; the Pub/Sub subscription filter limit is {PUBSUB_FILTER_MAX_LENGTH}"
        ));
    }
    Ok(value.to_string())
}

/// Parse `--seek-timestamp` into the library's UTC timestamp. clap already
/// validates the format with `parse_datetime`, so this keeps the builder total
/// rather than being a second gate the user can hit.
fn parse_pubsub_seek_timestamp(value: &str) -> CloudResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&chrono::Utc))
        .map_err(|error| {
            CloudError::usage(format!(
                "invalid --seek-timestamp '{value}': expected ISO 8601 / RFC 3339 format (e.g. 2026-04-10T12:00:00Z): {error}"
            ))
        })
}

/// Check the Pub/Sub seek and authentication rules clap cannot express. No
/// I/O, so it runs before credentials or network access, and again in the
/// builder. Returns the seek type and, for `SERVICE_ACCOUNT`, the key file
/// path; `None` means workload identity.
pub(in crate::cloud::clickpipes) fn validate_pubsub_source_args(
    args: &PubSubSourceFields,
) -> CloudResult<(
    clickhouse_cloud_api::models::ClickPipePostPubSubSourceSeektype,
    Option<&str>,
)> {
    use clickhouse_cloud_api::models::ClickPipePostPubSubSourceSeektype;

    let seek_type: ClickPipePostPubSubSourceSeektype = parse_enum(&args.seek_type)?;
    // The API rejects a seekTimestamp that does not match the seek type. clap
    // can require the flag for `timestamp` but cannot forbid it for the other
    // values, so the inverse relationship is checked here.
    if args.seek_timestamp.is_some() && seek_type != ClickPipePostPubSubSourceSeektype::Timestamp {
        return Err(CloudError::usage(format!(
            "--seek-timestamp can only be used with --seek-type timestamp, not --seek-type {}",
            args.seek_type
        )));
    }

    let service_account_file = match (
        parse_pubsub_authentication(&args.auth)?,
        args.service_account_file.as_deref(),
    ) {
        (GcpAuthentication::ServiceAccount, None) => {
            return Err(CloudError::usage(
                "--auth SERVICE_ACCOUNT requires --service-account-file",
            ));
        }
        (GcpAuthentication::WorkloadIdentity, Some(_)) => {
            return Err(CloudError::usage(
                "--service-account-file cannot be used with --auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ));
        }
        (GcpAuthentication::ServiceAccount, Some(path)) => Some(path),
        (GcpAuthentication::WorkloadIdentity, None) => None,
    };
    Ok((seek_type, service_account_file))
}

/// Build a `ClickPipePostPubSubSource` from the CLI args, reading the GCP
/// service-account key up front so a bad path or an unreadable key fails
/// before any network call. Shared by the `clickpipe create pubsub` and
/// `clickpipe schema-discover pubsub <SERVICE_ID>` handlers, so discovery and
/// creation send an identical `pubsub` source.
fn build_pubsub_source(
    args: &PubSubSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostPubSubSource> {
    use clickhouse_cloud_api::models::{
        ClickPipePostPubSubServiceAccountSource, ClickPipePostPubSubSourceAuthentication,
        ClickPipePostPubSubWorkloadIdentitySource,
        ClickPipePostPubSubWorkloadIdentitySourceAuthentication, ServiceAccount,
    };

    let (seek_type, service_account_file) = validate_pubsub_source_args(args)?;
    match service_account_file {
        Some(path) => {
            Ok(ClickPipePostPubSubServiceAccountSource {
                topic: args.topic.clone(),
                project_id: args.project_id.clone(),
                format: parse_enum(&args.format)?,
                authentication: ClickPipePostPubSubSourceAuthentication::ServiceAccount,
                seek_type,
                seek_timestamp: args
                    .seek_timestamp
                    .as_deref()
                    .map(parse_pubsub_seek_timestamp)
                    .transpose()?,
                service_account_key: ServiceAccount {
                    service_account_file: read_gcp_service_account_file(path)?,
                },
                filter: args.filter.clone(),
                enable_ordering: args.enable_ordering.then_some(true),
                ack_deadline: args.ack_deadline,
            }
            .into())
        }
        None => {
            Ok(ClickPipePostPubSubWorkloadIdentitySource {
                topic: args.topic.clone(),
                project_id: args.project_id.clone(),
                format: parse_enum(&args.format)?,
                authentication:
                    ClickPipePostPubSubWorkloadIdentitySourceAuthentication::ServiceAccountWorkloadIdentity,
                seek_type,
                seek_timestamp: args
                    .seek_timestamp
                    .as_deref()
                    .map(parse_pubsub_seek_timestamp)
                    .transpose()?,
                filter: args.filter.clone(),
                enable_ordering: args.enable_ordering.then_some(true),
                ack_deadline: args.ack_deadline,
            }
            .into())
        }
    }
}

/// Build the schema-discovery request body for a Pub/Sub source, from the same
/// helper `clickpipe create pubsub` uses; every other source key is absent.
pub(in crate::cloud::clickpipes) fn build_pubsub_schema_discovery_request(
    args: &PubSubSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeSchemaDiscoveryRequest, ClickPipeSchemaDiscoverySource,
    };

    Ok(ClickPipeSchemaDiscoveryRequest {
        source: ClickPipeSchemaDiscoverySource {
            kafka: None,
            kinesis: None,
            object_storage: None,
            pubsub: Some(build_pubsub_source(args)?),
        },
    })
}

pub(in crate::cloud::clickpipes) async fn clickpipe_create_pubsub(
    client: &CloudClient,
    args: &PubSubCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let request_args = build_create_request_args(&args.request, ClickPipeSourceKind::PubSub)?;
    let parsed_columns = args.destination.parsed_columns()?;
    let source = build_pubsub_source(&args.source)?;

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            pubsub: Some(source),
            bigquery: None,
            kafka: None,
            kinesis: None,
            mongodb: None,
            mysql: None,
            object_storage: None,
            postgres: None,
            validate_samples: None,
        },
        destination: args.destination.build(parsed_columns)?,
        field_mappings: None,
        scaling: None,
        settings: None,
        start_paused: None,
    };
    apply_create_request_args(&mut request, request_args);

    let org_id = resolve_org_id(client).await?;
    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::cli::{ClickPipeCommands, ClickPipeSchemaDiscoverCommands};
    use crate::cloud::clickpipes::test_support::{
        SERVICE_ACCOUNT_KEY_BASE64, assert_names, assert_rejected, clickpipe_parse_error,
        names_word, parse_clickpipe, parse_pubsub_create, parse_pubsub_discovery,
        pubsub_source_flags, service_account_key_file,
    };

    #[test]
    fn parses_pubsub_create_flags_and_defaults() {
        let args = parse_pubsub_create(&[
            "--topic",
            "events",
            "--project-id",
            "my-gcp-project",
            "--format",
            "Avro",
            "--seek-type",
            "timestamp",
            "--seek-timestamp",
            "2026-04-10T12:00:00Z",
            "--service-account-file",
            "/tmp/sa-key.json",
            "--auth",
            "SERVICE_ACCOUNT",
            "--filter",
            r#"attributes.region = "eu""#,
            "--enable-ordering",
            "--ack-deadline",
            "120",
            "--column",
            "event_id:Int64",
            "--column",
            "name:String",
            "--role",
            "analytics_reader",
            "--org-id",
            "org-1",
        ]);

        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.source.topic, "events");
        assert_eq!(args.source.project_id, "my-gcp-project");
        assert_eq!(args.source.format, "Avro");
        assert_eq!(args.source.seek_type, "timestamp");
        assert_eq!(
            args.source.seek_timestamp.as_deref(),
            Some("2026-04-10T12:00:00Z")
        );
        assert_eq!(
            args.source.service_account_file.as_deref(),
            Some("/tmp/sa-key.json")
        );
        assert_eq!(args.source.auth, "SERVICE_ACCOUNT");
        assert_eq!(
            args.source.filter.as_deref(),
            Some(r#"attributes.region = "eu""#)
        );
        assert!(args.source.enable_ordering);
        assert_eq!(args.source.ack_deadline, Some(120));
        assert_eq!(args.destination.database, "db");
        assert_eq!(args.destination.table, "events");
        assert_eq!(
            args.destination.columns,
            vec!["event_id:Int64", "name:String"]
        );
        assert_eq!(
            args.destination.destination_roles.roles,
            vec!["analytics_reader"]
        );

        // Defaults: only --auth has one, and the optional fields stay unset.
        let args = parse_pubsub_create(&pubsub_source_flags("./sa-key.json"));
        assert_eq!(args.source.auth, "SERVICE_ACCOUNT");
        assert_eq!(args.source.seek_timestamp, None);
        assert_eq!(args.source.filter, None);
        assert!(!args.source.enable_ordering);
        assert_eq!(args.source.ack_deadline, None);
        assert!(args.destination.destination_roles.roles.is_empty());
    }

    #[test]
    fn pubsub_create_requires_every_strict_source_field() {
        // Each field the library types as `T` is a required flag, so dropping
        // one is a usage error rather than a request the API rejects.
        for omitted in ["--topic", "--project-id", "--format", "--seek-type"] {
            let flags = pubsub_source_flags("./sa-key.json");
            let index = flags
                .iter()
                .position(|arg| *arg == omitted)
                .expect("baseline flag");
            let mut args = vec![
                "create",
                "pubsub",
                "svc-1",
                "--name",
                "pipe-1",
                "--database",
                "db",
                "--table",
                "events",
                "--column",
                "id:UInt64",
            ];
            args.extend(
                flags
                    .iter()
                    .copied()
                    .enumerate()
                    .filter_map(|(at, arg)| (at != index && at != index + 1).then_some(arg)),
            );
            let error = clickpipe_parse_error(&args);
            assert_eq!(error.exit_code(), 2, "{omitted}: {error}");
            assert!(error.to_string().contains(omitted), "{omitted}: {error}");
        }
    }

    #[test]
    fn pubsub_seek_timestamp_pairs_with_the_timestamp_seek_type() {
        let (_dir, key_path) = service_account_key_file();

        // `--seek-type timestamp` without the companion flag is a clap error.
        let mut flags = pubsub_source_flags(&key_path);
        let index = flags
            .iter()
            .position(|arg| *arg == "earliest")
            .expect("baseline seek type");
        flags[index] = "timestamp";
        let mut args = vec![
            "create",
            "pubsub",
            "svc-1",
            "--name",
            "pipe-1",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
        ];
        args.extend(flags.iter().copied());
        let error = clickpipe_parse_error(&args);
        assert_eq!(error.exit_code(), 2, "{error}");
        assert!(error.to_string().contains("--seek-timestamp"), "{error}");

        // With it, the pair parses and builds.
        let mut with_timestamp = flags.clone();
        with_timestamp.extend(["--seek-timestamp", "2026-04-10T12:00:00Z"]);
        let built = build_pubsub_source(&parse_pubsub_create(&with_timestamp).source)
            .expect("timestamp seek builds");
        let clickhouse_cloud_api::models::ClickPipePostPubSubSource::ClickPipePostPubSubServiceAccountSource(built) = built else {
            panic!("expected service-account source");
        };
        assert_eq!(
            built.seek_timestamp,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-04-10T12:00:00Z")
                    .expect("fixed timestamp")
                    .with_timezone(&chrono::Utc)
            )
        );

        // clap cannot express the inverse relationship, so the builder refuses
        // a timestamp given for another seek type before any network call.
        let mut mismatched = pubsub_source_flags(&key_path);
        mismatched.extend(["--seek-timestamp", "2026-04-10T12:00:00Z"]);
        let error = build_pubsub_source(&parse_pubsub_create(&mismatched).source)
            .expect_err("earliest seek with a timestamp is refused");
        assert_names(
            &error.to_string(),
            &[
                "--seek-timestamp",
                "--seek-type timestamp",
                "--seek-type earliest",
            ],
        );

        // A malformed timestamp never reaches the builder.
        let mut malformed = flags;
        malformed.extend(["--seek-timestamp", "yesterday"]);
        let mut args = vec![
            "create",
            "pubsub",
            "svc-1",
            "--name",
            "pipe-1",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
        ];
        args.extend(malformed);
        let error = clickpipe_parse_error(&args);
        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        assert_names(&error.to_string(), &["--seek-timestamp", "'yesterday'"]);
    }

    #[test]
    fn pubsub_ack_deadline_and_filter_stay_inside_the_spec_bounds() {
        for deadline in ["10", "600"] {
            let mut flags = pubsub_source_flags("./sa-key.json");
            flags.extend(["--ack-deadline", deadline]);
            let args = parse_pubsub_create(&flags);
            assert_eq!(
                args.source.ack_deadline,
                Some(deadline.parse::<i64>().expect("test literal"))
            );
        }
        for deadline in ["9", "601", "0"] {
            let mut flags = pubsub_source_flags("./sa-key.json");
            flags.extend(["--ack-deadline", deadline]);
            let mut args = vec![
                "create",
                "pubsub",
                "svc-1",
                "--name",
                "pipe-1",
                "--database",
                "db",
                "--table",
                "events",
                "--column",
                "id:UInt64",
            ];
            args.extend(flags);
            assert_rejected(&args);
        }

        // The filter's 256-character limit is enforced at parse time.
        let at_limit = "a".repeat(PUBSUB_FILTER_MAX_LENGTH);
        let mut flags = pubsub_source_flags("./sa-key.json");
        flags.extend(["--filter", &at_limit]);
        assert_eq!(
            parse_pubsub_create(&flags).source.filter.as_deref(),
            Some(at_limit.as_str())
        );

        let too_long = "a".repeat(PUBSUB_FILTER_MAX_LENGTH + 1);
        let mut flags = pubsub_source_flags("./sa-key.json");
        flags.extend(["--filter", &too_long]);
        let mut args = vec![
            "create",
            "pubsub",
            "svc-1",
            "--name",
            "pipe-1",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
        ];
        args.extend(flags);
        let error = clickpipe_parse_error(&args);
        assert_eq!(error.exit_code(), 2, "{error}");
        // The error names the flag and the rejected length.
        let message = error.to_string();
        assert_names(&message, &["--filter"]);
        assert!(
            names_word(&message, &(PUBSUB_FILTER_MAX_LENGTH + 1).to_string()),
            "{message}"
        );
    }

    #[test]
    fn parses_pubsub_schema_discovery_flags() {
        let ClickPipeCommands::SchemaDiscover {
            command: ClickPipeSchemaDiscoverCommands::PubSub(args),
        } = parse_clickpipe(&[
            "schema-discover",
            "pubsub",
            "--org-id",
            "org-pubsub",
            "svc-pubsub",
            "--topic",
            "events",
            "--project-id",
            "my-gcp-project",
            "--format",
            "Protobuf",
            "--seek-type",
            "latest",
            "--service-account-file",
            "/tmp/sa-key.json",
            "--filter",
            "attributes.tenant = \"acme\"",
            "--enable-ordering",
            "--ack-deadline",
            "30",
        ])
        else {
            panic!("expected pubsub schema discovery");
        };
        assert_eq!(args.service_id, "svc-pubsub");
        let args = args.source;

        assert_eq!(args.topic, "events");
        assert_eq!(args.project_id, "my-gcp-project");
        assert_eq!(args.format, "Protobuf");
        assert_eq!(args.seek_type, "latest");
        assert_eq!(
            args.service_account_file.as_deref(),
            Some("/tmp/sa-key.json")
        );
        assert_eq!(args.auth, "SERVICE_ACCOUNT");
        assert_eq!(args.filter.as_deref(), Some("attributes.tenant = \"acme\""));
        assert!(args.enable_ordering);
        assert_eq!(args.ack_deadline, Some(30));
    }

    #[test]
    fn build_pubsub_source_supports_minimal_fields() {
        use clickhouse_cloud_api::models::{
            ClickPipePostPubSubSourceAuthentication, ClickPipePostPubSubSourceFormat,
            ClickPipePostPubSubSourceSeektype,
        };

        let (_dir, key_path) = service_account_key_file();
        let args = parse_pubsub_create(&pubsub_source_flags(&key_path));
        let source = build_pubsub_source(&args.source).expect("minimal pubsub source builds");
        let clickhouse_cloud_api::models::ClickPipePostPubSubSource::ClickPipePostPubSubServiceAccountSource(source) = source else {
            panic!("expected service-account source");
        };

        assert_eq!(source.topic, "events");
        assert_eq!(source.project_id, "my-gcp-project");
        assert_eq!(source.format, ClickPipePostPubSubSourceFormat::JSONEachRow);
        assert_eq!(
            source.authentication,
            ClickPipePostPubSubSourceAuthentication::ServiceAccount
        );
        assert_eq!(
            source.seek_type,
            ClickPipePostPubSubSourceSeektype::Earliest
        );
        assert_eq!(source.seek_timestamp, None);
        // The key file is read and base64-encoded; the path is never sent.
        assert_eq!(
            source.service_account_key.service_account_file,
            SERVICE_ACCOUNT_KEY_BASE64
        );
        assert_eq!(source.filter, None);
        assert_eq!(source.enable_ordering, None);
        assert_eq!(source.ack_deadline, None);
    }

    #[test]
    fn build_pubsub_source_supports_maximal_fields() {
        use clickhouse_cloud_api::models::{
            ClickPipePostPubSubSourceFormat, ClickPipePostPubSubSourceSeektype,
        };

        let (_dir, key_path) = service_account_key_file();
        let mut flags = pubsub_source_flags(&key_path);
        let seek = flags
            .iter()
            .position(|arg| *arg == "earliest")
            .expect("baseline seek type");
        flags[seek] = "timestamp";
        let format = flags
            .iter()
            .position(|arg| *arg == "JSONEachRow")
            .expect("baseline format");
        flags[format] = "Protobuf";
        flags.extend([
            "--seek-timestamp",
            "2026-04-10T12:00:00+02:00",
            "--filter",
            "attributes.region = \"eu\"",
            "--enable-ordering",
            "--ack-deadline",
            "600",
        ]);
        let args = parse_pubsub_create(&flags);
        let source = build_pubsub_source(&args.source).expect("maximal pubsub source builds");
        let clickhouse_cloud_api::models::ClickPipePostPubSubSource::ClickPipePostPubSubServiceAccountSource(source) = source else {
            panic!("expected service-account source");
        };

        assert_eq!(source.format, ClickPipePostPubSubSourceFormat::Protobuf);
        assert_eq!(
            source.seek_type,
            ClickPipePostPubSubSourceSeektype::Timestamp
        );
        // The offset is normalized to UTC for the wire.
        assert_eq!(
            source.seek_timestamp,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-04-10T10:00:00Z")
                    .expect("fixed timestamp")
                    .with_timezone(&chrono::Utc)
            )
        );
        assert_eq!(source.filter.as_deref(), Some("attributes.region = \"eu\""));
        assert_eq!(source.enable_ordering, Some(true));
        assert_eq!(source.ack_deadline, Some(600));
        assert_eq!(
            source.service_account_key.service_account_file,
            SERVICE_ACCOUNT_KEY_BASE64
        );
    }

    #[test]
    fn build_pubsub_source_reports_an_unreadable_or_empty_key_without_echoing_it() {
        let (dir, key_path) = service_account_key_file();

        let mut flags = pubsub_source_flags("/missing/sa-key.json");
        let args = parse_pubsub_create(&flags);
        let error = build_pubsub_source(&args.source).expect_err("missing key file fails");
        assert!(!error.to_string().is_empty());

        let empty_path = dir.path().join("empty.json");
        std::fs::write(&empty_path, b"   \n").expect("write empty key file");
        let empty_path = empty_path.to_str().expect("utf-8 temp path").to_string();
        flags = pubsub_source_flags(&empty_path);
        let args = parse_pubsub_create(&flags);
        let error = build_pubsub_source(&args.source).expect_err("empty key file fails");
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert_names(&error.to_string(), &[&format!("'{empty_path}'")]);

        // The happy path still reads the same file the other tests use.
        let args = parse_pubsub_create(&pubsub_source_flags(&key_path));
        assert!(build_pubsub_source(&args.source).is_ok());
    }

    #[test]
    fn build_pubsub_schema_discovery_request_minimal() {
        use clickhouse_cloud_api::models::{
            ClickPipePostPubSubSourceFormat, ClickPipePostPubSubSourceSeektype,
        };

        let (_dir, key_path) = service_account_key_file();
        let args = parse_pubsub_discovery(&pubsub_source_flags(&key_path));
        let request = build_pubsub_schema_discovery_request(&args)
            .expect("minimal pubsub discovery request builds");

        // Only the pubsub source is populated.
        assert!(request.source.kafka.is_none());
        assert!(request.source.kinesis.is_none());
        assert!(request.source.object_storage.is_none());
        let clickhouse_cloud_api::models::ClickPipePostPubSubSource::ClickPipePostPubSubServiceAccountSource(source) = request.source.pubsub.expect("pubsub source is set") else { panic!("expected service-account source") };
        assert_eq!(source.topic, "events");
        assert_eq!(source.project_id, "my-gcp-project");
        assert_eq!(source.format, ClickPipePostPubSubSourceFormat::JSONEachRow);
        assert_eq!(
            source.seek_type,
            ClickPipePostPubSubSourceSeektype::Earliest
        );
        assert_eq!(
            source.service_account_key.service_account_file,
            SERVICE_ACCOUNT_KEY_BASE64
        );
        assert_eq!(source.seek_timestamp, None);
        assert_eq!(source.filter, None);
        assert_eq!(source.enable_ordering, None);
        assert_eq!(source.ack_deadline, None);
    }

    #[test]
    fn build_pubsub_schema_discovery_request_maximal() {
        use clickhouse_cloud_api::models::ClickPipePostPubSubSourceSeektype;

        let (_dir, key_path) = service_account_key_file();
        let mut flags = pubsub_source_flags(&key_path);
        let seek = flags
            .iter()
            .position(|arg| *arg == "earliest")
            .expect("baseline seek type");
        flags[seek] = "timestamp";
        flags.extend([
            "--seek-timestamp",
            "2026-04-10T12:00:00Z",
            "--filter",
            "attributes.region = \"eu\"",
            "--enable-ordering",
            "--ack-deadline",
            "45",
        ]);
        let args = parse_pubsub_discovery(&flags);
        let request = build_pubsub_schema_discovery_request(&args)
            .expect("maximal pubsub discovery request builds");

        assert!(request.source.kafka.is_none());
        assert!(request.source.kinesis.is_none());
        assert!(request.source.object_storage.is_none());
        let clickhouse_cloud_api::models::ClickPipePostPubSubSource::ClickPipePostPubSubServiceAccountSource(source) = request.source.pubsub.expect("pubsub source is set") else { panic!("expected service-account source") };
        assert_eq!(
            source.seek_type,
            ClickPipePostPubSubSourceSeektype::Timestamp
        );
        assert_eq!(
            source.seek_timestamp,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-04-10T12:00:00Z")
                    .expect("fixed timestamp")
                    .with_timezone(&chrono::Utc)
            )
        );
        assert_eq!(source.filter.as_deref(), Some("attributes.region = \"eu\""));
        assert_eq!(source.enable_ordering, Some(true));
        assert_eq!(source.ack_deadline, Some(45));
    }

    #[test]
    fn pubsub_workload_identity_selects_the_typed_union_without_a_key() {
        let args = parse_pubsub_create(&[
            "--topic",
            "events",
            "--project-id",
            "project-1",
            "--format",
            "JSONEachRow",
            "--seek-type",
            "earliest",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
        ]);
        let source = build_pubsub_source(&args.source).expect("Pub/Sub workload identity builds");
        let clickhouse_cloud_api::models::ClickPipePostPubSubSource::ClickPipePostPubSubWorkloadIdentitySource(source) = source else {
            panic!("expected workload-identity source");
        };
        assert_eq!(source.project_id, "project-1");
        assert_eq!(
            source.authentication,
            clickhouse_cloud_api::models::ClickPipePostPubSubWorkloadIdentitySourceAuthentication::ServiceAccountWorkloadIdentity
        );
        let json = serde_json::to_value(source).expect("source serializes");
        assert!(json.get("serviceAccountKey").is_none());
    }

    #[test]
    fn pubsub_authentication_rejects_mismatched_key_presence() {
        let workload_with_key = parse_pubsub_create(&[
            "--topic",
            "events",
            "--project-id",
            "project-1",
            "--format",
            "JSONEachRow",
            "--seek-type",
            "earliest",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            "--service-account-file",
            "/missing/must-not-be-read.json",
        ]);
        // Rejected before the key file is read, so the error is not an I/O one.
        assert_names(
            &build_pubsub_source(&workload_with_key.source)
                .unwrap_err()
                .message,
            &[
                "--service-account-file",
                "--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ],
        );

        let service_account_without_key = parse_pubsub_create(&[
            "--topic",
            "events",
            "--project-id",
            "project-1",
            "--format",
            "JSONEachRow",
            "--seek-type",
            "earliest",
        ]);
        let message = build_pubsub_source(&service_account_without_key.source)
            .unwrap_err()
            .message;
        assert_names(
            &message,
            &["--auth SERVICE_ACCOUNT", "--service-account-file"],
        );
        assert!(!message.contains("WORKLOAD_IDENTITY"), "{message}");
    }
}
