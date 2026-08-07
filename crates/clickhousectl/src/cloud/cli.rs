pub use crate::cloud::activity::ActivityCommands;
pub use crate::cloud::api_keys::KeyCommands;
pub use crate::cloud::auth::AuthCommands;
#[allow(unused_imports)]
pub use crate::cloud::backups::{BackupCommands, BackupConfigCommands};
pub use crate::cloud::organizations::{InvitationCommands, MemberCommands, OrgCommands};
#[allow(unused_imports)]
pub use crate::cloud::services::{PrivateEndpointCommands, QueryEndpointCommands, ServiceCommands};
use clap::builder::PossibleValuesParser;
use clap::{Args, Subcommand};

// Valid wire values for each ClickPipe enum the CLI accepts as a string argument.
// Kept in sync with the clickhouse-cloud-api library enums; extra variants are
// rejected by clap at parse time with a `[possible values: …]` hint.
const OBJECT_STORAGE_FORMATS: &[&str] = &[
    "JSONEachRow",
    "JSONAsObject",
    "CSV",
    "CSVWithNames",
    "TabSeparated",
    "TabSeparatedWithNames",
    "Parquet",
    "Avro",
];
const OBJECT_STORAGE_COMPRESSIONS: &[&str] = &[
    "none", "gzip", "gz", "brotli", "br", "xz", "LZMA", "zstd", "auto",
];
const OBJECT_STORAGE_TYPES: &[&str] = &[
    "s3",
    "gcs",
    "dospaces",
    "azureblobstorage",
    "cloudflarer2",
    "ovhobjectstorage",
];
const KAFKA_FORMATS: &[&str] = &["JSONEachRow", "Avro", "AvroConfluent", "Protobuf"];
const KAFKA_TYPES: &[&str] = &[
    "kafka",
    "redpanda",
    "msk",
    "gcmk",
    "confluent",
    "warpstream",
    "azureeventhub",
    "dokafka",
];
const KAFKA_AUTHS: &[&str] = &[
    "PLAIN",
    "SCRAM-SHA-256",
    "SCRAM-SHA-512",
    "IAM_ROLE",
    "IAM_USER",
    "MUTUAL_TLS",
];
const KAFKA_OFFSET_STRATEGIES: &[&str] = &["from_beginning", "from_latest", "from_timestamp"];
const KINESIS_FORMATS: &[&str] = &["JSONEachRow", "Avro", "AvroConfluent"];
const KINESIS_AUTHS: &[&str] = &["IAM_ROLE", "IAM_USER"];
const KINESIS_ITERATOR_TYPES: &[&str] = &["TRIM_HORIZON", "LATEST", "AT_TIMESTAMP"];
const POSTGRES_TYPES: &[&str] = &[
    "postgres",
    "supabase",
    "neon",
    "alloydb",
    "planetscale",
    "rdspostgres",
    "aurorapostgres",
    "cloudsqlpostgres",
    "azurepostgres",
    "crunchybridge",
    "tigerdata",
];
const DB_AUTHS: &[&str] = &["basic", "IAM_ROLE"];
const REPLICATION_MODES: &[&str] = &["cdc", "snapshot", "cdc_only"];
const MYSQL_TYPES: &[&str] = &["mysql", "rdsmysql", "auroramysql", "mariadb", "rdsmariadb"];
const MYSQL_REPLICATION_MECHANISMS: &[&str] = &["GTID", "FILE_POS"];
const MONGODB_READ_PREFERENCES: &[&str] = &[
    "primary",
    "primaryPreferred",
    "secondary",
    "secondaryPreferred",
    "nearest",
];

#[derive(Args)]
pub struct CloudArgs {
    /// API key override (highest precedence; see `cloud --help` for all sources)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret override (highest precedence; see `cloud --help` for all sources)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Print debug info (e.g. the credential source used) to stderr before running the command
    #[arg(long, global = true)]
    pub debug: bool,

    /// API base URL (default: auto-detect from OAuth tokens, or https://api.clickhouse.cloud)
    #[cfg_attr(debug_assertions, arg(long, global = true))]
    #[cfg_attr(not(debug_assertions), arg(long, global = true, hide = true))]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: CloudCommands,
}

