use crate::cloud::clickpipes::cli::{
    ClickPipeCreateRequestArgs, DestinationRoleArgs, StreamingDestinationTableArgs,
};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_request_args, print_created,
};
use crate::cloud::clickpipes::destination::{
    build_destination_roles, build_streaming_destination, parse_columns, parse_destination_column,
};
use crate::cloud::clickpipes::sources::{AuthenticationValueParser, read_gcp_service_account_file};
use crate::cloud::clickpipes::{ClickPipeSourceKind, parse_enum};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::shared::resolve_org_id;
use clap::Args;
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};

// Valid wire values for each ClickPipe enum the CLI accepts as a string argument.
// Kept in sync with the clickhouse-cloud-api library enums; extra variants are
// rejected by clap at parse time with a `[possible values: …]` hint.
pub(in crate::cloud::clickpipes) const OBJECT_STORAGE_FORMATS: &[&str] = &[
    "JSONEachRow",
    "JSONAsObject",
    "CSV",
    "CSVWithNames",
    "TabSeparated",
    "TabSeparatedWithNames",
    "Parquet",
    "Avro",
];
pub(in crate::cloud::clickpipes) const OBJECT_STORAGE_COMPRESSIONS: &[&str] = &[
    "none", "gzip", "gz", "brotli", "br", "xz", "LZMA", "zstd", "auto",
];
pub(in crate::cloud::clickpipes) const OBJECT_STORAGE_TYPES: &[&str] = &[
    "s3",
    "gcs",
    "dospaces",
    "azureblobstorage",
    "cloudflarer2",
    "ovhobjectstorage",
];

fn parse_object_storage_authentication(
    value: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceAuthentication> {
    use clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceAuthentication as Auth;

    match serde_json::from_value::<Auth>(serde_json::Value::String(value.to_string()))? {
        authentication @ (Auth::IAM_ROLE
        | Auth::IAM_USER
        | Auth::CONNECTION_STRING
        | Auth::SERVICE_ACCOUNT
        | Auth::ServiceAccountWorkloadIdentity) => Ok(authentication),
        Auth::Unknown(value) => Err(CloudError::usage(format!(
            "unknown object-storage authentication method '{value}'"
        ))),
    }
}

pub(in crate::cloud::clickpipes) fn parse_supported_object_storage_auth(
    value: &str,
) -> Result<String, String> {
    parse_object_storage_authentication(value)
        .map(|_| value.to_string())
        .map_err(|error| error.message)
}

#[derive(Args)]
pub struct ObjectStorageSchemaDiscoverArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    #[command(flatten)]
    pub source: ObjectStorageSourceFields,
}

/// Source-connection fields for an object-storage ClickPipe source.
/// Flattened into both `ObjectStorageCreateArgs` (pipe creation) and the
/// schema-discover object-storage subcommand so the source field set has a
/// single definition.
#[derive(Args, Debug)]
pub struct ObjectStorageSourceFields {
    /// Source URL (e.g., https://bucket.s3.region.amazonaws.com/path/*.json)
    #[arg(long)]
    pub source_url: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(OBJECT_STORAGE_FORMATS))]
    pub format: String,

    /// Storage type
    #[arg(
        long,
        default_value = "s3",
        value_parser = PossibleValuesParser::new(OBJECT_STORAGE_TYPES),
    )]
    pub storage_type: String,

    /// Compression
    #[arg(
        long,
        default_value = "auto",
        value_parser = PossibleValuesParser::new(OBJECT_STORAGE_COMPRESSIONS),
    )]
    pub compression: String,

    /// Authentication method
    ///
    /// Inferred from credential flags when omitted; with no credentials, no
    /// authentication is sent. Workload identity is only valid for GCS.
    #[arg(long, value_parser = AuthenticationValueParser { parse: parse_supported_object_storage_auth, values: &["IAM_ROLE", "IAM_USER", "CONNECTION_STRING", "SERVICE_ACCOUNT", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"] })]
    pub auth: Option<String>,

    /// Enable continuous ingestion
    #[arg(long)]
    pub continuous: bool,

    /// SQS queue URL for continuous ingestion notifications
    #[arg(long)]
    pub queue_url: Option<String>,

    /// Skip the initial load and ingest only queue-notification files
    ///
    /// Requires --queue-url.
    #[arg(long, requires = "queue_url")]
    pub skip_initial_load: bool,

    /// Object key to start continuous ingestion after
    ///
    /// Mutually exclusive with --skip-initial-load.
    #[arg(long, conflicts_with = "skip_initial_load")]
    pub start_after: Option<String>,

    /// CSV delimiter character (e.g., ",")
    #[arg(long)]
    pub delimiter: Option<String>,

    /// Leading lines to skip in each file (TabSeparated formats only)
    #[arg(long, value_name = "N")]
    pub skip_first_lines: Option<u32>,

    /// IAM role ARN (selects IAM_ROLE auth)
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID (selects IAM_USER auth; requires --secret-key)
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key (requires --access-key-id)
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Azure connection string (selects CONNECTION_STRING auth)
    #[arg(long)]
    pub connection_string: Option<String>,

    /// Azure container name
    #[arg(long)]
    pub azure_container_name: Option<String>,

    /// Object storage path (for Azure)
    #[arg(long)]
    pub path: Option<String>,

    /// Path to a GCP service account JSON key file, or - to read it from stdin
    #[arg(long, value_name = "PATH")]
    pub service_account_file: Option<String>,
}

