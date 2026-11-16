use super::ClickPipeSourceKind;
use super::create::build_create_request_args;
use super::destination::{parse_destination_column, parse_destination_role};
use super::sources::bigquery::{BigQueryCreateArgs, validate_bigquery_create_args};
use super::sources::kafka::{KafkaCreateArgs, KafkaSchemaDiscoverArgs, validate_kafka_source_args};
use super::sources::kinesis::{
    KinesisCreateArgs, KinesisSchemaDiscoverArgs, validate_kinesis_source_args,
};
use super::sources::mongodb::{MongoDbCreateArgs, validate_mongodb_create_args};
use super::sources::mysql::{MySqlCreateArgs, validate_mysql_create_args};
use super::sources::object_storage::{
    ObjectStorageCreateArgs, ObjectStorageSchemaDiscoverArgs, validate_object_storage_source_args,
};
use super::sources::postgres::{PostgresCreateArgs, validate_postgres_create_args};
use super::sources::pubsub::{
    PubSubCreateArgs, PubSubSchemaDiscoverArgs, validate_pubsub_source_args,
};
use crate::cloud::shared::NameSelector;
use clap::builder::NonEmptyStringValueParser;
use clap::{ArgGroup, Args, Subcommand};

fn parse_cdc_cpu_millicores(value: &str) -> Result<u32, String> {
    let value = value
        .parse::<u32>()
        .map_err(|_| "must be an integer from 1000 to 32000".to_string())?;
    if !(1000..=32000).contains(&value) || value % 1000 != 0 {
        return Err("must be from 1000 to 32000 in increments of 1000".to_string());
    }
    Ok(value)
}

fn parse_cdc_memory_gb(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "must be a number from 4 to 128".to_string())?;
    if !value.is_finite() || !(4.0..=128.0).contains(&value) || (value / 4.0).fract() != 0.0 {
        return Err("must be from 4 to 128 in increments of 4".to_string());
    }
    Ok(value)
}

