use crate::cloud::clickpipes::cli::{
    ClickPipeCreateRequestArgs, DestinationRoleArgs, StreamingDestinationTableArgs,
};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_request_args, print_created,
};
use crate::cloud::clickpipes::destination::{
    build_destination_roles, build_streaming_destination, parse_columns, parse_destination_column,
};
use crate::cloud::clickpipes::sources::read_protobuf_schema_file;
use crate::cloud::clickpipes::{ClickPipeSourceKind, parse_enum};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::shared::resolve_org_id;
use clap::Args;
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};

pub(in crate::cloud::clickpipes) const KINESIS_FORMATS: &[&str] =
    &["JSONEachRow", "Avro", "AvroConfluent", "Protobuf"];
pub(in crate::cloud::clickpipes) const KINESIS_AUTHS: &[&str] = &["IAM_ROLE", "IAM_USER"];
pub(in crate::cloud::clickpipes) const KINESIS_ITERATOR_TYPES: &[&str] =
    &["TRIM_HORIZON", "LATEST", "AT_TIMESTAMP"];

#[derive(Args)]
pub struct KinesisSchemaDiscoverArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    #[command(flatten)]
    pub source: KinesisSourceFields,
}

/// Source-connection fields for an Amazon Kinesis ClickPipe source.
/// Flattened into both `KinesisCreateArgs` (pipe creation) and the schema-discover
/// Kinesis subcommand so the source field set has a single definition.
#[derive(Args, Debug)]
pub struct KinesisSourceFields {
    /// Kinesis stream name
    #[arg(long)]
    pub stream_name: String,

    /// AWS region (e.g., us-east-1)
    #[arg(long)]
    pub region: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(KINESIS_FORMATS))]
    pub format: String,

    /// Protobuf schema file or - for stdin; requires --format Protobuf
    ///
    /// Required for Protobuf; accepts .proto source or a binary descriptor set.
    #[arg(long, value_name = "PATH", required_if_eq("format", "Protobuf"))]
    pub protobuf_schema_file: Option<String>,

    /// Authentication method (inferred when omitted)
    ///
    /// A complete access-key pair selects IAM_USER; otherwise IAM_ROLE.
    /// IAM_ROLE cannot be combined with either access-key flag.
    #[arg(
        long,
        value_parser = PossibleValuesParser::new(KINESIS_AUTHS),
    )]
    pub auth: Option<String>,

    /// IAM role ARN (conflicts with access keys and --auth IAM_USER)
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID (requires --secret-key; infers IAM_USER)
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key (requires --access-key-id; infers IAM_USER)
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Iterator type
    #[arg(
        long,
        default_value = "TRIM_HORIZON",
        value_parser = PossibleValuesParser::new(KINESIS_ITERATOR_TYPES),
    )]
    pub iterator_type: String,

    /// Unix timestamp for AT_TIMESTAMP iterator type
    #[arg(long)]
    pub iterator_timestamp: Option<u64>,

    /// Enable enhanced fan-out
    #[arg(long)]
    pub enhanced_fan_out: bool,
}

#[derive(Args, Debug)]
pub struct KinesisCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub request: ClickPipeCreateRequestArgs,

    #[command(flatten)]
    pub source: KinesisSourceFields,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination column as name:type (required, repeatable)
    #[arg(
        long = "column",
        required = true,
        value_name = "NAME:TYPE",
        value_parser = parse_destination_column
    )]
    pub columns: Vec<String>,

    #[command(flatten)]
    pub destination_table: StreamingDestinationTableArgs,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,
}