#[derive(Args, Debug)]
pub struct ObjectStorageCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub request: ClickPipeCreateRequestArgs,

    #[command(flatten)]
    pub source: ObjectStorageSourceFields,

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

/// Check the object-storage storage-type, authentication and credential-flag
/// rules clap cannot express. No I/O, so it runs before credentials or network
/// access, and again in the builder.
pub(in crate::cloud::clickpipes) fn validate_object_storage_source_args(
    args: &ObjectStorageSourceFields,
) -> CloudResult<(
    clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceType,
    Option<clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceAuthentication>,
)> {
    use clickhouse_cloud_api::models::{
        ClickPipePostObjectStorageSourceAuthentication, ClickPipePostObjectStorageSourceType,
    };

    let source_type: ClickPipePostObjectStorageSourceType = parse_enum(&args.storage_type)?;
    if let ClickPipePostObjectStorageSourceType::Unknown(value) = &source_type {
        return Err(CloudError::usage(format!(
            "unknown object-storage source type '{value}'"
        )));
    }

    let credential_groups = [
        args.iam_role.is_some(),
        args.access_key_id.is_some() || args.secret_key.is_some(),
        args.connection_string.is_some(),
        args.service_account_file.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if credential_groups > 1 {
        return Err(CloudError::usage(
            "object-storage credential flags select more than one authentication method",
        ));
    }

    let authentication = match args.auth.as_deref() {
        Some(value) => Some(parse_object_storage_authentication(value)?),
        None if args.iam_role.is_some() => {
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE)
        }
        None if args.access_key_id.is_some() || args.secret_key.is_some() => {
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER)
        }
        None if args.connection_string.is_some() => {
            Some(ClickPipePostObjectStorageSourceAuthentication::CONNECTION_STRING)
        }
        None if args.service_account_file.is_some() => {
            Some(ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT)
        }
        None => None,
    };

    let has_iam_role = args.iam_role.is_some();
    let has_access_key = args.access_key_id.is_some() && args.secret_key.is_some();
    let has_connection_string = args.connection_string.is_some();
    let has_service_account = args.service_account_file.is_some();
    match authentication.as_ref() {
        Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE) => {
            if source_type != ClickPipePostObjectStorageSourceType::S3 {
                return Err(CloudError::usage(
                    "IAM_ROLE authentication requires --storage-type s3",
                ));
            }
            if !has_iam_role {
                return Err(CloudError::usage("--auth IAM_ROLE requires --iam-role"));
            }
            if credential_groups != 1 {
                return Err(CloudError::usage(
                    "--auth IAM_ROLE accepts only --iam-role credentials",
                ));
            }
        }
        Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER) => {
            if !matches!(
                source_type,
                ClickPipePostObjectStorageSourceType::S3
                    | ClickPipePostObjectStorageSourceType::Gcs
                    | ClickPipePostObjectStorageSourceType::Dospaces
            ) {
                return Err(CloudError::usage(
                    "IAM_USER authentication requires --storage-type s3, gcs, or dospaces",
                ));
            }
            if !has_access_key {
                return Err(CloudError::usage(
                    "--auth IAM_USER requires --access-key-id and --secret-key",
                ));
            }
            if credential_groups != 1 {
                return Err(CloudError::usage(
                    "--auth IAM_USER accepts only access-key credentials",
                ));
            }
        }
        Some(ClickPipePostObjectStorageSourceAuthentication::CONNECTION_STRING) => {
            if source_type != ClickPipePostObjectStorageSourceType::Azureblobstorage {
                return Err(CloudError::usage(
                    "CONNECTION_STRING authentication requires --storage-type azureblobstorage",
                ));
            }
            if !has_connection_string {
                return Err(CloudError::usage(
                    "--auth CONNECTION_STRING requires --connection-string",
                ));
            }
            if credential_groups != 1 {
                return Err(CloudError::usage(
                    "--auth CONNECTION_STRING accepts only --connection-string credentials",
                ));
            }
        }
        Some(ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT) => {
            if source_type != ClickPipePostObjectStorageSourceType::Gcs {
                return Err(CloudError::usage(
                    "SERVICE_ACCOUNT authentication requires --storage-type gcs",
                ));
            }
            if !has_service_account {
                return Err(CloudError::usage(
                    "--auth SERVICE_ACCOUNT requires --service-account-file",
                ));
            }
            if credential_groups != 1 {
                return Err(CloudError::usage(
                    "--auth SERVICE_ACCOUNT accepts only --service-account-file credentials",
                ));
            }
        }
        Some(ClickPipePostObjectStorageSourceAuthentication::ServiceAccountWorkloadIdentity) => {
            if source_type != ClickPipePostObjectStorageSourceType::Gcs {
                return Err(CloudError::usage(
                    "SERVICE_ACCOUNT_WORKLOAD_IDENTITY authentication requires --storage-type gcs",
                ));
            }
            if credential_groups != 0 {
                return Err(CloudError::usage(
                    "--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY cannot be combined with credential flags",
                ));
            }
        }
        Some(ClickPipePostObjectStorageSourceAuthentication::Unknown(value)) => {
            return Err(CloudError::usage(format!(
                "unknown object-storage authentication method '{value}'"
            )));
        }
        None => {}
    }

    Ok((source_type, authentication))
}