fn parse_streaming_memory_gb(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "must be a number from 0.5 to 8".to_string())?;
    if !value.is_finite() || !(0.5..=8.0).contains(&value) {
        return Err("must be from 0.5 to 8".to_string());
    }
    Ok(value)
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ClickPipeCommands {
    /// List ClickPipes
    List {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,
    },

    /// Get ClickPipe details
    Get {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,
    },

    /// Update a ClickPipe
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The file is a typed PATCH body; omitted top-level fields remain unchanged.
  Updatable sources: kafka, kinesis, objectStorage, pubsub, postgres, mysql, mongodb (not BigQuery).
  For Postgres, MySQL, and MongoDB: stop -> get until Paused -> update --file.
  Update does not stop or restart the pipe; run start after a successful update.
  fieldMappings needs destination.columns, matched 1:1; [] and CDC pipes are rejected.
  GET returns fieldMappings: [] for CDC pipes: remove it before reusing the output.
  Root settings are rejected; change them with `clickpipe settings update`.")]
    Update {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,

        /// JSON PATCH body path, or `-` for stdin
        #[arg(
            long = "file",
            alias = "config-file",
            value_name = "PATH",
            required = true
        )]
        config_file: String,
    },

    /// Delete a ClickPipe
    Delete {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,
    },

    /// Start a ClickPipe
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Streaming and object-storage pipes use Stopped when halted; database CDC pipes
  use Paused. Inspect the current state with `get` before requesting start.
  Rejected while Running, Degraded, InternalError or Provisioning. Failed pipes re-run.
  A start answered with Pausing, or with Paused just after a stop, is not applied
  and exits 1: wait for Paused (`get`), rerun start until it returns Running.
  A Completed pipe does not re-run: start exits 1 and there is nothing to retry.")]
    Start {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,
    },

    /// Stop a ClickPipe
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Streaming and object-storage pipes transition through Stopping to Stopped;
  database CDC pipes transition through Pausing to Paused.
  Rejected while Stopped, Stopping, Paused, Pausing, Modifying, Resync or Provisioning.")]
    Stop {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,
    },

    /// Resync a ClickPipe (database pipes only)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Re-ingests data, so it is not a safe retry target. Only Postgres, MySQL, MongoDB and
  BigQuery pipes; rejected for a Postgres CDC-only pipe with a named replication slot.")]
    Resync {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,
    },

    /// Update ClickPipe scaling
    #[command(
        group(ArgGroup::new("scale_target").required(true).multiple(true).args(["replicas", "cpu_millicores", "memory_gb"]))
    )]
    Scale {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,

        /// Number of replicas (1-40, streaming pipes)
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=40))]
        replicas: Option<u32>,

        /// CPU millicores per replica (125-2000, streaming pipes)
        #[arg(long, value_parser = clap::value_parser!(u32).range(125..=2000))]
        cpu_millicores: Option<u32>,

        /// Memory GB per replica (0.5-8, streaming pipes)
        #[arg(long, value_parser = parse_streaming_memory_gb)]
        memory_gb: Option<f64>,
    },

    /// Manage service-wide CDC scaling
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The allocation applies to database CDC ClickPipes on the service.
  Typical flow: `get` -> `update` -> poll `get` until the new values show.")]
    CdcScaling {
        #[command(subcommand)]
        command: ClickPipeCdcScalingCommands,
    },

    /// Manage ingestion settings (streaming, object-storage pipes)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  CDC pipes (Postgres, MySQL, MongoDB, BigQuery) have no ingestion settings: their
  sync interval and pull batch size are fields of `clickhousectl cloud clickpipe get`.")]
    Settings {
        #[command(subcommand)]
        command: ClickPipeSettingsCommands,
    },

    /// Inspect GCP workload identity support and principal
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only applies to ClickHouse Cloud services hosted on GCP.
  Use `get` before create to verify readiness and obtain the service principal.
  Grant that principal access to GCS, GCMK, Pub/Sub or BigQuery source resources.")]
    Context {
        #[command(subcommand)]
        command: ClickPipeContextCommands,
    },

    /// Discover a source schema without creating a pipe (Beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Needs API key auth even though it only reads; OAuth is rejected here.
  Output is one inferred name/type per field. Save the --json output and pass it to
  `clickpipe create <source> --columns-from`, which takes the same source flags.
  Virtual fields are metadata ClickPipes adds (e.g. `_topic`, `_path`), not source data.
  GCP workload identity uses the principal from `clickpipe context get`.
  Typical flow: `schema-discover <source> <service-id>` -> `create <source> <service-id>`.")]
    SchemaDiscover {
        #[command(subcommand)]
        command: ClickPipeSchemaDiscoverCommands,
    },

    /// Manage reverse private endpoints
    #[command(
        name = "reverse-private-endpoint",
        after_help = "\
CONTEXT FOR AGENTS:
  A pipe can only use an endpoint whose status is Ready; an AWS PrivateLink endpoint
  stays in PendingAcceptance until accepted in the account that owns the source.
  Kafka: pass the endpoint's id to `clickpipe create kafka --reverse-private-endpoint-id`.
  Postgres and MySQL CDC: pass one of the endpoint's DNS names as --host (see `get`).
  Typical flow: `create` -> `get` until Ready -> `clickpipe create <source>`."
    )]
    ReversePrivateEndpoint {
        #[command(subcommand)]
        command: crate::cloud::clickpipe_endpoints::ReversePrivateEndpointCommands,
    },

    /// Create a ClickPipe
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  For kafka, kinesis, object-storage and pubsub, pass the --json output of
  `clickpipe schema-discover <source> <service-id>` as --columns-from (or use --column).
  GCP workload identity is private preview for GCS, GCMK, Pub/Sub and BigQuery:
  run `clickpipe context get`, grant its principal access, then pass --auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY.
  The source must be reachable from ClickPipes; allow the static egress IPs:
  https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips
  Prints the pipe's name, ID and state; it is not ready to query yet.
  Next: `clickpipe list <service-id>`, `clickpipe get <service-id> <clickpipe-id>`.")]
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
            ClickPipeCommands::Update { .. } => true,
            ClickPipeCommands::Delete { .. } => true,
            ClickPipeCommands::Start { .. } => true,
            ClickPipeCommands::Stop { .. } => true,
            ClickPipeCommands::Resync { .. } => true,
            ClickPipeCommands::Scale { .. } => true,
            ClickPipeCommands::CdcScaling { command } => command.is_write(),
            // Side-effect-free, but the API gateway rejects OAuth/JWT on
            // POST /clickpipes/schemaDiscovery ("This endpoint is not
            // available for JWT authentication"), so classify it as a
            // write to fail fast with the API-key guidance.
            ClickPipeCommands::SchemaDiscover { .. } => true,
            ClickPipeCommands::Create { .. } => true,
            ClickPipeCommands::Context { .. } => false,
            ClickPipeCommands::Settings { command } => command.is_write(),
            ClickPipeCommands::ReversePrivateEndpoint { command } => command.is_write(),
        }
    }

    /// The `clickpipe create` validation message for invalid source or request
    /// flags, paired with the source
    /// subcommand the usage error belongs to. clap cannot express "forbidden
    /// for this value of another argument", so these checks run after parsing.
    pub(crate) fn clickpipe_create_validation_error(&self) -> Option<(&'static str, String)> {
        let ClickPipeCommands::Create { command } = self else {
            return None;
        };

        // Exhaustive so every source runs common request validation before
        // credentials, file reads, or HTTP. Database-specific validation runs
        // after the common shape and compatibility checks.
        let (source, error) = match command {
            ClickPipeCreateCommands::ObjectStorage(args) => (
                "object-storage",
                build_create_request_args(&args.request, ClickPipeSourceKind::ObjectStorage)
                    .and_then(|_| validate_object_storage_source_args(&args.source))
                    .and_then(|_| {
                        args.destination.refuse_shared_stdin(&[(
                            "--service-account-file",
                            args.source.service_account_file.as_deref(),
                        )])
                    })
                    .err(),
            ),
            ClickPipeCreateCommands::Kafka(args) => (
                "kafka",
                build_create_request_args(&args.request, ClickPipeSourceKind::Kafka)
                    .and_then(|_| validate_kafka_source_args(&args.source))
                    .and_then(|_| {
                        args.destination.refuse_shared_stdin(&[(
                            "--protobuf-schema-file",
                            args.source.protobuf_schema_file.as_deref(),
                        )])
                    })
                    .err(),
            ),
            ClickPipeCreateCommands::Kinesis(args) => (
                "kinesis",
                build_create_request_args(&args.request, ClickPipeSourceKind::Kinesis)
                    .and_then(|_| validate_kinesis_source_args(&args.source))
                    .and_then(|_| {
                        args.destination.refuse_shared_stdin(&[(
                            "--protobuf-schema-file",
                            args.source.protobuf_schema_file.as_deref(),
                        )])
                    })
                    .err(),
            ),
            ClickPipeCreateCommands::Postgres(args) => {
                ("postgres", validate_postgres_create_args(args).err())
            }
            ClickPipeCreateCommands::MySQL(args) => {
                ("mysql", validate_mysql_create_args(args).err())
            }
            ClickPipeCreateCommands::MongoDB(args) => {
                ("mongodb", validate_mongodb_create_args(args).err())
            }
            ClickPipeCreateCommands::BigQuery(args) => {
                ("bigquery", validate_bigquery_create_args(args).err())
            }
            ClickPipeCreateCommands::PubSub(args) => (
                "pubsub",
                build_create_request_args(&args.request, ClickPipeSourceKind::PubSub)
                    .and_then(|_| validate_pubsub_source_args(&args.source))
                    .and_then(|_| {
                        args.destination.refuse_shared_stdin(&[(
                            "--service-account-file",
                            args.source.service_account_file.as_deref(),
                        )])
                    })
                    .err(),
            ),
        };

        error.map(|error| (source, error.message))
    }

    pub(crate) fn clickpipe_schema_discover_validation_error(
        &self,
    ) -> Option<(&'static str, String)> {
        let Self::SchemaDiscover { command } = self else {
            return None;
        };
        // Exhaustive so every source runs its no-I/O checks before credentials,
        // file reads, or HTTP.
        let (source, error) = match command {
            ClickPipeSchemaDiscoverCommands::Kinesis(args) => {
                ("kinesis", validate_kinesis_source_args(&args.source).err())
            }
            ClickPipeSchemaDiscoverCommands::Kafka(args) => {
                ("kafka", validate_kafka_source_args(&args.source).err())
            }
            ClickPipeSchemaDiscoverCommands::ObjectStorage(args) => (
                "object-storage",
                validate_object_storage_source_args(&args.source).err(),
            ),
            ClickPipeSchemaDiscoverCommands::PubSub(args) => {
                ("pubsub", validate_pubsub_source_args(&args.source).err())
            }
        };
        error.map(|error| (source, error.message))
    }

    pub(crate) fn reverse_private_endpoint_create_validation_error(&self) -> Option<String> {
        let ClickPipeCommands::ReversePrivateEndpoint { command } = self else {
            return None;
        };

        command.create_validation_error()
    }
}

#[derive(Subcommand)]
pub enum ClickPipeContextCommands {
    /// Get GCP workload identity readiness and principal
    Get {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,
    },
}

#[derive(Subcommand)]
pub enum ClickPipeCdcScalingCommands {
    /// Get CDC scaling
    Get {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,
    },

