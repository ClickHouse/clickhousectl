use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipe.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeState {
    #[default]
    Unknown,
    Provisioning,
    Running,
    Stopping,
    Stopped,
    Failed,
    Completed,
    InternalError,
    Setup,
    Snapshot,
    Paused,
    Pausing,
    Modifying,
    Resync,
    Degraded,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ClickPipeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Running => write!(f, "Running"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Failed => write!(f, "Failed"),
            Self::Completed => write!(f, "Completed"),
            Self::InternalError => write!(f, "InternalError"),
            Self::Setup => write!(f, "Setup"),
            Self::Snapshot => write!(f, "Snapshot"),
            Self::Paused => write!(f, "Paused"),
            Self::Pausing => write!(f, "Pausing"),
            Self::Modifying => write!(f, "Modifying"),
            Self::Resync => write!(f, "Resync"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeBigQueryPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeSettingsReplicationmode {
    #[serde(rename = "snapshot")]
    #[default]
    Snapshot,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot => write!(f, "snapshot"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeBigQueryPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeDestinationTableEngine.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeDestinationTableEngineType {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    SummingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeDestinationTableEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::SummingMergeTree => write!(f, "SummingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaOffset.strategy`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaOffsetStrategy {
    #[serde(rename = "from_beginning")]
    #[default]
    From_beginning,
    #[serde(rename = "from_latest")]
    From_latest,
    #[serde(rename = "from_timestamp")]
    From_timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaOffsetStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::From_beginning => write!(f, "from_beginning"),
            Self::From_latest => write!(f, "from_latest"),
            Self::From_timestamp => write!(f, "from_timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kafka => write!(f, "kafka"),
            Self::Redpanda => write!(f, "redpanda"),
            Self::Msk => write!(f, "msk"),
            Self::Gcmk => write!(f, "gcmk"),
            Self::Confluent => write!(f, "confluent"),
            Self::Warpstream => write!(f, "warpstream"),
            Self::Azureeventhub => write!(f, "azureeventhub"),
            Self::Dokafka => write!(f, "dokafka"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceIteratortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TRIM_HORIZON => write!(f, "TRIM_HORIZON"),
            Self::LATEST => write!(f, "LATEST"),
            Self::AT_TIMESTAMP => write!(f, "AT_TIMESTAMP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateKafkaSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutatePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutatePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutatePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutatePostgresSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
            Self::Neon => write!(f, "neon"),
            Self::Alloydb => write!(f, "alloydb"),
            Self::Planetscale => write!(f, "planetscale"),
            Self::Rdspostgres => write!(f, "rdspostgres"),
            Self::Aurorapostgres => write!(f, "aurorapostgres"),
            Self::Cloudsqlpostgres => write!(f, "cloudsqlpostgres"),
            Self::Azurepostgres => write!(f, "azurepostgres"),
            Self::Crunchybridge => write!(f, "crunchybridge"),
            Self::Tigerdata => write!(f, "tigerdata"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMechanism`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmechanism {
    #[default]
    GTID,
    FILE_POS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeSettingsReplicationmechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GTID => write!(f, "GTID"),
            Self::FILE_POS => write!(f, "FILE_POS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Gz => write!(f, "gz"),
            Self::Brotli => write!(f, "brotli"),
            Self::Br => write!(f, "br"),
            Self::Xz => write!(f, "xz"),
            Self::LZMA => write!(f, "LZMA"),
            Self::Zstd => write!(f, "zstd"),
            Self::Auto => write!(f, "auto"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::JSONAsObject => write!(f, "JSONAsObject"),
            Self::CSV => write!(f, "CSV"),
            Self::CSVWithNames => write!(f, "CSVWithNames"),
            Self::TabSeparated => write!(f, "TabSeparated"),
            Self::TabSeparatedWithNames => write!(f, "TabSeparatedWithNames"),
            Self::Parquet => write!(f, "Parquet"),
            Self::Avro => write!(f, "Avro"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Dospaces => write!(f, "dospaces"),
            Self::Azureblobstorage => write!(f, "azureblobstorage"),
            Self::Cloudflarer2 => write!(f, "cloudflarer2"),
            Self::Ovhobjectstorage => write!(f, "ovhobjectstorage"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMongoDBPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMySQLPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchPostgresPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchPubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchPubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kafka => write!(f, "kafka"),
            Self::Redpanda => write!(f, "redpanda"),
            Self::Msk => write!(f, "msk"),
            Self::Gcmk => write!(f, "gcmk"),
            Self::Confluent => write!(f, "confluent"),
            Self::Warpstream => write!(f, "warpstream"),
            Self::Azureeventhub => write!(f, "azureeventhub"),
            Self::Dokafka => write!(f, "dokafka"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceIteratortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TRIM_HORIZON => write!(f, "TRIM_HORIZON"),
            Self::LATEST => write!(f, "LATEST"),
            Self::AT_TIMESTAMP => write!(f, "AT_TIMESTAMP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Gz => write!(f, "gz"),
            Self::Brotli => write!(f, "brotli"),
            Self::Br => write!(f, "br"),
            Self::Xz => write!(f, "xz"),
            Self::LZMA => write!(f, "LZMA"),
            Self::Zstd => write!(f, "zstd"),
            Self::Auto => write!(f, "auto"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::JSONAsObject => write!(f, "JSONAsObject"),
            Self::CSV => write!(f, "CSV"),
            Self::CSVWithNames => write!(f, "CSVWithNames"),
            Self::TabSeparated => write!(f, "TabSeparated"),
            Self::TabSeparatedWithNames => write!(f, "TabSeparatedWithNames"),
            Self::Parquet => write!(f, "Parquet"),
            Self::Avro => write!(f, "Avro"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Dospaces => write!(f, "dospaces"),
            Self::Azureblobstorage => write!(f, "azureblobstorage"),
            Self::Cloudflarer2 => write!(f, "cloudflarer2"),
            Self::Ovhobjectstorage => write!(f, "ovhobjectstorage"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostPubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostPubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostPubSubSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostPubSubSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostPubSubSource.seekType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubSourceSeektype {
    #[serde(rename = "latest")]
    #[default]
    Latest,
    #[serde(rename = "earliest")]
    Earliest,
    #[serde(rename = "timestamp")]
    Timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostPubSubSourceSeektype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Earliest => write!(f, "earliest"),
            Self::Timestamp => write!(f, "timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ClickPipePostgresPipeTableMappingTableengine {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["MergeTree", "ReplacingMergeTree", "Null"];
}

/// Inline enum for `ClickPipePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
            Self::Neon => write!(f, "neon"),
            Self::Alloydb => write!(f, "alloydb"),
            Self::Planetscale => write!(f, "planetscale"),
            Self::Rdspostgres => write!(f, "rdspostgres"),
            Self::Aurorapostgres => write!(f, "aurorapostgres"),
            Self::Cloudsqlpostgres => write!(f, "cloudsqlpostgres"),
            Self::Azurepostgres => write!(f, "azurepostgres"),
            Self::Crunchybridge => write!(f, "crunchybridge"),
            Self::Tigerdata => write!(f, "tigerdata"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePubSubSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePubSubSource.seekType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceSeektype {
    #[serde(rename = "latest")]
    #[default]
    Latest,
    #[serde(rename = "earliest")]
    Earliest,
    #[serde(rename = "timestamp")]
    Timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceSeektype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Earliest => write!(f, "earliest"),
            Self::Timestamp => write!(f, "timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "resync")]
    Resync,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeStatePatchRequestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Resync => write!(f, "resync"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `CreateReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `CreateReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    GCP_PSC_SERVICE_ATTACHMENT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::GCP_PSC_SERVICE_ATTACHMENT => write!(f, "GCP_PSC_SERVICE_ATTACHMENT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointStatus {
    #[default]
    Unknown,
    Provisioning,
    Deleting,
    Ready,
    Failed,
    PendingAcceptance,
    Rejected,
    Expired,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ReversePrivateEndpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Deleting => write!(f, "Deleting"),
            Self::Ready => write!(f, "Ready"),
            Self::Failed => write!(f, "Failed"),
            Self::PendingAcceptance => write!(f, "PendingAcceptance"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    GCP_PSC_SERVICE_ATTACHMENT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::GCP_PSC_SERVICE_ATTACHMENT => write!(f, "GCP_PSC_SERVICE_ATTACHMENT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `AzureEventHub` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureEventHub {
    #[serde(rename = "connectionString")]
    pub connection_string: String,
}

/// `ClickPipe` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipe {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<ClickPipeDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none")]
    pub field_mappings: Option<Vec<ClickPipeFieldMappingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling: Option<ClickPipeScalingResponse>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeSettingsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickPipeSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ClickPipeState>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettings {
    #[serde(rename = "allowNullableColumns")]
    pub allow_nullable_columns: bool,
    #[serde(rename = "initialLoadParallelism")]
    pub initial_load_parallelism: f64,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeBigQueryPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition")]
    pub snapshot_num_rows_per_partition: f64,
    #[serde(rename = "snapshotNumberOfParallelTables")]
    pub snapshot_number_of_parallel_tables: f64,
}

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeBigQueryPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettingsResponse {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<f64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipeBigQueryPipeSettingsReplicationmode>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<f64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<f64>,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMapping {
    #[serde(rename = "excludedColumns")]
    pub excluded_columns: Vec<String>,
    #[serde(rename = "sortingKeys")]
    pub sorting_keys: Vec<String>,
    #[serde(rename = "sourceDatasetName")]
    pub source_dataset_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine")]
    pub table_engine: ClickPipeBigQueryPipeTableMappingTableengine,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey")]
    pub use_custom_sorting_key: bool,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeBigQueryPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMappingResponse {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceDatasetName", skip_serializing_if = "Option::is_none")]
    pub source_dataset_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeBigQueryPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQuerySource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeBigQueryPipeSettingsResponse>,
    #[serde(
        rename = "snapshotStagingPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_staging_path: Option<String>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeBigQueryPipeTableMappingResponse>>,
}

/// `ClickPipeDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickPipeDestinationColumnResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "managedTable", skip_serializing_if = "Option::is_none")]
    pub managed_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "tableDefinition", skip_serializing_if = "Option::is_none")]
    pub table_definition: Option<ClickPipeDestinationTableDefinitionResponse>,
}

/// `ClickPipeDestinationColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationColumn {
    pub name: String,
    pub r#type: String,
}

/// `ClickPipeDestinationColumn` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeDestinationColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationColumnResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// `ClickPipeDestinationTableDefinition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableDefinition {
    pub engine: ClickPipeDestinationTableEngine,
    // API rejects empty strings / empty arrays for these keys. Spec has no
    // `required` array so the description-heuristic treats them as required;
    // skip at serialize time when unset instead of modeling as Option<T>.
    #[serde(rename = "partitionBy", skip_serializing_if = "String::is_empty")]
    pub partition_by: String,
    #[serde(rename = "primaryKey", skip_serializing_if = "String::is_empty")]
    pub primary_key: String,
    #[serde(rename = "sortingKey", skip_serializing_if = "Vec::is_empty")]
    pub sorting_key: Vec<String>,
}

/// `ClickPipeDestinationTableDefinition` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeDestinationTableDefinition`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableDefinitionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<ClickPipeDestinationTableEngineResponse>,
    #[serde(rename = "partitionBy", skip_serializing_if = "Option::is_none")]
    pub partition_by: Option<String>,
    #[serde(rename = "primaryKey", skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<String>,
    #[serde(rename = "sortingKey", skip_serializing_if = "Option::is_none")]
    pub sorting_key: Option<Vec<String>>,
}

/// `ClickPipeDestinationTableEngine` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableEngine {
    // columnIds only valid for SummingMergeTree. Skip when empty to avoid API
    // rejection for MergeTree/ReplacingMergeTree/Null engines. Spec has no
    // `required` array so the heuristic treats this as required; API rejects
    // empty values despite that.
    #[serde(rename = "columnIds", skip_serializing_if = "Vec::is_empty")]
    pub column_ids: Vec<String>,
    pub r#type: ClickPipeDestinationTableEngineType,
    #[serde(rename = "versionColumnId", skip_serializing_if = "Option::is_none")]
    pub version_column_id: Option<String>,
}

/// `ClickPipeDestinationTableEngine` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeDestinationTableEngine`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableEngineResponse {
    #[serde(rename = "columnIds", skip_serializing_if = "Option::is_none")]
    pub column_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeDestinationTableEngineType>,
    #[serde(rename = "versionColumnId", skip_serializing_if = "Option::is_none")]
    pub version_column_id: Option<String>,
}

/// `ClickPipeFieldMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeFieldMapping {
    #[serde(rename = "destinationField")]
    pub destination_field: String,
    #[serde(rename = "sourceField")]
    pub source_field: String,
}

/// `ClickPipeFieldMapping` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeFieldMapping`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeFieldMappingResponse {
    #[serde(rename = "destinationField", skip_serializing_if = "Option::is_none")]
    pub destination_field: Option<String>,
    #[serde(rename = "sourceField", skip_serializing_if = "Option::is_none")]
    pub source_field: Option<String>,
}

/// `ClickPipeKafkaOffset` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaOffset {
    pub strategy: ClickPipeKafkaOffsetStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// `ClickPipeKafkaOffset` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeKafkaOffset`]: every field is `Option<T>`, so
/// a field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaOffsetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<ClickPipeKafkaOffsetStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// `ClickPipeKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKafkaSchemaRegistryAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickPipeKafkaSchemaRegistryCredentials` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistryCredentials {
    pub password: String,
    pub username: String,
}

/// `ClickPipeKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKafkaSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brokers: Option<String>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none")]
    pub consumer_group: Option<String>,
    #[serde(rename = "exactlyOnce", skip_serializing_if = "Option::is_none")]
    pub exactly_once: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipeKafkaSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<ClickPipeKafkaOffsetResponse>,
    #[serde(
        rename = "reversePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub reverse_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<ClickPipeKafkaSchemaRegistry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeKafkaSourceType>,
}

/// `ClickPipeKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKinesisSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKinesisSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipeKinesisSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType", skip_serializing_if = "Option::is_none")]
    pub iterator_type: Option<ClickPipeKinesisSourceIteratortype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(rename = "streamName", skip_serializing_if = "Option::is_none")]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettings {
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMongoDBPipeSettingsReplicationmode,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(
        rename = "useJsonNativeFormat",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_json_native_format: Option<bool>,
}

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMongoDBPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettingsResponse {
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipeMongoDBPipeSettingsReplicationmode>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(
        rename = "useJsonNativeFormat",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_json_native_format: Option<bool>,
}

/// `ClickPipeMongoDBPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: String,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
}

/// `ClickPipeMongoDBPipeTableMapping` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeMongoDBPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeTableMappingResponse {
    #[serde(rename = "sourceCollection", skip_serializing_if = "Option::is_none")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName", skip_serializing_if = "Option::is_none")]
    pub source_database_name: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
}

/// `ClickPipeMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference", skip_serializing_if = "Option::is_none")]
    pub read_preference: Option<ClickPipeMongoDBSourceReadpreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeMongoDBPipeSettingsResponse>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeMongoDBPipeTableMappingResponse>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// `ClickPipeMutateBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateBigQuerySource {
    pub credentials: ServiceAccount,
    pub settings: ClickPipeBigQueryPipeSettings,
    #[serde(rename = "snapshotStagingPath")]
    pub snapshot_staging_path: String,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeBigQueryPipeTableMapping>,
}

/// `ClickPipeMutateDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateDestination {
    // The spec describes `columns`, `managedTable`, `table`, and
    // `tableDefinition` as "Required field for all pipe types except database
    // pipes (Postgres, MySQL, BigQuery)" — all four must be omitted entirely
    // for database pipes. Modeled with skip-when-empty / Option so callers can
    // build a single destination type and database pipes serialize cleanly.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<ClickPipeDestinationColumn>,
    pub database: String,
    #[serde(rename = "managedTable", skip_serializing_if = "Option::is_none")]
    pub managed_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "tableDefinition", skip_serializing_if = "Option::is_none")]
    pub table_definition: Option<ClickPipeDestinationTableDefinition>,
}

/// `ClickPipeMutateKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateKafkaSchemaRegistry {
    pub authentication: ClickPipeMutateKafkaSchemaRegistryAuthentication,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: ClickPipeKafkaSchemaRegistryCredentials,
    pub url: String,
}

/// `ClickPipeMutateMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipeMutateMongoDBSourceReadpreference,
    pub settings: ClickPipeMongoDBPipeSettings,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMongoDBPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    pub uri: String,
}

