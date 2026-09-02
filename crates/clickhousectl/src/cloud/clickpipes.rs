use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use clap::builder::PossibleValuesParser;
use clap::{ArgGroup, Args, Subcommand};
use clickhouse_cloud_api::models::{
    ClickPipePostgresPipeTableMapping, ClickPipePostgresPipeTableMappingTableengine,
};
use tabled::{Table, Tabled, settings::Style};

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
    #[command(
        group(ArgGroup::new("scale_target").required(true).multiple(true).args(["replicas", "cpu_millicores", "memory_gb"]))
    )]
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

    /// Manage ClickPipe ingestion settings (streaming and object-storage pipes only)
    #[command(long_about = "\
Manage ClickPipe ingestion settings.

Ingestion settings apply only to streaming (Kafka, Kinesis) and object-storage
pipes. Database CDC pipes (Postgres, MySQL, MongoDB, BigQuery) have no ingestion
settings: their settings, such as the sync interval and pull batch size, live on
the pipe itself, so read them with `clickhousectl cloud clickpipe get <service>
<clickpipe>`.")]
    Settings {
        #[command(subcommand)]
        command: ClickPipeSettingsCommands,
    },

    /// Discover a source schema without creating a pipe (beta)
    #[command(after_help = "\\
CONTEXT FOR AGENTS:
  Infers the schema (column name + ClickHouse type) for a Kafka, Kinesis or
  object-storage source without creating a ClickPipe. Useful for filling in
  --column on `clickpipe create`.
  Related: `clickhousectl cloud clickpipe create kafka|kinesis|object-storage` to
  create a pipe with the discovered columns.")]
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

impl ClickPipeCommands {
    pub fn is_write(&self) -> bool {
        match self {
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
            ClickPipeCommands::Settings { command } => command.is_write(),
        }
    }

    pub(crate) fn postgres_create_validation_error(&self) -> Option<String> {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = self
        else {
            return None;
        };

        validate_postgres_create_args(args)
            .err()
            .map(|error| error.message)
    }
}

#[derive(Subcommand)]
pub enum ClickPipeSchemaDiscoverCommands {
    /// Discover schema from a Kafka or Kafka-compatible source
    Kafka(Box<KafkaSourceFields>),

    /// Discover schema from an Amazon Kinesis stream
    Kinesis(Box<KinesisSourceFields>),

    /// Discover schema from an object-storage source (S3, GCS, Azure Blob Storage)
    #[command(name = "object-storage")]
    ObjectStorage(Box<ObjectStorageSourceFields>),
}

#[derive(Subcommand)]
pub enum ClickPipeSettingsCommands {
    /// Get ClickPipe ingestion settings (streaming and object-storage pipes only)
    Get {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update ClickPipe ingestion settings (streaming and object-storage pipes only)
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

impl ClickPipeSettingsCommands {
    fn is_write(&self) -> bool {
        match self {
            ClickPipeSettingsCommands::Get { .. } => false,
            ClickPipeSettingsCommands::Update { .. } => true,
        }
    }
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
    #[command(after_help = "\
POSTGRES INPUT RULES:
  At least one --table-mapping is required, in schema.table:target_table form,
  or one --table-mapping-json.
  --auth IAM_ROLE requires --iam-role. With basic auth, --iam-role is rejected
  instead of being silently ignored.
  --replication-slot-name is valid only with --replication-mode cdc_only.

POSTGRES TABLE MAPPINGS:
  --table-mapping schema.table:target_table is unchanged: it maps one table
  and leaves every other per-table option at the ClickPipes default.
  --table-mapping-json takes the API's table mapping object verbatim, for the
  options that shape the destination table and cannot be changed once the pipe
  exists:

    --table-mapping-json '{\"sourceSchemaName\":\"public\",
      \"sourceTable\":\"users\",\"targetTable\":\"users\",
      \"excludedColumns\":[\"ssn\"],\"sortingKeys\":[\"created_at\",\"id\"],
      \"partitionByExpr\":\"toYYYYMM(created_at)\",
      \"tableEngine\":\"ReplacingMergeTree\"}'

  sourceSchemaName, sourceTable and targetTable are required and must not be
  empty. excludedColumns, sortingKeys, useCustomSortingKey, partitionByExpr
  and partitionKey are optional. tableEngine is one of MergeTree,
  ReplacingMergeTree or Null, and defaults to MergeTree, which is what the
  simple form sends. partitionKey partitions the initial snapshot for
  parallelism and is unrelated to the destination table's PARTITION BY, which
  is partitionByExpr.
  useCustomSortingKey is set to true automatically when sortingKeys is given,
  because the API ignores the keys without it; passing false alongside keys is
  rejected rather than silently dropping them. An unknown field is rejected,
  so a typo such as excludeColumns fails instead of being ignored.
  Both flags are repeatable and may be given together: the --table-mapping
  values are sent first, then the --table-mapping-json ones, each in the order
  given.

POSTGRES CDC SETTINGS:
  Every CDC setting is applied when the pipe is created. The API can patch only
  syncIntervalSeconds and pullBatchSize afterwards, so the snapshot and
  initial-load settings (--initial-load-parallelism,
  --snapshot-rows-per-partition, --snapshot-parallel-tables,
  --allow-nullable-columns, --enable-failover-slots, --delete-on-merge) cannot
  be changed after creation. `clickpipe settings update` is a different
  endpoint for streaming and object-storage pipes and does not cover them.
  The boolean settings take an explicit true or false; omitting one sends
  false, which is the API default.

POSTGRES TLS:
  TLS and certificate verification are enabled by default. A source whose
  certificate chain is publicly trusted needs no CA file. For a private or
  self-signed source CA, pass its PEM CA bundle with --ca-certificate <PATH>.
  Certificate hostname verification uses --host unless --tls-host <HOSTNAME>
  overrides it.

PREREQUISITES:
  ClickPipes must be able to reach the source; allow the ClickPipes static IPs
  through its network controls. For CDC, enable logical replication, put every
  mapped table in the publication, and grant the source user the required
  schema, table, and replication privileges.

DOCUMENTATION:
  PostgreSQL ClickPipes setup:
    https://clickhouse.com/docs/integrations/clickpipes/postgres
  Generic PostgreSQL source setup:
    https://clickhouse.com/docs/integrations/clickpipes/postgres/source/generic
  ClickPipes networking and static IPs:
    https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips")]
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

/// Destination-permission flags shared by every `clickpipe create` subcommand.
/// Flattened into each create args struct so `--role` and its help text have a
/// single definition, the way `KafkaSourceFields` shares the Kafka source flags.
#[derive(Args, Debug, Default)]
pub struct DestinationRoleArgs {
    /// Extra ClickHouse role to grant the ClickPipes destination user (repeatable)
    ///
    /// When omitted, ClickPipes grants that user the default role only. Each
    /// --role adds a role on top of the default; nothing is taken away. The
    /// API-reserved names `clickpipes` and `clickpipes_system` are rejected.
    #[arg(long = "role", value_name = "ROLE", value_parser = parse_destination_role)]
    pub roles: Vec<String>,
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
}

#[derive(Args, Debug)]
pub struct ObjectStorageCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub source: ObjectStorageSourceFields,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64" --column "name:String")
    #[arg(long = "column")]
    pub columns: Vec<String>,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

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

    /// Authentication method (inferred from the credential flags when omitted;
    /// no authentication is sent when no credential flags are given)
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

    /// Path to client certificate file (for MUTUAL_TLS auth; requires --client-key)
    #[arg(long, requires = "client_key")]
    pub client_certificate: Option<String>,

    /// Path to client private key file (for MUTUAL_TLS auth; requires --client-certificate)
    #[arg(long, requires = "client_certificate")]
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

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

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

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

/// The two table-mapping flags are one required "at least one of" group, so
/// clap names both in the missing-argument error and either alone is enough.
#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("postgres_table_mappings")
        .required(true)
        .multiple(true)
        .args(["table_mappings", "table_mappings_json"])
))]
pub struct PostgresCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// PostgreSQL host
    #[arg(long)]
    pub host: String,

    /// PostgreSQL port (1-65535)
    #[arg(
        long,
        default_value = "5432",
        value_parser = clap::value_parser!(u16).range(1..=65535)
    )]
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
    ///
    /// One of --table-mapping or --table-mapping-json is required. This form
    /// leaves every other per-table option at the ClickPipes default.
    #[arg(
        long = "table-mapping",
        value_name = "SCHEMA.TABLE:TARGET_TABLE",
        value_parser = parse_postgres_table_mapping
    )]
    pub table_mappings: Vec<String>,

    /// Full table mapping as a JSON object (repeatable)
    ///
    /// Takes the API's table mapping object verbatim, for the per-table
    /// options the simple form cannot express: excludedColumns, sortingKeys
    /// with useCustomSortingKey, partitionByExpr, partitionKey and
    /// tableEngine. Can be combined with --table-mapping. Unknown fields are
    /// rejected. See POSTGRES TABLE MAPPINGS for the field list and an
    /// example.
    #[arg(long = "table-mapping-json", value_name = "JSON")]
    pub table_mappings_json: Vec<String>,

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

    /// IAM role ARN (required with --auth IAM_ROLE; invalid with basic auth)
    #[arg(long, required_if_eq("auth", "IAM_ROLE"))]
    pub iam_role: Option<String>,

    /// Certificate hostname override (defaults to --host)
    #[arg(long, value_name = "HOSTNAME")]
    pub tls_host: Option<String>,

    /// Path to a PEM CA bundle for a private or self-signed source certificate
    #[arg(long, value_name = "PATH")]
    pub ca_certificate: Option<String>,

    /// Postgres publication name
    #[arg(long)]
    pub publication_name: Option<String>,

    /// Replication slot name (only with --replication-mode cdc_only)
    #[arg(long)]
    pub replication_slot_name: Option<String>,

    /// Interval in seconds to sync data from Postgres during CDC replication
    #[arg(long, value_name = "SECONDS")]
    pub sync_interval_seconds: Option<i64>,

    /// Number of rows to pull in each batch during CDC replication
    #[arg(long, value_name = "ROWS")]
    pub pull_batch_size: Option<i64>,

    /// Parallel workers per table in the initial snapshot phase (create-time only)
    #[arg(long, value_name = "WORKERS")]
    pub initial_load_parallelism: Option<i64>,

    /// Number of rows per partition during the snapshot phase (create-time only)
    #[arg(long, value_name = "ROWS")]
    pub snapshot_rows_per_partition: Option<i64>,

    /// Tables to snapshot in parallel during the initial load phase (create-time only)
    #[arg(long, value_name = "TABLES")]
    pub snapshot_parallel_tables: Option<i64>,

    /// Preserve Postgres nullability in the destination table (create-time only)
    #[arg(long, value_name = "true|false")]
    pub allow_nullable_columns: Option<bool>,

    /// Enable failover for the replication slot on PG17 and newer, when
    /// ClickPipes creates the slot (create-time only)
    #[arg(long, value_name = "true|false")]
    pub enable_failover_slots: Option<bool>,

    /// Enable hard deletes in ReplacingMergeTree for Postgres DELETEs
    /// (create-time only)
    #[arg(long, value_name = "true|false")]
    pub delete_on_merge: Option<bool>,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

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

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

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

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

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

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

pub async fn run(client: &CloudClient, command: ClickPipeCommands, json: bool) -> CloudResult<()> {
    match command {
        ClickPipeCommands::List { service_id, org_id } => {
            clickpipe_list(client, &service_id, org_id.as_deref(), json).await
        }
        ClickPipeCommands::Get {
            service_id,
            clickpipe_id,
            org_id,
        } => clickpipe_get(client, &service_id, &clickpipe_id, org_id.as_deref(), json).await,
        ClickPipeCommands::Delete {
            service_id,
            clickpipe_id,
            org_id,
        } => clickpipe_delete(client, &service_id, &clickpipe_id, org_id.as_deref(), json).await,
        ClickPipeCommands::Start {
            service_id,
            clickpipe_id,
            org_id,
        } => {
            clickpipe_state(
                client,
                &service_id,
                &clickpipe_id,
                "start",
                org_id.as_deref(),
                json,
            )
            .await
        }
        ClickPipeCommands::Stop {
            service_id,
            clickpipe_id,
            org_id,
        } => {
            clickpipe_state(
                client,
                &service_id,
                &clickpipe_id,
                "stop",
                org_id.as_deref(),
                json,
            )
            .await
        }
        ClickPipeCommands::Resync {
            service_id,
            clickpipe_id,
            org_id,
        } => {
            clickpipe_state(
                client,
                &service_id,
                &clickpipe_id,
                "resync",
                org_id.as_deref(),
                json,
            )
            .await
        }
        ClickPipeCommands::Scale {
            service_id,
            clickpipe_id,
            replicas,
            cpu_millicores,
            memory_gb,
            org_id,
        } => {
            clickpipe_scale(
                client,
                &service_id,
                &clickpipe_id,
                replicas,
                cpu_millicores,
                memory_gb,
                org_id.as_deref(),
                json,
            )
            .await
        }
        ClickPipeCommands::Settings { command } => match command {
            ClickPipeSettingsCommands::Get {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                clickpipe_settings_get(client, &service_id, &clickpipe_id, org_id.as_deref(), json)
                    .await
            }
            ClickPipeSettingsCommands::Update {
                service_id,
                clickpipe_id,
                streaming_max_insert_wait_ms,
                object_storage_concurrency,
                object_storage_polling_interval_ms,
                object_storage_max_insert_bytes,
                object_storage_max_file_count,
                clickhouse_max_threads,
                clickhouse_max_insert_threads,
                object_storage_use_cluster_function,
                clickhouse_parallel_view_processing,
                org_id,
            } => {
                let values = ClickPipeSettingsValues {
                    streaming_max_insert_wait_ms,
                    object_storage_concurrency,
                    object_storage_polling_interval_ms,
                    object_storage_max_insert_bytes,
                    object_storage_max_file_count,
                    clickhouse_max_threads,
                    clickhouse_max_insert_threads,
                    object_storage_use_cluster_function,
                    clickhouse_parallel_view_processing,
                };
                clickpipe_settings_update(
                    client,
                    &service_id,
                    &clickpipe_id,
                    &values,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
        },
        ClickPipeCommands::SchemaDiscover {
            service_id,
            command,
            org_id,
        } => {
            clickpipe_schema_discover(client, &service_id, &command, org_id.as_deref(), json).await
        }
        ClickPipeCommands::Create { command } => match command {
            ClickPipeCreateCommands::ObjectStorage(args) => {
                clickpipe_create_object_storage(client, &args, json).await
            }
            ClickPipeCreateCommands::Kafka(args) => {
                clickpipe_create_kafka(client, &args, json).await
            }
            ClickPipeCreateCommands::Kinesis(args) => {
                clickpipe_create_kinesis(client, &args, json).await
            }
            ClickPipeCreateCommands::Postgres(args) => {
                clickpipe_create_postgres(client, &args, json).await
            }
            ClickPipeCreateCommands::MySQL(args) => {
                clickpipe_create_mysql(client, &args, json).await
            }
            ClickPipeCreateCommands::MongoDB(args) => {
                clickpipe_create_mongodb(client, &args, json).await
            }
            ClickPipeCreateCommands::BigQuery(args) => {
                clickpipe_create_bigquery(client, &args, json).await
            }
        },
    }
}

async fn clickpipe_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipes = client.list_clickpipes(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipes)?);
    } else if clickpipes.is_empty() {
        println!("No ClickPipes found");
    } else {
        println!("ClickPipes:");
        for clickpipe in &clickpipes {
            println!(
                "  {} ({}) - {}",
                or_absent(clickpipe.name.as_deref()),
                or_absent(clickpipe.id.as_ref()),
                or_absent(clickpipe.state.as_ref())
            );
        }
    }
    Ok(())
}

/// Build a `ClickPipePostObjectStorageSource` from the CLI args, inferring the
/// authentication mechanism from the credential flags and reading any GCP
/// service-account file up front so bad invocations fail fast before any
/// network call. Shared by the `clickpipe create object-storage` and
/// `clickpipe schema-discover <SERVICE_ID> object-storage` handlers.
fn build_object_storage_source(
    args: &ObjectStorageSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostObjectStorageSource> {
    use clickhouse_cloud_api::models::{
        ClickPipePostObjectStorageSource, ClickPipePostObjectStorageSourceAuthentication,
        MskIamUser,
    };

    let (authentication, iam_role_val, access_key) = match (
        args.iam_role.as_deref(),
        args.access_key_id.as_deref(),
        args.secret_key.as_deref(),
    ) {
        (Some(role), _, _) => (
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE),
            Some(role.to_string()),
            None,
        ),
        (_, Some(key_id), Some(secret)) => (
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER),
            None,
            Some(MskIamUser {
                access_key_id: key_id.to_string(),
                secret_key: secret.to_string(),
            }),
        ),
        _ => (None, None, None),
    };
    let authentication = authentication
        .or_else(|| {
            args.connection_string
                .as_ref()
                .map(|_| ClickPipePostObjectStorageSourceAuthentication::CONNECTION_STRING)
        })
        .or_else(|| {
            args.service_account_file
                .as_ref()
                .map(|_| ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT)
        });

    let service_account_key = match args.service_account_file.as_deref() {
        Some(path) => Some(read_gcp_service_account_file(path)?),
        None => None,
    };

    Ok(ClickPipePostObjectStorageSource {
        r#type: parse_enum(&args.storage_type)?,
        format: parse_enum(&args.format)?,
        url: args.source_url.clone(),
        compression: Some(parse_enum(&args.compression)?),
        is_continuous: if args.continuous { Some(true) } else { None },
        queue_url: args.queue_url.clone(),
        delimiter: args.delimiter.clone(),
        authentication,
        iam_role: iam_role_val,
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
    })
}

