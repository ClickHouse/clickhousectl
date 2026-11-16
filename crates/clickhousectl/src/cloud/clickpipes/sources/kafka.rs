use crate::cloud::clickpipes::ClickPipeSourceKind;
use crate::cloud::clickpipes::cli::{ClickPipeCreateRequestArgs, StreamingDestinationArgs};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_request_args, print_created,
};
use crate::cloud::clickpipes::sources::{AuthenticationValueParser, read_protobuf_schema_file};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult, read_flag_file};
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};
use clap::{ArgGroup, Args};
use clickhouse_cloud_api::models::{
    ClickPipeKafkaOffsetStrategy, ClickPipePostKafkaSourceAuthentication,
    ClickPipePostKafkaSourceFormat, ClickPipePostKafkaSourceType,
};

pub(in crate::cloud::clickpipes) fn parse_kafka_authentication(
    value: &str,
) -> CloudResult<ClickPipePostKafkaSourceAuthentication> {
    let authentication = parse_serde_enum(
        value,
        "Kafka authentication",
        ClickPipePostKafkaSourceAuthentication::VALUES,
    )?;
    match &authentication {
        ClickPipePostKafkaSourceAuthentication::PLAIN
        | ClickPipePostKafkaSourceAuthentication::SCRAM_SHA_256
        | ClickPipePostKafkaSourceAuthentication::SCRAM_SHA_512
        | ClickPipePostKafkaSourceAuthentication::IAM_ROLE
        | ClickPipePostKafkaSourceAuthentication::IAM_USER
        | ClickPipePostKafkaSourceAuthentication::MUTUAL_TLS
        | ClickPipePostKafkaSourceAuthentication::ServiceAccountWorkloadIdentity => {
            Ok(authentication)
        }
        ClickPipePostKafkaSourceAuthentication::Unknown(value) => Err(CloudError::usage(format!(
            "unknown Kafka authentication method '{value}'"
        ))),
    }
}

pub(in crate::cloud::clickpipes) fn parse_supported_kafka_auth(
    value: &str,
) -> Result<String, String> {
    parse_kafka_authentication(value)
        .map(|_| value.to_string())
        .map_err(|error| error.message)
}

#[derive(Args)]
pub struct KafkaSchemaDiscoverArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    #[command(flatten)]
    pub source: KafkaSourceFields,
}

/// Glue-only schema registry flags; each conflicts with the Confluent flags.
const KAFKA_GLUE_SCHEMA_REGISTRY_ARGS: [&str; 3] =
    ["glue_region", "glue_registry_name", "glue_role_arn"];

/// Source-connection fields for a Kafka / Kafka-compatible ClickPipe source.
/// Flattened into both `KafkaCreateArgs` (pipe creation) and the schema-discover
/// Kafka subcommand so the source field set has a single definition.
///
/// The API requires authentication on every Kafka source, so `--auth` or a
/// credential flag that selects a mechanism is a required "at least one of".
#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("kafka_authentication")
        .required(true)
        .multiple(true)
        .args([
            "auth",
            "event_hubs_connection_string",
            "username",
            "iam_role",
            "access_key_id",
            "client_certificate",
        ])
))]
pub struct KafkaSourceFields {
    /// Kafka broker(s) (e.g., "broker1:9092,broker2:9092")
    #[arg(long)]
    pub brokers: String,

    /// Topic(s) to consume from
    #[arg(long)]
    pub topics: String,

    /// Data format
    #[arg(
        long,
        value_parser = PossibleValuesParser::new(ClickPipePostKafkaSourceFormat::VALUES)
    )]
    pub format: String,

    /// Kafka type
    #[arg(
        long,
        default_value = "kafka",
        value_parser = PossibleValuesParser::new(ClickPipePostKafkaSourceType::VALUES),
    )]
    pub kafka_type: String,

    /// Consumer group name
    #[arg(long)]
    pub consumer_group: Option<String>,

    /// Authentication method
    ///
    /// Required unless credential flags are given, which then select it.
    #[arg(long, value_parser = AuthenticationValueParser { parse: parse_supported_kafka_auth, values: ClickPipePostKafkaSourceAuthentication::VALUES })]
    pub auth: Option<String>,

    /// Azure Event Hubs connection string
    #[arg(
        long,
        value_name = "CONNECTION_STRING",
        conflicts_with_all = [
            "username",
            "password",
            "iam_role",
            "access_key_id",
            "secret_key",
            "client_certificate",
            "client_key"
        ]
    )]
    pub event_hubs_connection_string: Option<String>,

    /// Username for PLAIN/SCRAM authentication
    #[arg(long, requires = "password")]
    pub username: Option<String>,

    /// Password for PLAIN/SCRAM authentication
    #[arg(long, requires = "username")]
    pub password: Option<String>,

    /// IAM role ARN for MSK IAM authentication
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID for IAM_USER authentication
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key for IAM_USER authentication
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Offset strategy
    #[arg(
        long,
        default_value = "from_beginning",
        value_parser = PossibleValuesParser::new(ClickPipeKafkaOffsetStrategy::VALUES),
    )]
    pub offset: String,

    /// Timestamp for from_timestamp offset (e.g., "2021-01-01T00:00")
    #[arg(long)]
    pub offset_timestamp: Option<String>,

    /// Schema registry type; confluent when omitted
    #[arg(long, value_name = "TYPE", value_parser = ["confluent", "glue"])]
    pub schema_registry_type: Option<String>,

    /// Schema registry URL (for Avro/Protobuf formats; confluent only)
    #[arg(
        long,
        required_if_eq("schema_registry_type", "confluent"),
        conflicts_with_all = KAFKA_GLUE_SCHEMA_REGISTRY_ARGS
    )]
    pub schema_registry_url: Option<String>,

    /// Schema registry username (confluent only)
    #[arg(long, conflicts_with_all = KAFKA_GLUE_SCHEMA_REGISTRY_ARGS)]
    pub schema_registry_username: Option<String>,

    /// Schema registry password (confluent only)
    #[arg(long, conflicts_with_all = KAFKA_GLUE_SCHEMA_REGISTRY_ARGS)]
    pub schema_registry_password: Option<String>,

    /// Glue registry AWS region (only with --schema-registry-type glue)
    #[arg(
        long,
        value_name = "REGION",
        requires = "schema_registry_type",
        required_if_eq("schema_registry_type", "glue")
    )]
    pub glue_region: Option<String>,

    /// Glue registry name (only with --schema-registry-type glue)
    #[arg(
        long,
        value_name = "NAME",
        requires = "schema_registry_type",
        required_if_eq("schema_registry_type", "glue")
    )]
    pub glue_registry_name: Option<String>,

    /// IAM role ARN for Glue access (only with --schema-registry-type glue)
    ///
    /// Defaults to --iam-role, so it is required when --iam-role is not set.
    #[arg(long, value_name = "ARN", requires = "schema_registry_type")]
    pub glue_role_arn: Option<String>,

    /// Path to a .proto file or FileDescriptorSet, or - to read stdin
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = [
            "schema_registry_type",
            "schema_registry_url",
            "schema_registry_username",
            "schema_registry_password",
            "schema_registry_ca_certificate",
            "glue_region",
            "glue_registry_name",
            "glue_role_arn"
        ]
    )]
    pub protobuf_schema_file: Option<String>,

    /// Path to a PEM CA bundle for the broker
    #[arg(long, value_name = "PATH")]
    pub ca_certificate: Option<String>,

    /// Path to client certificate file (for MUTUAL_TLS auth; requires --client-key)
    #[arg(long, requires = "client_key")]
    pub client_certificate: Option<String>,

    /// Path to client private key file (for MUTUAL_TLS auth; requires --client-certificate)
    #[arg(long, requires = "client_certificate")]
    pub client_key: Option<String>,

    /// Path to schema registry CA certificate file (confluent only)
    #[arg(long, conflicts_with_all = KAFKA_GLUE_SCHEMA_REGISTRY_ARGS)]
    pub schema_registry_ca_certificate: Option<String>,

    /// Reverse private endpoint IDs (repeatable)
    #[arg(long = "reverse-private-endpoint-id")]
    pub reverse_private_endpoint_ids: Vec<String>,
}