/// Resolve authentication without reading credentials or contacting Cloud.
fn resolve_kinesis_auth(args: &KinesisSourceFields) -> CloudResult<&str> {
    let has_keys = args.access_key_id.is_some() || args.secret_key.is_some();
    if args.auth.as_deref() == Some("IAM_ROLE") && has_keys {
        return Err(CloudError::usage(
            "--auth IAM_ROLE cannot be combined with --access-key-id or --secret-key; remove the key flags or use IAM_USER",
        ));
    }
    if args.iam_role.is_some() && has_keys {
        return Err(CloudError::usage(
            "--iam-role cannot be combined with --access-key-id or --secret-key; choose role authentication or an access-key pair",
        ));
    }
    if args.auth.as_deref() == Some("IAM_USER") && args.iam_role.is_some() {
        return Err(CloudError::usage(
            "--auth IAM_USER cannot be combined with --iam-role; remove --iam-role or use IAM_ROLE",
        ));
    }
    if args.access_key_id.is_some() != args.secret_key.is_some() {
        return Err(CloudError::usage(
            "--access-key-id and --secret-key must be supplied together",
        ));
    }
    Ok(args
        .auth
        .as_deref()
        .unwrap_or(if has_keys { "IAM_USER" } else { "IAM_ROLE" }))
}

pub(in crate::cloud::clickpipes) fn validate_kinesis_source_args(
    args: &KinesisSourceFields,
) -> CloudResult<&str> {
    if args.format == "Protobuf" && args.protobuf_schema_file.is_none() {
        return Err(CloudError::usage(
            "--format Protobuf requires --protobuf-schema-file",
        ));
    }
    if args.format != "Protobuf" && args.protobuf_schema_file.is_some() {
        return Err(CloudError::usage(
            "--protobuf-schema-file can only be used with --format Protobuf",
        ));
    }
    resolve_kinesis_auth(args)
}

/// Build a `ClickPipePostKinesisSource` from the CLI args. Shared by the
/// `clickpipe create kinesis` and `clickpipe schema-discover kinesis <SERVICE_ID>`
/// handlers.
pub(in crate::cloud::clickpipes) fn build_kinesis_source(
    args: &KinesisSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostKinesisSource> {
    use clickhouse_cloud_api::models::{ClickPipePostKinesisSource, MskIamUser};

    let auth = validate_kinesis_source_args(args)?;
    let access_key = match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
        (Some(access_key_id), Some(secret_key)) => Some(MskIamUser {
            access_key_id: access_key_id.to_string(),
            secret_key: secret_key.to_string(),
        }),
        _ => None,
    };

    Ok(ClickPipePostKinesisSource {
        protobuf_schema: args
            .protobuf_schema_file
            .as_deref()
            .map(read_protobuf_schema_file)
            .transpose()?,
        format: parse_enum(&args.format)?,
        stream_name: args.stream_name.clone(),
        region: args.region.clone(),
        authentication: parse_enum(auth)?,
        iam_role: args.iam_role.clone(),
        access_key,
        schema_registry: None,
        use_enhanced_fan_out: if args.enhanced_fan_out {
            Some(true)
        } else {
            None
        },
        iterator_type: parse_enum(&args.iterator_type)?,
        timestamp: args
            .iterator_timestamp
            .map(|timestamp| {
                i64::try_from(timestamp).map_err(|_| {
                    CloudError::usage(format!("--iterator-timestamp {timestamp} is out of range"))
                })
            })
            .transpose()?,
    })
}

