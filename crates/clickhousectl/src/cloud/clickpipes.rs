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
        parse_clickpipe(&[
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
            flag,
            value,
        ]);
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

        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            org_id,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, None);
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, None);
        assert_eq!(org_id, None);
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
        assert_eq!(args.source_url, "https://bucket.example/data/*.csv");
        assert_eq!(args.format, "CSV");
        assert_eq!(args.database, "db");
        assert_eq!(args.table, "events");
        assert_eq!(args.columns, ["id:UInt64", "name:String"]);
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
        assert!(args.skip_initial_load);

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
            "snapshot",
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
        assert_eq!(args.replication_mode, "snapshot");
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
        ])
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.port, 5432);
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.postgres_type, "postgres");
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.auth, "basic");
        assert_eq!(args.iam_role, None);
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert_eq!(args.publication_name, None);
        assert_eq!(args.replication_slot_name, None);
        assert_eq!(args.org_id, None);
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
        assert_write(&["scale", "svc-1", "pipe-1"], true);
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
}