/// `ClickPipeMutateMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeMutateMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    pub host: String,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    pub port: i64,
    #[serde(rename = "serverId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    pub settings: ClickPipeMySQLPipeSettings,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMySQLPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMutateMySQLSourceType>,
}

/// `ClickPipeMutatePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutatePostgresSource {
    pub authentication: ClickPipeMutatePostgresSourceAuthentication,
    // caCertificate is `undefinedOr(isValidPEMCertificate)` server-side — sending
    // `""` (the bare-String default) fails PEM validation. Modeled as
    // `Option<String>` so callers can omit it.
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: PLAIN,
    pub database: String,
    #[serde(rename = "disableTls")]
    pub disable_tls: bool,
    pub host: String,
    // iamRole only applies to RDS-style Postgres + IAM_ROLE auth. Spec marks
    // it required but the server rejects "" for Basic-auth Postgres. Modeled
    // as Option<String> so callers can omit it; same pattern as ca_certificate.
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    pub port: i64,
    pub settings: ClickPipePostgresPipeSettings,
    #[serde(rename = "skipCertVerification")]
    pub skip_cert_verification: bool,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipePostgresPipeTableMapping>,
    // tlsHost is only set when the broker cert SAN doesn't match `host`.
    // Optional in practice; server rejects "" with PEM-style validation.
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMutatePostgresSourceType>,
}