async fn clickpipe_create_object_storage(
    client: &CloudClient,
    args: &ObjectStorageCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_object_storage_source(&args.source)?;
    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            object_storage: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            &args.database,
            &args.table,
            parsed_columns,
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

/// Infer the Kafka authentication mechanism from the credential flags that were
/// passed when `--auth` is omitted. Returning `None` means "no authentication":
/// the spec enum has no value for an unauthenticated broker, so the request
/// omits `authentication` entirely (see `build_kafka_source`). Never default to
/// a mechanism the user did not ask for — that used to reject no-auth brokers
/// client-side with a bogus "PLAIN requires --username and --password".
fn infer_kafka_authentication(
    args: &KafkaSourceFields,
) -> Option<clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
    if args.username.is_some() && args.password.is_some() {
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
/// top-level `iamRole` field on the source, not through credentials. An absent
/// mechanism (no authentication) likewise sends a null body.
///
/// `mtls_contents` is the pre-read (certificate, privateKey) PEM bundle used
/// only for MUTUAL_TLS; the caller reads these from disk so this function
/// stays pure and testable.
fn build_kafka_credentials(
    authentication: Option<&clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication>,
    args: &KafkaSourceFields,
    mtls_contents: Option<(String, String)>,
) -> CloudResult<serde_json::Value> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
    let Some(authentication) = authentication else {
        return Ok(serde_json::Value::Null);
    };
    match authentication {
        Auth::PLAIN | Auth::SCRAM_SHA_256 | Auth::SCRAM_SHA_512 => {
            match (args.username.as_deref(), args.password.as_deref()) {
                (Some(username), Some(password)) => {
                    Ok(serde_json::json!({ "username": username, "password": password }))
                }
                _ => Err(CloudError::new(format!(
                    "{authentication} requires --username and --password"
                ))),
            }
        }
        Auth::IAM_USER => match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
            (Some(access_key_id), Some(secret_key)) => Ok(serde_json::json!({
                "accessKeyId": access_key_id,
                "secretKey": secret_key
            })),
            _ => Err(CloudError::new(
                "IAM_USER requires --access-key-id and --secret-key",
            )),
        },
        Auth::IAM_ROLE => {
            if args.iam_role.is_none() {
                Err(CloudError::new("IAM_ROLE requires --iam-role"))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Auth::MUTUAL_TLS => match mtls_contents {
            Some((certificate, private_key)) => Ok(serde_json::json!({
                "certificate": certificate,
                "privateKey": private_key
            })),
            None => Err(CloudError::new(
                "MUTUAL_TLS requires --client-certificate and --client-key",
            )),
        },
        Auth::Unknown(_) => Ok(serde_json::Value::Null),
    }
}

/// Build a `ClickPipePostKafkaSource` from the CLI args, performing all
/// authentication/credential/schema-registry/CA validation up front so bad
/// invocations fail fast before any network call. Shared by the
/// `clickpipe create kafka` and `clickpipe schema-discover <SERVICE_ID> kafka`
/// handlers.
fn build_kafka_source(
    args: &KafkaSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostKafkaSource> {
    use clickhouse_cloud_api::models::{
        ClickPipeKafkaOffset, ClickPipeKafkaSchemaRegistryCredentials,
        ClickPipeMutateKafkaSchemaRegistry, ClickPipePostKafkaSource,
        ClickPipePostKafkaSourceAuthentication,
    };

    // An explicit `--auth` wins; otherwise infer the mechanism from the
    // credential flags, and send no authentication at all when none were given
    // so brokers that require none are reachable.
    let authentication: Option<ClickPipePostKafkaSourceAuthentication> = match args.auth.as_deref()
    {
        Some(authentication) => Some(parse_enum(authentication)?),
        None => infer_kafka_authentication(args),
    };

    let mtls_cert_contents = match (
        &authentication,
        args.client_certificate.as_deref(),
        args.client_key.as_deref(),
    ) {
        (
            Some(ClickPipePostKafkaSourceAuthentication::MUTUAL_TLS),
            Some(cert_path),
            Some(key_path),
        ) => Some((
            std::fs::read_to_string(cert_path)?,
            std::fs::read_to_string(key_path)?,
        )),
        _ => None,
    };
    let credentials = build_kafka_credentials(authentication.as_ref(), args, mtls_cert_contents)?;

    let schema_registry = args
        .schema_registry_url
        .as_ref()
        .map(|url| -> CloudResult<_> {
            let credentials = match (
                args.schema_registry_username.as_deref(),
                args.schema_registry_password.as_deref(),
            ) {
                (Some(username), Some(password)) => ClickPipeKafkaSchemaRegistryCredentials {
                    username: username.to_string(),
                    password: password.to_string(),
                },
                _ => ClickPipeKafkaSchemaRegistryCredentials::default(),
            };
            let ca_certificate = match args.schema_registry_ca_certificate.as_deref() {
                Some(path) => Some(std::fs::read_to_string(path)?),
                None => None,
            };
            Ok(ClickPipeMutateKafkaSchemaRegistry {
                url: url.clone(),
                authentication: Default::default(),
                credentials,
                ca_certificate,
            })
        })
        .transpose()?;

    let ca_certificate = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    Ok(ClickPipePostKafkaSource {
        r#type: parse_enum(&args.kafka_type)?,
        format: parse_enum(&args.format)?,
        brokers: args.brokers.clone(),
        topics: args.topics.clone(),
        consumer_group: args.consumer_group.clone(),
        exactly_once: None,
        authentication,
        credentials,
        iam_role: args.iam_role.clone(),
        offset: Some(ClickPipeKafkaOffset {
            strategy: parse_enum(&args.offset)?,
            timestamp: args.offset_timestamp.clone(),
        }),
        schema_registry,
        ca_certificate,
        reverse_private_endpoint_ids: args.reverse_private_endpoint_ids.clone(),
    })
}

/// Build a `ClickPipePostKinesisSource` from the CLI args. Shared by the
/// `clickpipe create kinesis` and `clickpipe schema-discover <SERVICE_ID> kinesis`
/// handlers.
fn build_kinesis_source(
    args: &KinesisSourceFields,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostKinesisSource> {
    use clickhouse_cloud_api::models::{ClickPipePostKinesisSource, MskIamUser};

    let access_key = match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
        (Some(access_key_id), Some(secret_key)) => Some(MskIamUser {
            access_key_id: access_key_id.to_string(),
            secret_key: secret_key.to_string(),
        }),
        _ => None,
    };

    Ok(ClickPipePostKinesisSource {
        format: parse_enum(&args.format)?,
        stream_name: args.stream_name.clone(),
        region: args.region.clone(),
        authentication: parse_enum(&args.auth)?,
        iam_role: args.iam_role.clone(),
        access_key,
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
                    CloudError::new(format!("--iterator-timestamp {timestamp} is out of range"))
                })
            })
            .transpose()?,
    })
}

async fn clickpipe_create_kafka(
    client: &CloudClient,
    args: &KafkaCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kafka_source(&args.source)?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kafka: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            &args.database,
            &args.table,
            parsed_columns,
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

async fn clickpipe_create_kinesis(
    client: &CloudClient,
    args: &KinesisCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kinesis_source(&args.source)?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kinesis: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            &args.database,
            &args.table,
            parsed_columns,
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    };

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
fn build_object_storage_schema_discovery_request(
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

/// Discover the inferred schema for a Kafka, Kinesis or object-storage source
/// without creating a ClickPipe (Beta). Side-effect-free, but the API gateway
/// rejects OAuth/Bearer on this POST endpoint, so it is classified as a write
/// command and requires API key auth.
async fn clickpipe_schema_discover(
    client: &CloudClient,
    service_id: &str,
    command: &ClickPipeSchemaDiscoverCommands,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickPipeSchemaDiscoveryRequest, ClickPipeSchemaDiscoverySource,
    };

    let request = match command {
        ClickPipeSchemaDiscoverCommands::Kafka(args) => ClickPipeSchemaDiscoveryRequest {
            source: ClickPipeSchemaDiscoverySource {
                kafka: Some(build_kafka_source(args)?),
                kinesis: None,
                object_storage: None,
                pubsub: None,
            },
        },
        ClickPipeSchemaDiscoverCommands::Kinesis(args) => ClickPipeSchemaDiscoveryRequest {
            source: ClickPipeSchemaDiscoverySource {
                kafka: None,
                kinesis: Some(build_kinesis_source(args)?),
                object_storage: None,
                pubsub: None,
            },
        },
        ClickPipeSchemaDiscoverCommands::ObjectStorage(args) => {
            build_object_storage_schema_discovery_request(args)?
        }
    };
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client
        .click_pipe_schema_discovery(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Type")]
            r#type: String,
            #[tabled(rename = "Optional")]
            optional: String,
        }
        let rows: Vec<Row> = response
            .fields
            .unwrap_or_default()
            .into_iter()
            .map(|field| Row {
                name: or_absent(field.name),
                r#type: or_absent(field.r#type),
                optional: match field.optional {
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                    None => "".to_string(),
                },
            })
            .collect();
        if rows.is_empty() {
            println!("No fields discovered");
        } else {
            println!("{}", Table::new(rows).with(Style::markdown()));
        }
    }
    Ok(())
}

async fn clickpipe_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        print_human(&clickpipe)?;
    }
    Ok(())
}

async fn clickpipe_delete(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    client
        .delete_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::json!({ "deleted": clickpipe_id }));
    } else {
        println!("ClickPipe {} deleted", clickpipe_id);
    }
    Ok(())
}

async fn clickpipe_state(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    command: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand;
    let command_value = match command {
        "start" => ClickPipeStatePatchRequestCommand::Start,
        "stop" => ClickPipeStatePatchRequestCommand::Stop,
        "resync" => ClickPipeStatePatchRequestCommand::Resync,
        other => {
            return Err(CloudError::new(format!("Unknown state command: {}", other)));
        }
    };
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipe = client
        .change_clickpipe_state(&org_id, service_id, clickpipe_id, command_value)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!(
            "ClickPipe {} {} (state: {})",
            or_absent(clickpipe.name.as_deref()),
            command,
            or_absent(clickpipe.state.as_ref())
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn clickpipe_scale(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    replicas: Option<u32>,
    cpu_millicores: Option<u32>,
    memory_gb: Option<f64>,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = clickhouse_cloud_api::models::ClickPipeScalingPatchRequest {
        replicas: replicas.map(i64::from),
        replica_cpu_millicores: cpu_millicores.map(i64::from),
        replica_memory_gb: memory_gb,
        #[cfg(feature = "deprecated-fields")]
        concurrency: None,
    };
    let clickpipe = client
        .update_clickpipe_scaling(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        let scaling = clickpipe.scaling.unwrap_or_default();
        println!(
            "ClickPipe {} scaling updated",
            or_absent(clickpipe.name.as_deref())
        );
        println!("  Replicas: {}", or_absent(scaling.replicas));
        println!("  CPU: {}m", or_absent(scaling.replica_cpu_millicores));
        println!("  Memory: {} GB", or_absent(scaling.replica_memory_gb));
    }
    Ok(())
}

async fn clickpipe_settings_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    // The settings endpoint only exists for streaming and object-storage pipes,
    // so the pipe is fetched first to classify its source and refuse a database
    // CDC pipe with an applicability error rather than the API's NOT_FOUND.
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;
    ensure_clickpipe_has_ingestion_settings(&clickpipe, service_id, clickpipe_id)?;
    let settings = client
        .get_clickpipe_settings(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        print_human(&settings)?;
    }
    Ok(())
}

/// The settings a `clickpipe settings update` invocation carries, decoupled from
/// clap so the request builder can be unit-tested.
///
/// Every field is source-agnostic: each one is only sent when the user passed
/// the matching flag, so the API validates applicability per source. Kafka-only
/// settings are not here — they are resolved from the pipe itself (see
/// [`build_clickpipe_settings_request`]).
#[derive(Debug, Clone, Default, PartialEq)]
struct ClickPipeSettingsValues {
    streaming_max_insert_wait_ms: Option<u32>,
    object_storage_concurrency: Option<u32>,
    object_storage_polling_interval_ms: Option<u32>,
    object_storage_max_insert_bytes: Option<u64>,
    object_storage_max_file_count: Option<u32>,
    clickhouse_max_threads: Option<u32>,
    clickhouse_max_insert_threads: Option<u32>,
    object_storage_use_cluster_function: Option<bool>,
    clickhouse_parallel_view_processing: Option<bool>,
}

/// Which source a fetched pipe reads from, as a closed vocabulary.
///
/// Two decisions hang off this: whether the Kafka-only `kafka_read_committed`
/// setting may appear in a settings PUT at all, and whether the ingestion
/// settings endpoints exist for the pipe in the first place. The endpoints are
/// only implemented for streaming and object-storage pipes, so a database CDC
/// pipe gets `NOT_FOUND: ingestion for pipe "<id>" not found`, which reads like
/// the pipe is gone (#643).
///
/// [`ClickPipeSourceKind::Absent`] covers a response that carries no `source`
/// (or an unrecognized source arm). Both settings commands proceed in that
/// case: the API is then the authority, which is the safe direction because
/// refusing locally on a shape the CLI does not understand would block a pipe
/// the endpoint does serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickPipeSourceKind {
    Kafka,
    Kinesis,
    PubSub,
    ObjectStorage,
    Postgres,
    MySql,
    MongoDb,
    BigQuery,
    Absent,
}

impl ClickPipeSourceKind {
    /// Kafka is the only source whose settings include `kafka_read_committed`.
    fn is_kafka(self) -> bool {
        self == Self::Kafka
    }

    /// The human label for a database CDC source, or `None` for a source that
    /// does have ingestion settings.
    ///
    /// The label is a literal owned by this enum, never anything read out of an
    /// API response body, so no response text can reach the error message.
    fn database_source_label(self) -> Option<&'static str> {
        match self {
            Self::Postgres => Some("Postgres CDC"),
            Self::MySql => Some("MySQL CDC"),
            Self::MongoDb => Some("MongoDB CDC"),
            Self::BigQuery => Some("BigQuery"),
            Self::Kafka | Self::Kinesis | Self::PubSub | Self::ObjectStorage | Self::Absent => None,
        }
    }
}

/// Classify the source of a fetched pipe.
///
/// Exactly one arm of `source` is populated in practice. The streaming and
/// object-storage arms are matched first anyway, so a response that somehow
/// sets several arms resolves to the kind that keeps the settings commands
/// working rather than to a refusal.
fn classify_clickpipe_source(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
) -> ClickPipeSourceKind {
    let Some(source) = clickpipe.source.as_ref() else {
        return ClickPipeSourceKind::Absent;
    };
    if source.kafka.is_some() {
        ClickPipeSourceKind::Kafka
    } else if source.kinesis.is_some() {
        ClickPipeSourceKind::Kinesis
    } else if source.pubsub.is_some() {
        ClickPipeSourceKind::PubSub
    } else if source.object_storage.is_some() {
        ClickPipeSourceKind::ObjectStorage
    } else if source.postgres.is_some() {
        ClickPipeSourceKind::Postgres
    } else if source.mysql.is_some() {
        ClickPipeSourceKind::MySql
    } else if source.mongodb.is_some() {
        ClickPipeSourceKind::MongoDb
    } else if source.bigquery.is_some() {
        ClickPipeSourceKind::BigQuery
    } else {
        ClickPipeSourceKind::Absent
    }
}

/// Refuse a `clickpipe settings` command that cannot apply to this pipe.
///
/// The ingestion settings endpoints exist for streaming and object-storage
/// pipes only. Calling them for a database CDC pipe returns a bare `NOT_FOUND`
/// about the pipe, so the CLI classifies the source first and explains
/// applicability instead of relaying that (#643).
fn ensure_clickpipe_has_ingestion_settings(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
    service_id: &str,
    clickpipe_id: &str,
) -> CloudResult<()> {
    let Some(label) = classify_clickpipe_source(clickpipe).database_source_label() else {
        return Ok(());
    };
    Err(CloudError::new(format!(
        "ClickPipe {clickpipe_id} is a {label} pipe; `clickpipe settings get` and \
         `settings update` apply only to streaming (Kafka, Kinesis) and object-storage \
         pipes. CDC pipe settings (sync interval, pull batch size) live on the pipe \
         itself: see `clickhousectl cloud clickpipe get {service_id} {clickpipe_id}`."
    )))
}

/// Build the settings PUT body from the flags the user passed.
///
/// `kafka_read_committed` is Kafka-only: the API rejects the key for every other
/// source, so callers pass `None` for a non-Kafka pipe and the pipe's current
/// value for a Kafka pipe (the PUT would otherwise reset it).
fn build_clickpipe_settings_request(
    values: &ClickPipeSettingsValues,
    kafka_read_committed: Option<bool>,
) -> clickhouse_cloud_api::models::ClickPipeSettingsPutRequest {
    clickhouse_cloud_api::models::ClickPipeSettingsPutRequest {
        streaming_max_insert_wait_ms: values.streaming_max_insert_wait_ms.map(i64::from),
        object_storage_concurrency: values.object_storage_concurrency.map(i64::from),
        object_storage_polling_interval_ms: values
            .object_storage_polling_interval_ms
            .map(i64::from),
        object_storage_max_insert_bytes: values
            .object_storage_max_insert_bytes
            .map(|value| value as i64),
        object_storage_max_file_count: values.object_storage_max_file_count.map(i64::from),
        clickhouse_max_threads: values.clickhouse_max_threads.map(i64::from),
        clickhouse_max_insert_threads: values.clickhouse_max_insert_threads.map(i64::from),
        object_storage_use_cluster_function: values.object_storage_use_cluster_function,
        clickhouse_parallel_view_processing: values.clickhouse_parallel_view_processing,
        kafka_read_committed,
        clickhouse_max_download_threads: None,
        clickhouse_min_insert_block_size_bytes: None,
        clickhouse_parallel_distributed_insert_select: None,
    }
}

async fn clickpipe_settings_update(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    values: &ClickPipeSettingsValues,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    // The source decides which settings may appear in the body at all: sending
    // `kafka_read_committed` for a non-Kafka pipe fails the entire request, so
    // the pipe is fetched to classify it, and its current value is only read
    // back (a PUT that omits it would reset it) for a Kafka pipe. The same
    // classification refuses a database CDC pipe, whose settings endpoint does
    // not exist at all.
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;
    ensure_clickpipe_has_ingestion_settings(&clickpipe, service_id, clickpipe_id)?;
    let kafka_read_committed = if classify_clickpipe_source(&clickpipe).is_kafka() {
        Some(
            client
                .get_clickpipe_settings(&org_id, service_id, clickpipe_id)
                .await?
                .kafka_read_committed
                .unwrap_or(false),
        )
    } else {
        None
    };
    let request = build_clickpipe_settings_request(values, kafka_read_committed);
    let settings = client
        .update_clickpipe_settings(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        println!("ClickPipe settings updated");
        let value = serde_json::to_value(&settings)?;
        if let Some(object) = value.as_object() {
            for (key, value) in object {
                if !value.is_null() {
                    println!("  {}: {}", key, value);
                }
            }
        }
    }
    Ok(())
}

/// Parse a CLI string into a library enum. Library enums have a
/// `#[serde(untagged)] Unknown(String)` variant so unknown inputs are
/// forwarded to the API (which returns the canonical validation error).
fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> CloudResult<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|error| CloudError::new(format!("invalid value '{}': {}", value, error)))
}

/// Parse `name:type` column specifications into library destination columns.
fn parse_columns(
    columns: &[String],
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
    columns
        .iter()
        .map(|column| {
            let (name, column_type) = column.split_once(':').ok_or_else(|| {
                CloudError::new(format!(
                    "Invalid column format '{}': expected name:type",
                    column
                ))
            })?;
            Ok(clickhouse_cloud_api::models::ClickPipeDestinationColumn {
                name: name.to_string(),
                r#type: column_type.to_string(),
            })
        })
        .collect()
}

/// Role names the API reserves for ClickPipes itself and rejects in
/// `destination.roles`. Caught client-side so the failure names the flag
/// instead of relaying a server-side validation error.
const RESERVED_DESTINATION_ROLES: &[&str] = &["clickpipes", "clickpipes_system"];

/// Validate one `--role` value, returning the trimmed role name.
fn parse_destination_role_name(role: &str) -> CloudResult<String> {
    let trimmed = role.trim();
    if trimmed.is_empty() {
        return Err(CloudError::new("a role name must not be empty"));
    }
    if RESERVED_DESTINATION_ROLES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(trimmed))
    {
        return Err(CloudError::new(format!(
            "role names reserved by ClickPipes cannot be granted: {}",
            RESERVED_DESTINATION_ROLES.join(", ")
        )));
    }
    Ok(trimmed.to_string())
}

/// clap `value_parser` wrapper for `--role`, so a bad name is a usage error
/// (exit 2) formatted against the create subcommand that was invoked, before
/// any credential lookup or network call.
fn parse_destination_role(role: &str) -> Result<String, String> {
    parse_destination_role_name(role).map_err(|error| error.message)
}