    /// Update CDC scaling
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Exit 0 means accepted, not completed; the change takes minutes and the output shows the old values.
  Poll `cdc-scaling get <service-id> --json` until they change; another update meanwhile is rejected.")]
    Update {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// CPU millicores per replica (1000-32000, in increments of 1000)
        #[arg(long, required = true, value_parser = parse_cdc_cpu_millicores)]
        cpu_millicores: u32,

        /// Memory GiB per replica (4-128, in increments of 4)
        ///
        /// Must equal --cpu-millicores / 250, for example 8 with 2000.
        #[arg(long, required = true, value_parser = parse_cdc_memory_gb)]
        memory_gb: f64,
    },
}

impl ClickPipeCdcScalingCommands {
    fn is_write(&self) -> bool {
        match self {
            Self::Get { .. } => false,
            Self::Update { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum ClickPipeSchemaDiscoverCommands {
    /// Discover schema from a Kafka or Kafka-compatible source
    Kafka(Box<KafkaSchemaDiscoverArgs>),

    /// Discover schema from an Amazon Kinesis stream
    Kinesis(Box<KinesisSchemaDiscoverArgs>),

    /// Discover schema from object storage
    #[command(
        name = "object-storage",
        after_help = "\
CONTEXT FOR AGENTS:
  Discovery runs on the destination service, which must be running."
    )]
    ObjectStorage(Box<ObjectStorageSchemaDiscoverArgs>),

    /// Discover schema from Pub/Sub (limited preview)
    #[command(name = "pubsub")]
    PubSub(Box<PubSubSchemaDiscoverArgs>),
}

#[derive(Subcommand)]
pub enum ClickPipeSettingsCommands {
    /// Get ingestion settings (streaming, object-storage pipes)
    Get {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,
    },

    /// Update ingestion settings (streaming, object-storage pipes)
    #[command(group(
        ArgGroup::new("settings")
            .required(true)
            .multiple(true)
            .args([
                "streaming_max_insert_wait_ms",
                "object_storage_concurrency",
                "object_storage_polling_interval_ms",
                "object_storage_max_insert_bytes",
                "object_storage_max_file_count",
                "object_storage_max_file_size",
                "clickhouse_max_threads",
                "clickhouse_max_insert_threads",
                "clickhouse_max_download_threads",
                "clickhouse_min_insert_block_size_bytes",
                "clickhouse_parallel_distributed_insert_select",
                "kafka_read_committed",
                "object_storage_use_cluster_function",
                "clickhouse_parallel_view_processing",
            ])
    ), after_help = "\
CONTEXT FOR AGENTS:
  Omitted object-storage settings keep their current values.
  Kafka read-committed is preserved unless explicitly changed.")]
    Update {
        /// Service ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// ClickPipe ID
        #[command(flatten)]
        clickpipe_id: NameSelector,

        #[command(flatten)]
        settings: ClickPipeSettingsValues,
    },
}

/// Ingestion settings shared by create and `settings update`.
///
/// Every field is optional so an omitted create sends no `settings` block and
/// an update only changes the selected keys. `false` and zero remain distinct
/// from omission because clap stores explicit values in `Option`.
#[derive(Args, Debug, Clone, Default, PartialEq)]
pub struct ClickPipeSettingsValues {
    /// Max wait before inserting data (ms, 500-60000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(500..=60000))]
    pub(super) streaming_max_insert_wait_ms: Option<u32>,

    /// Concurrent file processing threads (1-35)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=35))]
    pub(super) object_storage_concurrency: Option<u32>,

    /// Polling interval for continuous ingest (ms, 100-3600000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(100..=3600000))]
    pub(super) object_storage_polling_interval_ms: Option<u32>,

    /// Bytes per insert batch (524288000-10737418240)
    #[arg(long, value_parser = clap::value_parser!(u64).range(524288000..=10737418240))]
    pub(super) object_storage_max_insert_bytes: Option<u64>,

    /// Max files per insert batch (1-10000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10000))]
    pub(super) object_storage_max_file_count: Option<u32>,

    /// Max file size in bytes (private preview; can only lower the limit)
    #[arg(long, value_name = "BYTES", value_parser = clap::value_parser!(i64).range(0..))]
    pub(super) object_storage_max_file_size: Option<i64>,

    /// Max concurrent threads for file processing (0-64)
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=64))]
    pub(super) clickhouse_max_threads: Option<u32>,

    /// Max concurrent insert threads (0-16)
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=16))]
    pub(super) clickhouse_max_insert_threads: Option<u32>,

    /// Max concurrent download threads (0-32)
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=32))]
    pub(super) clickhouse_max_download_threads: Option<u32>,

    /// Minimum insert block size in bytes (0-10737418240)
    #[arg(long, value_parser = clap::value_parser!(u64).range(0..=10737418240))]
    pub(super) clickhouse_min_insert_block_size_bytes: Option<u64>,

    /// Distributed INSERT SELECT mode (0-2)
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=2))]
    pub(super) clickhouse_parallel_distributed_insert_select: Option<u32>,

    /// Read only committed messages (Kafka only)
    #[arg(long)]
    pub(super) kafka_read_committed: Option<bool>,

    /// Use ClickHouse cluster function
    #[arg(long)]
    pub(super) object_storage_use_cluster_function: Option<bool>,

    /// Push to attached views concurrently
    #[arg(long)]
    pub(super) clickhouse_parallel_view_processing: Option<bool>,
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
#[allow(clippy::large_enum_variant)]
pub enum ClickPipeCreateCommands {
    /// Create a ClickPipe from object storage
    #[command(
        name = "object-storage",
        after_help = "\
CONTEXT FOR AGENTS:
  First run `clickpipe schema-discover object-storage <service-id>` with the same
  source flags and --json, then pass its output as --columns-from (or --column).
  Auth is inferred from the credential flags, in order: --iam-role,
  --access-key-id/--secret-key, --connection-string, --service-account-file.
  GCS workload identity uses --auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY without
  credential flags.
  With no credential flag nothing is sent, so the source must be public."
    )]
    ObjectStorage(ObjectStorageCreateArgs),

    /// Create a ClickPipe from Kafka or Kafka-compatible source
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The selected broker credentials must allow topic discovery and consumption.
  Avro and Protobuf need a schema registry; Protobuf can instead use
  --protobuf-schema-file.
  https://clickhouse.com/docs/integrations/clickpipes/kafka/create-kafka-clickpipe")]
    Kafka(KafkaCreateArgs),

    /// Create a ClickPipe from Amazon Kinesis
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The IAM role or access keys must allow ClickPipes to read the selected stream.
  IAM role names must start with ClickHouseAccessRole- and trust the ClickHouse
  service IAM role:
  https://clickhouse.com/docs/integrations/clickpipes/kinesis/auth")]
    Kinesis(KinesisCreateArgs),

    /// Create a ClickPipe from PostgreSQL
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  For CDC the source needs logical replication, a publication containing every
  mapped table, and REPLICATION on the source user:
  https://clickhouse.com/docs/integrations/clickpipes/postgres
  TLS verifies certificates by default; use --ca-certificate for a private CA instead of disabling verification.
  Managed Postgres CA: `cloud postgres certs get <pg-id> --output ca.pem`.
  Only --sync-interval-seconds and --pull-batch-size can change after creation.
  Current-state queries: SELECT ... FROM target FINAL WHERE _peerdb_is_deleted = 0.")]
    Postgres(PostgresCreateArgs),

    /// Create a ClickPipe from MySQL
    #[command(
        name = "mysql",
        after_help = "\
CONTEXT FOR AGENTS:
  For CDC enable ROW/FULL binlogs and retain them for at least 72 hours.
  GTID is the default; use FILE_POS only for matching legacy replication.
  Grant SELECT, REPLICATION CLIENT and REPLICATION SLAVE to the source user:
  https://clickhouse.com/docs/integrations/clickpipes/mysql/source/generic"
    )]
    MySQL(MySqlCreateArgs),

    /// Create a ClickPipe from MongoDB
    #[command(
        name = "mongodb",
        after_help = "\
CONTEXT FOR AGENTS:
  CDC requires MongoDB 5.1+ in a replica set or sharded cluster.
  Keep at least 24 hours of oplog history; 72 hours or more is recommended.
  Grant the source user readAnyDatabase and clusterMonitor:
  https://clickhouse.com/docs/integrations/clickpipes/mongodb/source/generic"
    )]
    MongoDB(MongoDbCreateArgs),

    /// Create a ClickPipe from BigQuery
    #[command(
        name = "bigquery",
        after_help = "\
CONTEXT FOR AGENTS:
  Provision the GCS staging bucket before creating a BigQuery pipe.
  The selected identity needs BigQuery table and export-job access plus read/write
  access to staging objects:
  https://clickhouse.com/blog/bigquery-clickpipe-private-preview"
    )]
    BigQuery(BigQueryCreateArgs),

    /// Create a ClickPipe from Google Cloud Pub/Sub
    #[command(
        name = "pubsub",
        after_help = "\
CONTEXT FOR AGENTS:
  Pub/Sub ClickPipes are in limited preview: ask ClickHouse support to enable the
  feature for the organization before creating a pipe."
    )]
    PubSub(PubSubCreateArgs),
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