/// `ClickPipeMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeSettings {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "replicationMechanism",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_mechanism: Option<ClickPipeMySQLPipeSettingsReplicationmechanism>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMySQLPipeSettingsReplicationmode,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none")]
    pub use_compression: Option<bool>,
}

/// `ClickPipeMySQLPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMySQLPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeSettingsResponse {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "replicationMechanism",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_mechanism: Option<ClickPipeMySQLPipeSettingsReplicationmechanism>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipeMySQLPipeSettingsReplicationmode>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none")]
    pub use_compression: Option<bool>,
}

/// `ClickPipeMySQLPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMySQLPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeMySQLPipeTableMapping` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMySQLPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeTableMappingResponse {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMySQLPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(rename = "serverId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeMySQLPipeSettingsResponse>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeMySQLPipeTableMappingResponse>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMySQLSourceType>,
}

/// `ClickPipeObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeObjectStorageSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<ClickPipeObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipeObjectStorageSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none")]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none")]
    pub queue_url: Option<String>,
    #[serde(rename = "skipInitialLoad", skip_serializing_if = "Option::is_none")]
    pub skip_initial_load: Option<bool>,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeObjectStorageSourceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickPipePatchDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchDestination {
    pub columns: Vec<ClickPipeDestinationColumn>,
}