/// Build a `ClickPipePostObjectStorageSource` from the CLI args, inferring the
/// authentication mechanism from the credential flags and reading any GCP
/// service-account file up front so bad invocations fail fast before any
/// network call. Shared by the `clickpipe create object-storage` and
/// `clickpipe schema-discover object-storage <SERVICE_ID>` handlers.
fn build_object_storage_source(
    args: &ObjectStorageSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostObjectStorageSource> {
    use clickhouse_cloud_api::models::{
        ClickPipePostObjectStorageSource, ClickPipePostObjectStorageSourceAuthentication,
        MskIamUser,
    };

    let (source_type, authentication) = validate_object_storage_source_args(args)?;
    let has_access_key = args.access_key_id.is_some() && args.secret_key.is_some();
    let access_key = if has_access_key {
        Some(MskIamUser {
            access_key_id: args.access_key_id.clone().unwrap_or_default(),
            secret_key: args.secret_key.clone().unwrap_or_default(),
        })
    } else {
        None
    };
    let service_account_key = match (
        authentication.as_ref(),
        args.service_account_file.as_deref(),
    ) {
        (Some(ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT), Some(path)) => {
            Some(read_gcp_service_account_file(path)?)
        }
        _ => None,
    };

    Ok(ClickPipePostObjectStorageSource {
        r#type: source_type,
        format: parse_enum(&args.format)?,
        url: args.source_url.clone(),
        compression: Some(parse_enum(&args.compression)?),
        is_continuous: if args.continuous { Some(true) } else { None },
        queue_url: args.queue_url.clone(),
        delimiter: args.delimiter.clone(),
        authentication,
        iam_role: args.iam_role.clone(),
        access_key,
        connection_string: args.connection_string.clone(),
        azure_container_name: args.azure_container_name.clone(),
        path: args.path.clone(),
        service_account_key,
        skip_initial_load: if args.skip_initial_load {
            Some(true)
        } else {
            None
        },
        start_after: args.start_after.clone(),
        skip_first_lines: args.skip_first_lines.map(i64::from),
    })
}

pub(in crate::cloud::clickpipes) async fn clickpipe_create_object_storage(
    client: &CloudClient,
    args: &ObjectStorageCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let request_args =
        build_create_request_args(&args.request, ClickPipeSourceKind::ObjectStorage)?;
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_object_storage_source(&args.source)?;
    let destination = build_streaming_destination(
        &args.database,
        &args.table,
        parsed_columns,
        build_destination_roles(&args.destination_roles.roles),
        &args.destination_table,
    )?;
    let org_id = resolve_org_id(client).await?;

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            object_storage: Some(source),
            bigquery: None,
            kafka: None,
            kinesis: None,
            mongodb: None,
            mysql: None,
            postgres: None,
            pubsub: None,
            validate_samples: None,
        },
        destination,
        field_mappings: None,
        scaling: None,
        settings: None,
        start_paused: None,
    };
    apply_create_request_args(&mut request, request_args);

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