/// Sample validation shared by every `clickpipe create` source.
#[derive(Args, Debug, Default)]
pub struct ClickPipeCreateValidationArgs {
    /// Validate source data samples, not only connectivity
    #[arg(long, value_name = "true|false")]
    pub validate_samples: Option<bool>,
}

/// Request controls shared by streaming and object-storage creates.
#[derive(Args, Debug, Default)]
pub struct ClickPipeCreateRequestArgs {
    /// Create stopped; start ingestion later with `clickpipe start`
    #[arg(long)]
    pub start_paused: bool,

    #[command(flatten)]
    pub validation: ClickPipeCreateValidationArgs,

    /// Initial number of replicas (1-40)
    ///
    /// Requires --cpu-millicores and --memory-gb.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=40))]
    pub replicas: Option<u32>,

    /// Initial CPU millicores per replica (125-2000)
    ///
    /// Requires --replicas and --memory-gb.
    #[arg(long, value_parser = clap::value_parser!(u32).range(125..=2000))]
    pub cpu_millicores: Option<u32>,

    /// Initial memory GB per replica (0.5-8)
    ///
    /// Requires --replicas and --cpu-millicores.
    #[arg(long, value_parser = parse_streaming_memory_gb)]
    pub memory_gb: Option<f64>,

    /// Field mapping JSON with sourceField and destinationField (repeatable)
    ///
    /// Give one mapping per destination column.
    #[arg(long = "field-mapping", value_name = "JSON")]
    pub field_mappings: Vec<String>,

    #[command(flatten)]
    pub(super) settings: ClickPipeSettingsValues,
}

/// Destination flags shared by the streaming and object-storage creates
/// (Kafka, Kinesis, object storage, Pub/Sub), flattened so the four
/// subcommands cannot drift apart (#1243 was one copy marking `--column`
/// differently).
#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("destination_columns")
        .args(["columns", "columns_from"])
        .required(true)
        .multiple(true)
))]
pub struct StreamingDestinationArgs {
    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination column as name:type (repeatable)
    ///
    /// Give --column, --columns-from, or both.
    #[arg(
        long = "column",
        value_name = "NAME:TYPE",
        value_parser = parse_destination_column
    )]
    pub columns: Vec<String>,

    /// `schema-discover --json` output to use as columns, or - for stdin
    ///
    /// Imports every field except metadata disabled by default; a name
    /// also given with --column is an error.
    #[arg(long, value_name = "PATH|-")]
    pub columns_from: Option<String>,

    #[command(flatten)]
    pub destination_table: StreamingDestinationTableArgs,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,
}

/// Destination-table controls shared by streaming ClickPipe creates.
#[derive(Args, Debug)]
pub struct StreamingDestinationTableArgs {
    /// Whether ClickPipes manages the destination table
    #[arg(
        long,
        default_value_t = true,
        value_name = "true|false",
        action = clap::ArgAction::Set
    )]
    pub managed_table: bool,

    /// Complete destination table definition JSON path, or - for stdin
    #[arg(long, value_name = "PATH|-")]
    pub table_definition_file: Option<String>,
}

impl Default for StreamingDestinationTableArgs {
    fn default() -> Self {
        Self {
            managed_table: true,
            table_definition_file: None,
        }
    }
}