/// Resolve the repeatable `--role` values into `destination.roles`. Every value
/// is already validated and trimmed by `parse_destination_role`; duplicates are
/// dropped with declaration order preserved, and no flags at all leave the
/// field absent from the request body.
fn build_destination_roles(roles: &[String]) -> Option<Vec<String>> {
    if roles.is_empty() {
        return None;
    }
    let mut deduped: Vec<String> = Vec::with_capacity(roles.len());
    for role in roles {
        if !deduped.iter().any(|existing| existing == role) {
            deduped.push(role.clone());
        }
    }
    Some(deduped)
}

/// Build a managed-table destination with the default MergeTree engine.
fn build_destination(
    database: &str,
    table: &str,
    columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>,
    roles: Option<Vec<String>>,
) -> clickhouse_cloud_api::models::ClickPipeMutateDestination {
    // Database pipes (Postgres/MySQL/BigQuery) carry the destination table on
    // the per-mapping `targetTable` and reject any of {table, managedTable,
    // tableDefinition, columns} at the top level. Detect that case via empty
    // `table` and emit a destination with only `database` populated.
    if table.is_empty() {
        return clickhouse_cloud_api::models::ClickPipeMutateDestination {
            database: database.to_string(),
            // `roles` is not one of the four fields database pipes reject, so
            // it is wired here too.
            roles,
            ..Default::default()
        };
    }
    clickhouse_cloud_api::models::ClickPipeMutateDestination {
        database: database.to_string(),
        table: Some(table.to_string()),
        columns,
        managed_table: Some(true),
        roles,
        table_definition: Some(
            clickhouse_cloud_api::models::ClickPipeDestinationTableDefinition::default(),
        ),
    }
}

/// Read a GCP service-account JSON key file from disk and return the
/// base64-encoded contents. Used by both the object-storage and BigQuery
/// `create` handlers — the upstream API wants the encoded blob regardless
/// of which source it ends up on.
fn read_gcp_service_account_file(path: &str) -> CloudResult<String> {
    let contents = std::fs::read_to_string(path)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        contents.as_bytes(),
    ))
}

/// Print the standard "created" confirmation for any create_* handler.
fn print_created(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
    json: bool,
) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", or_absent(clickpipe.name.as_deref()));
        println!("  ID: {}", or_absent(clickpipe.id.as_ref()));
        println!("  State: {}", or_absent(clickpipe.state.as_ref()));
    }
    Ok(())
}

/// Parse `schema.table:target_table` mappings into (schema, table, target) tuples.
/// Source-specific handlers map these into their own TableMapping struct.
fn parse_db_table_mappings(mappings: &[String]) -> CloudResult<Vec<(String, String, String)>> {
    mappings
        .iter()
        .map(|mapping| {
            let (source, target) = mapping.split_once(':').ok_or_else(|| {
                CloudError::new(format!(
                    "Invalid table mapping '{}': expected schema.table:target_table",
                    mapping
                ))
            })?;
            let (schema, table) = source.split_once('.').ok_or_else(|| {
                CloudError::new(format!(
                    "Invalid source '{}': expected schema.table",
                    source
                ))
            })?;
            Ok((schema.to_string(), table.to_string(), target.to_string()))
        })
        .collect()
}

fn parse_postgres_table_mapping_parts(mapping: &str) -> CloudResult<(String, String, String)> {
    let (source, target) = mapping.split_once(':').ok_or_else(|| {
        CloudError::new(format!(
            "invalid table mapping '{}': expected schema.table:target_table",
            mapping
        ))
    })?;
    let (schema, table) = source.split_once('.').ok_or_else(|| {
        CloudError::new(format!(
            "invalid table mapping '{}': expected schema.table:target_table",
            mapping
        ))
    })?;
    if schema.trim().is_empty() {
        return Err(CloudError::new(format!(
            "invalid table mapping '{}': source schema must not be empty",
            mapping
        )));
    }
    if table.trim().is_empty() {
        return Err(CloudError::new(format!(
            "invalid table mapping '{}': source table must not be empty",
            mapping
        )));
    }
    if target.trim().is_empty() {
        return Err(CloudError::new(format!(
            "invalid table mapping '{}': target table must not be empty",
            mapping
        )));
    }

    Ok((
        schema.trim().to_string(),
        table.trim().to_string(),
        target.trim().to_string(),
    ))
}

fn parse_postgres_table_mapping(mapping: &str) -> Result<String, String> {
    parse_postgres_table_mapping_parts(mapping)
        .map(|_| mapping.to_string())
        .map_err(|error| error.message)
}

/// Wire field names `ClickPipePostgresPipeTableMapping` accepts, in the order
/// the help text and the error messages list them.
const POSTGRES_TABLE_MAPPING_JSON_FIELDS: &[&str] = &[
    "sourceSchemaName",
    "sourceTable",
    "targetTable",
    "excludedColumns",
    "sortingKeys",
    "useCustomSortingKey",
    "partitionByExpr",
    "partitionKey",
    "tableEngine",
];

/// One `--table-mapping-json` value, before validation.
///
/// The library's `ClickPipePostgresPipeTableMapping` is a request type and so
/// strict: every field is required, which a hand-written mapping object is not
/// expected to spell out. This mirror is all-`Option` so absence is
/// representable, and each field is then resolved explicitly. It carries no
/// `deny_unknown_fields`: unknown fields are reported against
/// `POSTGRES_TABLE_MAPPING_JSON_FIELDS` instead, so the diagnostic names the
/// wire fields rather than serde's view of this struct.
#[derive(serde::Deserialize)]
struct PostgresTableMappingJson {
    #[serde(rename = "sourceSchemaName")]
    source_schema_name: Option<String>,
    #[serde(rename = "sourceTable")]
    source_table: Option<String>,
    #[serde(rename = "targetTable")]
    target_table: Option<String>,
    #[serde(rename = "excludedColumns")]
    excluded_columns: Option<Vec<String>>,
    #[serde(rename = "sortingKeys")]
    sorting_keys: Option<Vec<String>>,
    #[serde(rename = "useCustomSortingKey")]
    use_custom_sorting_key: Option<bool>,
    #[serde(rename = "partitionByExpr")]
    partition_by_expr: Option<String>,
    #[serde(rename = "partitionKey")]
    partition_key: Option<String>,
    #[serde(rename = "tableEngine")]
    table_engine: Option<String>,
}

/// Parse and validate one `--table-mapping-json` value into the library's
/// table mapping. `position` is the flag's zero-based occurrence, so an error
/// names the offending value instead of the whole request.
fn parse_postgres_table_mapping_json(
    position: usize,
    raw: &str,
) -> CloudResult<ClickPipePostgresPipeTableMapping> {
    let flag = format!("--table-mapping-json #{}", position + 1);
    let invalid = |detail: String| CloudError::new(format!("{flag}: {detail}"));
    let fields = POSTGRES_TABLE_MAPPING_JSON_FIELDS.join(", ");

    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| invalid(format!("invalid JSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("expected a JSON object with the fields {fields}")))?;
    let unknown: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| !POSTGRES_TABLE_MAPPING_JSON_FIELDS.contains(key))
        .collect();
    if !unknown.is_empty() {
        return Err(invalid(format!(
            "unknown field{} {}; valid fields are {fields}",
            if unknown.len() == 1 { "" } else { "s" },
            unknown.join(", "),
        )));
    }
    let mapping: PostgresTableMappingJson =
        serde_json::from_value(value).map_err(|error| invalid(error.to_string()))?;

    let required = |field: &str, value: Option<String>| -> CloudResult<String> {
        let value = value.unwrap_or_default();
        if value.trim().is_empty() {
            return Err(invalid(format!(
                "{field} is required and must not be empty"
            )));
        }
        Ok(value.trim().to_string())
    };
    let entries = |field: &str, values: Option<Vec<String>>| -> CloudResult<Vec<String>> {
        values
            .unwrap_or_default()
            .into_iter()
            .map(|entry| {
                if entry.trim().is_empty() {
                    return Err(invalid(format!("{field} must not contain an empty entry")));
                }
                Ok(entry.trim().to_string())
            })
            .collect()
    };

    let source_schema_name = required("sourceSchemaName", mapping.source_schema_name)?;
    let source_table = required("sourceTable", mapping.source_table)?;
    let target_table = required("targetTable", mapping.target_table)?;
    let excluded_columns = entries("excludedColumns", mapping.excluded_columns)?;
    let sorting_keys = entries("sortingKeys", mapping.sorting_keys)?;
    // The API applies `sortingKeys` only when `useCustomSortingKey` is true,
    // so keys on their own would be silently ignored: turn the flag on for the
    // caller, and reject the two spellings that contradict each other.
    let use_custom_sorting_key = match mapping.use_custom_sorting_key {
        Some(true) if sorting_keys.is_empty() => {
            return Err(invalid(
                "useCustomSortingKey is true but sortingKeys is empty; list the destination \
                 ORDER BY columns in sortingKeys"
                    .to_string(),
            ));
        }
        Some(false) if !sorting_keys.is_empty() => {
            return Err(invalid(
                "sortingKeys is set but useCustomSortingKey is false, which would ignore the \
                 keys; omit useCustomSortingKey or set it to true"
                    .to_string(),
            ));
        }
        Some(explicit) => explicit,
        None => !sorting_keys.is_empty(),
    };
    let table_engine = match mapping.table_engine {
        Some(engine) => parse_serde_enum(
            &engine,
            "tableEngine",
            ClickPipePostgresPipeTableMappingTableengine::VALUES,
        )
        .map_err(|error| invalid(error.message))?,
        None => ClickPipePostgresPipeTableMappingTableengine::default(),
    };

    Ok(ClickPipePostgresPipeTableMapping {
        source_schema_name,
        source_table,
        target_table,
        excluded_columns,
        sorting_keys,
        use_custom_sorting_key,
        partition_by_expr: mapping.partition_by_expr.unwrap_or_default(),
        partition_key: mapping.partition_key.unwrap_or_default(),
        table_engine,
    })
}

/// Validate the cross-flag rules and resolve every table mapping, from both
/// `--table-mapping` and `--table-mapping-json`, before any request is made.
fn validate_postgres_create_args(
    args: &PostgresCreateArgs,
) -> CloudResult<Vec<ClickPipePostgresPipeTableMapping>> {
    if args.port == 0 {
        return Err(CloudError::new("--port must be in the range 1..=65535"));
    }
    if args.table_mappings.is_empty() && args.table_mappings_json.is_empty() {
        return Err(CloudError::new(
            "at least one --table-mapping <SCHEMA.TABLE:TARGET_TABLE> or \
             --table-mapping-json <JSON> is required",
        ));
    }
    if args.auth == "IAM_ROLE" && args.iam_role.is_none() {
        return Err(CloudError::new(
            "--auth IAM_ROLE requires --iam-role <IAM_ROLE>",
        ));
    }
    if args.auth == "basic" && args.iam_role.is_some() {
        return Err(CloudError::new(
            "--iam-role cannot be used with --auth basic; use --auth IAM_ROLE",
        ));
    }
    if args.replication_slot_name.is_some() && args.replication_mode != "cdc_only" {
        return Err(CloudError::new(
            "--replication-slot-name can only be used with --replication-mode cdc_only",
        ));
    }

    // The simple mappings are sent first, then the JSON ones, each in the
    // order given: clap's derive API does not expose argv indices, so
    // interleaving the two flags is not observable here.
    let mut mappings =
        Vec::with_capacity(args.table_mappings.len() + args.table_mappings_json.len());
    for mapping in &args.table_mappings {
        let (source_schema_name, source_table, target_table) =
            parse_postgres_table_mapping_parts(mapping)?;
        mappings.push(ClickPipePostgresPipeTableMapping {
            source_schema_name,
            source_table,
            target_table,
            ..Default::default()
        });
    }
    for (position, mapping) in args.table_mappings_json.iter().enumerate() {
        mappings.push(parse_postgres_table_mapping_json(position, mapping)?);
    }

    Ok(mappings)
}

/// Build the Postgres pipe settings sent at create time.
///
/// Every field the API models is wired here, because the settings are
/// create-time decisions: `ClickPipePatchPostgresPipeSettings` can only patch
/// `syncIntervalSeconds` and `pullBatchSize`, so anything else not set now can
/// never be applied to the pipe.
///
/// `allowNullableColumns`, `deleteOnMerge` and `enableFailoverSlots` are
/// required by the schema and therefore always serialized: absence is not
/// representable on the wire, so an omitted flag sends `false` — the request
/// shape the CLI has always sent. Every other setting is omitted when its flag
/// is omitted, via `skip_serializing_if`.
fn build_postgres_pipe_settings(
    args: &PostgresCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostgresPipeSettings> {
    Ok(
        clickhouse_cloud_api::models::ClickPipePostgresPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            publication_name: args.publication_name.clone(),
            replication_slot_name: args.replication_slot_name.clone(),
            sync_interval_seconds: args.sync_interval_seconds,
            pull_batch_size: args.pull_batch_size,
            initial_load_parallelism: args.initial_load_parallelism,
            snapshot_num_rows_per_partition: args.snapshot_rows_per_partition,
            snapshot_number_of_parallel_tables: args.snapshot_parallel_tables,
            allow_nullable_columns: args.allow_nullable_columns.unwrap_or(false),
            delete_on_merge: args.delete_on_merge.unwrap_or(false),
            enable_failover_slots: args.enable_failover_slots.unwrap_or(false),
        },
    )
}

fn build_postgres_request(
    args: &PostgresCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutatePostgresSource, ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let table_mappings = validate_postgres_create_args(args)?;
    let ca_certificate = args
        .ca_certificate
        .as_deref()
        .map(std::fs::read_to_string)
        .transpose()?;
    let source = ClickPipeMutatePostgresSource {
        r#type: Some(parse_enum(&args.postgres_type)?),
        credentials: PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        },
        host: args.host.clone(),
        port: i64::from(args.port),
        database: args.pg_database.clone(),
        disable_tls: false,
        skip_cert_verification: false,
        authentication: parse_enum(&args.auth)?,
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate,
        settings: build_postgres_pipe_settings(args)?,
        table_mappings,
    };

    Ok(ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            postgres: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            "default",
            "",
            vec![],
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    })
}

fn postgres_tls_error_hint(message: &str) -> Option<&'static str> {
    let message = message.to_ascii_lowercase();

    if message.contains("x509: certificate signed by unknown authority") {
        return Some(
            "The source certificate chain is not publicly trusted. For a private or \
             self-signed source CA, pass its PEM CA bundle with \
             `--ca-certificate <PATH>`.",
        );
    }

    if (message.contains("x509: certificate is valid for ") && message.contains(", not "))
        || (message.contains("x509: cannot validate certificate for ")
            && message.contains("because it doesn't contain any ip sans"))
    {
        return Some(
            "The source certificate does not match `--host`. Pass the certificate's \
             hostname with `--tls-host <HOSTNAME>`.",
        );
    }

    None
}

fn add_postgres_tls_error_hint(mut error: CloudError) -> CloudError {
    if let Some(hint) = postgres_tls_error_hint(&error.message) {
        error.message.push_str("\n\nHint: ");
        error.message.push_str(hint);
    }
    error
}

async fn clickpipe_create_postgres(
    client: &CloudClient,
    args: &PostgresCreateArgs,
    json: bool,
) -> CloudResult<()> {
    let request = build_postgres_request(args)?;
    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await
        .map_err(add_postgres_tls_error_hint)?;
    print_created(&clickpipe, json)?;
    Ok(())
}