/// `ClickPipePatchKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchKafkaSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: serde_json::Value,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "reversePrivateEndpointIds")]
    pub reverse_private_endpoint_ids: Vec<String>,
}

/// `ClickPipePatchKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchKinesisSourceAuthentication>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
}

/// `ClickPipePatchMongoDBPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeRemoveTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePatchMongoDBPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference", skip_serializing_if = "Option::is_none")]
    pub read_preference: Option<ClickPipePatchMongoDBSourceReadpreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePatchMongoDBPipeSettings>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none")]
    pub table_mappings_to_add: Option<Vec<ClickPipeMongoDBPipeTableMapping>>,
    #[serde(
        rename = "tableMappingsToRemove",
        skip_serializing_if = "Option::is_none"
    )]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMongoDBPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    pub uri: Option<String>,
}

/// `ClickPipePatchMySQLPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeRemoveTableMapping {
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePatchMySQLPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none")]
    pub use_compression: Option<bool>,
}

/// `ClickPipePatchMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    pub port: Option<i64>,
    #[serde(rename = "serverId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePatchMySQLPipeSettings>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none")]
    pub table_mappings_to_add: Option<Vec<ClickPipeMySQLPipeTableMapping>>,
    #[serde(
        rename = "tableMappingsToRemove",
        skip_serializing_if = "Option::is_none"
    )]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMySQLPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
    #[serde(rename = "skipInitialLoad", skip_serializing_if = "Option::is_none")]
    pub skip_initial_load: Option<bool>,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
}

/// `ClickPipePatchPostgresPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeRemoveTableMapping {
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePatchPostgresPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchPostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchPostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: PLAIN,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    pub settings: ClickPipePatchPostgresPipeSettings,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd")]
    pub table_mappings_to_add: Vec<ClickPipePostgresPipeTableMapping>,
    #[serde(rename = "tableMappingsToRemove")]
    pub table_mappings_to_remove: Vec<ClickPipePatchPostgresPipeRemoveTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchPubSubSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPubSubSource {
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchPubSubSourceAuthentication>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<ServiceAccount>,
}

/// `ClickPipePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<ClickPipePatchDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none")]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickPipePatchSource>,
}

/// `ClickPipePatchSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipePatchKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipePatchKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<ClickPipePatchMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<ClickPipePatchMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipePatchObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<ClickPipePatchPostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePatchPubSubSource>,
    #[serde(rename = "validateSamples")]
    pub validate_samples: bool,
}

/// `ClickPipePostKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKafkaSource {
    /// Omitted for a broker that requires no authentication: the spec enum has
    /// no "none" value, and the control plane's own field is `omitempty`, so
    /// absence — not a sentinel value — is how "no auth" is expressed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostKafkaSourceAuthentication>,
    pub brokers: String,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none")]
    pub consumer_group: Option<String>,
    pub credentials: serde_json::Value,
    #[serde(rename = "exactlyOnce", skip_serializing_if = "Option::is_none")]
    pub exactly_once: Option<bool>,
    pub format: ClickPipePostKafkaSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<ClickPipeKafkaOffset>,
    #[serde(rename = "reversePrivateEndpointIds")]
    pub reverse_private_endpoint_ids: Vec<String>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<ClickPipeMutateKafkaSchemaRegistry>,
    pub topics: String,
    pub r#type: ClickPipePostKafkaSourceType,
}