/// Destination database shared by database-source ClickPipe creates.
#[derive(Args, Debug)]
pub struct DatabaseDestinationArgs {
    /// Destination ClickHouse database
    #[arg(long, default_value = "default", value_name = "DATABASE")]
    pub destination_database: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::cloud::clickpipes::parse_enum;
    use crate::cloud::clickpipes::sources::bigquery::parse_supported_bigquery_auth;
    use crate::cloud::clickpipes::sources::kafka::{
        parse_kafka_authentication, parse_supported_kafka_auth,
    };
    use crate::cloud::clickpipes::sources::kinesis::{
        KINESIS_AUTHS, KINESIS_FORMATS, KINESIS_ITERATOR_TYPES,
    };
    use crate::cloud::clickpipes::sources::object_storage::{
        OBJECT_STORAGE_COMPRESSIONS, OBJECT_STORAGE_FORMATS, OBJECT_STORAGE_TYPES,
        parse_supported_object_storage_auth,
    };
    use crate::cloud::clickpipes::sources::postgres::POSTGRES_TYPES;
    use crate::cloud::clickpipes::sources::pubsub::{
        PUBSUB_FORMATS, PUBSUB_SEEK_TYPES, parse_supported_pubsub_auth,
    };
    use crate::cloud::clickpipes::sources::{DB_AUTHS, REPLICATION_MODES};
    use crate::cloud::clickpipes::test_support::{
        assert_bigquery_value, assert_kafka_value, assert_kinesis_value, assert_mongodb_value,
        assert_mysql_value, assert_names, assert_object_storage_value, assert_postgres_value,
        assert_pubsub_value, assert_rejected, assert_write, clickpipe_parse_error,
        clickpipe_validation_message, kafka_create_cli_args, kafka_create_cli_args_without_auth,
        kafka_discover_cli_args, kafka_source_with_plain_auth, mysql_create_cli_args,
        object_storage_create_cli_args, parse_clickpipe, parse_object_storage_discovery,
        parse_pubsub_discovery, postgres_cli_args, pubsub_source_flags,
    };
    use clickhouse_cloud_api::models::{
        ClickPipeBigQueryPipeSettingsReplicationmode, ClickPipeKafkaOffsetStrategy,
        ClickPipeMongoDBPipeSettingsReplicationmode, ClickPipeMutateMongoDBSourceReadpreference,
        ClickPipeMutateMySQLSourceType, ClickPipeMySQLPipeSettingsReplicationmechanism,
        ClickPipeMySQLPipeSettingsReplicationmode, ClickPipePostKafkaSourceAuthentication,
        ClickPipePostKafkaSourceFormat, ClickPipePostKafkaSourceType,
    };

    #[test]
    fn every_create_source_exposes_cross_source_request_flags() {
        use clap::CommandFactory;

        let mut command = Cli::command();
        let create = command
            .find_subcommand_mut("cloud")
            .and_then(|cloud| cloud.find_subcommand_mut("clickpipe"))
            .and_then(|clickpipe| clickpipe.find_subcommand_mut("create"))
            .expect("clickpipe create command");
        for source in [
            "object-storage",
            "kafka",
            "kinesis",
            "postgres",
            "mysql",
            "mongodb",
            "bigquery",
            "pubsub",
        ] {
            let command = create.find_subcommand(source).expect("source subcommand");
            assert!(
                command
                    .get_arguments()
                    .any(|argument| argument.get_id() == "validate_samples"),
                "clickpipe create {source} is missing sample validation"
            );
        }
        for source in ["object-storage", "kafka", "kinesis", "pubsub"] {
            let command = create.find_subcommand(source).expect("source subcommand");
            for id in [
                "replicas",
                "cpu_millicores",
                "memory_gb",
                "field_mappings",
                "streaming_max_insert_wait_ms",
                "kafka_read_committed",
            ] {
                assert!(
                    command
                        .get_arguments()
                        .any(|argument| argument.get_id() == id),
                    "clickpipe create {source} is missing argument id {id}"
                );
            }
        }
        for source in ["postgres", "mysql", "mongodb", "bigquery"] {
            let command = create.find_subcommand(source).expect("source subcommand");
            for id in ["replicas", "cpu_millicores", "memory_gb", "field_mappings"] {
                assert!(
                    command
                        .get_arguments()
                        .all(|argument| argument.get_id() != id),
                    "clickpipe create {source} unexpectedly exposes {id}"
                );
            }
        }
    }