/// Build the schema-discovery request body for an object-storage source. The
/// source connection is built by the same helper `clickpipe create
/// object-storage` uses, so discovery and creation send an identical
/// `objectStorage` source; every other source key is left absent.
pub(in crate::cloud::clickpipes) fn build_object_storage_schema_discovery_request(
    args: &ObjectStorageSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeSchemaDiscoveryRequest, ClickPipeSchemaDiscoverySource,
    };

    Ok(ClickPipeSchemaDiscoveryRequest {
        source: ClickPipeSchemaDiscoverySource {
            kafka: None,
            kinesis: None,
            object_storage: Some(build_object_storage_source(args)?),
            pubsub: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::cli::{
        ClickPipeCommands, ClickPipeCreateCommands, ClickPipeSchemaDiscoverCommands,
    };
    use crate::cloud::clickpipes::test_support::{
        assert_names, assert_rejected, parse_clickpipe, parse_object_storage_discovery,
    };

    #[test]
    fn parses_object_storage_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::ObjectStorage(args),
        } = parse_clickpipe(&[
            "create",
            "object-storage",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data/*.csv",
            "--format",
            "CSV",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
            "--column",
            "name:String",
            "--storage-type",
            "gcs",
            "--compression",
            "gzip",
            "--continuous",
            "--queue-url",
            "https://queue.example/q",
            "--start-after",
            "key-1",
            "--delimiter",
            ",",
            "--skip-first-lines",
            "3",
            "--iam-role",
            "arn:role",
            "--access-key-id",
            "access",
            "--secret-key",
            "secret",
            "--connection-string",
            "connection",
            "--azure-container-name",
            "container",
            "--path",
            "path/*.csv",
            "--service-account-file",
            "/tmp/account.json",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected object-storage create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.source.source_url, "https://bucket.example/data/*.csv");
        assert_eq!(args.source.format, "CSV");
        assert_eq!(args.database, "db");
        assert_eq!(args.table, "events");
        assert_eq!(args.columns, ["id:UInt64", "name:String"]);
        assert_eq!(args.source.storage_type, "gcs");
        assert_eq!(args.source.compression, "gzip");
        assert_eq!(args.source.auth, None);
        assert!(args.source.continuous);
        assert_eq!(
            args.source.queue_url.as_deref(),
            Some("https://queue.example/q")
        );
        assert!(!args.source.skip_initial_load);
        assert_eq!(args.source.start_after.as_deref(), Some("key-1"));
        assert_eq!(args.source.delimiter.as_deref(), Some(","));
        assert_eq!(args.source.skip_first_lines, Some(3));
        assert_eq!(args.source.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.source.access_key_id.as_deref(), Some("access"));
        assert_eq!(args.source.secret_key.as_deref(), Some("secret"));
        assert_eq!(args.source.connection_string.as_deref(), Some("connection"));
        assert_eq!(
            args.source.azure_container_name.as_deref(),
            Some("container")
        );
        assert_eq!(args.source.path.as_deref(), Some("path/*.csv"));
        assert_eq!(
            args.source.service_account_file.as_deref(),
            Some("/tmp/account.json")
        );

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::ObjectStorage(args),
        } = parse_clickpipe(&[
            "create",
            "object-storage",
            "--column",
            "id:Int64",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
        ])
        else {
            panic!("expected object-storage create");
        };
        assert_eq!(args.columns, ["id:Int64"]);
        assert_eq!(args.source.storage_type, "s3");
        assert_eq!(args.source.compression, "auto");
        assert_eq!(args.source.auth, None);
        assert!(!args.source.continuous);
        assert_eq!(args.source.queue_url, None);
        assert!(!args.source.skip_initial_load);
        assert_eq!(args.source.start_after, None);
        assert_eq!(args.source.delimiter, None);
        assert_eq!(args.source.skip_first_lines, None);
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id, None);
        assert_eq!(args.source.secret_key, None);
        assert_eq!(args.source.connection_string, None);
        assert_eq!(args.source.azure_container_name, None);
        assert_eq!(args.source.path, None);
        assert_eq!(args.source.service_account_file, None);
    }

    #[test]
    fn parses_object_storage_skip_initial_load_and_constraints() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::ObjectStorage(args),
        } = parse_clickpipe(&[
            "create",
            "object-storage",
            "--column",
            "id:Int64",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
            "--queue-url",
            "https://queue.example/q",
            "--skip-initial-load",
        ])
        else {
            panic!("expected object-storage create");
        };
        assert!(args.source.skip_initial_load);

        let base = [
            "create",
            "object-storage",
            "--column",
            "id:Int64",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
        ];
        let mut missing_queue = base.to_vec();
        missing_queue.push("--skip-initial-load");
        assert_rejected(&missing_queue);

        let mut conflict = base.to_vec();
        conflict.extend([
            "--queue-url",
            "https://queue.example/q",
            "--skip-initial-load",
            "--start-after",
            "key-1",
        ]);
        assert_rejected(&conflict);
    }

    /// `schema-discover object-storage` takes the same source flags as
    /// `create object-storage`, minus the destination options, and keeps the
    /// same `--storage-type`/`--compression` defaults (issue #588).
    #[test]
    fn parses_object_storage_schema_discovery_flags() {
        let ClickPipeCommands::SchemaDiscover {
            command: ClickPipeSchemaDiscoverCommands::ObjectStorage(args),
        } = parse_clickpipe(&[
            "schema-discover",
            "object-storage",
            "--org-id",
            "org-object-storage",
            "svc-object-storage",
            "--source-url",
            "https://bucket.example/data/*.csv",
            "--format",
            "CSV",
            "--storage-type",
            "gcs",
            "--compression",
            "gzip",
            "--continuous",
            "--queue-url",
            "https://queue.example/q",
            "--start-after",
            "key-1",
            "--delimiter",
            ",",
            "--iam-role",
            "arn:role",
            "--access-key-id",
            "access",
            "--secret-key",
            "secret",
            "--connection-string",
            "connection",
            "--azure-container-name",
            "container",
            "--path",
            "path/*.csv",
            "--service-account-file",
            "/tmp/account.json",
        ])
        else {
            panic!("expected object-storage schema discovery");
        };
        assert_eq!(args.service_id, "svc-object-storage");
        let args = args.source;

        assert_eq!(args.source_url, "https://bucket.example/data/*.csv");
        assert_eq!(args.format, "CSV");
        assert_eq!(args.storage_type, "gcs");
        assert_eq!(args.compression, "gzip");
        assert!(args.continuous);
        assert_eq!(args.queue_url.as_deref(), Some("https://queue.example/q"));
        assert!(!args.skip_initial_load);
        assert_eq!(args.start_after.as_deref(), Some("key-1"));
        assert_eq!(args.delimiter.as_deref(), Some(","));
        assert_eq!(args.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.access_key_id.as_deref(), Some("access"));
        assert_eq!(args.secret_key.as_deref(), Some("secret"));
        assert_eq!(args.connection_string.as_deref(), Some("connection"));
        assert_eq!(args.azure_container_name.as_deref(), Some("container"));
        assert_eq!(args.path.as_deref(), Some("path/*.csv"));
        assert_eq!(
            args.service_account_file.as_deref(),
            Some("/tmp/account.json")
        );

        // Only the source connection is required: no --name/--database/--table.
        let ClickPipeCommands::SchemaDiscover {
            command: ClickPipeSchemaDiscoverCommands::ObjectStorage(args),
            ..
        } = parse_clickpipe(&[
            "schema-discover",
            "object-storage",
            "svc-object-storage",
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
        ])
        else {
            panic!("expected object-storage schema discovery");
        };
        let args = args.source;
        assert_eq!(args.storage_type, "s3");
        assert_eq!(args.compression, "auto");
        assert!(!args.continuous);
        assert_eq!(args.queue_url, None);
        assert!(!args.skip_initial_load);
        assert_eq!(args.start_after, None);
        assert_eq!(args.delimiter, None);
        assert_eq!(args.iam_role, None);
        assert_eq!(args.access_key_id, None);
        assert_eq!(args.secret_key, None);
        assert_eq!(args.connection_string, None);
        assert_eq!(args.azure_container_name, None);
        assert_eq!(args.path, None);
        assert_eq!(args.service_account_file, None);

        // --skip-initial-load still requires --queue-url and still conflicts
        // with --start-after, exactly as on `create object-storage`.
        let base = [
            "schema-discover",
            "object-storage",
            "svc-object-storage",
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
        ];
        let mut rejected = base.to_vec();
        rejected.push("--skip-initial-load");
        assert_rejected(&rejected);
        let mut rejected = base.to_vec();
        rejected.extend([
            "--queue-url",
            "https://queue.example/q",
            "--skip-initial-load",
            "--start-after",
            "key-1",
        ]);
        assert_rejected(&rejected);
    }

    #[test]
    fn build_object_storage_schema_discovery_request_minimal() {
        use clickhouse_cloud_api::models::{
            ClickPipePostObjectStorageSourceCompression, ClickPipePostObjectStorageSourceFormat,
            ClickPipePostObjectStorageSourceType,
        };

        let args = parse_object_storage_discovery(&[
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
        ]);
        let request = build_object_storage_schema_discovery_request(&args)
            .expect("minimal object-storage discovery request builds");

        // Only the objectStorage source is populated.
        assert!(request.source.kafka.is_none());
        assert!(request.source.kinesis.is_none());
        assert!(request.source.pubsub.is_none());
        let source = request
            .source
            .object_storage
            .expect("objectStorage source is set");
        assert_eq!(source.url, "https://bucket.example/data/*.json");
        assert_eq!(
            source.format,
            ClickPipePostObjectStorageSourceFormat::JSONEachRow
        );
        assert_eq!(source.r#type, ClickPipePostObjectStorageSourceType::S3);
        assert_eq!(
            source.compression,
            Some(ClickPipePostObjectStorageSourceCompression::Auto)
        );
        assert_eq!(source.authentication, None);
        assert_eq!(source.iam_role, None);
        assert_eq!(source.access_key, None);
        assert_eq!(source.connection_string, None);
        assert_eq!(source.azure_container_name, None);
        assert_eq!(source.path, None);
        assert_eq!(source.service_account_key, None);
        assert_eq!(source.delimiter, None);
        assert_eq!(source.queue_url, None);
        assert_eq!(source.is_continuous, None);
        assert_eq!(source.skip_initial_load, None);
        assert_eq!(source.start_after, None);
        assert_eq!(source.skip_first_lines, None);
    }

    #[test]
    fn build_object_storage_schema_discovery_request_maximal() {
        use clickhouse_cloud_api::models::{
            ClickPipePostObjectStorageSourceAuthentication,
            ClickPipePostObjectStorageSourceCompression, ClickPipePostObjectStorageSourceFormat,
            ClickPipePostObjectStorageSourceType, MskIamUser,
        };
        use std::io::Write;

        let dir = tempfile::tempdir().expect("temp dir");
        let sa_path = dir.path().join("service-account.json");
        let mut sa_file = std::fs::File::create(&sa_path).expect("create service account file");
        sa_file
            .write_all(br#"{"type":"service_account"}"#)
            .expect("write service account file");

        let args = parse_object_storage_discovery(&[
            "--source-url",
            "https://bucket.example/data/*.csv",
            "--format",
            "CSV",
            "--storage-type",
            "gcs",
            "--compression",
            "gzip",
            "--continuous",
            "--queue-url",
            "https://queue.example/q",
            "--start-after",
            "key-1",
            "--delimiter",
            ",",
            "--skip-first-lines",
            "2",
            "--azure-container-name",
            "container",
            "--path",
            "path/*.csv",
            "--service-account-file",
            sa_path.to_str().expect("utf-8 temp path"),
        ]);
        let request = build_object_storage_schema_discovery_request(&args)
            .expect("maximal object-storage discovery request builds");

        assert!(request.source.kafka.is_none());
        assert!(request.source.kinesis.is_none());
        assert!(request.source.pubsub.is_none());
        let source = request
            .source
            .object_storage
            .expect("objectStorage source is set");
        assert_eq!(source.url, "https://bucket.example/data/*.csv");
        assert_eq!(source.format, ClickPipePostObjectStorageSourceFormat::CSV);
        assert_eq!(source.r#type, ClickPipePostObjectStorageSourceType::Gcs);
        assert_eq!(
            source.compression,
            Some(ClickPipePostObjectStorageSourceCompression::Gzip)
        );
        assert_eq!(
            source.authentication,
            Some(ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT)
        );
        assert_eq!(source.iam_role, None);
        assert_eq!(source.access_key, None);
        assert_eq!(source.connection_string, None);
        assert_eq!(source.azure_container_name.as_deref(), Some("container"));
        assert_eq!(source.path.as_deref(), Some("path/*.csv"));
        // The GCP key file is read and base64-encoded, not passed by path.
        assert_eq!(
            source.service_account_key.as_deref(),
            Some("eyJ0eXBlIjoic2VydmljZV9hY2NvdW50In0=")
        );
        assert_eq!(source.delimiter.as_deref(), Some(","));
        assert_eq!(source.queue_url.as_deref(), Some("https://queue.example/q"));
        assert_eq!(source.is_continuous, Some(true));
        assert_eq!(source.skip_initial_load, None);
        assert_eq!(source.start_after.as_deref(), Some("key-1"));
        assert_eq!(source.skip_first_lines, Some(2));

        // Without --iam-role the access key pair infers IAM_USER, and
        // --skip-initial-load is carried through.
        let args = parse_object_storage_discovery(&[
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
            "--access-key-id",
            "access",
            "--secret-key",
            "secret",
            "--queue-url",
            "https://queue.example/q",
            "--skip-initial-load",
        ]);
        let source = build_object_storage_schema_discovery_request(&args)
            .expect("IAM_USER object-storage discovery request builds")
            .source
            .object_storage
            .expect("objectStorage source is set");
        assert_eq!(
            source.authentication,
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER)
        );
        assert_eq!(source.iam_role, None);
        assert_eq!(
            source.access_key,
            Some(MskIamUser {
                access_key_id: "access".to_string(),
                secret_key: "secret".to_string(),
            })
        );
        assert_eq!(source.skip_initial_load, Some(true));
    }

    #[test]
    fn object_storage_workload_identity_builds_for_gcs_without_credentials() {
        let args = parse_object_storage_discovery(&[
            "--source-url",
            "gs://bucket/events/*.json",
            "--format",
            "JSONEachRow",
            "--storage-type",
            "gcs",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
        ]);
        let source = build_object_storage_source(&args).expect("GCS workload identity builds");
        let json = serde_json::to_value(source).expect("source serializes");
        assert_eq!(json["authentication"], "SERVICE_ACCOUNT_WORKLOAD_IDENTITY");
        for field in [
            "iamRole",
            "accessKey",
            "connectionString",
            "serviceAccountKey",
        ] {
            assert!(json.get(field).is_none(), "{field} leaked into {json}");
        }
    }

    #[test]
    fn object_storage_workload_identity_rejects_provider_and_credentials() {
        let wrong_provider = parse_object_storage_discovery(&[
            "--source-url",
            "https://bucket.s3.amazonaws.com/events.json",
            "--format",
            "JSONEachRow",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
        ]);
        assert_names(
            &build_object_storage_source(&wrong_provider)
                .unwrap_err()
                .message,
            &["SERVICE_ACCOUNT_WORKLOAD_IDENTITY", "--storage-type gcs"],
        );

        let with_key = parse_object_storage_discovery(&[
            "--source-url",
            "gs://bucket/events.json",
            "--format",
            "JSONEachRow",
            "--storage-type",
            "gcs",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            "--service-account-file",
            "/missing/must-not-be-read.json",
        ]);
        // Rejected before the key file is read, so the error is not an I/O one.
        assert_names(
            &build_object_storage_source(&with_key).unwrap_err().message,
            &["--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY"],
        );
    }
}