#[derive(Args, Debug)]
pub struct KafkaCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub request: ClickPipeCreateRequestArgs,

    #[command(flatten)]
    pub source: KafkaSourceFields,

    /// Enable exactly-once delivery
    #[arg(long, value_name = "true|false")]
    pub exactly_once: Option<bool>,

    #[command(flatten)]
    pub destination: StreamingDestinationArgs,
}

/// Infer the Kafka authentication mechanism from the credential flags that were
/// passed when `--auth` is omitted. Returning `None` means no complete
/// credential set was given; the caller rejects that, since the API requires
/// authentication. Never default to a mechanism the user did not ask for.
fn infer_kafka_authentication(
    args: &KafkaSourceFields,
) -> Option<clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
    if args.event_hubs_connection_string.is_some()
        || (args.username.is_some() && args.password.is_some())
    {
        Some(Auth::PLAIN)
    } else if args.access_key_id.is_some() && args.secret_key.is_some() {
        Some(Auth::IAM_USER)
    } else if args.iam_role.is_some() {
        Some(Auth::IAM_ROLE)
    } else if args.client_certificate.is_some() && args.client_key.is_some() {
        Some(Auth::MUTUAL_TLS)
    } else {
        None
    }
}

/// Build the Kafka `credentials` JSON body, whose shape is a `oneOf` determined
/// by the auth mode (see the `ClickPipePostKafkaSource.credentials` schema).
/// IAM_ROLE sends a null body — the role ARN flows through the separate
/// top-level `iamRole` field on the source, not through credentials.
///
/// `mtls_contents` is the pre-read (certificate, privateKey) PEM bundle used
/// only for MUTUAL_TLS; the caller reads these from disk so this function
/// stays pure and testable.
fn build_kafka_credentials(
    authentication: &clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication,
    args: &KafkaSourceFields,
    mtls_contents: Option<(String, String)>,
) -> CloudResult<serde_json::Value> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
    match authentication {
        Auth::PLAIN | Auth::SCRAM_SHA_256 | Auth::SCRAM_SHA_512 => {
            if let Some(connection_string) = args.event_hubs_connection_string.as_deref() {
                return Ok(serde_json::json!({ "connectionString": connection_string }));
            }
            match (args.username.as_deref(), args.password.as_deref()) {
                (Some(username), Some(password)) => {
                    Ok(serde_json::json!({ "username": username, "password": password }))
                }
                _ => Err(CloudError::usage(format!(
                    "{authentication} requires --username and --password"
                ))),
            }
        }
        Auth::IAM_USER => match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
            (Some(access_key_id), Some(secret_key)) => Ok(serde_json::json!({
                "accessKeyId": access_key_id,
                "secretKey": secret_key
            })),
            _ => Err(CloudError::usage(
                "IAM_USER requires --access-key-id and --secret-key",
            )),
        },
        Auth::IAM_ROLE => {
            if args.iam_role.is_none() {
                Err(CloudError::usage("IAM_ROLE requires --iam-role"))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Auth::MUTUAL_TLS => match mtls_contents {
            Some((certificate, private_key)) => Ok(serde_json::json!({
                "certificate": certificate,
                "privateKey": private_key
            })),
            None => Err(CloudError::usage(
                "MUTUAL_TLS requires --client-certificate and --client-key",
            )),
        },
        Auth::ServiceAccountWorkloadIdentity => Ok(serde_json::Value::Null),
        Auth::Unknown(value) => Err(CloudError::usage(format!(
            "unknown Kafka authentication method '{value}'"
        ))),
    }
}