#[derive(Subcommand)]
pub enum CloudCommands {
    /// Manage authentication (OAuth login, API keys)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Default `login` opens a browser for OAuth (read-only).
  Use `login --api-key X --api-secret Y` for full read/write access, or set
  CLICKHOUSE_CLOUD_API_KEY / CLICKHOUSE_CLOUD_API_SECRET env vars (exported or in .env).
  Create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl
  `logout` clears all saved credentials (OAuth tokens and API keys).
  Related: `clickhousectl cloud org list` to verify credentials work.")]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Organization commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud organizations. Subcommands: list, get, update, prometheus, usage.
  Org IDs are needed for most service and backup operations.
  Start with `clickhousectl cloud org list` to discover available org IDs.
  Related: `clickhousectl cloud service list` (uses org ID).")]
    Org {
        #[command(subcommand)]
        command: OrgCommands,
    },

    /// Service commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Most commands need a service ID — get it from `clickhousectl cloud service list`.
  Org ID is auto-detected if you have only one org; otherwise pass --org-id.
  Write commands (create, delete, start, stop, update, scale) require API key auth — OAuth is read-only.
  Use `query` to run SQL against a service over HTTP.
  Related: `clickhousectl cloud org list` for org IDs.")]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },

    /// Backup commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud backups. Subcommands: list, get.
  Requires a service ID — get it from `clickhousectl cloud service list`.
  Backup IDs from `backup list` can be used with `service create --backup-id` to restore.
  Related: `clickhousectl cloud service list` for service IDs.")]
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    // Clickpipe commands
    #[command(
        name = "clickpipe",
        after_help = "\
CONTEXT FOR AGENTS:
    Manage ClickPipes for ingesting data into ClickHouse Cloud.
    Subcommands: list, get, delete, start, stop, resync, scale, settings, create.
    Requires a service ID — get it from `clickhousectl cloud service list`."
    )]
    ClickPipe {
        #[command(subcommand)]
        command: Box<ClickPipeCommands>,
    },

    /// Manage organization members
    Member {
        #[command(subcommand)]
        command: MemberCommands,
    },

    /// Manage organization invitations
    Invitation {
        #[command(subcommand)]
        command: InvitationCommands,
    },

    /// Manage API keys
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    /// View activity log
    Activity {
        #[command(subcommand)]
        command: ActivityCommands,
    },

    /// Manage ClickHouse Cloud Postgres services (beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud managed Postgres services. Subcommands cover CRUD, lifecycle
  (restart/promote/switchover), CA certs, runtime config, password reset, read replicas,
  and point-in-time restore. Service IDs come from `postgres list`.
  Write commands require API key auth — OAuth is read-only.")]
    Postgres {
        #[command(subcommand)]
        command: crate::cloud::postgres::PostgresCommands,
    },
}