async fn clickpipe_create_mysql(
    client: &CloudClient,
    args: &MySqlCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutateMySQLSource, ClickPipeMySQLPipeSettings, ClickPipeMySQLPipeTableMapping,
        ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let mappings = parse_db_table_mappings(&args.table_mappings)?;

    let ca_certificate = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let table_mappings = mappings
        .into_iter()
        .map(
            |(source_schema_name, source_table, target_table)| ClickPipeMySQLPipeTableMapping {
                source_schema_name,
                source_table,
                target_table,
                ..Default::default()
            },
        )
        .collect();

    let source = ClickPipeMutateMySQLSource {
        r#type: Some(parse_enum(&args.mysql_type)?),
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        host: args.host.clone(),
        port: i64::from(args.port),
        authentication: Some(parse_enum(&args.auth)?),
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: if args.skip_cert_verification {
            Some(true)
        } else {
            None
        },
        server_id: args.server_id.map(|value| value as i64),
        settings: ClickPipeMySQLPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            replication_mechanism: Some(parse_enum(&args.replication_mechanism)?),
            ..Default::default()
        },
        table_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mysql: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            "default",
            "",
            vec![],
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

async fn clickpipe_create_mongodb(
    client: &CloudClient,
    args: &MongoDbCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickPipeMongoDBPipeSettings, ClickPipeMongoDBPipeTableMapping,
        ClickPipeMutateMongoDBSource, ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;

    // MongoDB uses `database.collection:target_table` format.
    let table_mappings: Vec<ClickPipeMongoDBPipeTableMapping> = args
        .table_mappings
        .iter()
        .map(|mapping| {
            let (source, target_table) = mapping.split_once(':').ok_or_else(|| {
                CloudError::new(format!(
                    "Invalid table mapping '{}': expected database.collection:target_table",
                    mapping
                ))
            })?;
            let (source_database_name, source_collection) =
                source.split_once('.').ok_or_else(|| {
                    CloudError::new(format!(
                        "Invalid source '{}': expected database.collection",
                        source
                    ))
                })?;
            Ok(ClickPipeMongoDBPipeTableMapping {
                source_database_name: source_database_name.to_string(),
                source_collection: source_collection.to_string(),
                target_table: target_table.to_string(),
                table_engine: None,
            })
        })
        .collect::<CloudResult<Vec<_>>>()?;

    let ca_certificate = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let source = ClickPipeMutateMongoDBSource {
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        uri: args.uri.clone(),
        read_preference: parse_enum(&args.read_preference)?,
        tls_host: args.tls_host.clone(),
        ca_certificate,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: None,
        settings: ClickPipeMongoDBPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            ..Default::default()
        },
        table_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mongodb: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            "default",
            "",
            vec![],
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

async fn clickpipe_create_bigquery(
    client: &CloudClient,
    args: &BigQueryCreateArgs,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickPipeBigQueryPipeSettings, ClickPipeBigQueryPipeTableMapping,
        ClickPipeMutateBigQuerySource, ClickPipePostRequest, ClickPipePostSource, ServiceAccount,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let service_account_file = read_gcp_service_account_file(&args.service_account_file)?;

    // BigQuery uses `dataset.table:target_table` format.
    let table_mappings: Vec<ClickPipeBigQueryPipeTableMapping> = args
        .table_mappings
        .iter()
        .map(|mapping| {
            let (source, target_table) = mapping.split_once(':').ok_or_else(|| {
                CloudError::new(format!(
                    "Invalid table mapping '{}': expected dataset.table:target_table",
                    mapping
                ))
            })?;
            let (source_dataset_name, source_table) = source.split_once('.').ok_or_else(|| {
                CloudError::new(format!(
                    "Invalid source '{}': expected dataset.table",
                    source
                ))
            })?;
            Ok(ClickPipeBigQueryPipeTableMapping {
                source_dataset_name: source_dataset_name.to_string(),
                source_table: source_table.to_string(),
                target_table: target_table.to_string(),
                ..Default::default()
            })
        })
        .collect::<CloudResult<Vec<_>>>()?;

    let source = ClickPipeMutateBigQuerySource {
        credentials: ServiceAccount {
            service_account_file,
        },
        snapshot_staging_path: args.staging_path.clone(),
        settings: ClickPipeBigQueryPipeSettings {
            replication_mode: parse_enum("snapshot")?,
            ..Default::default()
        },
        table_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            bigquery: Some(source),
            ..Default::default()
        },
        destination: build_destination(
            "default",
            "",
            vec![],
            build_destination_roles(&args.destination_roles.roles),
        ),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

impl CloudClient {
    pub async fn list_clickpipes(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::ClickPipe>> {
        let response = self
            .api()
            .click_pipe_get_list(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .api()
            .click_pipe_get(org_id, service_id, clickpipe_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .api()
            .click_pipe_create(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> crate::cloud::client::Result<crate::cloud::types::DeleteResponse> {
        let response = self
            .api()
            .click_pipe_delete(org_id, service_id, clickpipe_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(crate::cloud::types::DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn change_clickpipe_state(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        command: clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        use clickhouse_cloud_api::models::ClickPipeStatePatchRequest;
        let request = ClickPipeStatePatchRequest {
            command: Some(command),
        };
        let response = self
            .api()
            .click_pipe_state_update(org_id, service_id, clickpipe_id, &request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeScalingPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .api()
            .click_pipe_scaling_update(org_id, service_id, clickpipe_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe_settings(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipeSettingsResponse> {
        let response = self
            .api()
            .click_pipe_settings_get(org_id, service_id, clickpipe_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_settings(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeSettingsPutRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipeSettingsResponse> {
        let response = self
            .api()
            .click_pipe_settings_update(org_id, service_id, clickpipe_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn click_pipe_schema_discovery(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryResponse>
    {
        let response = self
            .api()
            .click_pipe_schema_discovery(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;

    fn parse_cloud_command(args: &[&str]) -> CloudCommands {
        let cli = Cli::try_parse_from(
            ["clickhousectl", "cloud", "clickpipe"]
                .into_iter()
                .chain(args.iter().copied()),
        )
        .expect("ClickPipe command should parse");
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        cloud.command
    }

    fn parse_clickpipe(args: &[&str]) -> ClickPipeCommands {
        let CloudCommands::ClickPipe { command } = parse_cloud_command(args) else {
            panic!("expected clickpipe command");
        };
        *command
    }

    fn assert_rejected(args: &[&str]) {
        assert!(
            Cli::try_parse_from(
                ["clickhousectl", "cloud", "clickpipe"]
                    .into_iter()
                    .chain(args.iter().copied())
            )
            .is_err(),
            "expected parse failure for: {}",
            args.join(" ")
        );
    }

    fn clickpipe_parse_error(args: &[&str]) -> clap::Error {
        Cli::try_parse_from(
            ["clickhousectl", "cloud", "clickpipe"]
                .into_iter()
                .chain(args.iter().copied()),
        )
        .err()
        .unwrap_or_else(|| panic!("expected parse failure for: {}", args.join(" ")))
    }

    /// Minimal `clickpipe create kafka` invocation, before any auth flags.
    fn kafka_create_cli_args() -> Vec<&'static str> {
        vec![
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
        ]
    }

    /// Minimal `clickpipe schema-discover <SERVICE_ID> kafka` invocation, before
    /// any auth flags. `KafkaSourceFields` is flattened into both commands, so
    /// credential-pairing rules must hold for each.
    fn kafka_discover_cli_args() -> Vec<&'static str> {
        vec![
            "schema-discover",
            "svc-1",
            "kafka",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
        ]
    }

    fn postgres_cli_args(mapping: Option<&str>) -> Vec<&str> {
        let mut args = vec![
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
        ];
        if let Some(mapping) = mapping {
            args.extend(["--table-mapping", mapping]);
        }
        args
    }

    fn assert_write(args: &[&str], expected: bool) {
        assert_eq!(
            parse_cloud_command(args).is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    fn assert_object_storage_value(flag: &str, value: &str) {
        if flag == "--format" {
            parse_clickpipe(&[
                "create",
                "object-storage",
                "svc-1",
                "--name",
                "pipe-1",
                "--source-url",
                "https://bucket.example/data",
                "--format",
                value,
                "--database",
                "db",
                "--table",
                "events",
            ]);
            return;
        }
        parse_clickpipe(&[
            "create",
            "object-storage",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
            flag,
            value,
        ]);
    }

    fn assert_kafka_value(flag: &str, value: &str) {
        if flag == "--format" {
            parse_clickpipe(&[
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
                value,
                "--database",
                "db",
                "--table",
                "events",
            ]);
            return;
        }
        parse_clickpipe(&[
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
            flag,
            value,
        ]);
    }

    fn assert_kinesis_value(flag: &str, value: &str) {
        if flag == "--format" {
            parse_clickpipe(&[
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
                value,
                "--database",
                "db",
                "--table",
                "events",
            ]);
            return;
        }
        parse_clickpipe(&[
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
            flag,
            value,
        ]);
    }

    fn assert_postgres_value(flag: &str, value: &str) {
        let mut args = vec![
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "public.events:events",
            flag,
            value,
        ];
        if flag == "--auth" && value == "IAM_ROLE" {
            args.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
        }
        parse_clickpipe(&args);
    }

    fn assert_mysql_value(flag: &str, value: &str) {
        parse_clickpipe(&[
            "create",
            "mysql",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "mysql.example",
            "--username",
            "user",
            "--password",
            "password",
            flag,
            value,
        ]);
    }

    fn assert_mongodb_value(flag: &str, value: &str) {
        parse_clickpipe(&[
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe-1",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "user",
            "--password",
            "password",
            flag,
            value,
        ]);
    }

    #[test]
    fn parses_lifecycle_commands_and_flags() {
        let ClickPipeCommands::List { service_id, org_id } =
            parse_clickpipe(&["list", "svc-list", "--org-id", "org-list"])
        else {
            panic!("expected list");
        };
        assert_eq!(service_id, "svc-list");
        assert_eq!(org_id.as_deref(), Some("org-list"));

        let ClickPipeCommands::Get {
            service_id,
            clickpipe_id,
            org_id,
        } = parse_clickpipe(&["get", "svc-get", "pipe-get", "--org-id", "org-get"])
        else {
            panic!("expected get");
        };
        assert_eq!(service_id, "svc-get");
        assert_eq!(clickpipe_id, "pipe-get");
        assert_eq!(org_id.as_deref(), Some("org-get"));

        let ClickPipeCommands::Delete {
            service_id,
            clickpipe_id,
            org_id,
        } = parse_clickpipe(&[
            "delete",
            "svc-delete",
            "pipe-delete",
            "--org-id",
            "org-delete",
        ])
        else {
            panic!("expected delete");
        };
        assert_eq!(service_id, "svc-delete");
        assert_eq!(clickpipe_id, "pipe-delete");
        assert_eq!(org_id.as_deref(), Some("org-delete"));

        let ClickPipeCommands::Start {
            service_id,
            clickpipe_id,
            org_id,
        } = parse_clickpipe(&["start", "svc-start", "pipe-start", "--org-id", "org-start"])
        else {
            panic!("expected start");
        };
        assert_eq!(service_id, "svc-start");
        assert_eq!(clickpipe_id, "pipe-start");
        assert_eq!(org_id.as_deref(), Some("org-start"));

        let ClickPipeCommands::Stop {
            service_id,
            clickpipe_id,
            org_id,
        } = parse_clickpipe(&["stop", "svc-stop", "pipe-stop", "--org-id", "org-stop"])
        else {
            panic!("expected stop");
        };
        assert_eq!(service_id, "svc-stop");
        assert_eq!(clickpipe_id, "pipe-stop");
        assert_eq!(org_id.as_deref(), Some("org-stop"));

        let ClickPipeCommands::Resync {
            service_id,
            clickpipe_id,
            org_id,
        } = parse_clickpipe(&[
            "resync",
            "svc-resync",
            "pipe-resync",
            "--org-id",
            "org-resync",
        ])
        else {
            panic!("expected resync");
        };
        assert_eq!(service_id, "svc-resync");
        assert_eq!(clickpipe_id, "pipe-resync");
        assert_eq!(org_id.as_deref(), Some("org-resync"));
    }

    #[test]
    fn parses_scale_flags_and_defaults() {
        let ClickPipeCommands::Scale {
            service_id,
            clickpipe_id,
            replicas,
            cpu_millicores,
            memory_gb,
            org_id,
        } = parse_clickpipe(&[
            "scale",
            "svc-1",
            "pipe-1",
            "--replicas",
            "4",
            "--cpu-millicores",
            "500",
            "--memory-gb",
            "1.5",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected scale");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(clickpipe_id, "pipe-1");
        assert_eq!(replicas, Some(4));
        assert_eq!(cpu_millicores, Some(500));
        assert_eq!(memory_gb, Some(1.5));
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn scale_requires_at_least_one_of_replicas_cpu_or_memory() {
        assert_rejected(&["scale", "svc-1", "pipe-1"]);
    }

    #[test]
    fn scale_accepts_a_single_flag() {
        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1", "--replicas", "4"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, Some(4));
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, None);

        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1", "--cpu-millicores", "500"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, None);
        assert_eq!(cpu_millicores, Some(500));
        assert_eq!(memory_gb, None);

        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1", "--memory-gb", "1.5"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, None);
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, Some(1.5));
    }

    #[test]
    fn scale_accepts_any_combination_of_flags() {
        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&[
            "scale",
            "svc-1",
            "pipe-1",
            "--replicas",
            "4",
            "--memory-gb",
            "1.5",
        ])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, Some(4));
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, Some(1.5));
    }

    #[test]
    fn parses_settings_commands_flags_and_defaults() {
        let ClickPipeCommands::Settings {
            command:
                ClickPipeSettingsCommands::Get {
                    service_id,
                    clickpipe_id,
                    org_id,
                },
        } = parse_clickpipe(&["settings", "get", "svc-1", "pipe-1", "--org-id", "org-1"])
        else {
            panic!("expected settings get");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(clickpipe_id, "pipe-1");
        assert_eq!(org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Settings {
            command:
                ClickPipeSettingsCommands::Update {
                    service_id,
                    clickpipe_id,
                    streaming_max_insert_wait_ms,
                    object_storage_concurrency,
                    object_storage_polling_interval_ms,
                    object_storage_max_insert_bytes,
                    object_storage_max_file_count,
                    clickhouse_max_threads,
                    clickhouse_max_insert_threads,
                    object_storage_use_cluster_function,
                    clickhouse_parallel_view_processing,
                    org_id,
                },
        } = parse_clickpipe(&[
            "settings",
            "update",
            "svc-1",
            "pipe-1",
            "--streaming-max-insert-wait-ms",
            "1000",
            "--object-storage-concurrency",
            "2",
            "--object-storage-polling-interval-ms",
            "3000",
            "--object-storage-max-insert-bytes",
            "4000",
            "--object-storage-max-file-count",
            "5",
            "--clickhouse-max-threads",
            "6",
            "--clickhouse-max-insert-threads",
            "7",
            "--object-storage-use-cluster-function",
            "true",
            "--clickhouse-parallel-view-processing",
            "false",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected settings update");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(clickpipe_id, "pipe-1");
        assert_eq!(streaming_max_insert_wait_ms, Some(1000));
        assert_eq!(object_storage_concurrency, Some(2));
        assert_eq!(object_storage_polling_interval_ms, Some(3000));
        assert_eq!(object_storage_max_insert_bytes, Some(4000));
        assert_eq!(object_storage_max_file_count, Some(5));
        assert_eq!(clickhouse_max_threads, Some(6));
        assert_eq!(clickhouse_max_insert_threads, Some(7));
        assert_eq!(object_storage_use_cluster_function, Some(true));
        assert_eq!(clickhouse_parallel_view_processing, Some(false));
        assert_eq!(org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Settings {
            command:
                ClickPipeSettingsCommands::Update {
                    streaming_max_insert_wait_ms,
                    object_storage_concurrency,
                    object_storage_polling_interval_ms,
                    object_storage_max_insert_bytes,
                    object_storage_max_file_count,
                    clickhouse_max_threads,
                    clickhouse_max_insert_threads,
                    object_storage_use_cluster_function,
                    clickhouse_parallel_view_processing,
                    org_id,
                    ..
                },
        } = parse_clickpipe(&["settings", "update", "svc-1", "pipe-1"])
        else {
            panic!("expected settings update");
        };
        assert_eq!(streaming_max_insert_wait_ms, None);
        assert_eq!(object_storage_concurrency, None);
        assert_eq!(object_storage_polling_interval_ms, None);
        assert_eq!(object_storage_max_insert_bytes, None);
        assert_eq!(object_storage_max_file_count, None);
        assert_eq!(clickhouse_max_threads, None);
        assert_eq!(clickhouse_max_insert_threads, None);
        assert_eq!(object_storage_use_cluster_function, None);
        assert_eq!(clickhouse_parallel_view_processing, None);
        assert_eq!(org_id, None);
    }

    // A non-Kafka pipe must not carry `kafka_read_committed`: the API rejects
    // the key for every other source, which broke every non-Kafka settings
    // update (#602).
    #[test]
    fn builds_settings_request_without_kafka_only_settings_for_non_kafka_pipes() {
        let values = ClickPipeSettingsValues {
            object_storage_max_file_count: Some(200),
            ..Default::default()
        };
        let request = build_clickpipe_settings_request(&values, None);
        assert_eq!(request.kafka_read_committed, None);
        assert_eq!(request.object_storage_max_file_count, Some(200));
        assert_eq!(request.streaming_max_insert_wait_ms, None);
        assert_eq!(request.object_storage_concurrency, None);
        assert_eq!(request.object_storage_polling_interval_ms, None);
        assert_eq!(request.object_storage_max_insert_bytes, None);
        assert_eq!(request.clickhouse_max_threads, None);
        assert_eq!(request.clickhouse_max_insert_threads, None);
        assert_eq!(request.object_storage_use_cluster_function, None);
        assert_eq!(request.clickhouse_parallel_view_processing, None);
        assert_eq!(request.clickhouse_max_download_threads, None);
        assert_eq!(request.clickhouse_min_insert_block_size_bytes, None);
        assert_eq!(request.clickhouse_parallel_distributed_insert_select, None);
        // Nothing at all was passed: the body stays empty rather than
        // resetting settings the user did not name.
        let empty = build_clickpipe_settings_request(&ClickPipeSettingsValues::default(), None);
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            serde_json::json!({}),
            "a non-Kafka update with no flags must send an empty body"
        );
    }

    #[test]
    fn builds_settings_request_with_every_flag_and_kafka_read_committed() {
        let values = ClickPipeSettingsValues {
            streaming_max_insert_wait_ms: Some(1000),
            object_storage_concurrency: Some(2),
            object_storage_polling_interval_ms: Some(3000),
            object_storage_max_insert_bytes: Some(4000),
            object_storage_max_file_count: Some(5),
            clickhouse_max_threads: Some(6),
            clickhouse_max_insert_threads: Some(7),
            object_storage_use_cluster_function: Some(true),
            clickhouse_parallel_view_processing: Some(false),
        };
        let request = build_clickpipe_settings_request(&values, Some(true));
        assert_eq!(request.streaming_max_insert_wait_ms, Some(1000));
        assert_eq!(request.object_storage_concurrency, Some(2));
        assert_eq!(request.object_storage_polling_interval_ms, Some(3000));
        assert_eq!(request.object_storage_max_insert_bytes, Some(4000));
        assert_eq!(request.object_storage_max_file_count, Some(5));
        assert_eq!(request.clickhouse_max_threads, Some(6));
        assert_eq!(request.clickhouse_max_insert_threads, Some(7));
        assert_eq!(request.object_storage_use_cluster_function, Some(true));
        assert_eq!(request.clickhouse_parallel_view_processing, Some(false));
        assert_eq!(request.kafka_read_committed, Some(true));
        // A Kafka pipe with the setting disabled still sends it explicitly, so
        // the PUT does not silently flip it.
        let request = build_clickpipe_settings_request(&values, Some(false));
        assert_eq!(request.kafka_read_committed, Some(false));
    }

    #[test]
    fn readme_documents_source_aware_settings_update() {
        let readme = include_str!("../../../../README.md");
        let clickpipes = readme
            .split_once("### ClickPipes")
            .expect("ClickPipes section")
            .1
            .split_once("#### Creating ClickPipes")
            .expect("next ClickPipes section")
            .0;

        for expected in [
            "only sends the settings you name on the command line",
            "first reads the pipe to find its source type",
            "`kafka_read_committed`",
            "omitted for every other source",
            // Applicability by pipe type (#643).
            "apply to streaming (Kafka, Kinesis) and\nobject-storage pipes only",
            "Database CDC pipes (Postgres, MySQL, MongoDB, BigQuery) are refused",
            "`clickhousectl cloud clickpipe get <service-id> <clickpipe-id>`",
        ] {
            assert!(
                clickpipes.contains(expected),
                "missing `{expected}`:\n{clickpipes}"
            );
        }
    }

    #[test]
    fn classifies_every_clickpipe_source_and_an_absent_one() {
        use clickhouse_cloud_api::models::{
            ClickPipe, ClickPipeBigQuerySource, ClickPipeKafkaSource, ClickPipeKinesisSource,
            ClickPipeMongoDBSource, ClickPipeMySQLSource, ClickPipeObjectStorageSource,
            ClickPipePostgresSource, ClickPipePubSubSource, ClickPipeSource,
        };

        fn pipe(source: ClickPipeSource) -> ClickPipe {
            ClickPipe {
                source: Some(source),
                ..Default::default()
            }
        }

        let cases: Vec<(ClickPipe, ClickPipeSourceKind)> = vec![
            (
                pipe(ClickPipeSource {
                    kafka: Some(ClickPipeKafkaSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::Kafka,
            ),
            (
                pipe(ClickPipeSource {
                    kinesis: Some(ClickPipeKinesisSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::Kinesis,
            ),
            (
                pipe(ClickPipeSource {
                    pubsub: Some(ClickPipePubSubSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::PubSub,
            ),
            (
                pipe(ClickPipeSource {
                    object_storage: Some(ClickPipeObjectStorageSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::ObjectStorage,
            ),
            (
                pipe(ClickPipeSource {
                    postgres: Some(ClickPipePostgresSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::Postgres,
            ),
            (
                pipe(ClickPipeSource {
                    mysql: Some(ClickPipeMySQLSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::MySql,
            ),
            (
                pipe(ClickPipeSource {
                    mongodb: Some(ClickPipeMongoDBSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::MongoDb,
            ),
            (
                pipe(ClickPipeSource {
                    bigquery: Some(ClickPipeBigQuerySource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::BigQuery,
            ),
            // An absent source, and a source object with no arm set, are both
            // unclassifiable: the API stays the authority for those.
            (ClickPipe::default(), ClickPipeSourceKind::Absent),
            (
                pipe(ClickPipeSource::default()),
                ClickPipeSourceKind::Absent,
            ),
        ];

        for (clickpipe, expected) in &cases {
            assert_eq!(classify_clickpipe_source(clickpipe), *expected);
        }

        // Only Kafka carries the Kafka-only settings key.
        for (clickpipe, expected) in &cases {
            assert_eq!(
                classify_clickpipe_source(clickpipe).is_kafka(),
                *expected == ClickPipeSourceKind::Kafka,
            );
        }
    }

    #[test]
    fn only_database_sources_have_no_ingestion_settings() {
        for (kind, label) in [
            (ClickPipeSourceKind::Postgres, Some("Postgres CDC")),
            (ClickPipeSourceKind::MySql, Some("MySQL CDC")),
            (ClickPipeSourceKind::MongoDb, Some("MongoDB CDC")),
            (ClickPipeSourceKind::BigQuery, Some("BigQuery")),
            (ClickPipeSourceKind::Kafka, None),
            (ClickPipeSourceKind::Kinesis, None),
            (ClickPipeSourceKind::PubSub, None),
            (ClickPipeSourceKind::ObjectStorage, None),
            (ClickPipeSourceKind::Absent, None),
        ] {
            assert_eq!(kind.database_source_label(), label, "{kind:?}");
        }
    }

    #[test]
    fn settings_refusal_names_the_source_and_points_at_clickpipe_get() {
        use clickhouse_cloud_api::models::{
            ClickPipe, ClickPipeMySQLSource, ClickPipeObjectStorageSource, ClickPipeSource,
        };

        let mysql = ClickPipe {
            source: Some(ClickPipeSource {
                mysql: Some(ClickPipeMySQLSource::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = ensure_clickpipe_has_ingestion_settings(&mysql, "svc-1", "pipe-1")
            .expect_err("a MySQL CDC pipe has no ingestion settings");
        assert_eq!(
            error.message,
            "ClickPipe pipe-1 is a MySQL CDC pipe; `clickpipe settings get` and \
             `settings update` apply only to streaming (Kafka, Kinesis) and object-storage \
             pipes. CDC pipe settings (sync interval, pull batch size) live on the pipe \
             itself: see `clickhousectl cloud clickpipe get svc-1 pipe-1`."
        );
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);

        // Streaming, object-storage and unclassifiable pipes are not refused.
        let object_storage = ClickPipe {
            source: Some(ClickPipeSource {
                object_storage: Some(ClickPipeObjectStorageSource::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            ensure_clickpipe_has_ingestion_settings(&object_storage, "svc-1", "pipe-1").is_ok()
        );
        assert!(
            ensure_clickpipe_has_ingestion_settings(&ClickPipe::default(), "svc-1", "pipe-1")
                .is_ok()
        );
    }

    #[test]
    fn settings_help_states_which_pipe_types_it_applies_to() {
        use clap::CommandFactory;

        let mut command = Cli::command();
        let settings = command
            .find_subcommand_mut("cloud")
            .expect("cloud subcommand")
            .find_subcommand_mut("clickpipe")
            .expect("clickpipe subcommand")
            .find_subcommand_mut("settings")
            .expect("settings subcommand");
        let help = settings.render_long_help().to_string();
        for expected in [
            "apply only to streaming (Kafka, Kinesis) and object-storage",
            "Database CDC pipes (Postgres, MySQL, MongoDB, BigQuery) have no ingestion",
            "clickhousectl cloud clickpipe get <service>",
        ] {
            assert!(help.contains(expected), "missing `{expected}`:\n{help}");
        }
        for subcommand in ["get", "update"] {
            let subcommand_help = settings
                .find_subcommand_mut(subcommand)
                .expect("settings subcommand")
                .render_long_help()
                .to_string();
            assert!(
                subcommand_help.contains("streaming and object-storage pipes only"),
                "missing applicability in `settings {subcommand}` help:\n{subcommand_help}"
            );
        }
    }

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
        assert!(args.source.continuous);
        assert_eq!(
            args.source.queue_url.as_deref(),
            Some("https://queue.example/q")
        );
        assert!(!args.source.skip_initial_load);
        assert_eq!(args.source.start_after.as_deref(), Some("key-1"));
        assert_eq!(args.source.delimiter.as_deref(), Some(","));
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
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::ObjectStorage(args),
        } = parse_clickpipe(&[
            "create",
            "object-storage",
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
        assert!(args.columns.is_empty());
        assert_eq!(args.source.storage_type, "s3");
        assert_eq!(args.source.compression, "auto");
        assert!(!args.source.continuous);
        assert_eq!(args.source.queue_url, None);
        assert!(!args.source.skip_initial_load);
        assert_eq!(args.source.start_after, None);
        assert_eq!(args.source.delimiter, None);
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id, None);
        assert_eq!(args.source.secret_key, None);
        assert_eq!(args.source.connection_string, None);
        assert_eq!(args.source.azure_container_name, None);
        assert_eq!(args.source.path, None);
        assert_eq!(args.source.service_account_file, None);
        assert_eq!(args.org_id, None);
    }

    #[test]
    fn parses_object_storage_skip_initial_load_and_constraints() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::ObjectStorage(args),
        } = parse_clickpipe(&[
            "create",
            "object-storage",
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
        assert_eq!(args.database, "db");
        assert_eq!(args.table, "events");
        assert_eq!(args.columns, ["id:UInt64", "name:String"]);
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

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
        ])
        else {
            panic!("expected kafka create");
        };
        assert_eq!(args.source.kafka_type, "kafka");
        assert_eq!(args.source.offset, "from_beginning");
        assert_eq!(args.source.consumer_group, None);
        assert_eq!(args.source.auth, None);
        assert_eq!(args.source.username, None);
        assert_eq!(args.source.password, None);
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id, None);
        assert_eq!(args.source.secret_key, None);
        assert_eq!(args.source.offset_timestamp, None);
        assert_eq!(args.source.schema_registry_url, None);
        assert_eq!(args.source.schema_registry_username, None);
        assert_eq!(args.source.schema_registry_password, None);
        assert_eq!(args.source.ca_certificate, None);
        assert_eq!(args.source.client_certificate, None);
        assert_eq!(args.source.client_key, None);
        assert_eq!(args.source.schema_registry_ca_certificate, None);
        assert!(args.source.reverse_private_endpoint_ids.is_empty());
        assert!(args.columns.is_empty());
        assert_eq!(args.org_id, None);
    }

    #[test]
    fn kafka_credential_flags_must_be_given_in_pairs() {
        // Every Kafka credential pair is joined with clap `requires`. Half a
        // pair must be a usage error: since `--auth` is now optional and the
        // mechanism is inferred only from a *complete* pair, an unpaired flag
        // would otherwise infer nothing and silently send an unauthenticated
        // create that exits 0 (issue #606).
        let bases: [fn() -> Vec<&'static str>; 2] =
            [kafka_create_cli_args, kafka_discover_cli_args];
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
            "--iam-role",
            "arn:role",
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
        assert_eq!(args.source.auth, "IAM_USER");
        assert_eq!(args.source.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.source.access_key_id.as_deref(), Some("access"));
        assert_eq!(args.source.secret_key.as_deref(), Some("secret"));
        assert_eq!(args.source.iterator_type, "AT_TIMESTAMP");
        assert_eq!(args.source.iterator_timestamp, Some(1_720_000_000));
        assert!(args.source.enhanced_fan_out);
        assert_eq!(args.database, "db");
        assert_eq!(args.table, "events");
        assert_eq!(args.columns, ["id:UInt64", "name:String"]);
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

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
        ])
        else {
            panic!("expected kinesis create");
        };
        assert_eq!(args.source.auth, "IAM_ROLE");
        assert_eq!(args.source.iam_role, None);
        assert_eq!(args.source.access_key_id, None);
        assert_eq!(args.source.secret_key, None);
        assert_eq!(args.source.iterator_type, "TRIM_HORIZON");
        assert_eq!(args.source.iterator_timestamp, None);
        assert!(!args.source.enhanced_fan_out);
        assert!(args.columns.is_empty());
        assert_eq!(args.org_id, None);
    }

    #[test]
    fn parses_schema_discovery_commands_and_flags() {
        let ClickPipeCommands::SchemaDiscover {
            service_id,
            command: ClickPipeSchemaDiscoverCommands::Kafka(args),
            org_id,
        } = parse_clickpipe(&[
            "schema-discover",
            "svc-kafka",
            "--org-id",
            "org-kafka",
            "kafka",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
        ])
        else {
            panic!("expected kafka schema discovery");
        };
        assert_eq!(service_id, "svc-kafka");
        assert_eq!(args.brokers, "broker:9092");
        assert_eq!(args.kafka_type, "kafka");
        assert_eq!(args.offset, "from_beginning");
        assert_eq!(org_id.as_deref(), Some("org-kafka"));
        // Auth flags are all optional for discovery too, and an unauthenticated
        // broker builds a source with no authentication at all (issue #606).
        assert_eq!(args.auth, None);
        assert_eq!(args.username, None);
        assert_eq!(args.password, None);
        let discovery_source = build_kafka_source(&args).expect("no-auth discovery source builds");
        assert_eq!(discovery_source.authentication, None);
        assert!(discovery_source.credentials.is_null());

        let ClickPipeCommands::SchemaDiscover {
            service_id,
            command: ClickPipeSchemaDiscoverCommands::Kinesis(args),
            org_id,
        } = parse_clickpipe(&[
            "schema-discover",
            "svc-kinesis",
            "--org-id",
            "org-kinesis",
            "kinesis",
            "--stream-name",
            "stream-1",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
        ])
        else {
            panic!("expected kinesis schema discovery");
        };
        assert_eq!(service_id, "svc-kinesis");
        assert_eq!(args.stream_name, "stream-1");
        assert_eq!(args.auth, "IAM_ROLE");
        assert_eq!(args.iterator_type, "TRIM_HORIZON");
        assert_eq!(org_id.as_deref(), Some("org-kinesis"));
    }

    /// `schema-discover object-storage` takes the same source flags as
    /// `create object-storage`, minus the destination options, and keeps the
    /// same `--storage-type`/`--compression` defaults (issue #588).
    #[test]
    fn parses_object_storage_schema_discovery_flags() {
        let ClickPipeCommands::SchemaDiscover {
            service_id,
            command: ClickPipeSchemaDiscoverCommands::ObjectStorage(args),
            org_id,
        } = parse_clickpipe(&[
            "schema-discover",
            "svc-object-storage",
            "--org-id",
            "org-object-storage",
            "object-storage",
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
        assert_eq!(service_id, "svc-object-storage");
        assert_eq!(org_id.as_deref(), Some("org-object-storage"));
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
            org_id,
            ..
        } = parse_clickpipe(&[
            "schema-discover",
            "svc-object-storage",
            "object-storage",
            "--source-url",
            "https://bucket.example/data/*.json",
            "--format",
            "JSONEachRow",
        ])
        else {
            panic!("expected object-storage schema discovery");
        };
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
        assert_eq!(org_id, None);

        // --skip-initial-load still requires --queue-url and still conflicts
        // with --start-after, exactly as on `create object-storage`.
        let base = [
            "schema-discover",
            "svc-object-storage",
            "object-storage",
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
    fn parses_postgres_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&[
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--port",
            "5433",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "public.one:one",
            "--table-mapping",
            "public.two:two",
            "--postgres-type",
            "neon",
            "--replication-mode",
            "cdc_only",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:role",
            "--tls-host",
            "tls.example",
            "--ca-certificate",
            "/tmp/ca.pem",
            "--publication-name",
            "publication",
            "--replication-slot-name",
            "slot",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.host, "postgres.example");
        assert_eq!(args.port, 5433);
        assert_eq!(args.pg_database, "source-db");
        assert_eq!(args.username, "user");
        assert_eq!(args.password, "password");
        assert_eq!(args.table_mappings, ["public.one:one", "public.two:two"]);
        assert_eq!(args.postgres_type, "neon");
        assert_eq!(args.replication_mode, "cdc_only");
        assert_eq!(args.auth, "IAM_ROLE");
        assert_eq!(args.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.tls_host.as_deref(), Some("tls.example"));
        assert_eq!(args.ca_certificate.as_deref(), Some("/tmp/ca.pem"));
        assert_eq!(args.publication_name.as_deref(), Some("publication"));
        assert_eq!(args.replication_slot_name.as_deref(), Some("slot"));
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&[
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "public.events:events",
        ])
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.port, 5432);
        assert_eq!(args.table_mappings, ["public.events:events"]);
        assert_eq!(args.postgres_type, "postgres");
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.auth, "basic");
        assert_eq!(args.iam_role, None);
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert_eq!(args.publication_name, None);
        assert_eq!(args.replication_slot_name, None);
        assert_eq!(args.sync_interval_seconds, None);
        assert_eq!(args.pull_batch_size, None);
        assert_eq!(args.initial_load_parallelism, None);
        assert_eq!(args.snapshot_rows_per_partition, None);
        assert_eq!(args.snapshot_parallel_tables, None);
        assert_eq!(args.allow_nullable_columns, None);
        assert_eq!(args.enable_failover_slots, None);
        assert_eq!(args.delete_on_merge, None);
        assert_eq!(args.org_id, None);
    }

    #[test]
    fn parses_postgres_cdc_settings_flags() {
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.extend([
            "--sync-interval-seconds",
            "30",
            "--pull-batch-size",
            "50000",
            "--initial-load-parallelism",
            "4",
            "--snapshot-rows-per-partition",
            "1000000",
            "--snapshot-parallel-tables",
            "3",
            "--allow-nullable-columns",
            "true",
            "--enable-failover-slots",
            "false",
            "--delete-on-merge",
            "true",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };

        assert_eq!(args.sync_interval_seconds, Some(30));
        assert_eq!(args.pull_batch_size, Some(50_000));
        assert_eq!(args.initial_load_parallelism, Some(4));
        assert_eq!(args.snapshot_rows_per_partition, Some(1_000_000));
        assert_eq!(args.snapshot_parallel_tables, Some(3));
        // Tri-state booleans: an explicit `false` is distinguishable from an
        // omitted flag at the clap layer.
        assert_eq!(args.allow_nullable_columns, Some(true));
        assert_eq!(args.enable_failover_slots, Some(false));
        assert_eq!(args.delete_on_merge, Some(true));
    }

    #[test]
    fn postgres_cdc_settings_booleans_require_an_explicit_value() {
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.push("--allow-nullable-columns");
        let error = clickpipe_parse_error(&cli_args);
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        let message = error.to_string();
        assert!(message.contains("--allow-nullable-columns"), "{message}");
    }

    #[test]
    fn postgres_port_accepts_boundaries_and_rejects_out_of_range_values() {
        for port in ["1", "65535"] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.extend(["--port", port]);
            parse_clickpipe(&args);
        }

        for port in ["0", "65536"] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.extend(["--port", port]);
            let error = clickpipe_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            let message = error.to_string();
            assert!(message.contains("--port"), "{message}");
        }
    }

    #[test]
    fn postgres_requires_at_least_one_complete_table_mapping() {
        let error = clickpipe_parse_error(&postgres_cli_args(None));
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(error.to_string().contains("--table-mapping"));

        for (mapping, diagnostic) in [
            ("public.events", "expected schema.table:target_table"),
            ("events:target", "expected schema.table:target_table"),
            (".events:target", "source schema must not be empty"),
            ("public.:target", "source table must not be empty"),
            ("public.events:", "target table must not be empty"),
            (" .events:target", "source schema must not be empty"),
            ("public. :target", "source table must not be empty"),
            ("public.events: ", "target table must not be empty"),
        ] {
            let error = clickpipe_parse_error(&postgres_cli_args(Some(mapping)));
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            let message = error.to_string();
            assert!(message.contains(diagnostic), "{message}");
        }
    }

    /// One `--table-mapping-json` object with every field set, reused by the
    /// clap, validation and builder tests.
    const MAXIMAL_TABLE_MAPPING_JSON: &str = r#"{
        "sourceSchemaName": "public",
        "sourceTable": "users",
        "targetTable": "users_raw",
        "excludedColumns": ["ssn", "dob"],
        "sortingKeys": ["created_at", "id"],
        "useCustomSortingKey": true,
        "partitionByExpr": "toYYYYMM(created_at)",
        "partitionKey": "id",
        "tableEngine": "ReplacingMergeTree"
    }"#;

    #[test]
    fn parses_postgres_table_mapping_json_flag() {
        let second =
            r#"{"sourceSchemaName":"audit","sourceTable":"events","targetTable":"audit_events"}"#;

        // JSON only: the simple form is not required when the JSON form is given.
        let mut cli_args = postgres_cli_args(None);
        cli_args.extend([
            "--table-mapping-json",
            MAXIMAL_TABLE_MAPPING_JSON,
            "--table-mapping-json",
            second,
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert!(args.table_mappings.is_empty());
        assert_eq!(
            args.table_mappings_json,
            [MAXIMAL_TABLE_MAPPING_JSON, second]
        );

        // Both forms together: each flag keeps its own values, verbatim.
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.extend(["--table-mapping-json", second]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.table_mappings, ["public.events:events"]);
        assert_eq!(args.table_mappings_json, [second]);

        // Simple form only: the JSON list stays empty.
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&postgres_cli_args(Some("public.events:events")))
        else {
            panic!("expected postgres create");
        };
        assert!(args.table_mappings_json.is_empty());
    }

    #[test]
    fn postgres_missing_both_table_mapping_flags_names_both() {
        let error = clickpipe_parse_error(&postgres_cli_args(None));
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let message = error.to_string();
        assert!(message.contains("--table-mapping "), "{message}");
        assert!(message.contains("--table-mapping-json"), "{message}");
    }

    #[test]
    fn postgres_table_mapping_json_is_not_parsed_by_clap() {
        // Content validation belongs to `validate_postgres_create_args`, which
        // runs before any request; clap only collects the raw strings.
        let mut cli_args = postgres_cli_args(None);
        cli_args.extend(["--table-mapping-json", "{ not json"]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.table_mappings_json, ["{ not json"]);

        let command = parse_clickpipe(&cli_args);
        assert_eq!(
            command
                .postgres_create_validation_error()
                .as_deref()
                .map(|message| message.contains("--table-mapping-json #1: invalid JSON")),
            Some(true)
        );
    }

    #[test]
    fn postgres_iam_role_auth_requires_role_arn() {
        let mut args = postgres_cli_args(Some("public.events:events"));
        args.extend(["--auth", "IAM_ROLE"]);
        let error = clickpipe_parse_error(&args);
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let message = error.to_string();
        assert!(message.contains("--iam-role"), "{message}");

        args.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
        parse_clickpipe(&args);
    }

    #[test]
    fn postgres_cross_value_relationships_have_specific_validation_errors() {
        let mut basic_with_role = postgres_cli_args(Some("public.events:events"));
        basic_with_role.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
        let command = parse_clickpipe(&basic_with_role);
        assert_eq!(
            command.postgres_create_validation_error().as_deref(),
            Some("--iam-role cannot be used with --auth basic; use --auth IAM_ROLE")
        );

        for mode in ["cdc", "snapshot"] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.extend([
                "--replication-mode",
                mode,
                "--replication-slot-name",
                "existing_slot",
            ]);
            let command = parse_clickpipe(&args);
            assert_eq!(
                command.postgres_create_validation_error().as_deref(),
                Some("--replication-slot-name can only be used with --replication-mode cdc_only")
            );
        }
    }

    #[test]
    fn postgres_help_documents_the_json_table_mapping_form() {
        let error = clickpipe_parse_error(&["create", "postgres", "--help"]);
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        assert!(help.contains("POSTGRES TABLE MAPPINGS:"), "{help}");
        assert!(help.contains("--table-mapping-json <JSON>"), "{help}");
        assert!(
            help.contains("--table-mapping schema.table:target_table is unchanged"),
            "{help}"
        );
        // The example shows the fields the simple form cannot express.
        for field in POSTGRES_TABLE_MAPPING_JSON_FIELDS {
            assert!(help.contains(field), "missing `{field}`:\n{help}");
        }
        for excerpt in [
            r#""excludedColumns":["ssn"]"#,
            r#""sortingKeys":["created_at","id"]"#,
            r#""partitionByExpr":"toYYYYMM(created_at)""#,
            r#""tableEngine":"ReplacingMergeTree""#,
        ] {
            assert!(help.contains(excerpt), "missing `{excerpt}`:\n{help}");
        }
        // Every table engine the library accepts is named, so the help cannot
        // drift from the enum's wire values.
        for engine in ClickPipePostgresPipeTableMappingTableengine::VALUES {
            assert!(help.contains(engine), "missing `{engine}`:\n{help}");
        }
        assert!(
            help.contains("useCustomSortingKey is set to true automatically"),
            "{help}"
        );
        assert!(help.contains("An unknown field is rejected"), "{help}");
    }

    #[test]
    fn postgres_help_documents_input_tls_and_source_requirements() {
        let error = clickpipe_parse_error(&["create", "postgres", "--help"]);
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(
            help.contains("At least one --table-mapping is required"),
            "{help}"
        );
        assert!(help.contains("or one --table-mapping-json"), "{help}");
        assert!(
            help.contains("--auth IAM_ROLE requires --iam-role"),
            "{help}"
        );
        assert!(help.contains("silently ignored"), "{help}");
        assert!(help.contains("--replication-mode cdc_only"), "{help}");
        assert!(
            help.contains("cannot\n  be changed after creation"),
            "{help}"
        );
        assert!(
            help.contains("can patch only\n  syncIntervalSeconds and pullBatchSize"),
            "{help}"
        );
        assert!(
            help.contains("omitting one sends\n  false, which is the API default"),
            "{help}"
        );
        for flag in [
            "--sync-interval-seconds <SECONDS>",
            "--pull-batch-size <ROWS>",
            "--initial-load-parallelism <WORKERS>",
            "--snapshot-rows-per-partition <ROWS>",
            "--snapshot-parallel-tables <TABLES>",
            "--allow-nullable-columns <true|false>",
            "--enable-failover-slots <true|false>",
            "--delete-on-merge <true|false>",
        ] {
            assert!(help.contains(flag), "missing `{flag}`:\n{help}");
        }
        assert!(
            help.contains("TLS and certificate verification are enabled by default"),
            "{help}"
        );
        assert!(help.contains("publicly trusted needs no CA file"), "{help}");
        assert!(help.contains("PEM CA bundle"), "{help}");
        assert!(help.contains("--ca-certificate <PATH>"), "{help}");
        assert!(help.contains("--tls-host <HOSTNAME>"), "{help}");
        assert!(help.contains("uses --host unless"), "{help}");
        assert!(help.contains("enable logical replication"), "{help}");
        assert!(help.contains("mapped table in the publication"), "{help}");
        assert!(
            help.contains("schema, table, and replication privileges"),
            "{help}"
        );
        assert!(
            help.contains("https://clickhouse.com/docs/integrations/clickpipes/postgres"),
            "{help}"
        );
        assert!(
            help.contains(
                "https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips"
            ),
            "{help}"
        );
    }

    #[test]
    fn readme_documents_postgres_tls_and_cdc_prerequisites() {
        let readme = include_str!("../../../../README.md");
        let postgres = readme
            .split_once("### ClickPipes")
            .expect("ClickPipes section")
            .1
            .split_once("#### Discovering a source schema")
            .expect("next ClickPipes section")
            .0;

        for expected in [
            "publicly trusted certificate",
            "private or self-signed CA",
            "--ca-certificate ./postgres-ca.pem",
            "--tls-host postgres.internal.example.com",
            "TLS and certificate verification are enabled by default",
            "defaults to `--host`",
            "ClickPipes static egress IPs",
            "`wal_level=logical`",
            "publication must contain every source table",
            "`USAGE` on each mapped schema",
            "https://clickhouse.com/docs/integrations/clickpipes/postgres/source/generic",
            "https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips",
        ] {
            assert!(
                postgres.contains(expected),
                "missing `{expected}`:\n{postgres}"
            );
        }

        assert_eq!(
            postgres.matches("--publication-name clickpipes").count(),
            5,
            "every PostgreSQL example must use the publication created in the prerequisites"
        );
    }

    #[test]
    fn readme_documents_the_json_table_mapping_form() {
        let readme = include_str!("../../../../README.md");
        let mappings = readme
            .split_once("#### PostgreSQL table mappings")
            .expect("PostgreSQL table mappings section")
            .1
            .split_once("#### PostgreSQL CDC pipe settings")
            .expect("end of the table mappings section")
            .0;

        for expected in [
            "--table-mapping schema.table:target_table",
            "--table-mapping-json <JSON>",
            "\"excludedColumns\": [\"ssn\"]",
            "\"sortingKeys\": [\"created_at\", \"id\"]",
            "\"partitionByExpr\": \"toYYYYMM(created_at)\"",
            "\"tableEngine\": \"ReplacingMergeTree\"",
            "Set to `true` automatically when `sortingKeys` is given",
            "An unknown field is rejected",
            "usage\nerror (exit code 2)",
            "--table-mapping-json #2: targetTable is required and must not be empty",
            "not yet available on\n`clickpipe create mysql`",
        ] {
            assert!(
                mappings.contains(expected),
                "missing `{expected}`:\n{mappings}"
            );
        }
        // Every field of the mapping object is documented in the table.
        for field in POSTGRES_TABLE_MAPPING_JSON_FIELDS {
            assert!(
                mappings.contains(&format!("| `{field}` |")),
                "missing `{field}` row:\n{mappings}"
            );
        }
        for engine in ClickPipePostgresPipeTableMappingTableengine::VALUES {
            assert!(
                mappings.contains(&format!("`{engine}`")),
                "missing `{engine}`:\n{mappings}"
            );
        }
    }

    #[test]
    fn readme_documents_postgres_cdc_settings_flags() {
        let readme = include_str!("../../../../README.md");
        let settings = readme
            .split_once("#### PostgreSQL CDC pipe settings")
            .expect("PostgreSQL CDC pipe settings section")
            .1
            .split_once("Use `clickhousectl cloud clickpipe create <source> --help`")
            .expect("end of the CDC settings section")
            .0;

        for expected in [
            "--sync-interval-seconds <SECONDS>",
            "--pull-batch-size <ROWS>",
            "--initial-load-parallelism <WORKERS>",
            "--snapshot-rows-per-partition <ROWS>",
            "--snapshot-parallel-tables <TABLES>",
            "--allow-nullable-columns <true\\|false>",
            "--enable-failover-slots <true\\|false>",
            "--delete-on-merge <true\\|false>",
            "they send `false`, which is the API default",
            "cannot be changed later",
            "not yet exposed on `clickpipe create mysql`",
        ] {
            assert!(
                settings.contains(expected),
                "missing `{expected}`:\n{settings}"
            );
        }
    }

    #[test]
    fn postgres_tls_error_hints_are_narrow_and_preserve_api_detail() {
        let unknown_authority = "BAD_REQUEST: failed to establish connection: tls: failed to verify certificate: \
             x509: certificate signed by unknown authority";
        let error = add_postgres_tls_error_hint(CloudError::new(unknown_authority));
        assert!(error.message.starts_with(unknown_authority));
        assert!(error.message.contains("--ca-certificate <PATH>"));

        let hostname = postgres_tls_error_hint(
            "x509: certificate is valid for postgres.internal.example.com, not 10.0.0.8",
        )
        .expect("hostname mismatch hint");
        assert!(hostname.contains("--tls-host <HOSTNAME>"));
        let ip_sans = postgres_tls_error_hint(
            "x509: cannot validate certificate for 10.0.0.8 because it doesn't contain any IP Sans",
        )
        .expect("IP SAN hostname mismatch hint");
        assert!(ip_sans.contains("--tls-host <HOSTNAME>"));
        assert!(
            postgres_tls_error_hint(
                "BAD_REQUEST: failed to establish connection: connection refused"
            )
            .is_none()
        );
        assert!(
            postgres_tls_error_hint("tls: failed to verify certificate: certificate expired")
                .is_none()
        );
    }

    #[test]
    fn parses_mysql_flags_defaults_repeatability_and_server_id_range() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MySQL(args),
        } = parse_clickpipe(&[
            "create",
            "mysql",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "mysql.example",
            "--port",
            "3307",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "source.one:one",
            "--table-mapping",
            "source.two:two",
            "--mysql-type",
            "mariadb",
            "--replication-mode",
            "cdc_only",
            "--replication-mechanism",
            "FILE_POS",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:role",
            "--tls-host",
            "tls.example",
            "--ca-certificate",
            "/tmp/ca.pem",
            "--disable-tls",
            "--skip-cert-verification",
            "--server-id",
            "4294967295",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected mysql create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.host, "mysql.example");
        assert_eq!(args.port, 3307);
        assert_eq!(args.username, "user");
        assert_eq!(args.password, "password");
        assert_eq!(args.table_mappings, ["source.one:one", "source.two:two"]);
        assert_eq!(args.mysql_type, "mariadb");
        assert_eq!(args.replication_mode, "cdc_only");
        assert_eq!(args.replication_mechanism, "FILE_POS");
        assert_eq!(args.auth, "IAM_ROLE");
        assert_eq!(args.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.tls_host.as_deref(), Some("tls.example"));
        assert_eq!(args.ca_certificate.as_deref(), Some("/tmp/ca.pem"));
        assert!(args.disable_tls);
        assert!(args.skip_cert_verification);
        assert_eq!(args.server_id, Some(4_294_967_295));
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MySQL(args),
        } = parse_clickpipe(&[
            "create",
            "mysql",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "mysql.example",
            "--username",
            "user",
            "--password",
            "password",
            "--server-id",
            "1",
        ])
        else {
            panic!("expected mysql create");
        };
        assert_eq!(args.port, 3306);
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.mysql_type, "mysql");
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.replication_mechanism, "GTID");
        assert_eq!(args.auth, "basic");
        assert_eq!(args.iam_role, None);
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert!(!args.disable_tls);
        assert!(!args.skip_cert_verification);
        assert_eq!(args.server_id, Some(1));
        assert_eq!(args.org_id, None);

        for invalid in ["0", "4294967296"] {
            assert_rejected(&[
                "create",
                "mysql",
                "svc-1",
                "--name",
                "pipe-1",
                "--host",
                "mysql.example",
                "--username",
                "user",
                "--password",
                "password",
                "--server-id",
                invalid,
            ]);
        }
    }

    #[test]
    fn parses_mongodb_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MongoDB(args),
        } = parse_clickpipe(&[
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe-1",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "source.one:one",
            "--table-mapping",
            "source.two:two",
            "--replication-mode",
            "snapshot",
            "--read-preference",
            "nearest",
            "--tls-host",
            "tls.example",
            "--ca-certificate",
            "/tmp/ca.pem",
            "--disable-tls",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected mongodb create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.uri, "mongodb://mongo.example/source");
        assert_eq!(args.username, "user");
        assert_eq!(args.password, "password");
        assert_eq!(args.table_mappings, ["source.one:one", "source.two:two"]);
        assert_eq!(args.replication_mode, "snapshot");
        assert_eq!(args.read_preference, "nearest");
        assert_eq!(args.tls_host.as_deref(), Some("tls.example"));
        assert_eq!(args.ca_certificate.as_deref(), Some("/tmp/ca.pem"));
        assert!(args.disable_tls);
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MongoDB(args),
        } = parse_clickpipe(&[
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe-1",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "user",
            "--password",
            "password",
        ])
        else {
            panic!("expected mongodb create");
        };
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.read_preference, "secondaryPreferred");
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert!(!args.disable_tls);
        assert_eq!(args.org_id, None);
    }

    #[test]
    fn parses_bigquery_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::BigQuery(args),
        } = parse_clickpipe(&[
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "pipe-1",
            "--service-account-file",
            "/tmp/account.json",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.one:one",
            "--table-mapping",
            "dataset.two:two",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected bigquery create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.service_account_file, "/tmp/account.json");
        assert_eq!(args.staging_path, "gs://bucket/staging");
        assert_eq!(args.table_mappings, ["dataset.one:one", "dataset.two:two"]);
        assert_eq!(args.org_id.as_deref(), Some("org-1"));

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::BigQuery(args),
        } = parse_clickpipe(&[
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "pipe-1",
            "--service-account-file",
            "/tmp/account.json",
            "--staging-path",
            "gs://bucket/staging",
        ])
        else {
            panic!("expected bigquery create");
        };
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.org_id, None);
    }

    #[test]
    fn accepted_possible_values_are_pinned() {
        assert_eq!(
            OBJECT_STORAGE_FORMATS,
            &[
                "JSONEachRow",
                "JSONAsObject",
                "CSV",
                "CSVWithNames",
                "TabSeparated",
                "TabSeparatedWithNames",
                "Parquet",
                "Avro",
            ]
        );
        assert_eq!(
            OBJECT_STORAGE_COMPRESSIONS,
            &[
                "none", "gzip", "gz", "brotli", "br", "xz", "LZMA", "zstd", "auto",
            ]
        );
        assert_eq!(
            OBJECT_STORAGE_TYPES,
            &[
                "s3",
                "gcs",
                "dospaces",
                "azureblobstorage",
                "cloudflarer2",
                "ovhobjectstorage",
            ]
        );
        assert_eq!(
            KAFKA_FORMATS,
            &["JSONEachRow", "Avro", "AvroConfluent", "Protobuf"]
        );
        assert_eq!(
            KAFKA_TYPES,
            &[
                "kafka",
                "redpanda",
                "msk",
                "gcmk",
                "confluent",
                "warpstream",
                "azureeventhub",
                "dokafka",
            ]
        );
        assert_eq!(
            KAFKA_AUTHS,
            &[
                "PLAIN",
                "SCRAM-SHA-256",
                "SCRAM-SHA-512",
                "IAM_ROLE",
                "IAM_USER",
                "MUTUAL_TLS",
            ]
        );
        assert_eq!(
            KAFKA_OFFSET_STRATEGIES,
            &["from_beginning", "from_latest", "from_timestamp"]
        );
        assert_eq!(KINESIS_FORMATS, &["JSONEachRow", "Avro", "AvroConfluent"]);
        assert_eq!(KINESIS_AUTHS, &["IAM_ROLE", "IAM_USER"]);
        assert_eq!(
            KINESIS_ITERATOR_TYPES,
            &["TRIM_HORIZON", "LATEST", "AT_TIMESTAMP"]
        );
        assert_eq!(
            POSTGRES_TYPES,
            &[
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
            ]
        );
        assert_eq!(DB_AUTHS, &["basic", "IAM_ROLE"]);
        assert_eq!(REPLICATION_MODES, &["cdc", "snapshot", "cdc_only"]);
        assert_eq!(
            MYSQL_TYPES,
            &["mysql", "rdsmysql", "auroramysql", "mariadb", "rdsmariadb"]
        );
        assert_eq!(MYSQL_REPLICATION_MECHANISMS, &["GTID", "FILE_POS"]);
        assert_eq!(
            MONGODB_READ_PREFERENCES,
            &[
                "primary",
                "primaryPreferred",
                "secondary",
                "secondaryPreferred",
                "nearest",
            ]
        );

        for &value in OBJECT_STORAGE_FORMATS {
            assert_object_storage_value("--format", value);
        }
        for &value in OBJECT_STORAGE_COMPRESSIONS {
            assert_object_storage_value("--compression", value);
        }
        for &value in OBJECT_STORAGE_TYPES {
            assert_object_storage_value("--storage-type", value);
        }
        for &value in KAFKA_FORMATS {
            assert_kafka_value("--format", value);
        }
        for &value in KAFKA_TYPES {
            assert_kafka_value("--kafka-type", value);
        }
        for &value in KAFKA_AUTHS {
            assert_kafka_value("--auth", value);
        }
        for &value in KAFKA_OFFSET_STRATEGIES {
            assert_kafka_value("--offset", value);
        }
        for &value in KINESIS_FORMATS {
            assert_kinesis_value("--format", value);
        }
        for &value in KINESIS_AUTHS {
            assert_kinesis_value("--auth", value);
        }
        for &value in KINESIS_ITERATOR_TYPES {
            assert_kinesis_value("--iterator-type", value);
        }
        for &value in POSTGRES_TYPES {
            assert_postgres_value("--postgres-type", value);
        }
        for &value in DB_AUTHS {
            assert_postgres_value("--auth", value);
            assert_mysql_value("--auth", value);
        }
        for &value in REPLICATION_MODES {
            assert_postgres_value("--replication-mode", value);
            assert_mysql_value("--replication-mode", value);
            assert_mongodb_value("--replication-mode", value);
        }
        for &value in MYSQL_TYPES {
            assert_mysql_value("--mysql-type", value);
        }
        for &value in MYSQL_REPLICATION_MECHANISMS {
            assert_mysql_value("--replication-mechanism", value);
        }
        for &value in MONGODB_READ_PREFERENCES {
            assert_mongodb_value("--read-preference", value);
        }
    }

    #[test]
    fn possible_value_parsers_reject_unknown_values() {
        let invalid = "not-a-valid-value";
        let object_base = [
            "create",
            "object-storage",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
        ];
        assert_rejected(&[
            "create",
            "object-storage",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data",
            "--format",
            invalid,
            "--database",
            "db",
            "--table",
            "events",
        ]);
        for flag in ["--compression", "--storage-type"] {
            let mut args = object_base.to_vec();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }

        let kafka_base = [
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
        ];
        assert_rejected(&[
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
            invalid,
            "--database",
            "db",
            "--table",
            "events",
        ]);
        for flag in ["--kafka-type", "--auth", "--offset"] {
            let mut args = kafka_base.to_vec();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }

        let kinesis_base = [
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
        ];
        assert_rejected(&[
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
            invalid,
            "--database",
            "db",
            "--table",
            "events",
        ]);
        for flag in ["--auth", "--iterator-type"] {
            let mut args = kinesis_base.to_vec();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }

        let postgres_base = [
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
        ];
        for flag in ["--postgres-type", "--replication-mode", "--auth"] {
            let mut args = postgres_base.to_vec();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }

        let mysql_base = [
            "create",
            "mysql",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "mysql.example",
            "--username",
            "user",
            "--password",
            "password",
        ];
        for flag in [
            "--mysql-type",
            "--replication-mode",
            "--replication-mechanism",
            "--auth",
        ] {
            let mut args = mysql_base.to_vec();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }

        let mongodb_base = [
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe-1",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "user",
            "--password",
            "password",
        ];
        for flag in ["--replication-mode", "--read-preference"] {
            let mut args = mongodb_base.to_vec();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }
    }

    #[test]
    fn credential_flags_keep_pair_requirements() {
        let object_base = [
            "create",
            "object-storage",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
        ];
        for pair in [["--access-key-id", "access"], ["--secret-key", "secret"]] {
            let mut args = object_base.to_vec();
            args.extend(pair);
            assert_rejected(&args);
        }

        let kafka_base = [
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
        ];
        for pair in [
            ["--username", "user"],
            ["--password", "password"],
            ["--access-key-id", "access"],
            ["--secret-key", "secret"],
        ] {
            let mut args = kafka_base.to_vec();
            args.extend(pair);
            assert_rejected(&args);
        }

        let kinesis_base = [
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
        ];
        for pair in [["--access-key-id", "access"], ["--secret-key", "secret"]] {
            let mut args = kinesis_base.to_vec();
            args.extend(pair);
            assert_rejected(&args);
        }
    }

    #[test]
    fn clickpipe_write_classification_delegates_from_cloud_commands() {
        assert_write(&["list", "svc-1"], false);
        assert_write(&["get", "svc-1", "pipe-1"], false);
        assert_write(&["delete", "svc-1", "pipe-1"], true);
        assert_write(&["start", "svc-1", "pipe-1"], true);
        assert_write(&["stop", "svc-1", "pipe-1"], true);
        assert_write(&["resync", "svc-1", "pipe-1"], true);
        assert_write(&["scale", "svc-1", "pipe-1", "--replicas", "4"], true);
        assert_write(&["settings", "get", "svc-1", "pipe-1"], false);
        assert_write(&["settings", "update", "svc-1", "pipe-1"], true);
        assert_write(
            &[
                "schema-discover",
                "svc-1",
                "kafka",
                "--brokers",
                "broker:9092",
                "--topics",
                "topic",
                "--format",
                "JSONEachRow",
            ],
            true,
        );
        assert_write(
            &[
                "schema-discover",
                "svc-1",
                "kinesis",
                "--stream-name",
                "stream-1",
                "--region",
                "us-east-1",
                "--format",
                "JSONEachRow",
            ],
            true,
        );
        assert_write(
            &[
                "schema-discover",
                "svc-1",
                "object-storage",
                "--source-url",
                "https://bucket.example/data/*.json",
                "--format",
                "JSONEachRow",
            ],
            true,
        );
        assert_write(
            &[
                "create",
                "object-storage",
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
            ],
            true,
        );
    }

    /// Parse `schema-discover object-storage` args and return the source
    /// fields, so the builder tests exercise the real clap defaults.
    fn parse_object_storage_discovery(flags: &[&str]) -> Box<ObjectStorageSourceFields> {
        let mut args = vec!["schema-discover", "svc-1", "object-storage"];
        args.extend(flags.iter().copied());
        let ClickPipeCommands::SchemaDiscover {
            command: ClickPipeSchemaDiscoverCommands::ObjectStorage(source),
            ..
        } = parse_clickpipe(&args)
        else {
            panic!("expected object-storage schema discovery");
        };
        source
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
        // --iam-role wins over the other credential flags, exactly as it does
        // on `create object-storage`.
        assert_eq!(
            source.authentication,
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE)
        );
        assert_eq!(source.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(source.access_key, None);
        assert_eq!(source.connection_string.as_deref(), Some("connection"));
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
    fn build_kinesis_source_rejects_out_of_range_iterator_timestamp() {
        let args = KinesisSourceFields {
            stream_name: "stream".to_string(),
            region: "us-east-1".to_string(),
            format: "JSONEachRow".to_string(),
            auth: "IAM_ROLE".to_string(),
            iam_role: None,
            access_key_id: None,
            secret_key: None,
            iterator_type: "AT_TIMESTAMP".to_string(),
            iterator_timestamp: Some(u64::MAX),
            enhanced_fan_out: false,
        };
        let error = build_kinesis_source(&args).unwrap_err();
        assert!(
            error.to_string().contains("out of range"),
            "error should mention the range: {}",
            error
        );

        let args = KinesisSourceFields {
            iterator_timestamp: Some(1_750_000_000),
            ..args
        };
        let source = build_kinesis_source(&args).unwrap();
        assert_eq!(source.timestamp, Some(1_750_000_000));
    }

    fn postgres_builder_args() -> PostgresCreateArgs {
        PostgresCreateArgs {
            service_id: "svc-1".into(),
            name: "pipe-1".into(),
            host: "postgres.example".into(),
            port: 5432,
            pg_database: "source-db".into(),
            username: "user".into(),
            password: "password".into(),
            table_mappings: vec!["public.events:events".into()],
            table_mappings_json: vec![],
            postgres_type: "postgres".into(),
            replication_mode: "cdc".into(),
            auth: "basic".into(),
            iam_role: None,
            tls_host: None,
            ca_certificate: None,
            publication_name: None,
            replication_slot_name: None,
            sync_interval_seconds: None,
            pull_batch_size: None,
            initial_load_parallelism: None,
            snapshot_rows_per_partition: None,
            snapshot_parallel_tables: None,
            allow_nullable_columns: None,
            enable_failover_slots: None,
            delete_on_merge: None,
            destination_roles: DestinationRoleArgs::default(),
            org_id: None,
        }
    }

    #[test]
    fn build_postgres_request_supports_minimal_fields() {
        let request = build_postgres_request(&postgres_builder_args()).unwrap();

        assert_eq!(request.name, "pipe-1");
        assert!(request.field_mappings.is_empty());
        assert_eq!(request.scaling, None);
        assert_eq!(request.settings, None);
        assert_eq!(request.destination.database, "default");
        assert!(request.destination.columns.is_empty());
        assert_eq!(request.destination.table, None);
        assert_eq!(request.destination.managed_table, None);
        assert_eq!(request.destination.roles, None);
        assert_eq!(request.destination.table_definition, None);
        assert!(request.source.bigquery.is_none());
        assert!(request.source.kafka.is_none());
        assert!(request.source.kinesis.is_none());
        assert!(request.source.mongodb.is_none());
        assert!(request.source.mysql.is_none());
        assert!(request.source.object_storage.is_none());
        assert!(request.source.pubsub.is_none());
        assert!(!request.source.validate_samples);

        let source = request.source.postgres.as_ref().expect("postgres source");
        assert_eq!(source.r#type.as_ref().unwrap().to_string(), "postgres");
        assert_eq!(source.authentication.to_string(), "basic");
        assert_eq!(source.credentials.username, "user");
        assert_eq!(source.credentials.password, "password");
        assert_eq!(source.host, "postgres.example");
        assert_eq!(source.port, 5432);
        assert_eq!(source.database, "source-db");
        assert!(!source.disable_tls);
        assert!(!source.skip_cert_verification);
        assert_eq!(source.iam_role, None);
        assert_eq!(source.tls_host, None);
        assert_eq!(source.ca_certificate, None);
        assert_eq!(source.settings.replication_mode.to_string(), "cdc");
        assert_eq!(source.settings.publication_name, None);
        assert_eq!(source.settings.replication_slot_name, None);
        assert!(!source.settings.allow_nullable_columns);
        assert!(!source.settings.delete_on_merge);
        assert!(!source.settings.enable_failover_slots);
        assert_eq!(source.settings.initial_load_parallelism, None);
        assert_eq!(source.settings.pull_batch_size, None);
        assert_eq!(source.settings.snapshot_num_rows_per_partition, None);
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, None);
        assert_eq!(source.settings.sync_interval_seconds, None);
        assert_eq!(source.table_mappings.len(), 1);
        let mapping = &source.table_mappings[0];
        assert_eq!(mapping.source_schema_name, "public");
        assert_eq!(mapping.source_table, "events");
        assert_eq!(mapping.target_table, "events");
        assert!(mapping.excluded_columns.is_empty());
        assert_eq!(mapping.partition_by_expr, "");
        assert_eq!(mapping.partition_key, "");
        assert!(mapping.sorting_keys.is_empty());
        assert!(!mapping.use_custom_sorting_key);
    }

    #[test]
    fn build_postgres_request_supports_maximal_fields_and_certificate_content() {
        let directory = tempfile::tempdir().unwrap();
        let ca_certificate = directory.path().join("postgres-ca.pem");
        std::fs::write(&ca_certificate, "POSTGRES_CA").unwrap();
        let mut args = postgres_builder_args();
        args.name = "maximal-pipe".into();
        args.host = "rds.example".into();
        args.port = 65535;
        args.pg_database = "production".into();
        args.username = "iam-user".into();
        args.password = "iam-password".into();
        args.table_mappings = vec![
            "public.users:users_raw".into(),
            "audit.events:audit_events".into(),
        ];
        args.postgres_type = "rdspostgres".into();
        args.replication_mode = "cdc_only".into();
        args.auth = "IAM_ROLE".into();
        args.iam_role = Some("arn:aws:iam::123456789012:role/clickpipe".into());
        args.tls_host = Some("database.internal".into());
        args.ca_certificate = Some(ca_certificate.to_string_lossy().into_owned());
        args.publication_name = Some("clickpipe_publication".into());
        args.replication_slot_name = Some("clickpipe_slot".into());
        args.sync_interval_seconds = Some(30);
        args.pull_batch_size = Some(50_000);
        args.initial_load_parallelism = Some(4);
        args.snapshot_rows_per_partition = Some(1_000_000);
        args.snapshot_parallel_tables = Some(3);
        args.allow_nullable_columns = Some(true);
        args.enable_failover_slots = Some(true);
        args.delete_on_merge = Some(true);
        args.destination_roles = DestinationRoleArgs {
            roles: vec!["analytics_reader".into(), "analytics_writer".into()],
        };
        args.org_id = Some("org-1".into());

        let request = build_postgres_request(&args).unwrap();
        assert_eq!(request.name, "maximal-pipe");
        assert_eq!(request.destination.database, "default");
        assert_eq!(request.destination.table, None);
        assert_eq!(
            request.destination.roles,
            Some(vec![
                "analytics_reader".to_string(),
                "analytics_writer".to_string()
            ])
        );
        let source = request.source.postgres.as_ref().expect("postgres source");
        assert_eq!(source.r#type.as_ref().unwrap().to_string(), "rdspostgres");
        assert_eq!(source.authentication.to_string(), "IAM_ROLE");
        assert_eq!(source.credentials.username, "iam-user");
        assert_eq!(source.credentials.password, "iam-password");
        assert_eq!(source.host, "rds.example");
        assert_eq!(source.port, 65535);
        assert_eq!(source.database, "production");
        assert_eq!(
            source.iam_role.as_deref(),
            Some("arn:aws:iam::123456789012:role/clickpipe")
        );
        assert_eq!(source.tls_host.as_deref(), Some("database.internal"));
        assert_eq!(source.ca_certificate.as_deref(), Some("POSTGRES_CA"));
        assert_eq!(source.settings.replication_mode.to_string(), "cdc_only");
        assert_eq!(
            source.settings.publication_name.as_deref(),
            Some("clickpipe_publication")
        );
        assert_eq!(
            source.settings.replication_slot_name.as_deref(),
            Some("clickpipe_slot")
        );
        assert_eq!(source.settings.sync_interval_seconds, Some(30));
        assert_eq!(source.settings.pull_batch_size, Some(50_000));
        assert_eq!(source.settings.initial_load_parallelism, Some(4));
        assert_eq!(
            source.settings.snapshot_num_rows_per_partition,
            Some(1_000_000)
        );
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, Some(3));
        assert!(source.settings.allow_nullable_columns);
        assert!(source.settings.enable_failover_slots);
        assert!(source.settings.delete_on_merge);
        assert_eq!(source.table_mappings.len(), 2);
        assert_eq!(source.table_mappings[0].source_schema_name, "public");
        assert_eq!(source.table_mappings[0].source_table, "users");
        assert_eq!(source.table_mappings[0].target_table, "users_raw");
        assert_eq!(source.table_mappings[1].source_schema_name, "audit");
        assert_eq!(source.table_mappings[1].source_table, "events");
        assert_eq!(source.table_mappings[1].target_table, "audit_events");
    }

    #[test]
    fn build_postgres_pipe_settings_omits_unset_settings_and_sends_explicit_false() {
        let mut args = postgres_builder_args();
        args.sync_interval_seconds = Some(15);
        args.allow_nullable_columns = Some(false);
        args.delete_on_merge = Some(false);
        args.enable_failover_slots = Some(false);

        let settings = build_postgres_pipe_settings(&args).unwrap();
        assert_eq!(settings.sync_interval_seconds, Some(15));
        assert!(!settings.allow_nullable_columns);
        assert!(!settings.delete_on_merge);
        assert!(!settings.enable_failover_slots);
        assert_eq!(settings.pull_batch_size, None);
        assert_eq!(settings.initial_load_parallelism, None);
        assert_eq!(settings.snapshot_num_rows_per_partition, None);
        assert_eq!(settings.snapshot_number_of_parallel_tables, None);

        // The three schema-required booleans always serialize; every other
        // unset setting is omitted rather than sent as a zero value.
        let value = serde_json::to_value(&settings).unwrap();
        let mut keys: Vec<&str> = value
            .as_object()
            .expect("settings object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "allowNullableColumns",
                "deleteOnMerge",
                "enableFailoverSlots",
                "replicationMode",
                "syncIntervalSeconds",
            ]
        );
    }

    #[test]
    fn build_postgres_pipe_settings_defaults_optional_booleans_to_false() {
        let settings = build_postgres_pipe_settings(&postgres_builder_args()).unwrap();

        assert!(!settings.allow_nullable_columns);
        assert!(!settings.delete_on_merge);
        assert!(!settings.enable_failover_slots);
    }

    #[test]
    fn build_postgres_request_trims_table_mapping_components() {
        let mut args = postgres_builder_args();
        args.table_mappings = vec![" public.events : events_raw ".into()];

        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");
        let mapping = &source.table_mappings[0];
        assert_eq!(mapping.source_schema_name, "public");
        assert_eq!(mapping.source_table, "events");
        assert_eq!(mapping.target_table, "events_raw");
    }

    #[test]
    fn parse_postgres_table_mapping_json_accepts_a_minimal_object() {
        let mapping = parse_postgres_table_mapping_json(
            0,
            r#"{"sourceSchemaName":" public ","sourceTable":" users ","targetTable":" users_raw "}"#,
        )
        .unwrap();

        // Absent optional fields fall back to the same request shape the
        // simple `schema.table:target` form sends.
        assert_eq!(
            mapping,
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "users".into(),
                target_table: "users_raw".into(),
                excluded_columns: vec![],
                sorting_keys: vec![],
                use_custom_sorting_key: false,
                partition_by_expr: String::new(),
                partition_key: String::new(),
                table_engine: ClickPipePostgresPipeTableMappingTableengine::MergeTree,
            }
        );
    }

    #[test]
    fn parse_postgres_table_mapping_json_accepts_a_maximal_object() {
        let mapping = parse_postgres_table_mapping_json(0, MAXIMAL_TABLE_MAPPING_JSON).unwrap();

        assert_eq!(
            mapping,
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "users".into(),
                target_table: "users_raw".into(),
                excluded_columns: vec!["ssn".into(), "dob".into()],
                sorting_keys: vec!["created_at".into(), "id".into()],
                use_custom_sorting_key: true,
                partition_by_expr: "toYYYYMM(created_at)".into(),
                partition_key: "id".into(),
                table_engine: ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
            }
        );
    }

    #[test]
    fn parse_postgres_table_mapping_json_enables_the_custom_sorting_key() {
        // The API ignores `sortingKeys` unless `useCustomSortingKey` is true,
        // so omitting the flag must not silently drop the keys.
        let mapping = parse_postgres_table_mapping_json(
            0,
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users",
                "sortingKeys":["created_at"," id "]}"#,
        )
        .unwrap();
        assert!(mapping.use_custom_sorting_key);
        assert_eq!(mapping.sorting_keys, ["created_at", "id"]);

        // Without keys, the flag stays false and the pipe keeps the default
        // ordering key.
        let mapping = parse_postgres_table_mapping_json(
            0,
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users"}"#,
        )
        .unwrap();
        assert!(!mapping.use_custom_sorting_key);
    }

    #[test]
    fn parse_postgres_table_mapping_json_accepts_every_table_engine() {
        for engine in ClickPipePostgresPipeTableMappingTableengine::VALUES {
            let mapping = parse_postgres_table_mapping_json(
                0,
                &format!(
                    r#"{{"sourceSchemaName":"public","sourceTable":"users",
                        "targetTable":"users","tableEngine":"{engine}"}}"#
                ),
            )
            .unwrap();
            assert_eq!(mapping.table_engine.to_string(), *engine);
        }
    }

    #[test]
    fn parse_postgres_table_mapping_json_rejects_invalid_objects() {
        let cases = [
            // Not JSON, and not an object.
            ("{ nope", "invalid JSON"),
            (
                r#"["public.users"]"#,
                "expected a JSON object with the fields sourceSchemaName",
            ),
            // A typo in a field name is rejected instead of being ignored.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","excludeColumns":["ssn"]}"#,
                "unknown field excludeColumns; valid fields are sourceSchemaName",
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","engine":"Null","order":"id"}"#,
                "unknown fields engine, order",
            ),
            // Wrong JSON type for a known field.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":"id"}"#,
                "invalid type: string \"id\", expected a sequence",
            ),
            // Required fields, absent or empty.
            (
                r#"{"sourceTable":"users","targetTable":"users"}"#,
                "sourceSchemaName is required and must not be empty",
            ),
            (
                r#"{"sourceSchemaName":"public","targetTable":"users"}"#,
                "sourceTable is required and must not be empty",
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users"}"#,
                "targetTable is required and must not be empty",
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"  "}"#,
                "targetTable is required and must not be empty",
            ),
            // Empty list entries.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","excludedColumns":["ssn",""]}"#,
                "excludedColumns must not contain an empty entry",
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":[" "]}"#,
                "sortingKeys must not contain an empty entry",
            ),
            // The two contradictions between sortingKeys and its flag.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","useCustomSortingKey":true}"#,
                "useCustomSortingKey is true but sortingKeys is empty",
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":["id"],"useCustomSortingKey":false}"#,
                "sortingKeys is set but useCustomSortingKey is false",
            ),
            // An unknown table engine is caught here rather than reaching the
            // API, because the engine cannot be changed after creation.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","tableEngine":"MergeTre"}"#,
                "invalid tableEngine: unknown value 'MergeTre', expected one of: \
                 MergeTree, ReplacingMergeTree, Null",
            ),
        ];

        for (raw, diagnostic) in cases {
            let error = parse_postgres_table_mapping_json(0, raw).unwrap_err();
            assert!(
                error.message.contains(diagnostic),
                "expected `{diagnostic}` in: {}",
                error.message
            );
            // Every diagnostic names the offending flag occurrence.
            assert!(
                error.message.starts_with("--table-mapping-json #1: "),
                "{}",
                error.message
            );
        }

        // The occurrence number is one-based over the flag's own values.
        let error = parse_postgres_table_mapping_json(2, "{ nope").unwrap_err();
        assert!(
            error.message.starts_with("--table-mapping-json #3: "),
            "{}",
            error.message
        );
    }

    #[test]
    fn build_postgres_request_combines_both_table_mapping_flags() {
        let mut args = postgres_builder_args();
        args.table_mappings = vec!["public.events:events".into()];
        args.table_mappings_json = vec![
            MAXIMAL_TABLE_MAPPING_JSON.to_string(),
            r#"{"sourceSchemaName":"audit","sourceTable":"events","targetTable":"audit_events","tableEngine":"Null"}"#
                .to_string(),
        ];

        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");

        // The simple mappings come first, then the JSON ones in order.
        assert_eq!(source.table_mappings.len(), 3);
        assert_eq!(
            source.table_mappings[0],
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "events".into(),
                target_table: "events".into(),
                ..Default::default()
            }
        );
        assert_eq!(
            source.table_mappings[1],
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "users".into(),
                target_table: "users_raw".into(),
                excluded_columns: vec!["ssn".into(), "dob".into()],
                sorting_keys: vec!["created_at".into(), "id".into()],
                use_custom_sorting_key: true,
                partition_by_expr: "toYYYYMM(created_at)".into(),
                partition_key: "id".into(),
                table_engine: ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
            }
        );
        assert_eq!(
            source.table_mappings[2].table_engine,
            ClickPipePostgresPipeTableMappingTableengine::Null
        );
    }

    #[test]
    fn build_postgres_request_accepts_json_mappings_alone() {
        let mut args = postgres_builder_args();
        args.table_mappings.clear();
        args.table_mappings_json = vec![MAXIMAL_TABLE_MAPPING_JSON.to_string()];

        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");
        assert_eq!(source.table_mappings.len(), 1);
        assert_eq!(source.table_mappings[0].target_table, "users_raw");
    }

    #[test]
    fn build_postgres_request_preserves_basic_auth_replication_modes() {
        for mode in REPLICATION_MODES {
            let mut args = postgres_builder_args();
            args.replication_mode = (*mode).into();
            if *mode == "cdc_only" {
                args.publication_name = Some("publication".into());
                args.replication_slot_name = Some("slot".into());
            }

            let request = build_postgres_request(&args).unwrap();
            let source = request.source.postgres.as_ref().expect("postgres source");
            assert_eq!(source.authentication.to_string(), "basic");
            assert_eq!(source.iam_role, None);
            assert_eq!(source.settings.replication_mode.to_string(), *mode);
        }
    }

    #[test]
    fn build_postgres_request_defensively_rejects_invalid_inputs() {
        let cases = [
            {
                let mut args = postgres_builder_args();
                args.port = 0;
                (args, "--port must be in the range 1..=65535")
            },
            {
                let mut args = postgres_builder_args();
                args.table_mappings.clear();
                (
                    args,
                    "at least one --table-mapping <SCHEMA.TABLE:TARGET_TABLE> or \
                     --table-mapping-json <JSON> is required",
                )
            },
            {
                let mut args = postgres_builder_args();
                args.table_mappings_json = vec!["{}".into()];
                (
                    args,
                    "--table-mapping-json #1: sourceSchemaName is required",
                )
            },
            {
                let mut args = postgres_builder_args();
                args.table_mappings = vec![".events:events".into()];
                (args, "source schema must not be empty")
            },
            {
                let mut args = postgres_builder_args();
                args.auth = "IAM_ROLE".into();
                (args, "--auth IAM_ROLE requires --iam-role")
            },
            {
                let mut args = postgres_builder_args();
                args.iam_role = Some("arn:role".into());
                (args, "--iam-role cannot be used with --auth basic")
            },
            {
                let mut args = postgres_builder_args();
                args.replication_slot_name = Some("slot".into());
                (args, "--replication-slot-name can only be used")
            },
        ];

        for (args, diagnostic) in cases {
            let error = build_postgres_request(&args).unwrap_err();
            assert!(error.message.contains(diagnostic), "{}", error.message);
        }
    }

    #[test]
    fn parse_db_table_mappings_valid() {
        let mappings = vec![
            "public.users:public_users".to_string(),
            "schema1.orders:schema1_orders".to_string(),
        ];
        let result = parse_db_table_mappings(&mappings).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ("public".into(), "users".into(), "public_users".into())
        );
        assert_eq!(
            result[1],
            ("schema1".into(), "orders".into(), "schema1_orders".into())
        );
    }

    #[test]
    fn parse_db_table_mappings_missing_colon() {
        let mappings = vec!["public.users".to_string()];
        let result = parse_db_table_mappings(&mappings);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("expected schema.table:target_table")
        );
    }

    #[test]
    fn parse_db_table_mappings_missing_dot() {
        let mappings = vec!["users:target".to_string()];
        let result = parse_db_table_mappings(&mappings);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("expected schema.table")
        );
    }

    #[test]
    fn parse_db_table_mappings_empty() {
        let mappings: Vec<String> = vec![];
        let result = parse_db_table_mappings(&mappings).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_enum_known_variant() {
        use clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceFormat;
        let format: ClickPipePostObjectStorageSourceFormat = parse_enum("JSONEachRow").unwrap();
        assert_eq!(format, ClickPipePostObjectStorageSourceFormat::JSONEachRow);
    }

    #[test]
    fn parse_enum_unknown_falls_through() {
        // Unknown values map to the catch-all Unknown(String) variant —
        // forwarded to the API which returns the canonical validation error.
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceType;
        let kafka_type: ClickPipePostKafkaSourceType = parse_enum("not-a-real-type").unwrap();
        assert_eq!(
            kafka_type,
            ClickPipePostKafkaSourceType::Unknown("not-a-real-type".to_string())
        );
    }

    #[test]
    fn parse_enum_preserves_rename_spellings() {
        // Enums use `#[serde(rename = "s3")]` etc. — wire format is authoritative.
        use clickhouse_cloud_api::models::{
            ClickPipePostKafkaSourceAuthentication, ClickPipePostObjectStorageSourceType,
        };
        let source_type: ClickPipePostObjectStorageSourceType = parse_enum("s3").unwrap();
        assert_eq!(source_type, ClickPipePostObjectStorageSourceType::S3);
        let authentication: ClickPipePostKafkaSourceAuthentication =
            parse_enum("SCRAM-SHA-256").unwrap();
        assert_eq!(
            authentication,
            ClickPipePostKafkaSourceAuthentication::SCRAM_SHA_256
        );
    }

    #[test]
    fn parse_columns_valid() {
        let columns = vec!["id:Int64".to_string(), "name:String".to_string()];
        let parsed = parse_columns(&columns).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "id");
        assert_eq!(parsed[0].r#type, "Int64");
        assert_eq!(parsed[1].name, "name");
        assert_eq!(parsed[1].r#type, "String");
    }

    #[test]
    fn parse_columns_missing_colon_errors() {
        let columns = vec!["id_without_type".to_string()];
        let error = parse_columns(&columns).unwrap_err();
        assert!(error.message.contains("expected name:type"));
    }

    #[test]
    fn build_destination_uses_defaults_for_table_definition() {
        let destination = build_destination("mydb", "events", vec![], None);
        assert_eq!(destination.database, "mydb");
        assert_eq!(destination.table.as_deref(), Some("events"));
        assert_eq!(destination.managed_table, Some(true));
        // Default table engine is MergeTree, not something else.
        assert_eq!(
            destination
                .table_definition
                .as_ref()
                .expect("non-database pipe gets a tableDefinition")
                .engine
                .r#type,
            clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType::MergeTree
        );
    }

    #[test]
    fn build_destination_omits_table_fields_for_database_pipes() {
        let destination = build_destination("default", "", vec![], None);
        assert_eq!(destination.database, "default");
        assert_eq!(destination.table, None);
        assert!(destination.columns.is_empty());
        assert_eq!(destination.managed_table, None);
        assert_eq!(destination.roles, None);
        assert_eq!(destination.table_definition, None);
    }

    // `--role` → `destination.roles` (issue #568). Omitted means the field is
    // absent from the body, so ClickPipes applies the default role on its own.

    #[test]
    fn build_destination_carries_roles_on_both_destination_shapes() {
        let roles = Some(vec!["reader".to_string(), "writer".to_string()]);
        let streaming = build_destination("mydb", "events", vec![], roles.clone());
        assert_eq!(streaming.roles, roles);
        // Database pipes send only `database`, but `roles` is not one of the
        // four fields they reject, so it must survive that branch too.
        let database_pipe = build_destination("default", "", vec![], roles.clone());
        assert_eq!(database_pipe.roles, roles);
        assert_eq!(database_pipe.table, None);
    }

    #[test]
    fn build_destination_roles_omits_field_when_no_flags_passed() {
        assert_eq!(build_destination_roles(&[]), None);
    }

    #[test]
    fn build_destination_roles_dedupes_preserving_declaration_order() {
        let roles = [
            "writer".to_string(),
            "reader".to_string(),
            "writer".to_string(),
        ];
        assert_eq!(
            build_destination_roles(&roles),
            Some(vec!["writer".to_string(), "reader".to_string()])
        );
    }

    #[test]
    fn parse_destination_role_name_trims_and_accepts_ordinary_names() {
        assert_eq!(
            parse_destination_role_name("  analytics_reader  ").unwrap(),
            "analytics_reader"
        );
    }

    #[test]
    fn parse_destination_role_name_rejects_blank_names() {
        for blank in ["", "   ", "\t"] {
            let error = parse_destination_role_name(blank)
                .expect_err("a blank role name must be rejected client-side");
            assert!(error.message.contains("must not be empty"), "{error:?}");
        }
    }

    #[test]
    fn parse_destination_role_name_rejects_api_reserved_names() {
        for reserved in [
            "clickpipes",
            "clickpipes_system",
            "ClickPipes",
            "  clickpipes_system  ",
        ] {
            let error = parse_destination_role_name(reserved)
                .expect_err("reserved role names must be rejected client-side");
            assert!(
                error.message.contains("reserved by ClickPipes"),
                "{error:?}"
            );
            assert!(error.message.contains("clickpipes_system"), "{error:?}");
        }
    }

    #[test]
    fn parses_repeatable_role_on_kafka_create() {
        let mut args = kafka_create_cli_args();
        args.extend(["--role", "reader", "--role", "writer"]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected clickpipe create kafka");
        };
        assert_eq!(parsed.destination_roles.roles, vec!["reader", "writer"]);

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(parsed),
        } = parse_clickpipe(&kafka_create_cli_args())
        else {
            panic!("expected clickpipe create kafka");
        };
        assert!(parsed.destination_roles.roles.is_empty());
    }

    #[test]
    fn parses_repeatable_role_on_postgres_create() {
        let mut args = postgres_cli_args(Some("public.events:events"));
        args.extend(["--role", "reader", "--role", "writer"]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected clickpipe create postgres");
        };
        assert_eq!(parsed.destination_roles.roles, vec!["reader", "writer"]);

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(parsed),
        } = parse_clickpipe(&postgres_cli_args(Some("public.events:events")))
        else {
            panic!("expected clickpipe create postgres");
        };
        assert!(parsed.destination_roles.roles.is_empty());
    }

    #[test]
    fn role_is_available_on_every_create_subcommand() {
        for (subcommand, extra) in [
            (
                "object-storage",
                vec![
                    "--source-url",
                    "https://bucket.example/data",
                    "--format",
                    "JSONEachRow",
                    "--database",
                    "db",
                    "--table",
                    "events",
                ],
            ),
            (
                "kafka",
                vec![
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
                ],
            ),
            (
                "kinesis",
                vec![
                    "--stream-name",
                    "stream",
                    "--region",
                    "us-east-1",
                    "--format",
                    "JSONEachRow",
                    "--database",
                    "db",
                    "--table",
                    "events",
                ],
            ),
            (
                "postgres",
                vec![
                    "--host",
                    "postgres.example",
                    "--pg-database",
                    "source-db",
                    "--username",
                    "user",
                    "--password",
                    "password",
                    "--table-mapping",
                    "public.events:events",
                ],
            ),
            (
                "mysql",
                vec![
                    "--host",
                    "mysql.example",
                    "--username",
                    "user",
                    "--password",
                    "password",
                    "--table-mapping",
                    "mydb.events:events",
                ],
            ),
            (
                "mongodb",
                vec![
                    "--uri",
                    "mongodb://mongo.example:27017",
                    "--username",
                    "user",
                    "--password",
                    "password",
                    "--table-mapping",
                    "mydb.events:events",
                ],
            ),
            (
                "bigquery",
                vec![
                    "--service-account-file",
                    "./sa-key.json",
                    "--staging-path",
                    "gs://bucket/staging",
                    "--table-mapping",
                    "dataset.events:events",
                ],
            ),
        ] {
            let mut args = vec!["create", subcommand, "svc-1", "--name", "pipe-1"];
            args.extend(extra);
            args.extend(["--role", "reader"]);
            parse_clickpipe(&args);

            let mut reserved = args.clone();
            reserved.pop();
            reserved.push("clickpipes");
            let error = clickpipe_parse_error(&reserved);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert_eq!(error.exit_code(), 2);
            assert!(
                error.to_string().contains("reserved by ClickPipes"),
                "{subcommand}: {error}"
            );
        }
    }

    // `build_kafka_credentials` tests — lock the wire shape for each auth mode.
    // Authoritative source: `ClickPipePostKafkaSource.credentials` in
    // `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json`.

    fn kafka_args() -> KafkaCreateArgs {
        KafkaCreateArgs {
            service_id: "svc".into(),
            name: "pipe".into(),
            source: KafkaSourceFields {
                brokers: "b:9092".into(),
                topics: "t".into(),
                format: "JSONEachRow".into(),
                kafka_type: "kafka".into(),
                consumer_group: None,
                auth: None,
                username: None,
                password: None,
                iam_role: None,
                access_key_id: None,
                secret_key: None,
                offset: "from_beginning".into(),
                offset_timestamp: None,
                schema_registry_url: None,
                schema_registry_username: None,
                schema_registry_password: None,
                ca_certificate: None,
                client_certificate: None,
                client_key: None,
                schema_registry_ca_certificate: None,
                reverse_private_endpoint_ids: vec![],
            },
            database: "d".into(),
            table: "t".into(),
            columns: vec![],
            destination_roles: DestinationRoleArgs::default(),
            org_id: None,
        }
    }

    /// Wire value of the built source's authentication, or `None` when the
    /// request omits authentication entirely (no-auth broker).
    fn kafka_auth(
        source: &clickhouse_cloud_api::models::ClickPipePostKafkaSource,
    ) -> Option<String> {
        source.authentication.as_ref().map(ToString::to_string)
    }

    #[test]
    fn build_kafka_source_defaults_to_no_authentication() {
        // No --auth and no credential flags: the request must omit
        // authentication rather than inventing PLAIN, so brokers that require
        // no authentication are reachable (issue #606).
        let args = kafka_args().source;
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source), None);
        assert!(
            source.credentials.is_null(),
            "no-auth source must not carry credentials: {}",
            source.credentials
        );
        assert_eq!(source.iam_role, None);
    }

    #[test]
    fn build_kafka_source_infers_plain_from_username_and_password() {
        let mut args = kafka_args().source;
        args.username = Some("user".into());
        args.password = Some("password".into());
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source).as_deref(), Some("PLAIN"));
        assert_eq!(source.credentials["username"], "user");
        assert_eq!(source.credentials["password"], "password");
    }

    #[test]
    fn build_kafka_source_infers_iam_user_from_access_keys() {
        let mut args = kafka_args().source;
        args.access_key_id = Some("AKIA".into());
        args.secret_key = Some("secret".into());
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source).as_deref(), Some("IAM_USER"));
        assert_eq!(source.credentials["accessKeyId"], "AKIA");
        assert_eq!(source.credentials["secretKey"], "secret");
    }

    #[test]
    fn build_kafka_source_infers_iam_role_from_role_arn() {
        let mut args = kafka_args().source;
        args.iam_role = Some("arn:aws:iam::123:role/Foo".into());
        let source = build_kafka_source(&args).unwrap();
        assert_eq!(kafka_auth(&source).as_deref(), Some("IAM_ROLE"));
        assert!(source.credentials.is_null());
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
        assert_eq!(kafka_auth(&source).as_deref(), Some("MUTUAL_TLS"));
        assert_eq!(source.credentials["certificate"], "CLIENT_CERT");
        assert_eq!(source.credentials["privateKey"], "CLIENT_KEY");
    }

    #[test]
    fn build_kafka_source_errors_when_explicit_mechanism_lacks_credentials() {
        // An explicitly selected mechanism still fails fast, and the message
        // names the mechanism the user actually asked for.
        for auth in ["PLAIN", "SCRAM-SHA-256", "SCRAM-SHA-512"] {
            let mut args = kafka_args().source;
            args.auth = Some(auth.into());
            let error = build_kafka_source(&args).unwrap_err();
            assert_eq!(
                error.message,
                format!("{auth} requires --username and --password")
            );
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
    fn kafka_credentials_absent_authentication_is_null() {
        let args = kafka_args();
        let credentials = build_kafka_credentials(None, &args.source, None).unwrap();
        assert!(credentials.is_null());
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
        assert_eq!(kafka_auth(&source).as_deref(), Some("PLAIN"));
        assert_eq!(source.credentials["username"], "user");
        assert_eq!(source.credentials["password"], "password");
        assert_eq!(source.consumer_group, None);
        assert_eq!(source.exactly_once, None);
        assert_eq!(source.iam_role, None);
        assert_eq!(source.ca_certificate, None);
        assert_eq!(source.schema_registry, None);
        assert!(source.reverse_private_endpoint_ids.is_empty());
        let offset = source.offset.expect("Kafka offset is always populated");
        assert_eq!(offset.strategy.to_string(), "from_beginning");
        assert_eq!(offset.timestamp, None);
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
        args.username = Some("user".into());
        args.password = Some("password".into());
        args.iam_role = Some("arn:role".into());
        args.access_key_id = Some("access".into());
        args.secret_key = Some("secret".into());
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
        assert_eq!(kafka_auth(&source).as_deref(), Some("MUTUAL_TLS"));
        assert_eq!(source.credentials["certificate"], "CLIENT_CERT");
        assert_eq!(source.credentials["privateKey"], "CLIENT_KEY");
        assert_eq!(source.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(source.ca_certificate.as_deref(), Some("BROKER_CA"));
        assert_eq!(
            source.reverse_private_endpoint_ids,
            ["endpoint-1", "endpoint-2"]
        );
        let offset = source.offset.expect("Kafka offset is always populated");
        assert_eq!(offset.strategy.to_string(), "from_timestamp");
        assert_eq!(offset.timestamp.as_deref(), Some("2021-01-01T00:00"));
        let registry = source
            .schema_registry
            .expect("schema registry is populated");
        assert_eq!(registry.url, "https://registry.example");
        assert_eq!(registry.credentials.username, "registry-user");
        assert_eq!(registry.credentials.password, "registry-password");
        assert_eq!(registry.ca_certificate.as_deref(), Some("REGISTRY_CA"));
    }

    #[test]
    fn build_kinesis_source_supports_minimal_fields() {
        let args = KinesisSourceFields {
            stream_name: "stream".into(),
            region: "us-east-1".into(),
            format: "JSONEachRow".into(),
            auth: "IAM_ROLE".into(),
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
            stream_name: "stream".into(),
            region: "us-east-1".into(),
            format: "AvroConfluent".into(),
            auth: "IAM_USER".into(),
            iam_role: Some("arn:role".into()),
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
        assert_eq!(source.iam_role.as_deref(), Some("arn:role"));
        let access_key = source.access_key.expect("access key is populated");
        assert_eq!(access_key.access_key_id, "access");
        assert_eq!(access_key.secret_key, "secret");
        assert_eq!(source.timestamp, Some(1_750_000_000));
        assert_eq!(source.use_enhanced_fan_out, Some(true));
    }

    #[test]
    fn kafka_credentials_plain_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("PLAIN".into());
        args.source.username = Some("u".into());
        args.source.password = Some("p".into());
        let credentials = build_kafka_credentials(Some(&Auth::PLAIN), &args.source, None).unwrap();
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
        let credentials =
            build_kafka_credentials(Some(&Auth::IAM_USER), &args.source, None).unwrap();
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
        let credentials =
            build_kafka_credentials(Some(&Auth::IAM_ROLE), &args.source, None).unwrap();
        assert!(credentials.is_null());
    }

    #[test]
    fn kafka_credentials_mutual_tls_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let contents = Some(("CERT_PEM".into(), "KEY_PEM".into()));
        let credentials =
            build_kafka_credentials(Some(&Auth::MUTUAL_TLS), &args.source, contents).unwrap();
        assert_eq!(credentials["certificate"], "CERT_PEM");
        assert_eq!(credentials["privateKey"], "KEY_PEM");
    }

    #[test]
    fn kafka_credentials_iam_user_missing_args_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let error = build_kafka_credentials(Some(&Auth::IAM_USER), &args.source, None).unwrap_err();
        assert!(error.message.contains("--access-key-id"));
    }

    #[test]
    fn kafka_credentials_iam_role_missing_arn_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        let error = build_kafka_credentials(Some(&Auth::IAM_ROLE), &args.source, None).unwrap_err();
        assert!(error.message.contains("--iam-role"));
    }
}