pub(in crate::cloud::clickpipes) async fn clickpipe_create_kinesis(
    client: &CloudClient,
    args: &KinesisCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    let request_args = build_create_request_args(&args.request, ClickPipeSourceKind::Kinesis)?;
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kinesis_source(&args.source)?;

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kinesis: Some(source),
            bigquery: None,
            kafka: None,
            mongodb: None,
            mysql: None,
            object_storage: None,
            postgres: None,
            pubsub: None,
            validate_samples: None,
        },
        destination: build_streaming_destination(
            &args.database,
            &args.table,
            parsed_columns,
            build_destination_roles(&args.destination_roles.roles),
            &args.destination_table,
        )?,
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
    use crate::cloud::clickpipes::cli::{
        ClickPipeCommands, ClickPipeCreateCommands, ClickPipeSchemaDiscoverCommands,
    };
    use crate::cloud::clickpipes::test_support::{
        assert_names, clickpipe_parse_error, parse_clickpipe,
    };

    #[test]
    fn parses_kinesis_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kinesis(args),
        } = parse_clickpipe(&[
            "create",
            "kinesis",
            "svc-1",
            "--name",
            "pipe-1",
            "--stream-name",
            "stream-1",
            "--region",
            "us-east-1",
            "--format",
            "Avro",
            "--auth",
            "IAM_USER",
            "--access-key-id",
            "access",
            "--secret-key",
            "secret",
            "--iterator-type",
            "AT_TIMESTAMP",
            "--iterator-timestamp",
            "1720000000",
            "--enhanced-fan-out",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
            "--column",
            "name:String",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected kinesis create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.source.stream_name, "stream-1");
        assert_eq!(args.source.region, "us-east-1");
        assert_eq!(args.source.format, "Avro");
        assert_eq!(args.source.auth.as_deref(), Some("IAM_USER"));
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id.as_deref(), Some("access"));
        assert_eq!(args.source.secret_key.as_deref(), Some("secret"));
        assert_eq!(args.source.iterator_type, "AT_TIMESTAMP");
        assert_eq!(args.source.iterator_timestamp, Some(1_720_000_000));
        assert!(args.source.enhanced_fan_out);
        assert_eq!(args.database, "db");
        assert_eq!(args.table, "events");
        assert_eq!(args.columns, ["id:UInt64", "name:String"]);

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kinesis(args),
        } = parse_clickpipe(&[
            "create",
            "kinesis",
            "svc-1",
            "--name",
            "pipe-1",
            "--stream-name",
            "stream-1",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
        ])
        else {
            panic!("expected kinesis create");
        };
        assert_eq!(args.source.auth, None);
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id, None);
        assert_eq!(args.source.secret_key, None);
        assert_eq!(args.source.iterator_type, "TRIM_HORIZON");
        assert_eq!(args.source.iterator_timestamp, None);
        assert!(!args.source.enhanced_fan_out);
    }

    #[test]
    fn build_kinesis_source_rejects_out_of_range_iterator_timestamp() {
        let args = KinesisSourceFields {
            protobuf_schema_file: None,
            stream_name: "stream".to_string(),
            region: "us-east-1".to_string(),
            format: "JSONEachRow".to_string(),
            auth: None,
            iam_role: None,
            access_key_id: None,
            secret_key: None,
            iterator_type: "AT_TIMESTAMP".to_string(),
            iterator_timestamp: Some(u64::MAX),
            enhanced_fan_out: false,
        };
        let error = build_kinesis_source(&args).unwrap_err();
        assert_names(
            &error.to_string(),
            &["--iterator-timestamp", &u64::MAX.to_string()],
        );

        let args = KinesisSourceFields {
            iterator_timestamp: Some(1_750_000_000),
            ..args
        };
        let source = build_kinesis_source(&args).unwrap();
        assert_eq!(source.timestamp, Some(1_750_000_000));
    }

    #[test]
    fn kinesis_protobuf_parsing_and_builder_cover_create_and_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let schema = dir.path().join("schema.proto");
        std::fs::write(&schema, b"syntax = \"proto3\"; message Event {}").unwrap();
        for operation in ["create", "schema-discover"] {
            let mut args = vec![
                operation,
                "kinesis",
                "svc-1",
                "--stream-name",
                "stream",
                "--region",
                "us-east-1",
                "--format",
                "Protobuf",
                "--protobuf-schema-file",
                schema.to_str().unwrap(),
            ];
            if operation == "create" {
                args.extend([
                    "--name",
                    "pipe",
                    "--database",
                    "default",
                    "--table",
                    "events",
                    "--column",
                    "id:UInt64",
                ]);
            }
            let source_args = match parse_clickpipe(&args) {
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::Kinesis(args),
                } => args.source,
                ClickPipeCommands::SchemaDiscover {
                    command: ClickPipeSchemaDiscoverCommands::Kinesis(args),
                } => args.source,
                _ => panic!("Kinesis"),
            };
            assert_eq!(source_args.protobuf_schema_file.as_deref(), schema.to_str());
            let source = build_kinesis_source(&source_args).unwrap();
            assert_eq!(source.format.to_string(), "Protobuf");
            assert_eq!(
                source.protobuf_schema.as_deref(),
                Some("c3ludGF4ID0gInByb3RvMyI7IG1lc3NhZ2UgRXZlbnQge30=")
            );
            assert_eq!(source.authentication.to_string(), "IAM_ROLE");
        }
    }

    #[test]
    fn kinesis_protobuf_schema_is_required_at_parse_time() {
        for operation in ["create", "schema-discover"] {
            let mut args = vec![
                operation,
                "kinesis",
                "svc-1",
                "--stream-name",
                "stream",
                "--region",
                "us-east-1",
                "--format",
                "Protobuf",
            ];
            if operation == "create" {
                args.extend([
                    "--name",
                    "pipe",
                    "--database",
                    "default",
                    "--table",
                    "events",
                    "--column",
                    "id:UInt64",
                ]);
            }
            assert_eq!(
                clickpipe_parse_error(&args).kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }
    }

    #[test]
    fn kinesis_protobuf_rejects_missing_incompatible_and_invalid_files() {
        use crate::cloud::client::CloudErrorKind;
        let mut args = parsed_kinesis_source("create", &[]);
        assert!(
            build_kinesis_source(&args)
                .unwrap()
                .protobuf_schema
                .is_none()
        );
        args.protobuf_schema_file = Some("/missing/schema.proto".into());
        assert_eq!(
            build_kinesis_source(&args).unwrap_err().kind,
            CloudErrorKind::Usage
        );
        args.format = "Protobuf".into();
        let error = build_kinesis_source(&args).unwrap_err();
        assert_eq!(error.kind, CloudErrorKind::Generic);
        assert_eq!(error.failure.unwrap().kind, crate::failure::FailureKind::Io);
        args.protobuf_schema_file = None;
        assert_eq!(
            build_kinesis_source(&args).unwrap_err().kind,
            CloudErrorKind::Usage
        );
        let dir = tempfile::tempdir().unwrap();
        let schema = dir.path().join("schema.proto");
        args.protobuf_schema_file = Some(schema.to_string_lossy().into_owned());
        std::fs::write(&schema, []).unwrap();
        assert_eq!(
            build_kinesis_source(&args).unwrap_err().kind,
            CloudErrorKind::Usage
        );
    }

    fn parsed_kinesis_source(operation: &str, flags: &[&str]) -> KinesisSourceFields {
        let mut args = vec![
            operation,
            "kinesis",
            "svc-1",
            "--stream-name",
            "stream",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
        ];
        if operation == "create" {
            args.extend([
                "--name",
                "pipe",
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:UInt64",
            ]);
        }
        args.extend(flags);
        match parse_clickpipe(&args) {
            ClickPipeCommands::Create {
                command: ClickPipeCreateCommands::Kinesis(args),
            } => args.source,
            ClickPipeCommands::SchemaDiscover {
                command: ClickPipeSchemaDiscoverCommands::Kinesis(args),
            } => args.source,
            _ => panic!("expected Kinesis source"),
        }
    }

    #[test]
    fn kinesis_auth_parsing_and_builder_inference_agree_for_both_operations() {
        for operation in ["create", "schema-discover"] {
            for (flags, explicit, expected, role, keys) in [
                (vec![], None, "IAM_ROLE", None, false),
                (
                    vec!["--iam-role", "arn:role"],
                    None,
                    "IAM_ROLE",
                    Some("arn:role"),
                    false,
                ),
                (
                    vec!["--auth", "IAM_ROLE"],
                    Some("IAM_ROLE"),
                    "IAM_ROLE",
                    None,
                    false,
                ),
                (
                    vec!["--access-key-id", "access", "--secret-key", "secret"],
                    None,
                    "IAM_USER",
                    None,
                    true,
                ),
                (
                    vec![
                        "--auth",
                        "IAM_USER",
                        "--access-key-id",
                        "access",
                        "--secret-key",
                        "secret",
                    ],
                    Some("IAM_USER"),
                    "IAM_USER",
                    None,
                    true,
                ),
            ] {
                let args = parsed_kinesis_source(operation, &flags);
                assert_eq!(args.auth.as_deref(), explicit);
                assert_eq!(args.iam_role.as_deref(), role);
                assert_eq!(args.access_key_id.as_deref(), keys.then_some("access"));
                assert_eq!(args.secret_key.as_deref(), keys.then_some("secret"));
                let source = build_kinesis_source(&args).unwrap();
                assert_eq!(source.authentication.to_string(), expected);
                assert_eq!(source.iam_role.as_deref(), role);
                assert_eq!(
                    source
                        .access_key
                        .as_ref()
                        .map(|key| key.access_key_id.as_str()),
                    keys.then_some("access")
                );
                assert_eq!(
                    source
                        .access_key
                        .as_ref()
                        .map(|key| key.secret_key.as_str()),
                    keys.then_some("secret")
                );
            }
        }
    }

    #[test]
    fn build_kinesis_source_rejects_conflicts_and_incomplete_pairs() {
        for (auth, role, access, secret) in [
            (Some("IAM_ROLE"), false, true, true),
            (Some("IAM_ROLE"), false, true, false),
            (Some("IAM_ROLE"), false, false, true),
            (None, true, true, true),
            (None, true, true, false),
            (None, true, false, true),
            (Some("IAM_USER"), true, false, false),
            (Some("IAM_USER"), true, true, true),
            (None, false, true, false),
            (None, false, false, true),
            (Some("IAM_USER"), false, true, false),
            (Some("IAM_USER"), false, false, true),
        ] {
            let mut args = parsed_kinesis_source("create", &[]);
            args.auth = auth.map(str::to_string);
            args.iam_role = role.then(|| "private-role-arn".into());
            args.access_key_id = access.then(|| "private-access-id".into());
            args.secret_key = secret.then(|| "private-secret-key".into());
            let error = build_kinesis_source(&args).unwrap_err();
            for value in [
                "private-role-arn",
                "private-access-id",
                "private-secret-key",
            ] {
                assert!(!error.message.contains(value));
            }
        }
    }

    #[test]
    fn build_kinesis_source_supports_minimal_fields() {
        let args = KinesisSourceFields {
            protobuf_schema_file: None,
            stream_name: "stream".into(),
            region: "us-east-1".into(),
            format: "JSONEachRow".into(),
            auth: None,
            iam_role: None,
            access_key_id: None,
            secret_key: None,
            iterator_type: "TRIM_HORIZON".into(),
            iterator_timestamp: None,
            enhanced_fan_out: false,
        };

        let source = build_kinesis_source(&args).unwrap();
        assert_eq!(source.stream_name, "stream");
        assert_eq!(source.region, "us-east-1");
        assert_eq!(source.format.to_string(), "JSONEachRow");
        assert_eq!(source.authentication.to_string(), "IAM_ROLE");
        assert_eq!(source.iterator_type.to_string(), "TRIM_HORIZON");
        assert_eq!(source.iam_role, None);
        assert_eq!(source.access_key, None);
        assert_eq!(source.timestamp, None);
        assert_eq!(source.use_enhanced_fan_out, None);
    }

    #[test]
    fn build_kinesis_source_supports_maximal_fields() {
        let args = KinesisSourceFields {
            protobuf_schema_file: None,
            stream_name: "stream".into(),
            region: "us-east-1".into(),
            format: "AvroConfluent".into(),
            auth: Some("IAM_USER".into()),
            iam_role: None,
            access_key_id: Some("access".into()),
            secret_key: Some("secret".into()),
            iterator_type: "AT_TIMESTAMP".into(),
            iterator_timestamp: Some(1_750_000_000),
            enhanced_fan_out: true,
        };

        let source = build_kinesis_source(&args).unwrap();
        assert_eq!(source.stream_name, "stream");
        assert_eq!(source.region, "us-east-1");
        assert_eq!(source.format.to_string(), "AvroConfluent");
        assert_eq!(source.authentication.to_string(), "IAM_USER");
        assert_eq!(source.iterator_type.to_string(), "AT_TIMESTAMP");
        assert_eq!(source.iam_role, None);
        let access_key = source.access_key.expect("access key is populated");
        assert_eq!(access_key.access_key_id, "access");
        assert_eq!(access_key.secret_key, "secret");
        assert_eq!(source.timestamp, Some(1_750_000_000));
        assert_eq!(source.use_enhanced_fan_out, Some(true));
    }
}