/// `ClickPipePostKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    pub authentication: ClickPipePostKinesisSourceAuthentication,
    pub format: ClickPipePostKinesisSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType")]
    pub iterator_type: ClickPipePostKinesisSourceIteratortype,
    pub region: String,
    #[serde(rename = "streamName")]
    pub stream_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipeSchemaDiscoveryField` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoveryField {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

/// `ClickPipeSchemaDiscoveryRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoveryRequest {
    pub source: ClickPipeSchemaDiscoverySource,
}

/// `ClickPipeSchemaDiscoveryResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoveryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<ClickPipeSchemaDiscoveryField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ClickPipeSchemaDiscoveryMeta>,
}

/// `ClickPipeSchemaDiscoveryMeta` from the ClickHouse Cloud API.
pub type ClickPipeSchemaDiscoveryMeta = std::collections::BTreeMap<String, String>;

/// `ClickPipeSchemaDiscoverySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoverySource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipePostKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipePostKinesisSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipePostObjectStorageSource>,
    #[serde(rename = "pubsub", skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePostPubSubSource>,
}

/// `ClickPipePostObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<ClickPipePostObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    pub format: ClickPipePostObjectStorageSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none")]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none")]
    pub queue_url: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
    #[serde(rename = "skipInitialLoad", skip_serializing_if = "Option::is_none")]
    pub skip_initial_load: Option<bool>,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
    pub r#type: ClickPipePostObjectStorageSourceType,
    pub url: String,
}