    #[test]
    fn parses_cross_source_create_request_values() {
        let mut args = kafka_create_cli_args();
        args.extend([
            "--validate-samples",
            "false",
            "--replicas",
            "2",
            "--cpu-millicores",
            "500",
            "--memory-gb",
            "2",
            "--field-mapping",
            r#"{"sourceField":"source","destinationField":"destination"}"#,
            "--clickhouse-max-threads",
            "0",
            "--kafka-read-committed",
            "false",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(args),
        } = parse_clickpipe(&args)
        else {
            panic!("expected kafka create");
        };

        assert_eq!(args.request.validation.validate_samples, Some(false));
        assert_eq!(args.request.replicas, Some(2));
        assert_eq!(args.request.cpu_millicores, Some(500));
        assert_eq!(args.request.memory_gb, Some(2.0));
        assert_eq!(args.request.field_mappings.len(), 1);
        assert_eq!(args.request.settings.clickhouse_max_threads, Some(0));
        assert_eq!(args.request.settings.kafka_read_committed, Some(false));
    }

    #[test]
    fn parses_schema_discovery_commands_and_flags() {
        let ClickPipeCommands::SchemaDiscover {
            command: ClickPipeSchemaDiscoverCommands::Kafka(args),
        } = parse_clickpipe(&[
            "schema-discover",
            "kafka",
            "--org-id",
            "org-kafka",
            "svc-kafka",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--iam-role",
            "arn:role",
        ])
        else {
            panic!("expected kafka schema discovery");
        };
        assert_eq!(args.service_id, "svc-kafka");
        let args = args.source;
        assert_eq!(args.brokers, "broker:9092");
        assert_eq!(args.kafka_type, "kafka");
        assert_eq!(args.offset, "from_beginning");

        // A credential flag alone satisfies discovery authentication; --auth is
        // then inferred by the builder.
        assert_eq!(args.auth, None);
        assert_eq!(args.iam_role.as_deref(), Some("arn:role"));

        let ClickPipeCommands::SchemaDiscover {
            command: ClickPipeSchemaDiscoverCommands::Kinesis(args),
        } = parse_clickpipe(&[
            "schema-discover",
            "kinesis",
            "--org-id",
            "org-kinesis",
            "svc-kinesis",
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
        assert_eq!(args.service_id, "svc-kinesis");
        let args = args.source;
        assert_eq!(args.stream_name, "stream-1");
        assert_eq!(args.auth, None);
        assert_eq!(args.iterator_type, "TRIM_HORIZON");
    }

    fn schema_discovery_sources() -> [(&'static str, &'static [&'static str]); 4] {
        [
            (
                "kafka",
                &[
                    "--brokers",
                    "broker:9092",
                    "--topics",
                    "topic",
                    "--format",
                    "JSONEachRow",
                    "--iam-role",
                    "arn:role",
                ],
            ),
            (
                "kinesis",
                &[
                    "--stream-name",
                    "stream-1",
                    "--region",
                    "us-east-1",
                    "--format",
                    "JSONEachRow",
                ],
            ),
            (
                "object-storage",
                &[
                    "--source-url",
                    "https://bucket.example/data/*.json",
                    "--format",
                    "JSONEachRow",
                ],
            ),
            (
                "pubsub",
                &[
                    "--topic",
                    "events",
                    "--project-id",
                    "my-gcp-project",
                    "--format",
                    "JSONEachRow",
                    "--seek-type",
                    "earliest",
                    "--service-account-file",
                    "./sa-key.json",
                ],
            ),
        ]
    }

    #[test]
    fn every_schema_discovery_source_accepts_flags_around_service_id() {
        for (source, source_args) in schema_discovery_sources() {
            for service_position in (0..=source_args.len()).step_by(2) {
                for org_position in [
                    "before-source",
                    "before-service",
                    "after-service",
                    "after-options",
                ] {
                    let mut args = vec!["schema-discover"];
                    if org_position == "before-source" {
                        args.extend(["--org-id", "org-1"]);
                    }
                    args.push(source);
                    args.extend_from_slice(&source_args[..service_position]);
                    if org_position == "before-service" {
                        args.extend(["--org-id", "org-1"]);
                    }
                    args.push("svc-1");
                    if org_position == "after-service" {
                        args.extend(["--org-id", "org-1"]);
                    }
                    args.extend_from_slice(&source_args[service_position..]);
                    if org_position == "after-options" {
                        args.extend(["--org-id", "org-1"]);
                    }

                    let ClickPipeCommands::SchemaDiscover { command } = parse_clickpipe(&args)
                    else {
                        panic!("expected {source} schema discovery");
                    };
                    let (parsed_source, service_id) = match command {
                        ClickPipeSchemaDiscoverCommands::Kafka(args) => ("kafka", args.service_id),
                        ClickPipeSchemaDiscoverCommands::Kinesis(args) => {
                            ("kinesis", args.service_id)
                        }
                        ClickPipeSchemaDiscoverCommands::ObjectStorage(args) => {
                            ("object-storage", args.service_id)
                        }
                        ClickPipeSchemaDiscoverCommands::PubSub(args) => {
                            ("pubsub", args.service_id)
                        }
                    };
                    assert_eq!(parsed_source, source, "{args:?}");
                    assert_eq!(service_id, "svc-1", "{args:?}");
                }
            }
        }
    }

    #[test]
    fn every_schema_discovery_source_rejects_service_before_source() {
        for (source, source_args) in schema_discovery_sources() {
            let mut args = vec!["schema-discover", "svc-1", source];
            args.extend_from_slice(source_args);
            let error = clickpipe_parse_error(&args);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::InvalidSubcommand,
                "{args:?}: {error}"
            );
            assert_eq!(error.exit_code(), 2, "{args:?}: {error}");

            let mut missing_id = vec!["schema-discover", source];
            missing_id.extend_from_slice(source_args);
            let error = clickpipe_parse_error(&missing_id);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{missing_id:?}: {error}"
            );
            assert_eq!(error.exit_code(), 2, "{missing_id:?}: {error}");
        }
    }

    #[test]
    fn every_schema_discovery_source_exposes_org_id() {
        use clap::CommandFactory;

        let mut command = Cli::command();
        command.build();
        let schema_discover = command
            .find_subcommand("cloud")
            .and_then(|cloud| cloud.find_subcommand("clickpipe"))
            .and_then(|clickpipe| clickpipe.find_subcommand("schema-discover"))
            .expect("clickpipe schema-discover command");
        assert_eq!(schema_discover.get_positionals().count(), 0);
        for source in ["kafka", "kinesis", "object-storage", "pubsub"] {
            let command = schema_discover
                .find_subcommand(source)
                .expect("schema-discover source subcommand");
            assert!(
                command
                    .get_arguments()
                    .any(|argument| argument.get_id() == "org_id" && argument.is_global_set()),
                "clickpipe schema-discover {source} is missing global --org-id"
            );
            let positionals: Vec<_> = command.get_positionals().collect();
            assert_eq!(positionals.len(), 1, "{source}");
            assert_eq!(positionals[0].get_id(), "service_id", "{source}");
            assert!(positionals[0].is_required_set(), "{source}");
        }
    }

    /// `main` formats the `clickpipe create` usage error against the source
    /// subcommand named by `clickpipe_create_validation_error`, so every name
    /// that hook can return has to resolve in the real clap command tree.
    #[test]
    fn every_clickpipe_create_validation_source_resolves_to_a_subcommand() {
        use clap::CommandFactory;

        // One failing invocation per source whose flag relationships are
        // validated after parsing. The match in
        // `clickpipe_create_validation_error` is exhaustive, so a new source
        // that gains checks has to be added here too.
        let mut postgres = postgres_cli_args(Some("public.events:events"));
        postgres.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
        let sources: Vec<&'static str> = [postgres, mysql_create_cli_args()]
            .iter()
            .map(|args| {
                parse_clickpipe(args)
                    .clickpipe_create_validation_error()
                    .unwrap_or_else(|| panic!("expected a validation error for: {args:?}"))
                    .0
            })
            .collect();
        assert_eq!(sources, ["postgres", "mysql"]);

        let mut command = Cli::command();
        let create = command
            .find_subcommand_mut("cloud")
            .and_then(|cloud| cloud.find_subcommand_mut("clickpipe"))
            .and_then(|clickpipe| clickpipe.find_subcommand_mut("create"))
            .expect("clickpipe create command must exist");
        for source in sources {
            assert!(
                create.find_subcommand(source).is_some(),
                "`clickpipe create {source}` must exist for the usage error to name it"
            );
        }
    }

    #[test]
    fn non_postgres_database_sources_require_one_mapping_form() {
        let cases = [
            vec![
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
            ],
            vec![
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
            ],
            vec![
                "create",
                "bigquery",
                "svc-1",
                "--name",
                "pipe-1",
                "--service-account-file",
                "/tmp/account.json",
                "--staging-path",
                "gs://bucket/staging",
            ],
        ];

        for args in cases {
            let error = clickpipe_parse_error(&args);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
            let message = error.to_string();
            assert!(message.contains("--table-mapping"), "{message}");
            assert!(message.contains("--table-mapping-json"), "{message}");
        }
    }

    /// Every value a CLI-owned list accepts must deserialize to a known
    /// variant of the library enum the builder converts it to; a value the
    /// library would read as `Unknown` means the two have drifted. Each value
    /// is then parsed on its flag, which pins the list each flag is wired to.
    #[test]
    fn accepted_possible_values_are_library_variants() {
        use clickhouse_cloud_api::models::{
            ClickPipeMutateMySQLSourceAuthentication, ClickPipeMutatePostgresSourceAuthentication,
            ClickPipeMutatePostgresSourceType, ClickPipePostKinesisSourceAuthentication,
            ClickPipePostKinesisSourceFormat, ClickPipePostKinesisSourceIteratortype,
            ClickPipePostObjectStorageSourceCompression, ClickPipePostObjectStorageSourceFormat,
            ClickPipePostObjectStorageSourceType, ClickPipePostPubSubSourceFormat,
            ClickPipePostPubSubSourceSeektype, ClickPipePostgresPipeSettingsReplicationmode,
        };
        macro_rules! assert_known {
            ($values:expr, $enum:ident) => {
                assert!(!$values.is_empty(), stringify!($enum));
                for &value in $values {
                    let parsed: $enum = parse_enum(value).unwrap();
                    assert!(
                        !matches!(parsed, $enum::Unknown(_)),
                        "{value} is not a known {}",
                        stringify!($enum)
                    );
                }
            };
        }
        assert_known!(
            OBJECT_STORAGE_FORMATS,
            ClickPipePostObjectStorageSourceFormat
        );
        assert_known!(
            OBJECT_STORAGE_COMPRESSIONS,
            ClickPipePostObjectStorageSourceCompression
        );
        assert_known!(OBJECT_STORAGE_TYPES, ClickPipePostObjectStorageSourceType);
        assert_known!(KINESIS_FORMATS, ClickPipePostKinesisSourceFormat);
        assert_known!(KINESIS_AUTHS, ClickPipePostKinesisSourceAuthentication);
        assert_known!(
            KINESIS_ITERATOR_TYPES,
            ClickPipePostKinesisSourceIteratortype
        );
        assert_known!(POSTGRES_TYPES, ClickPipeMutatePostgresSourceType);
        assert_known!(DB_AUTHS, ClickPipeMutatePostgresSourceAuthentication);
        assert_known!(DB_AUTHS, ClickPipeMutateMySQLSourceAuthentication);
        assert_known!(
            REPLICATION_MODES,
            ClickPipePostgresPipeSettingsReplicationmode
        );
        assert_known!(PUBSUB_FORMATS, ClickPipePostPubSubSourceFormat);
        assert_known!(PUBSUB_SEEK_TYPES, ClickPipePostPubSubSourceSeektype);

        for &value in OBJECT_STORAGE_FORMATS {
            assert_object_storage_value("--format", value);
        }
        for &value in OBJECT_STORAGE_COMPRESSIONS {
            assert_object_storage_value("--compression", value);
        }
        for &value in OBJECT_STORAGE_TYPES {
            assert_object_storage_value("--storage-type", value);
        }
        for &value in ClickPipePostKafkaSourceFormat::VALUES {
            assert_kafka_value("--format", value);
        }
        for &value in ClickPipePostKafkaSourceType::VALUES {
            assert_kafka_value("--kafka-type", value);
        }
        for &value in ClickPipePostKafkaSourceAuthentication::VALUES {
            if parse_kafka_authentication(value).is_ok() {
                assert_kafka_value("--auth", value);
            } else {
                let mut args = kafka_create_cli_args_without_auth();
                args.extend(["--auth", value]);
                assert_rejected(&args);
            }
        }
        for &value in ClickPipeKafkaOffsetStrategy::VALUES {
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
        }
        for &value in REPLICATION_MODES {
            assert_postgres_value("--replication-mode", value);
        }
        for &value in ClickPipeBigQueryPipeSettingsReplicationmode::VALUES {
            assert_bigquery_value("--replication-mode", value);
        }
        for &value in DB_AUTHS {
            assert_mysql_value("--auth", value);
        }
        for &value in ClickPipeMySQLPipeSettingsReplicationmode::VALUES {
            assert_mysql_value("--replication-mode", value);
        }
        for &value in ClickPipeMongoDBPipeSettingsReplicationmode::VALUES {
            assert_mongodb_value("--replication-mode", value);
        }
        for &value in ClickPipeMutateMySQLSourceType::VALUES {
            assert_mysql_value("--mysql-type", value);
        }
        for &value in ClickPipeMySQLPipeSettingsReplicationmechanism::VALUES {
            assert_mysql_value("--replication-mechanism", value);
        }
        for &value in ClickPipeMutateMongoDBSourceReadpreference::VALUES {
            assert_mongodb_value("--read-preference", value);
        }
        for &value in PUBSUB_FORMATS {
            assert_pubsub_value("--format", value);
        }
        for value in ["SERVICE_ACCOUNT", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"] {
            assert_pubsub_value("--auth", value);
        }
        for &value in PUBSUB_SEEK_TYPES {
            // `timestamp` needs its companion flag, which is covered by
            // `pubsub_seek_timestamp_pairs_with_the_timestamp_seek_type`.
            if value == "timestamp" {
                continue;
            }
            assert_pubsub_value("--seek-type", value);
        }
    }

    #[test]
    fn every_clickpipe_auth_flag_exposes_valid_canonical_choices() {
        use clap::CommandFactory;
        fn visit(command: &clap::Command, count: &mut usize) {
            for argument in command
                .get_arguments()
                .filter(|arg| arg.get_long() == Some("auth"))
            {
                let parser = argument.get_value_parser();
                let choices: Vec<_> = parser.possible_values().expect("auth choices").collect();
                assert!(!choices.is_empty(), "{}", command.get_name());
                for choice in choices {
                    let value = choice.get_name();
                    // Validate every advertised choice through the existing domain parser.
                    match command.get_name() {
                        "object-storage" => {
                            parse_supported_object_storage_auth(value).unwrap();
                        }
                        "kafka" => {
                            parse_supported_kafka_auth(value).unwrap();
                        }
                        "pubsub" => {
                            parse_supported_pubsub_auth(value).unwrap();
                        }
                        "bigquery" => {
                            parse_supported_bigquery_auth(value).unwrap();
                        }
                        _ => {}
                    }
                }
                *count += 1;
            }
            for child in command.get_subcommands() {
                visit(child, count);
            }
        }
        // The domain parsers behind the custom auth value parsers reject what
        // they do not advertise.
        for parse in [
            parse_supported_object_storage_auth,
            parse_supported_kafka_auth,
            parse_supported_pubsub_auth,
            parse_supported_bigquery_auth,
        ] {
            assert!(parse("not-a-valid-value").is_err());
        }
        let tree = crate::cli::Cli::command();
        let clickpipe = tree
            .find_subcommand("cloud")
            .unwrap()
            .find_subcommand("clickpipe")
            .unwrap();
        let mut count = 0;
        visit(clickpipe, &mut count);
        assert_eq!(count, 11);
    }

    #[test]
    fn credential_flags_keep_pair_requirements() {
        let object_base = [
            "create",
            "object-storage",
            "--column",
            "id:Int64",
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
            "--column",
            "id:UInt64",
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
            "--column",
            "id:UInt64",
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
        assert_write(&["update", "svc-1", "pipe-1", "--file", "patch.json"], true);
        assert_write(&["delete", "svc-1", "pipe-1"], true);
        assert_write(&["start", "svc-1", "pipe-1"], true);
        assert_write(&["stop", "svc-1", "pipe-1"], true);
        assert_write(&["resync", "svc-1", "pipe-1"], true);
        assert_write(&["scale", "svc-1", "pipe-1", "--replicas", "4"], true);
        assert_write(&["cdc-scaling", "get", "svc-1"], false);
        assert_write(
            &[
                "cdc-scaling",
                "update",
                "svc-1",
                "--cpu-millicores",
                "1000",
                "--memory-gb",
                "4",
            ],
            true,
        );
        assert_write(&["settings", "get", "svc-1", "pipe-1"], false);
        assert_write(
            &[
                "settings",
                "update",
                "svc-1",
                "pipe-1",
                "--kafka-read-committed",
                "false",
            ],
            true,
        );
        assert_write(&kafka_discover_cli_args(), true);
        assert_write(
            &[
                "schema-discover",
                "kinesis",
                "svc-1",
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
                "object-storage",
                "svc-1",
                "--source-url",
                "https://bucket.example/data/*.json",
                "--format",
                "JSONEachRow",
            ],
            true,
        );
        let mut pubsub_discover = vec!["schema-discover", "pubsub", "svc-1"];
        pubsub_discover.extend(pubsub_source_flags("./sa-key.json"));
        assert_write(&pubsub_discover, true);
        let mut pubsub_create = vec![
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
        pubsub_create.extend(pubsub_source_flags("./sa-key.json"));
        assert_write(&pubsub_create, true);
        assert_write(
            &[
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
            ],
            true,
        );
    }

    #[test]
    fn kafka_glue_schema_registry_runtime_rules_run_before_execution() {
        let mut create = kafka_create_cli_args();
        create.extend([
            "--schema-registry-type",
            "glue",
            "--glue-region",
            "us-east-1",
            "--glue-registry-name",
            "my-registry",
        ]);
        let message = clickpipe_validation_message(&parse_clickpipe(&create))
            .expect("Glue without a role is rejected before execution");
        assert!(message.contains("--glue-role-arn"), "{message}");

        let mut discover = kafka_discover_cli_args();
        discover.extend(&create[create.len() - 6..]);
        let message = parse_clickpipe(&discover)
            .clickpipe_schema_discover_validation_error()
            .map(|(source, message)| {
                assert_eq!(source, "kafka");
                message
            })
            .expect("Glue without a role is rejected before schema discovery");
        assert!(message.contains("--glue-role-arn"), "{message}");

        // A source authenticating with --iam-role supplies the Glue role.
        let mut iam_role = kafka_create_cli_args_without_auth();
        iam_role.extend(&create[create.len() - 6..]);
        iam_role.extend(["--iam-role", "arn:role"]);
        assert_eq!(
            clickpipe_validation_message(&parse_clickpipe(&iam_role)),
            None
        );
    }

    /// The source rules each operation checks after parsing, as
    /// `(operation, source, error)` for the parsed invocation.
    fn source_validation_error(args: &[&str]) -> Option<(&'static str, &'static str, String)> {
        let command = parse_clickpipe(args);
        command
            .clickpipe_create_validation_error()
            .map(|(source, message)| ("create", source, message))
            .or_else(|| {
                command
                    .clickpipe_schema_discover_validation_error()
                    .map(|(source, message)| ("schema-discover", source, message))
            })
    }

    #[test]
    fn kafka_broker_auth_rules_run_before_execution() {
        let mut create = kafka_create_cli_args();
        create.extend(["--iam-role", "arn:role"]);
        let mut discover = kafka_discover_cli_args();
        discover.extend(["--iam-role", "arn:role"]);
        for (args, operation) in [(create, "create"), (discover, "schema-discover")] {
            let (actual, source, message) =
                source_validation_error(&args).expect("PLAIN with an IAM role is rejected");
            assert_eq!((actual, source), (operation, "kafka"));
            assert_names(&message, &["--auth PLAIN"]);
        }
    }

    #[test]
    fn object_storage_auth_rules_run_before_execution() {
        let flags = ["--storage-type", "gcs", "--iam-role", "arn:role"];
        let mut create = object_storage_create_cli_args();
        create.extend(["--column", "id:UInt64"]);
        create.extend(flags);
        let mut discover = vec![
            "schema-discover",
            "object-storage",
            "svc-1",
            "--source-url",
            "gs://bucket/events/*.json",
            "--format",
            "JSONEachRow",
        ];
        discover.extend(flags);
        for (args, operation) in [(create, "create"), (discover, "schema-discover")] {
            let (actual, source, message) =
                source_validation_error(&args).expect("IAM_ROLE on GCS is rejected");
            assert_eq!((actual, source), (operation, "object-storage"));
            assert_names(&message, &["--storage-type s3"]);
        }
    }

    #[test]
    fn pubsub_seek_and_auth_rules_run_before_execution() {
        // The key file does not exist: the rules reject the invocation before
        // anything reads it.
        let mut flags = pubsub_source_flags("/missing/sa-key.json");
        flags.extend(["--seek-timestamp", "2026-04-10T12:00:00Z"]);
        let mut create = vec![
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
        create.extend(flags.iter().copied());
        let mut discover = vec!["schema-discover", "pubsub", "svc-1"];
        discover.extend(flags.iter().copied());
        for (args, operation) in [(create, "create"), (discover, "schema-discover")] {
            let (actual, source, message) =
                source_validation_error(&args).expect("a timestamp without timestamp seek");
            assert_eq!((actual, source), (operation, "pubsub"));
            assert_names(&message, &["--seek-timestamp", "--seek-type earliest"]);
        }
    }

    #[test]
    fn source_rules_found_without_io_are_usage_errors() {
        let mut kafka = kafka_source_with_plain_auth();
        kafka.auth = None;
        kafka.iam_role = Some("arn:role".into());
        assert_eq!(
            validate_kafka_source_args(&kafka).unwrap_err().kind,
            crate::cloud::client::CloudErrorKind::Usage
        );

        let mut object_storage = parse_object_storage_discovery(&[
            "--source-url",
            "https://bucket.s3.amazonaws.com/events.json",
            "--format",
            "JSONEachRow",
            "--iam-role",
            "arn:role",
        ]);
        object_storage.connection_string = Some("DefaultEndpointsProtocol=https".into());
        assert_eq!(
            validate_object_storage_source_args(&object_storage)
                .unwrap_err()
                .kind,
            crate::cloud::client::CloudErrorKind::Usage
        );

        let mut flags = pubsub_source_flags("/missing/sa-key.json");
        flags.extend(["--auth", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"]);
        assert_eq!(
            validate_pubsub_source_args(&parse_pubsub_discovery(&flags))
                .unwrap_err()
                .kind,
            crate::cloud::client::CloudErrorKind::Usage
        );
    }
}