pub(in crate::cloud::clickpipes) fn validate_kafka_source_args(
    args: &KafkaSourceFields,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;

    let source_type: ClickPipePostKafkaSourceType = parse_serde_enum(
        &args.kafka_type,
        "Kafka source type",
        ClickPipePostKafkaSourceType::VALUES,
    )?;
    let format: ClickPipePostKafkaSourceFormat = parse_serde_enum(
        &args.format,
        "Kafka format",
        ClickPipePostKafkaSourceFormat::VALUES,
    )?;

    if args.protobuf_schema_file.is_some() && format != ClickPipePostKafkaSourceFormat::Protobuf {
        return Err(CloudError::usage(
            "--protobuf-schema-file can only be used with --format Protobuf",
        ));
    }
    if args.protobuf_schema_file.is_some()
        && (has_confluent_registry_flags(args)
            || has_glue_registry_flags(args)
            || args.schema_registry_type.is_some())
    {
        return Err(CloudError::usage(
            "--protobuf-schema-file cannot be combined with schema registry flags",
        ));
    }
    validate_kafka_schema_registry_args(args)?;

    if args.event_hubs_connection_string.is_some()
        && source_type != ClickPipePostKafkaSourceType::Azureeventhub
    {
        return Err(CloudError::usage(
            "--event-hubs-connection-string requires --kafka-type azureeventhub",
        ));
    }
    if args.event_hubs_connection_string.is_some()
        && args
            .auth
            .as_deref()
            .is_some_and(|authentication| authentication != "PLAIN")
    {
        return Err(CloudError::usage(
            "--event-hubs-connection-string supports only --auth PLAIN",
        ));
    }
    if args.event_hubs_connection_string.is_some()
        && (args.username.is_some()
            || args.password.is_some()
            || args.iam_role.is_some()
            || args.access_key_id.is_some()
            || args.secret_key.is_some()
            || args.client_certificate.is_some()
            || args.client_key.is_some())
    {
        return Err(CloudError::usage(
            "--event-hubs-connection-string cannot be combined with other broker credentials",
        ));
    }

    let credential_groups = [
        args.event_hubs_connection_string.is_some(),
        args.username.is_some() || args.password.is_some(),
        args.iam_role.is_some(),
        args.access_key_id.is_some() || args.secret_key.is_some(),
        args.client_certificate.is_some() || args.client_key.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();

    let authentication = args
        .auth
        .as_deref()
        .map(parse_kafka_authentication)
        .transpose()?;
    match authentication.as_ref() {
        Some(Auth::ServiceAccountWorkloadIdentity) => {
            if source_type != ClickPipePostKafkaSourceType::Gcmk {
                return Err(CloudError::usage(
                    "SERVICE_ACCOUNT_WORKLOAD_IDENTITY authentication requires --kafka-type gcmk",
                ));
            }
            if credential_groups != 0 {
                return Err(CloudError::usage(
                    "--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY cannot be combined with broker credential flags",
                ));
            }
        }
        Some(Auth::PLAIN | Auth::SCRAM_SHA_256 | Auth::SCRAM_SHA_512) => {
            let expected_groups = usize::from(args.event_hubs_connection_string.is_some())
                + usize::from(args.username.is_some() || args.password.is_some());
            if credential_groups != expected_groups {
                return Err(CloudError::usage(format!(
                    "--auth {} cannot be combined with IAM or mutual-TLS credential flags",
                    args.auth.as_deref().unwrap_or_default()
                )));
            }
        }
        Some(Auth::IAM_ROLE) => {
            if credential_groups != usize::from(args.iam_role.is_some()) {
                return Err(CloudError::usage(
                    "--auth IAM_ROLE accepts only --iam-role credentials",
                ));
            }
        }
        Some(Auth::IAM_USER) => {
            if credential_groups
                != usize::from(args.access_key_id.is_some() || args.secret_key.is_some())
            {
                return Err(CloudError::usage(
                    "--auth IAM_USER accepts only access-key credentials",
                ));
            }
        }
        Some(Auth::MUTUAL_TLS) => {
            if credential_groups
                != usize::from(args.client_certificate.is_some() || args.client_key.is_some())
            {
                return Err(CloudError::usage(
                    "--auth MUTUAL_TLS accepts only client-certificate credentials",
                ));
            }
        }
        Some(Auth::Unknown(value)) => {
            return Err(CloudError::usage(format!(
                "unknown Kafka authentication method '{value}'"
            )));
        }
        None if credential_groups > 1 => {
            return Err(CloudError::usage(
                "Kafka broker credential flags select more than one authentication method",
            ));
        }
        None => {}
    }

    Ok(())
}

fn has_confluent_registry_flags(args: &KafkaSourceFields) -> bool {
    args.schema_registry_url.is_some()
        || args.schema_registry_username.is_some()
        || args.schema_registry_password.is_some()
        || args.schema_registry_ca_certificate.is_some()
}

fn has_glue_registry_flags(args: &KafkaSourceFields) -> bool {
    args.glue_region.is_some() || args.glue_registry_name.is_some() || args.glue_role_arn.is_some()
}

/// Check the schema-registry flag combination clap cannot express on its own.
/// Runs before credentials or network access, and again in the builder.
fn validate_kafka_schema_registry_args(args: &KafkaSourceFields) -> CloudResult<()> {
    match args.schema_registry_type.as_deref() {
        Some("glue") => {
            if has_confluent_registry_flags(args) {
                return Err(CloudError::usage(
                    "--schema-registry-type glue cannot be combined with --schema-registry-url, \
                     --schema-registry-username, --schema-registry-password or \
                     --schema-registry-ca-certificate",
                ));
            }
            if args.glue_region.is_none() || args.glue_registry_name.is_none() {
                return Err(CloudError::usage(
                    "--schema-registry-type glue requires --glue-region and --glue-registry-name",
                ));
            }
            if args.glue_role_arn.is_none() && args.iam_role.is_none() {
                return Err(CloudError::usage(
                    "--schema-registry-type glue requires --glue-role-arn when the source does not \
                     authenticate with --iam-role",
                ));
            }
        }
        explicit => {
            if has_glue_registry_flags(args) {
                return Err(CloudError::usage(
                    "--glue-region, --glue-registry-name and --glue-role-arn require \
                     --schema-registry-type glue",
                ));
            }
            if explicit.is_some() && args.schema_registry_url.is_none() {
                return Err(CloudError::usage(
                    "--schema-registry-type confluent requires --schema-registry-url",
                ));
            }
        }
    }
    Ok(())
}

/// Build the Kafka source's schema registry from validated flags; `None` when
/// no registry was requested.
fn build_kafka_schema_registry(
    args: &KafkaSourceFields,
) -> CloudResult<Option<clickhouse_cloud_api::models::ClickPipeMutateKafkaSchemaRegistry>> {
    use clickhouse_cloud_api::models::{
        ClickPipeKafkaGlueSchemaRegistry, ClickPipeKafkaGlueSchemaRegistryType,
        ClickPipeKafkaSchemaRegistryCredentials, ClickPipeMutateKafkaConfluentSchemaRegistry,
        ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication,
        ClickPipeMutateKafkaConfluentSchemaRegistryType,
    };

    if args.schema_registry_type.as_deref() == Some("glue") {
        let (Some(glue_region), Some(glue_registry_name)) =
            (args.glue_region.as_ref(), args.glue_registry_name.as_ref())
        else {
            return Err(CloudError::usage(
                "--schema-registry-type glue requires --glue-region and --glue-registry-name",
            ));
        };
        return Ok(Some(
            ClickPipeKafkaGlueSchemaRegistry {
                r#type: ClickPipeKafkaGlueSchemaRegistryType::Glue,
                glue_region: glue_region.clone(),
                glue_registry_name: glue_registry_name.clone(),
                glue_role_arn: args.glue_role_arn.clone(),
            }
            .into(),
        ));
    }

    let Some(url) = args.schema_registry_url.as_ref() else {
        return Ok(None);
    };
    let credentials = match (
        args.schema_registry_username.as_deref(),
        args.schema_registry_password.as_deref(),
    ) {
        (Some(username), Some(password)) => ClickPipeKafkaSchemaRegistryCredentials {
            username: username.to_string(),
            password: password.to_string(),
        },
        // The API requires the credentials object; empty strings are what an
        // unauthenticated registry has always been sent.
        _ => ClickPipeKafkaSchemaRegistryCredentials {
            username: String::new(),
            password: String::new(),
        },
    };
    let ca_certificate = args
        .schema_registry_ca_certificate
        .as_deref()
        .map(|path| read_flag_file("--schema-registry-ca-certificate", path))
        .transpose()?;
    Ok(Some(
        ClickPipeMutateKafkaConfluentSchemaRegistry {
            // Send `type` only when the user named it; the API defaults to confluent.
            r#type: args
                .schema_registry_type
                .as_ref()
                .map(|_| ClickPipeMutateKafkaConfluentSchemaRegistryType::Confluent),
            url: url.clone(),
            // PLAIN is the only registry authentication the API defines.
            authentication: ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication::PLAIN,
            credentials,
            ca_certificate,
        }
        .into(),
    ))
}

/// Build a `ClickPipePostKafkaSource` from the CLI args, performing all
/// authentication/credential/schema-registry/CA validation up front so bad
/// invocations fail fast before any network call. Shared by the
/// `clickpipe create kafka` and `clickpipe schema-discover kafka <SERVICE_ID>`
/// handlers.
fn build_kafka_source_with_exactly_once(
    args: &KafkaSourceFields,
    exactly_once: Option<bool>,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostKafkaSource> {
    use clickhouse_cloud_api::models::{
        ClickPipeKafkaOffset, ClickPipePostKafkaSource, ClickPipePostKafkaSourceAuthentication,
    };

    validate_kafka_source_args(args)?;

    // An explicit `--auth` wins; otherwise infer the mechanism from the
    // credential flags. The API requires authentication, and clap requires
    // `--auth` or a credential flag, so there is no unauthenticated source.
    let authentication: ClickPipePostKafkaSourceAuthentication = match args.auth.as_deref() {
        Some(authentication) => parse_kafka_authentication(authentication)?,
        None => infer_kafka_authentication(args).ok_or_else(|| {
            CloudError::usage("Kafka ClickPipes require --auth or broker credential flags")
        })?,
    };

    let mtls_cert_contents = match (
        &authentication,
        args.client_certificate.as_deref(),
        args.client_key.as_deref(),
    ) {
        (ClickPipePostKafkaSourceAuthentication::MUTUAL_TLS, Some(cert_path), Some(key_path)) => {
            Some((
                read_flag_file("--client-certificate", cert_path)?,
                read_flag_file("--client-key", key_path)?,
            ))
        }
        _ => None,
    };
    let credentials = build_kafka_credentials(&authentication, args, mtls_cert_contents)?;

    let schema_registry = build_kafka_schema_registry(args)?;

    let ca_certificate = args
        .ca_certificate
        .as_deref()
        .map(|path| read_flag_file("--ca-certificate", path))
        .transpose()?;
    let protobuf_schema = args
        .protobuf_schema_file
        .as_deref()
        .map(read_protobuf_schema_file)
        .transpose()?;

    Ok(ClickPipePostKafkaSource {
        r#type: parse_serde_enum(
            &args.kafka_type,
            "Kafka source type",
            ClickPipePostKafkaSourceType::VALUES,
        )?,
        format: parse_serde_enum(
            &args.format,
            "Kafka format",
            ClickPipePostKafkaSourceFormat::VALUES,
        )?,
        brokers: args.brokers.clone(),
        topics: args.topics.clone(),
        consumer_group: args.consumer_group.clone(),
        exactly_once,
        authentication,
        // Always sent, as `null` for IAM_ROLE and workload identity.
        credentials: Some(credentials),
        iam_role: args.iam_role.clone(),
        offset: Some(ClickPipeKafkaOffset {
            strategy: parse_serde_enum(
                &args.offset,
                "Kafka offset strategy",
                ClickPipeKafkaOffsetStrategy::VALUES,
            )?,
            timestamp: args.offset_timestamp.clone(),
        }),
        schema_registry,
        protobuf_schema,
        tombstone_mode: None,
        ca_certificate,
        // Always sent, as `[]` when no endpoint was given.
        reverse_private_endpoint_ids: Some(args.reverse_private_endpoint_ids.clone()),
    })
}

pub(in crate::cloud::clickpipes) fn build_kafka_source(
    args: &KafkaSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostKafkaSource> {
    build_kafka_source_with_exactly_once(args, None)
}

pub(in crate::cloud::clickpipes) async fn clickpipe_create_kafka(
    client: &CloudClient,
    args: &KafkaCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let request_args = build_create_request_args(&args.request, ClickPipeSourceKind::Kafka)?;
    let parsed_columns = args.destination.parsed_columns()?;
    let source = build_kafka_source_with_exactly_once(&args.source, args.exactly_once)?;

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kafka: Some(source),
            bigquery: None,
            kinesis: None,
            mongodb: None,
            mysql: None,
            object_storage: None,
            postgres: None,
            pubsub: None,
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
    use crate::cloud::clickpipes::cli::{
        ClickPipeCommands, ClickPipeCreateCommands, ClickPipeSchemaDiscoverCommands,
    };
    use crate::cloud::clickpipes::test_support::{
        assert_names, clickpipe_parse_error, kafka_args, kafka_create_cli_args,
        kafka_create_cli_args_without_auth, kafka_discover_cli_args,
        kafka_discover_cli_args_without_auth, kafka_source_with_plain_auth, parse_clickpipe,
    };

    #[test]
    fn parses_kafka_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(args),
        } = parse_clickpipe(&[
            "create",
            "kafka",
            "svc-1",
            "--name",
            "pipe-1",
            "--brokers",
            "broker-1:9092,broker-2:9092",
            "--topics",
            "topic-1,topic-2",
            "--format",
            "AvroConfluent",
            "--kafka-type",
            "msk",
            "--consumer-group",
            "group-1",
            "--auth",
            "PLAIN",
            "--username",
            "user",
            "--password",
            "password",
            "--iam-role",
            "arn:role",
            "--access-key-id",
            "access",
            "--secret-key",
            "secret",
            "--offset",
            "from_timestamp",
            "--offset-timestamp",
            "2021-01-01T00:00",
            "--schema-registry-url",
            "https://registry.example",
            "--schema-registry-username",
            "registry-user",
            "--schema-registry-password",
            "registry-password",
            "--ca-certificate",
            "/tmp/broker-ca.pem",
            "--client-certificate",
            "/tmp/client.pem",
            "--client-key",
            "/tmp/client.key",
            "--schema-registry-ca-certificate",
            "/tmp/registry-ca.pem",
            "--reverse-private-endpoint-id",
            "endpoint-1",
            "--reverse-private-endpoint-id",
            "endpoint-2",
            "--exactly-once",
            "true",
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
            panic!("expected kafka create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.source.brokers, "broker-1:9092,broker-2:9092");
        assert_eq!(args.source.topics, "topic-1,topic-2");
        assert_eq!(args.source.format, "AvroConfluent");
        assert_eq!(args.source.kafka_type, "msk");
        assert_eq!(args.source.consumer_group.as_deref(), Some("group-1"));
        assert_eq!(args.source.auth.as_deref(), Some("PLAIN"));
        assert_eq!(args.source.event_hubs_connection_string, None);
        assert_eq!(args.source.username.as_deref(), Some("user"));
        assert_eq!(args.source.password.as_deref(), Some("password"));
        assert_eq!(args.source.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.source.access_key_id.as_deref(), Some("access"));
        assert_eq!(args.source.secret_key.as_deref(), Some("secret"));
        assert_eq!(args.source.offset, "from_timestamp");
        assert_eq!(
            args.source.offset_timestamp.as_deref(),
            Some("2021-01-01T00:00")
        );
        assert_eq!(
            args.source.schema_registry_url.as_deref(),
            Some("https://registry.example")
        );
        assert_eq!(
            args.source.schema_registry_username.as_deref(),
            Some("registry-user")
        );
        assert_eq!(
            args.source.schema_registry_password.as_deref(),
            Some("registry-password")
        );
        assert_eq!(args.source.protobuf_schema_file, None);
        assert_eq!(
            args.source.ca_certificate.as_deref(),
            Some("/tmp/broker-ca.pem")
        );
        assert_eq!(
            args.source.client_certificate.as_deref(),
            Some("/tmp/client.pem")
        );
        assert_eq!(args.source.client_key.as_deref(), Some("/tmp/client.key"));
        assert_eq!(
            args.source.schema_registry_ca_certificate.as_deref(),
            Some("/tmp/registry-ca.pem")
        );
        assert_eq!(
            args.source.reverse_private_endpoint_ids,
            ["endpoint-1", "endpoint-2"]
        );
        assert_eq!(args.exactly_once, Some(true));
        assert_eq!(args.destination.database, "db");
        assert_eq!(args.destination.table, "events");
        assert_eq!(args.destination.columns, ["id:UInt64", "name:String"]);

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(args),
        } = parse_clickpipe(&[
            "create",
            "kafka",
            "svc-1",
            "--name",
            "pipe-1",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
            "--kafka-type",
            "gcmk",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
        ])
        else {
            panic!("expected kafka create");
        };
        assert_eq!(args.source.offset, "from_beginning");
        assert_eq!(args.source.consumer_group, None);
        assert_eq!(args.source.event_hubs_connection_string, None);
        assert_eq!(args.source.username, None);
        assert_eq!(args.source.password, None);
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id, None);
        assert_eq!(args.source.secret_key, None);
        assert_eq!(args.source.offset_timestamp, None);
        assert_eq!(args.source.schema_registry_url, None);
        assert_eq!(args.source.schema_registry_username, None);
        assert_eq!(args.source.schema_registry_password, None);
        assert_eq!(args.source.protobuf_schema_file, None);
        assert_eq!(args.source.ca_certificate, None);
        assert_eq!(args.source.client_certificate, None);
        assert_eq!(args.source.client_key, None);
        assert_eq!(args.source.schema_registry_ca_certificate, None);
        assert!(args.source.reverse_private_endpoint_ids.is_empty());
        assert_eq!(args.exactly_once, None);
    }

    #[test]
    fn parses_kafka_protobuf_and_event_hubs_fields_on_their_surfaces() {
        for base in [kafka_create_cli_args(), kafka_discover_cli_args()] {
            let mut args = base;
            let format = args
                .iter()
                .position(|value| *value == "JSONEachRow")
                .expect("minimal Kafka args contain a format");
            args[format] = "Protobuf";
            args.extend(["--protobuf-schema-file", "/tmp/events.proto"]);

            match parse_clickpipe(&args) {
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::Kafka(parsed),
                } => assert_eq!(
                    parsed.source.protobuf_schema_file.as_deref(),
                    Some("/tmp/events.proto")
                ),
                ClickPipeCommands::SchemaDiscover {
                    command: ClickPipeSchemaDiscoverCommands::Kafka(parsed),
                    ..
                } => assert_eq!(
                    parsed.source.protobuf_schema_file.as_deref(),
                    Some("/tmp/events.proto")
                ),
                _ => panic!("expected a Kafka command"),
            }
        }

        let mut args = kafka_create_cli_args_without_auth();
        args.extend([
            "--kafka-type",
            "azureeventhub",
            "--event-hubs-connection-string",
            "Endpoint=sb://events.example/;SharedAccessKey=secret",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected Kafka create");
        };
        assert_eq!(
            parsed.source.event_hubs_connection_string.as_deref(),
            Some("Endpoint=sb://events.example/;SharedAccessKey=secret")
        );
    }

    #[test]
    fn kafka_protobuf_and_event_hubs_conflicts_are_usage_errors() {
        for (flag, value) in [
            ("--schema-registry-url", "https://registry.example"),
            ("--schema-registry-username", "registry-user"),
            ("--schema-registry-password", "registry-password"),
            ("--schema-registry-ca-certificate", "/tmp/registry-ca.pem"),
        ] {
            let mut protobuf = kafka_create_cli_args();
            protobuf.extend(["--protobuf-schema-file", "/tmp/events.proto", flag, value]);
            assert_eq!(
                clickpipe_parse_error(&protobuf).kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{flag}"
            );
        }

        let mut event_hubs = kafka_create_cli_args_without_auth();
        event_hubs.extend([
            "--event-hubs-connection-string",
            "Endpoint=sb://events.example/",
            "--username",
            "user",
            "--password",
            "password",
        ]);
        assert_eq!(
            clickpipe_parse_error(&event_hubs).kind(),
            clap::error::ErrorKind::ArgumentConflict
        );
    }

    #[test]
    fn kafka_requires_authentication_on_both_surfaces() {
        // The API requires authentication on every Kafka source (issue #841),
        // so --auth or a mechanism-selecting credential flag is required.
        let bases: [fn() -> Vec<&'static str>; 2] = [
            kafka_create_cli_args_without_auth,
            kafka_discover_cli_args_without_auth,
        ];
        for base in bases {
            let args = base();
            let error = clickpipe_parse_error(&args);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{}",
                args.join(" ")
            );
            assert_eq!(error.exit_code(), 2, "{}", args.join(" "));

            for selector in [
                &["--auth", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"][..],
                &["--event-hubs-connection-string", "Endpoint=sb://e/"],
                &["--iam-role", "arn:role"],
                &["--username", "user", "--password", "password"],
                &["--access-key-id", "AKIA", "--secret-key", "secret"],
                &[
                    "--client-certificate",
                    "cert.pem",
                    "--client-key",
                    "key.pem",
                ],
            ] {
                let mut args = base();
                args.extend(selector);
                parse_clickpipe(&args);
            }
        }
    }

    #[test]
    fn kafka_credential_flags_must_be_given_in_pairs() {
        // Every Kafka credential pair is joined with clap `requires`. Half a
        // pair must be a usage error: the mechanism is inferred only from a
        // *complete* pair, so an unpaired flag would otherwise infer nothing
        // (issue #606).
        let bases: [fn() -> Vec<&'static str>; 2] = [
            kafka_create_cli_args_without_auth,
            kafka_discover_cli_args_without_auth,
        ];
        for base in bases {
            for (half, missing) in [
                (["--username", "user"], "--password"),
                (["--password", "password"], "--username"),
                (["--access-key-id", "access"], "--secret-key"),
                (["--secret-key", "secret"], "--access-key-id"),
                (["--client-certificate", "/tmp/client.pem"], "--client-key"),
                (["--client-key", "/tmp/client.key"], "--client-certificate"),
            ] {
                let mut args = base();
                args.extend(half);
                let error = clickpipe_parse_error(&args);
                assert_eq!(
                    error.kind(),
                    clap::error::ErrorKind::MissingRequiredArgument,
                    "{}",
                    args.join(" ")
                );
                assert_eq!(error.exit_code(), 2, "{}", args.join(" "));
                assert!(error.to_string().contains(missing), "{error}");
            }

            for pair in [
                ["--username", "user", "--password", "password"],
                ["--access-key-id", "access", "--secret-key", "secret"],
                [
                    "--client-certificate",
                    "/tmp/client.pem",
                    "--client-key",
                    "/tmp/client.key",
                ],
            ] {
                let mut args = base();
                args.extend(pair);
                parse_clickpipe(&args);
            }
        }
    }

    // `build_kafka_credentials` tests — lock the wire shape for each auth mode.
    // Authoritative source: `ClickPipePostKafkaSource.credentials` in
    // `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json`.

    /// Wire value of the built source's authentication.
    fn kafka_auth(source: &clickhouse_cloud_api::models::ClickPipePostKafkaSource) -> String {
        source.authentication.to_string()
    }

    #[test]
    fn build_kafka_source_infers_plain_from_username_and_password() {
        let mut args = kafka_args().source;
        args.username = Some("user".into());
        args.password = Some("password".into());
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), "PLAIN");
        assert_eq!(source.credentials.as_ref().unwrap()["username"], "user");
        assert_eq!(source.credentials.as_ref().unwrap()["password"], "password");
    }

    #[test]
    fn build_kafka_source_infers_iam_user_from_access_keys() {
        let mut args = kafka_args().source;
        args.access_key_id = Some("AKIA".into());
        args.secret_key = Some("secret".into());
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), "IAM_USER");
        assert_eq!(source.credentials.as_ref().unwrap()["accessKeyId"], "AKIA");
        assert_eq!(source.credentials.as_ref().unwrap()["secretKey"], "secret");
    }

    #[test]
    fn build_kafka_source_infers_iam_role_from_role_arn() {
        let mut args = kafka_args().source;
        args.iam_role = Some("arn:aws:iam::123:role/Foo".into());
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), "IAM_ROLE");
        assert_eq!(source.credentials, Some(serde_json::Value::Null));
        assert_eq!(
            source.iam_role.as_deref(),
            Some("arn:aws:iam::123:role/Foo")
        );
    }

    #[test]
    fn build_kafka_source_infers_mutual_tls_from_client_certificate_pair() {
        let directory = tempfile::tempdir().unwrap();
        let client_certificate = directory.path().join("client.pem");
        let client_key = directory.path().join("client.key");
        std::fs::write(&client_certificate, "CLIENT_CERT").unwrap();
        std::fs::write(&client_key, "CLIENT_KEY").unwrap();

        let mut args = kafka_args().source;
        args.client_certificate = Some(client_certificate.to_string_lossy().into_owned());
        args.client_key = Some(client_key.to_string_lossy().into_owned());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), "MUTUAL_TLS");
        assert_eq!(
            source.credentials.as_ref().unwrap()["certificate"],
            "CLIENT_CERT"
        );
        assert_eq!(
            source.credentials.as_ref().unwrap()["privateKey"],
            "CLIENT_KEY"
        );
    }

    #[test]
    fn build_kafka_source_errors_when_explicit_mechanism_lacks_credentials() {
        // An explicitly selected mechanism still fails fast, and the message
        // names the mechanism the user actually asked for.
        for auth in ["PLAIN", "SCRAM-SHA-256", "SCRAM-SHA-512"] {
            let mut args = kafka_args().source;
            args.auth = Some(auth.into());
            let error = build_kafka_source(&args).unwrap_err();
            assert_names(&error.message, &[auth, "--username", "--password"]);
        }

        let mut args = kafka_args().source;
        args.auth = Some("IAM_USER".into());
        let error = build_kafka_source(&args).unwrap_err();
        assert!(error.message.contains("--access-key-id"));

        let mut args = kafka_args().source;
        args.auth = Some("MUTUAL_TLS".into());
        let error = build_kafka_source(&args).unwrap_err();
        assert!(error.message.contains("--client-certificate"));
    }

    #[test]
    fn infer_kafka_authentication_prefers_sasl_over_certificates() {
        // Both SASL and client-certificate flags present: SASL wins, matching
        // the credential body that gets built.
        let mut args = kafka_args().source;
        args.username = Some("user".into());
        args.password = Some("password".into());
        args.client_certificate = Some("cert-path".into());
        args.client_key = Some("key-path".into());
        assert_eq!(
            infer_kafka_authentication(&args).map(|auth| auth.to_string()),
            Some("PLAIN".to_string())
        );
    }

    #[test]
    fn infer_kafka_authentication_ignores_half_specified_credentials() {
        // Half a credential pair is not enough to infer a mechanism. Every
        // pair is also paired with clap `requires`, so a half-specified
        // invocation never reaches this function (see
        // `kafka_credential_flags_must_be_given_in_pairs`); these assertions
        // pin the defensive behaviour of the inference itself, so a future
        // caller cannot turn half a pair into an unintended mechanism.
        let mut args = kafka_args().source;
        args.username = Some("user".into());
        assert_eq!(infer_kafka_authentication(&args), None);

        let mut args = kafka_args().source;
        args.secret_key = Some("secret".into());
        assert_eq!(infer_kafka_authentication(&args), None);

        let mut args = kafka_args().source;
        args.client_key = Some("key-path".into());
        assert_eq!(infer_kafka_authentication(&args), None);
    }

    #[test]
    fn build_kafka_source_supports_minimal_fields() {
        let mut args = kafka_args().source;
        args.auth = Some("PLAIN".into());
        args.username = Some("user".into());
        args.password = Some("password".into());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(source.r#type.to_string(), "kafka");
        assert_eq!(source.format.to_string(), "JSONEachRow");
        assert_eq!(source.brokers, "b:9092");
        assert_eq!(source.topics, "t");
        assert_eq!(kafka_auth(&source), "PLAIN");
        assert_eq!(source.credentials.as_ref().unwrap()["username"], "user");
        assert_eq!(source.credentials.as_ref().unwrap()["password"], "password");
        assert_eq!(source.consumer_group, None);
        assert_eq!(source.exactly_once, None);
        assert_eq!(source.iam_role, None);
        assert_eq!(source.ca_certificate, None);
        assert_eq!(source.schema_registry, None);
        assert_eq!(source.reverse_private_endpoint_ids, Some(vec![]));
        let offset = source.offset.expect("Kafka offset is always populated");
        assert_eq!(offset.strategy.to_string(), "from_beginning");
        assert_eq!(offset.timestamp, None);
    }

    #[test]
    fn build_kafka_source_sends_exactly_once_only_for_create() {
        let args = kafka_source_with_plain_auth();
        for value in [true, false] {
            let create = build_kafka_source_with_exactly_once(&args, Some(value)).unwrap();
            assert_eq!(create.exactly_once, Some(value));
        }

        let discovery = build_kafka_source(&args).unwrap();
        assert_eq!(discovery.exactly_once, None);
    }

    #[test]
    fn build_kafka_source_encodes_an_inline_protobuf_schema() {
        let directory = tempfile::tempdir().unwrap();
        let schema = directory.path().join("events.proto");
        std::fs::write(&schema, b"syntax = \"proto3\";").unwrap();
        let mut args = kafka_source_with_plain_auth();
        args.format = "Protobuf".into();
        args.protobuf_schema_file = Some(schema.to_string_lossy().into_owned());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(
            source.protobuf_schema.as_deref(),
            Some("c3ludGF4ID0gInByb3RvMyI7")
        );
    }

    #[test]
    fn build_kafka_source_validates_protobuf_flags_before_reading_files() {
        let mut args = kafka_args().source;
        args.protobuf_schema_file = Some("/file/that/does/not/exist".into());
        let error = build_kafka_source(&args).unwrap_err();
        assert_names(
            &error.message,
            &["--protobuf-schema-file", "--format Protobuf"],
        );

        // Already Protobuf, so this rejects the registry flags instead.
        args.format = "Protobuf".into();
        args.schema_registry_url = Some("https://registry.example".into());
        let registry = build_kafka_source(&args).unwrap_err();
        assert_names(&registry.message, &["--protobuf-schema-file"]);
        assert_ne!(registry.message, error.message);
    }

    #[test]
    fn build_kafka_source_rejects_an_empty_protobuf_schema() {
        let directory = tempfile::tempdir().unwrap();
        let schema = directory.path().join("events.proto");
        std::fs::write(&schema, []).unwrap();
        let mut args = kafka_source_with_plain_auth();
        args.format = "Protobuf".into();
        args.protobuf_schema_file = Some(schema.to_string_lossy().into_owned());

        let error = build_kafka_source(&args).unwrap_err();
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert_names(&error.message, &[&format!("'{}'", schema.display())]);
    }

    #[test]
    fn build_kafka_source_sends_event_hubs_connection_string_credentials() {
        let mut args = kafka_args().source;
        args.kafka_type = "azureeventhub".into();
        args.event_hubs_connection_string =
            Some("Endpoint=sb://events.example/;SharedAccessKey=secret".into());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(source.r#type.to_string(), "azureeventhub");
        assert_eq!(kafka_auth(&source), "PLAIN");
        assert_eq!(
            source.credentials.as_ref().unwrap()["connectionString"],
            "Endpoint=sb://events.example/;SharedAccessKey=secret"
        );
        assert!(
            source
                .credentials
                .as_ref()
                .unwrap()
                .get("username")
                .is_none()
        );
    }

    #[test]
    fn build_kafka_source_rejects_event_hubs_credentials_on_other_sources() {
        let mut args = kafka_args().source;
        args.event_hubs_connection_string = Some("Endpoint=sb://events.example/".into());
        let error = build_kafka_source(&args).unwrap_err();
        assert_names(
            &error.message,
            &[
                "--event-hubs-connection-string",
                "--kafka-type azureeventhub",
            ],
        );

        args.kafka_type = "azureeventhub".into();
        args.auth = Some("SCRAM-SHA-256".into());
        let error = build_kafka_source(&args).unwrap_err();
        assert_names(
            &error.message,
            &["--event-hubs-connection-string", "--auth PLAIN"],
        );
    }

    #[test]
    fn build_kafka_source_supports_maximal_fields_and_certificate_files() {
        let directory = tempfile::tempdir().unwrap();
        let broker_ca = directory.path().join("broker-ca.pem");
        let client_certificate = directory.path().join("client.pem");
        let client_key = directory.path().join("client.key");
        let registry_ca = directory.path().join("registry-ca.pem");
        std::fs::write(&broker_ca, "BROKER_CA").unwrap();
        std::fs::write(&client_certificate, "CLIENT_CERT").unwrap();
        std::fs::write(&client_key, "CLIENT_KEY").unwrap();
        std::fs::write(&registry_ca, "REGISTRY_CA").unwrap();

        let mut args = kafka_args().source;
        args.brokers = "broker-1:9092,broker-2:9092".into();
        args.topics = "topic-1,topic-2".into();
        args.format = "AvroConfluent".into();
        args.kafka_type = "msk".into();
        args.consumer_group = Some("group".into());
        args.auth = Some("MUTUAL_TLS".into());
        args.offset = "from_timestamp".into();
        args.offset_timestamp = Some("2021-01-01T00:00".into());
        args.schema_registry_url = Some("https://registry.example".into());
        args.schema_registry_username = Some("registry-user".into());
        args.schema_registry_password = Some("registry-password".into());
        args.ca_certificate = Some(broker_ca.to_string_lossy().into_owned());
        args.client_certificate = Some(client_certificate.to_string_lossy().into_owned());
        args.client_key = Some(client_key.to_string_lossy().into_owned());
        args.schema_registry_ca_certificate = Some(registry_ca.to_string_lossy().into_owned());
        args.reverse_private_endpoint_ids = vec!["endpoint-1".into(), "endpoint-2".into()];

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(source.r#type.to_string(), "msk");
        assert_eq!(source.format.to_string(), "AvroConfluent");
        assert_eq!(source.brokers, "broker-1:9092,broker-2:9092");
        assert_eq!(source.topics, "topic-1,topic-2");
        assert_eq!(source.consumer_group.as_deref(), Some("group"));
        assert_eq!(kafka_auth(&source), "MUTUAL_TLS");
        assert_eq!(
            source.credentials.as_ref().unwrap()["certificate"],
            "CLIENT_CERT"
        );
        assert_eq!(
            source.credentials.as_ref().unwrap()["privateKey"],
            "CLIENT_KEY"
        );
        assert_eq!(source.iam_role, None);
        assert_eq!(source.ca_certificate.as_deref(), Some("BROKER_CA"));
        assert_eq!(
            source.reverse_private_endpoint_ids,
            Some(vec!["endpoint-1".to_string(), "endpoint-2".to_string()])
        );
        let offset = source.offset.expect("Kafka offset is always populated");
        assert_eq!(offset.strategy.to_string(), "from_timestamp");
        assert_eq!(offset.timestamp.as_deref(), Some("2021-01-01T00:00"));
        let Some(clickhouse_cloud_api::models::ClickPipeMutateKafkaSchemaRegistry::ClickPipeMutateKafkaConfluentSchemaRegistry(
            registry,
        )) = source.schema_registry
        else {
            panic!("a Confluent schema registry is populated");
        };
        assert_eq!(registry.r#type, None);
        assert_eq!(registry.url, "https://registry.example");
        assert_eq!(registry.credentials.username, "registry-user");
        assert_eq!(registry.credentials.password, "registry-password");
        assert_eq!(registry.ca_certificate.as_deref(), Some("REGISTRY_CA"));
    }

    fn glue_registry(
        source: clickhouse_cloud_api::models::ClickPipePostKafkaSource,
    ) -> clickhouse_cloud_api::models::ClickPipeKafkaGlueSchemaRegistry {
        let Some(
            clickhouse_cloud_api::models::ClickPipeMutateKafkaSchemaRegistry::ClickPipeKafkaGlueSchemaRegistry(
                registry,
            ),
        ) = source.schema_registry
        else {
            panic!("a Glue schema registry is populated");
        };
        registry
    }

    #[test]
    fn build_kafka_source_builds_a_minimal_glue_schema_registry() {
        let mut args = kafka_args().source;
        args.format = "Avro".into();
        args.kafka_type = "msk".into();
        args.iam_role = Some("arn:aws:iam::123456789012:role/Source".into());
        args.schema_registry_type = Some("glue".into());
        args.glue_region = Some("us-east-1".into());
        args.glue_registry_name = Some("my-registry".into());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), "IAM_ROLE");
        let registry = glue_registry(source);
        assert_eq!(
            registry.r#type,
            clickhouse_cloud_api::models::ClickPipeKafkaGlueSchemaRegistryType::Glue
        );
        assert_eq!(registry.glue_region, "us-east-1");
        assert_eq!(registry.glue_registry_name, "my-registry");
        assert_eq!(registry.glue_role_arn, None);
    }

    #[test]
    fn build_kafka_source_builds_a_maximal_glue_schema_registry() {
        let mut args = kafka_args().source;
        args.format = "AvroConfluent".into();
        args.kafka_type = "msk".into();
        args.username = Some("user".into());
        args.password = Some("password".into());
        args.schema_registry_type = Some("glue".into());
        args.glue_region = Some("eu-west-1".into());
        args.glue_registry_name = Some("registry".into());
        args.glue_role_arn = Some("arn:aws:iam::123456789012:role/Glue".into());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), "PLAIN");
        assert_eq!(source.iam_role, None);
        let registry = glue_registry(source);
        assert_eq!(registry.glue_region, "eu-west-1");
        assert_eq!(registry.glue_registry_name, "registry");
        assert_eq!(
            registry.glue_role_arn.as_deref(),
            Some("arn:aws:iam::123456789012:role/Glue")
        );
    }

    #[test]
    fn build_kafka_source_sends_confluent_type_only_when_named() {
        let mut args = kafka_source_with_plain_auth();
        args.schema_registry_url = Some("https://registry.example".into());
        args.schema_registry_type = Some("confluent".into());

        let source = build_kafka_source(&args).unwrap();
        let Some(
            clickhouse_cloud_api::models::ClickPipeMutateKafkaSchemaRegistry::ClickPipeMutateKafkaConfluentSchemaRegistry(
                registry,
            ),
        ) = source.schema_registry
        else {
            panic!("a Confluent schema registry is populated");
        };
        assert_eq!(
            registry.r#type,
            Some(clickhouse_cloud_api::models::ClickPipeMutateKafkaConfluentSchemaRegistryType::Confluent)
        );
        assert_eq!(registry.url, "https://registry.example");

        args.schema_registry_type = None;
        args.schema_registry_url = None;
        assert_eq!(build_kafka_source(&args).unwrap().schema_registry, None);
    }

    #[test]
    fn build_kafka_source_sends_plain_registry_auth_and_empty_credentials_without_flags() {
        // The API requires `authentication` and `credentials` on a Confluent
        // registry; without credential flags the CLI sends PLAIN and empty
        // strings, the body it has always sent for an unauthenticated registry.
        let mut args = kafka_source_with_plain_auth();
        args.schema_registry_url = Some("https://registry.example".into());

        let source = build_kafka_source(&args).unwrap();
        assert_eq!(
            serde_json::to_value(source.schema_registry).unwrap(),
            serde_json::json!({
                "url": "https://registry.example",
                "authentication": "PLAIN",
                "credentials": { "username": "", "password": "" },
            })
        );
    }

    #[test]
    fn kafka_schema_registry_misuse_is_a_usage_error() {
        let glue = || {
            let mut args = kafka_args().source;
            args.schema_registry_type = Some("glue".into());
            args.glue_region = Some("us-east-1".into());
            args.glue_registry_name = Some("my-registry".into());
            args
        };
        let mut cases = Vec::new();

        // No role: neither --glue-role-arn nor a source --iam-role.
        cases.push(glue());
        let mut confluent_flag = glue();
        confluent_flag.glue_role_arn = Some("arn:role".into());
        confluent_flag.schema_registry_url = Some("https://registry.example".into());
        cases.push(confluent_flag);
        let mut missing_name = glue();
        missing_name.glue_role_arn = Some("arn:role".into());
        missing_name.glue_registry_name = None;
        cases.push(missing_name);
        let mut untyped = kafka_args().source;
        untyped.glue_region = Some("us-east-1".into());
        cases.push(untyped);
        let mut confluent_without_url = kafka_args().source;
        confluent_without_url.schema_registry_type = Some("confluent".into());
        cases.push(confluent_without_url);

        for args in cases {
            let error = build_kafka_source(&args).unwrap_err();
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{error:?}"
            );
        }

        let mut protobuf = kafka_args().source;
        protobuf.format = "Protobuf".into();
        protobuf.protobuf_schema_file = Some("/file/that/does/not/exist".into());
        protobuf.schema_registry_type = Some("glue".into());
        let error = build_kafka_source(&protobuf).unwrap_err();
        assert_names(&error.message, &["--protobuf-schema-file"]);
    }

    #[test]
    fn parses_kafka_glue_schema_registry_flags_on_both_surfaces() {
        for base in [kafka_create_cli_args(), kafka_discover_cli_args()] {
            let mut args = base;
            args.extend([
                "--schema-registry-type",
                "glue",
                "--glue-region",
                "us-east-1",
                "--glue-registry-name",
                "my-registry",
                "--glue-role-arn",
                "arn:role",
            ]);
            let source = match parse_clickpipe(&args) {
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::Kafka(args),
                } => args.source,
                ClickPipeCommands::SchemaDiscover {
                    command: ClickPipeSchemaDiscoverCommands::Kafka(args),
                } => args.source,
                _ => panic!("expected a Kafka command"),
            };
            assert_eq!(source.schema_registry_type.as_deref(), Some("glue"));
            assert_eq!(source.glue_region.as_deref(), Some("us-east-1"));
            assert_eq!(source.glue_registry_name.as_deref(), Some("my-registry"));
            assert_eq!(source.glue_role_arn.as_deref(), Some("arn:role"));
            assert_eq!(source.schema_registry_url, None);
        }

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(args),
        } = parse_clickpipe(&kafka_create_cli_args())
        else {
            panic!("expected kafka create");
        };
        assert_eq!(args.source.schema_registry_type, None);
        assert_eq!(args.source.glue_region, None);
        assert_eq!(args.source.glue_registry_name, None);
        assert_eq!(args.source.glue_role_arn, None);
    }

    #[test]
    fn kafka_glue_schema_registry_flag_constraints_are_clap_errors() {
        use clap::error::ErrorKind;
        let glue = [
            "--schema-registry-type",
            "glue",
            "--glue-region",
            "us-east-1",
            "--glue-registry-name",
            "my-registry",
        ];
        let cases: Vec<(Vec<&str>, ErrorKind)> = vec![
            (
                vec!["--schema-registry-type", "avro"],
                ErrorKind::InvalidValue,
            ),
            (
                vec!["--schema-registry-type", "glue"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec![
                    "--schema-registry-type",
                    "glue",
                    "--glue-region",
                    "us-east-1",
                ],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["--glue-region", "us-east-1"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["--glue-role-arn", "arn:role"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["--schema-registry-type", "confluent"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                [
                    &glue[..],
                    &["--schema-registry-url", "https://registry.example"],
                ]
                .concat(),
                ErrorKind::ArgumentConflict,
            ),
            (
                [&glue[..], &["--schema-registry-username", "user"]].concat(),
                ErrorKind::ArgumentConflict,
            ),
            (
                [&glue[..], &["--schema-registry-password", "password"]].concat(),
                ErrorKind::ArgumentConflict,
            ),
            (
                [
                    &glue[..],
                    &["--schema-registry-ca-certificate", "/tmp/ca.pem"],
                ]
                .concat(),
                ErrorKind::ArgumentConflict,
            ),
            (
                [
                    &[
                        "--format",
                        "Protobuf",
                        "--protobuf-schema-file",
                        "/tmp/e.proto",
                    ][..],
                    &glue[..],
                ]
                .concat(),
                ErrorKind::ArgumentConflict,
            ),
        ];
        for (extra, kind) in cases {
            let mut args = kafka_create_cli_args();
            args.extend(extra.iter().copied());
            assert_eq!(clickpipe_parse_error(&args).kind(), kind, "{extra:?}");
        }
    }

    #[test]
    fn kafka_credentials_plain_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("PLAIN".into());
        args.source.username = Some("u".into());
        args.source.password = Some("p".into());
        let credentials = build_kafka_credentials(&Auth::PLAIN, &args.source, None).unwrap();
        assert_eq!(credentials["username"], "u");
        assert_eq!(credentials["password"], "p");
    }

    #[test]
    fn kafka_credentials_iam_user_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_USER".into());
        args.source.access_key_id = Some("AKIA".into());
        args.source.secret_key = Some("secret".into());
        let credentials = build_kafka_credentials(&Auth::IAM_USER, &args.source, None).unwrap();
        // MskIamUser wire shape is {accessKeyId, secretKey} — NOT snake_case.
        assert_eq!(credentials["accessKeyId"], "AKIA");
        assert_eq!(credentials["secretKey"], "secret");
        assert!(credentials.get("access_key_id").is_none());
    }

    #[test]
    fn kafka_credentials_iam_role_is_null() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        args.source.iam_role = Some("arn:aws:iam::123:role/Foo".into());
        // IAM_ROLE sends credentials=null; the role ARN flows through the
        // top-level `iamRole` field on the Kafka source, not credentials.
        let credentials = build_kafka_credentials(&Auth::IAM_ROLE, &args.source, None).unwrap();
        assert!(credentials.is_null());
    }

    #[test]
    fn kafka_credentials_mutual_tls_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let contents = Some(("CERT_PEM".into(), "KEY_PEM".into()));
        let credentials =
            build_kafka_credentials(&Auth::MUTUAL_TLS, &args.source, contents).unwrap();
        assert_eq!(credentials["certificate"], "CERT_PEM");
        assert_eq!(credentials["privateKey"], "KEY_PEM");
    }

    #[test]
    fn kafka_credentials_iam_user_missing_args_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let error = build_kafka_credentials(&Auth::IAM_USER, &args.source, None).unwrap_err();
        assert!(error.message.contains("--access-key-id"));
    }

    #[test]
    fn kafka_credentials_iam_role_missing_arn_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        let error = build_kafka_credentials(&Auth::IAM_ROLE, &args.source, None).unwrap_err();
        assert!(error.message.contains("--iam-role"));
    }

    #[test]
    fn kafka_workload_identity_builds_for_gcmk_without_credentials() {
        let mut args = kafka_args().source;
        args.kafka_type = "gcmk".into();
        args.auth = Some("SERVICE_ACCOUNT_WORKLOAD_IDENTITY".into());
        let source = build_kafka_source(&args).expect("GCMK workload identity builds");
        assert_eq!(
            source.authentication,
            ClickPipePostKafkaSourceAuthentication::ServiceAccountWorkloadIdentity
        );
        assert_eq!(source.credentials, Some(serde_json::Value::Null));
        assert!(source.iam_role.is_none());
    }

    #[test]
    fn kafka_workload_identity_rejects_provider_and_credentials() {
        let mut args = kafka_args().source;
        args.auth = Some("SERVICE_ACCOUNT_WORKLOAD_IDENTITY".into());
        assert_names(
            &build_kafka_source(&args).unwrap_err().message,
            &["SERVICE_ACCOUNT_WORKLOAD_IDENTITY", "--kafka-type gcmk"],
        );

        args.kafka_type = "gcmk".into();
        args.username = Some("user".into());
        args.password = Some("password".into());
        let message = build_kafka_source(&args).unwrap_err().message;
        assert_names(&message, &["--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY"]);
        assert!(!message.contains("--kafka-type"), "{message}");
    }
}