/// `ClickPipePostPubSubSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostPubSubSource {
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    pub authentication: ClickPipePostPubSubSourceAuthentication,
    #[serde(rename = "enableOrdering", skip_serializing_if = "Option::is_none")]
    pub enable_ordering: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    pub format: ClickPipePostPubSubSourceFormat,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "seekTimestamp", skip_serializing_if = "Option::is_none")]
    pub seek_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "seekType")]
    pub seek_type: ClickPipePostPubSubSourceSeektype,
    #[serde(rename = "serviceAccountKey")]
    pub service_account_key: ServiceAccount,
    pub topic: String,
}

/// `ClickPipePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostRequest {
    pub destination: ClickPipeMutateDestination,
    // Empty arrays rejected by some API paths and never useful on create —
    // skip when empty. Non-Option to match the spec description heuristic.
    #[serde(rename = "fieldMappings", skip_serializing_if = "Vec::is_empty")]
    pub field_mappings: Vec<ClickPipeFieldMapping>,
    pub name: String,
    // scaling block default-serializes as {replicas: 0, ...} which the API
    // rejects ("replicas: Not between 1 and 40"). Modeled as Option so the
    // whole block is omitted when the caller doesn't set it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling: Option<ClickPipeScaling>,
    // settings default-serializes as `{}` which the API also rejects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeSettings>,
    pub source: ClickPipePostSource,
}

/// `ClickPipePostSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery: Option<ClickPipeMutateBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipePostKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipePostKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<ClickPipeMutateMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<ClickPipeMutateMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipePostObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<ClickPipeMutatePostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePostPubSubSource>,
    #[serde(rename = "validateSamples")]
    pub validate_samples: bool,
}

/// `ClickPipePostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeSettings {
    #[serde(rename = "allowNullableColumns")]
    pub allow_nullable_columns: bool,
    #[serde(rename = "deleteOnMerge")]
    pub delete_on_merge: bool,
    #[serde(rename = "enableFailoverSlots")]
    pub enable_failover_slots: bool,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "publicationName", skip_serializing_if = "Option::is_none")]
    pub publication_name: Option<String>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipePostgresPipeSettingsReplicationmode,
    #[serde(
        rename = "replicationSlotName",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_slot_name: Option<String>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePostgresPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipePostgresPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeSettingsResponse {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(
        rename = "enableFailoverSlots",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_failover_slots: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "publicationName", skip_serializing_if = "Option::is_none")]
    pub publication_name: Option<String>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipePostgresPipeSettingsReplicationmode>,
    #[serde(
        rename = "replicationSlotName",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_slot_name: Option<String>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePostgresPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeTableMapping {
    #[serde(rename = "excludedColumns")]
    pub excluded_columns: Vec<String>,
    #[serde(rename = "partitionByExpr")]
    pub partition_by_expr: String,
    #[serde(rename = "partitionKey")]
    pub partition_key: String,
    #[serde(rename = "sortingKeys")]
    pub sorting_keys: Vec<String>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine")]
    pub table_engine: ClickPipePostgresPipeTableMappingTableengine,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey")]
    pub use_custom_sorting_key: bool,
}

/// `ClickPipePostgresPipeTableMapping` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipePostgresPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeTableMappingResponse {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePostgresPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostgresSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePostgresPipeSettingsResponse>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipePostgresPipeTableMappingResponse>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipePostgresSourceType>,
}