impl CloudCommands {
    /// Returns true if this command performs a write/mutating operation.
    /// OAuth (Bearer) auth is read-only and cannot execute write commands.
    ///
    /// Every variant is explicitly matched — no wildcards — so the compiler
    /// will error when a new command is added, forcing the developer to
    /// classify it as read or write.
    pub fn is_write_command(&self) -> bool {
        match self {
            CloudCommands::Auth { command } => command.is_write(),
            CloudCommands::Org { command } => command.is_write(),
            CloudCommands::Service { command } => command.is_write(),
            CloudCommands::Backup { command } => command.is_write(),
            CloudCommands::Member { command } => command.is_write(),
            CloudCommands::Invitation { command } => command.is_write(),
            CloudCommands::Key { command } => command.is_write(),
            CloudCommands::Activity { command } => command.is_write(),
            CloudCommands::Postgres { command } => command.is_write(),
            CloudCommands::ClickPipe { command } => match command.as_ref() {
                ClickPipeCommands::List { .. } => false,
                ClickPipeCommands::Get { .. } => false,
                ClickPipeCommands::Delete { .. } => true,
                ClickPipeCommands::Start { .. } => true,
                ClickPipeCommands::Stop { .. } => true,
                ClickPipeCommands::Resync { .. } => true,
                ClickPipeCommands::Scale { .. } => true,
                // Side-effect-free, but the API gateway rejects OAuth/JWT on
                // POST /clickpipes/schemaDiscovery ("This endpoint is not
                // available for JWT authentication"), so classify it as a
                // write to fail fast with the API-key guidance.
                ClickPipeCommands::SchemaDiscover { .. } => true,
                ClickPipeCommands::Create { .. } => true,
                ClickPipeCommands::Settings { command } => match command {
                    ClickPipeSettingsCommands::Get { .. } => false,
                    ClickPipeSettingsCommands::Update { .. } => true,
                },
            },
        }
    }
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ClickPipeCommands {
    /// List ClickPipes
    List {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get ClickPipe details
    Get {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete a ClickPipe
    Delete {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Start a ClickPipe
    Start {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Stop a ClickPipe
    Stop {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Resync a ClickPipe (CDC pipes only)
    Resync {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update scaling configuration
    Scale {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Number of replicas (1-40)
        #[arg(long)]
        replicas: Option<u32>,

        /// CPU millicores per replica (125-2000, streaming pipes)
        #[arg(long)]
        cpu_millicores: Option<u32>,

        /// Memory GB per replica (0.5-8, streaming pipes)
        #[arg(long)]
        memory_gb: Option<f64>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Manage ClickPipe settings
    Settings {
        #[command(subcommand)]
        command: ClickPipeSettingsCommands,
    },

    /// Discover a source schema without creating a pipe (beta)
    #[command(after_help = "\\
CONTEXT FOR AGENTS:
  Infers the schema (column name + ClickHouse type) for a Kafka or Kinesis source
  without creating a ClickPipe. Useful for filling in --column on `clickpipe create`.
  Related: `clickhousectl cloud clickpipe create kafka|kinesis` to create a pipe with the discovered columns.")]
    SchemaDiscover {
        /// Service ID
        service_id: String,

        #[command(subcommand)]
        command: ClickPipeSchemaDiscoverCommands,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create a ClickPipe
    Create {
        #[command(subcommand)]
        command: ClickPipeCreateCommands,
    },
}

#[derive(Subcommand)]
pub enum ClickPipeSchemaDiscoverCommands {
    /// Discover schema from a Kafka or Kafka-compatible source
    Kafka(Box<KafkaSourceFields>),

    /// Discover schema from an Amazon Kinesis stream
    Kinesis(Box<KinesisSourceFields>),
}

#[derive(Subcommand)]
pub enum ClickPipeSettingsCommands {
    /// Get ClickPipe settings
    Get {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update ClickPipe settings
    Update {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Max wait before inserting data (ms, 500-60000)
        #[arg(long)]
        streaming_max_insert_wait_ms: Option<u32>,

        /// Concurrent file processing threads (1-35)
        #[arg(long)]
        object_storage_concurrency: Option<u32>,

        /// Polling interval for continuous ingest (ms, 100-3600000)
        #[arg(long)]
        object_storage_polling_interval_ms: Option<u32>,

        /// Bytes per insert batch
        #[arg(long)]
        object_storage_max_insert_bytes: Option<u64>,

        /// Max files per insert batch (1-10000)
        #[arg(long)]
        object_storage_max_file_count: Option<u32>,

        /// Max concurrent threads for file processing (0-64)
        #[arg(long)]
        clickhouse_max_threads: Option<u32>,

        /// Max concurrent insert threads (0-16)
        #[arg(long)]
        clickhouse_max_insert_threads: Option<u32>,

        /// Use ClickHouse cluster function
        #[arg(long)]
        object_storage_use_cluster_function: Option<bool>,

        /// Push to attached views concurrently
        #[arg(long)]
        clickhouse_parallel_view_processing: Option<bool>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ClickPipeCreateCommands {
    /// Create a ClickPipe from S3, GCS, Azure Blob, or other object storage
    #[command(name = "object-storage")]
    ObjectStorage(ObjectStorageCreateArgs),

    /// Create a ClickPipe from Kafka or Kafka-compatible source
    Kafka(KafkaCreateArgs),

    /// Create a ClickPipe from Amazon Kinesis
    Kinesis(KinesisCreateArgs),

    /// Create a ClickPipe from PostgreSQL
    Postgres(PostgresCreateArgs),

    /// Create a ClickPipe from MySQL
    #[command(name = "mysql")]
    MySQL(MySqlCreateArgs),

    /// Create a ClickPipe from MongoDB
    #[command(name = "mongodb")]
    MongoDB(MongoDbCreateArgs),

    /// Create a ClickPipe from BigQuery
    #[command(name = "bigquery")]
    BigQuery(BigQueryCreateArgs),
}

#[derive(Args, Debug)]
pub struct ObjectStorageCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// Source URL (e.g., https://bucket.s3.region.amazonaws.com/path/*.json)
    #[arg(long)]
    pub source_url: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(OBJECT_STORAGE_FORMATS))]
    pub format: String,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64" --column "name:String")
    #[arg(long = "column")]
    pub columns: Vec<String>,

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

    /// Enable continuous ingestion
    #[arg(long)]
    pub continuous: bool,

    /// SQS queue URL for continuous ingestion notifications
    #[arg(long)]
    pub queue_url: Option<String>,

    /// Skip the initial load of existing objects and ingest only queue-notification
    /// files. Only applicable when --queue-url is provided.
    #[arg(long, requires = "queue_url")]
    pub skip_initial_load: bool,

    /// Object key to start continuous ingestion after. Mutually exclusive with
    /// --skip-initial-load (the API rejects both being set).
    #[arg(long, conflicts_with = "skip_initial_load")]
    pub start_after: Option<String>,

    /// CSV delimiter character (e.g., ",")
    #[arg(long)]
    pub delimiter: Option<String>,

    /// IAM role ARN for authentication
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID for authentication
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key for authentication
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Azure connection string for authentication
    #[arg(long)]
    pub connection_string: Option<String>,

    /// Azure container name
    #[arg(long)]
    pub azure_container_name: Option<String>,

    /// Object storage path (for Azure)
    #[arg(long)]
    pub path: Option<String>,

    /// Path to GCP service account JSON key file
    #[arg(long)]
    pub service_account_file: Option<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

/// Source-connection fields for a Kafka / Kafka-compatible ClickPipe source.
/// Flattened into both `KafkaCreateArgs` (pipe creation) and the schema-discover
/// Kafka subcommand so the source field set has a single definition.
#[derive(Args, Debug)]
pub struct KafkaSourceFields {
    /// Kafka broker(s) (e.g., "broker1:9092,broker2:9092")
    #[arg(long)]
    pub brokers: String,

    /// Topic(s) to consume from
    #[arg(long)]
    pub topics: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(KAFKA_FORMATS))]
    pub format: String,

    /// Kafka type
    #[arg(
        long,
        default_value = "kafka",
        value_parser = PossibleValuesParser::new(KAFKA_TYPES),
    )]
    pub kafka_type: String,

    /// Consumer group name
    #[arg(long)]
    pub consumer_group: Option<String>,

    /// Authentication method
    #[arg(long, value_parser = PossibleValuesParser::new(KAFKA_AUTHS))]
    pub auth: Option<String>,

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
        value_parser = PossibleValuesParser::new(KAFKA_OFFSET_STRATEGIES),
    )]
    pub offset: String,

    /// Timestamp for from_timestamp offset (e.g., "2021-01-01T00:00")
    #[arg(long)]
    pub offset_timestamp: Option<String>,

    /// Schema registry URL (for Avro/Protobuf formats)
    #[arg(long)]
    pub schema_registry_url: Option<String>,

    /// Schema registry username
    #[arg(long)]
    pub schema_registry_username: Option<String>,

    /// Schema registry password
    #[arg(long)]
    pub schema_registry_password: Option<String>,

    /// Path to broker CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Path to client certificate file (for MUTUAL_TLS auth)
    #[arg(long)]
    pub client_certificate: Option<String>,

    /// Path to client private key file (for MUTUAL_TLS auth)
    #[arg(long)]
    pub client_key: Option<String>,

    /// Path to schema registry CA certificate file
    #[arg(long)]
    pub schema_registry_ca_certificate: Option<String>,

    /// Reverse private endpoint IDs (repeatable)
    #[arg(long = "reverse-private-endpoint-id")]
    pub reverse_private_endpoint_ids: Vec<String>,
}

#[derive(Args, Debug)]
pub struct KafkaCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub source: KafkaSourceFields,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64")
    #[arg(long = "column")]
    pub columns: Vec<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
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

    /// Authentication
    #[arg(
        long,
        default_value = "IAM_ROLE",
        value_parser = PossibleValuesParser::new(KINESIS_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID for IAM_USER authentication
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key for IAM_USER authentication
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
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub source: KinesisSourceFields,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64")
    #[arg(long = "column")]
    pub columns: Vec<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct PostgresCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// PostgreSQL host
    #[arg(long)]
    pub host: String,

    /// PostgreSQL port
    #[arg(long, default_value = "5432")]
    pub port: u16,

    /// Source database name
    #[arg(long)]
    pub pg_database: String,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as schema.table:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// Postgres type
    #[arg(
        long,
        default_value = "postgres",
        value_parser = PossibleValuesParser::new(POSTGRES_TYPES),
    )]
    pub postgres_type: String,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Authentication
    #[arg(
        long,
        default_value = "basic",
        value_parser = PossibleValuesParser::new(DB_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN
    #[arg(long)]
    pub iam_role: Option<String>,

    /// TLS hostname
    #[arg(long)]
    pub tls_host: Option<String>,

    /// Path to CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Postgres publication name
    #[arg(long)]
    pub publication_name: Option<String>,

    /// Replication slot name
    #[arg(long)]
    pub replication_slot_name: Option<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct MySqlCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// MySQL host
    #[arg(long)]
    pub host: String,

    /// MySQL port
    #[arg(long, default_value = "3306")]
    pub port: u16,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as schema.table:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// MySQL type
    #[arg(
        long,
        default_value = "mysql",
        value_parser = PossibleValuesParser::new(MYSQL_TYPES),
    )]
    pub mysql_type: String,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Replication mechanism
    #[arg(
        long,
        default_value = "GTID",
        value_parser = PossibleValuesParser::new(MYSQL_REPLICATION_MECHANISMS),
    )]
    pub replication_mechanism: String,

    /// Authentication
    #[arg(
        long,
        default_value = "basic",
        value_parser = PossibleValuesParser::new(DB_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN
    #[arg(long)]
    pub iam_role: Option<String>,

    /// TLS hostname
    #[arg(long)]
    pub tls_host: Option<String>,

    /// Path to CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Disable TLS
    #[arg(long)]
    pub disable_tls: bool,

    /// Skip certificate verification
    #[arg(long)]
    pub skip_cert_verification: bool,

    /// Optional MySQL server_id the pipe declares itself as in the MySQL
    /// replication topology (1-4294967295). Must be unique across replicas
    /// connected to the source. If omitted, one is assigned automatically.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=4294967295))]
    pub server_id: Option<u64>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct MongoDbCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// MongoDB connection URI (e.g., mongodb+srv://cluster0.example.mongodb.net/mydb)
    #[arg(long)]
    pub uri: String,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as database.collection:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Read preference
    #[arg(
        long,
        default_value = "secondaryPreferred",
        value_parser = PossibleValuesParser::new(MONGODB_READ_PREFERENCES),
    )]
    pub read_preference: String,

    /// TLS hostname
    #[arg(long)]
    pub tls_host: Option<String>,

    /// Path to CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Disable TLS
    #[arg(long)]
    pub disable_tls: bool,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct BigQueryCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// Path to GCP service account JSON key file
    #[arg(long)]
    pub service_account_file: String,

    /// GCS staging path for snapshot data
    #[arg(long)]
    pub staging_path: String,

    /// Table mappings as dataset.table:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn parses_clickpipe_object_storage_ingestion_control_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://b.s3.us-east-1.amazonaws.com/d/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--queue-url",
            "https://sqs.us-east-1.amazonaws.com/123/q",
            "--start-after",
            "key1",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = args.command else {
            panic!("expected clickpipe command");
        };
        let ClickPipeCommands::Create { command } = *command else {
            panic!("expected create");
        };
        let ClickPipeCreateCommands::ObjectStorage(args) = command else {
            panic!("expected object-storage");
        };
        assert!(!args.skip_initial_load);
        assert_eq!(args.start_after.as_deref(), Some("key1"));
        assert_eq!(
            args.queue_url.as_deref(),
            Some("https://sqs.us-east-1.amazonaws.com/123/q")
        );
    }

    #[test]
    fn rejects_skip_initial_load_without_queue_url() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://b.s3.us-east-1.amazonaws.com/d/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--skip-initial-load",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_skip_initial_load_with_start_after() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://b.s3.us-east-1.amazonaws.com/d/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--queue-url",
            "https://sqs.us-east-1.amazonaws.com/123/q",
            "--skip-initial-load",
            "--start-after",
            "key1",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_clickpipe_mysql_server_id() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "h",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.t:t",
            "--server-id",
            "4242",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = args.command else {
            panic!("expected clickpipe command");
        };
        let ClickPipeCommands::Create { command } = *command else {
            panic!("expected create");
        };
        let ClickPipeCreateCommands::MySQL(args) = command else {
            panic!("expected mysql");
        };
        assert_eq!(args.server_id, Some(4242));
    }

    #[test]
    fn rejects_clickpipe_mysql_server_id_out_of_range() {
        // 0 is below the minimum (1)
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "h",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.t:t",
            "--server-id",
            "0",
        ]);
        assert!(result.is_err());

        // 4294967296 is above the maximum (4294967295)
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "h",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.t:t",
            "--server-id",
            "4294967296",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_clickpipe_schema_discover_kafka() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "schema-discover",
            "svc-1",
            "kafka",
            "--brokers",
            "b:9092",
            "--topics",
            "t",
            "--format",
            "JSONEachRow",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = args.command else {
            panic!("expected clickpipe command");
        };
        let ClickPipeCommands::SchemaDiscover {
            service_id,
            command,
            ..
        } = *command
        else {
            panic!("expected schema-discover");
        };
        assert_eq!(service_id, "svc-1");
        assert!(matches!(command, ClickPipeSchemaDiscoverCommands::Kafka(_)));
    }

    /// Helper to assert a command parsed from CLI args is classified correctly.
    fn assert_write(args: &[&str], expected: bool) {
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        assert_eq!(
            cloud_args.command.is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    #[test]
    fn is_write_command_read_only_commands() {
        // Backup reads
        assert_write(
            &["clickhousectl", "cloud", "backup", "list", "svc-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "backup", "get", "svc-1", "bk-1"],
            false,
        );

        // Key reads
        assert_write(&["clickhousectl", "cloud", "key", "list"], false);
        assert_write(&["clickhousectl", "cloud", "key", "get", "key-1"], false);

        // Activity reads
        assert_write(&["clickhousectl", "cloud", "activity", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "activity", "get", "act-1"],
            false,
        );

        // Postgres reads
        assert_write(&["clickhousectl", "cloud", "postgres", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "postgres", "get", "pg-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "certs", "get", "pg-1"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "get",
                "pg-1",
            ],
            false,
        );
    }

    #[test]
    fn is_write_command_destructive_commands() {
        // ClickPipe schema discovery is side-effect-free, but the API gateway
        // rejects OAuth/JWT on the endpoint, so it requires API-key auth.
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "schema-discover",
                "svc-1",
                "kafka",
                "--brokers",
                "b:9092",
                "--topics",
                "t",
                "--format",
                "JSONEachRow",
            ],
            true,
        );

        // Key writes
        assert_write(
            &["clickhousectl", "cloud", "key", "create", "--name", "k"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "key",
                "update",
                "key-1",
                "--name",
                "new",
            ],
            true,
        );
        assert_write(&["clickhousectl", "cloud", "key", "delete", "key-1"], true);

        // Postgres writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "create",
                "--name",
                "pg",
                "--region",
                "us-east-1",
                "--size",
                "m7i.2xlarge",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "update",
                "pg-1",
                "--size",
                "c6gd.large",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "delete", "pg-1"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "replace",
                "pg-1",
                "--file",
                "/tmp/c.json",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "patch",
                "pg-1",
                "--set",
                "max_connections=500",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "reset-password",
                "pg-1",
                "--generate",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "r1",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "r",
                "--restore-target",
                "2026-04-16T12:00:00Z",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "restart", "pg-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "promote", "pg-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "switchover", "pg-1"],
            true,
        );
    }
}