/// `ClickPipePubSubSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePubSubSource {
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePubSubSourceAuthentication>,
    #[serde(rename = "enableOrdering", skip_serializing_if = "Option::is_none")]
    pub enable_ordering: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipePubSubSourceFormat>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(rename = "seekTimestamp", skip_serializing_if = "Option::is_none")]
    pub seek_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "seekType", skip_serializing_if = "Option::is_none")]
    pub seek_type: Option<ClickPipePubSubSourceSeektype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// `ClickPipeScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScaling {
    #[cfg(feature = "deprecated-fields")]
    pub concurrency: i64,
    #[serde(rename = "replicaCpuMillicores")]
    pub replica_cpu_millicores: i64,
    #[serde(rename = "replicaMemoryGb")]
    pub replica_memory_gb: f64,
    pub replicas: i64,
}

/// `ClickPipeScaling` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeScaling`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScalingResponse {
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<i64>,
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i64>,
}

/// `ClickPipeScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScalingPatchRequest {
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<i64>,
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i64>,
}

/// `ClickPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(rename = "kafka_read_committed")]
    pub kafka_read_committed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSettings` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeSettings`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettingsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(
        rename = "kafka_read_committed",
        skip_serializing_if = "Option::is_none"
    )]
    pub kafka_read_committed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSettingsPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettingsPutRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_view_processing: Option<bool>,
    /// Kafka-only: the API rejects this key outright for any other source
    /// ("Setting 'kafka_read_committed' is only supported for Kafka
    /// ClickPipes"), so absence — not `false` — is how a non-Kafka settings
    /// update expresses "this setting does not apply".
    #[serde(
        rename = "kafka_read_committed",
        skip_serializing_if = "Option::is_none"
    )]
    pub kafka_read_committed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery: Option<ClickPipeBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipeKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipeKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<ClickPipeMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<ClickPipeMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipeObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<ClickPipePostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePubSubSource>,
}

/// `ClickPipeStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<ClickPipeStatePatchRequestCommand>,
}

/// `ClickPipesCdcScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScaling {
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
}

/// `ClickPipesCdcScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScalingPatchRequest {
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
}

/// `CreateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMapping>>,
    pub description: String,
    #[serde(
        rename = "gcpServiceAttachment",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_service_attachment: Option<String>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none")]
    pub msk_authentication: Option<CreateReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none")]
    pub msk_cluster_arn: Option<String>,
    pub r#type: CreateReversePrivateEndpointType,
    #[serde(
        rename = "vpcEndpointServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(
        rename = "vpcResourceConfigurationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(
        rename = "vpcResourceShareArn",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_share_arn: Option<String>,
}

/// `CustomPrivateDnsMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CustomPrivateDnsMapping {
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
}

/// `CustomPrivateDnsMapping` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`CustomPrivateDnsMapping`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CustomPrivateDnsMappingResponse {
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
}

/// `MskIamUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MskIamUser {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "secretKey")]
    pub secret_key: String,
}

/// `MutualTLS` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MutualTLS {
    pub certificate: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

/// `PLAIN` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PLAIN {
    pub password: String,
    pub username: String,
}

/// `ReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMappingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "dnsNames", skip_serializing_if = "Option::is_none")]
    pub dns_names: Option<Vec<String>>,
    #[serde(rename = "endpointId", skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
    #[serde(
        rename = "gcpServiceAttachment",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_service_attachment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none")]
    pub msk_authentication: Option<ReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none")]
    pub msk_cluster_arn: Option<String>,
    #[serde(rename = "privateDnsNames", skip_serializing_if = "Option::is_none")]
    pub private_dns_names: Option<Vec<String>>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ReversePrivateEndpointStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ReversePrivateEndpointType>,
    #[serde(
        rename = "vpcEndpointServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(
        rename = "vpcResourceConfigurationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(
        rename = "vpcResourceShareArn",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_share_arn: Option<String>,
}

/// `ServiceAccount` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "serviceAccountFile")]
    pub service_account_file: String,
}

/// `UpdateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMapping>>,
}
