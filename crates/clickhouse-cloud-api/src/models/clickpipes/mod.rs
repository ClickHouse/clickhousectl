mod bigquery;
mod kafka;
mod kinesis;
mod mongodb;
mod mysql;
mod object_storage;
mod postgres;
mod pubsub;
mod reverse_private_endpoints;
mod schema_discovery;

pub use bigquery::*;
pub use kafka::*;
pub use kinesis::*;
pub use mongodb::*;
pub use mysql::*;
pub use object_storage::*;
pub use postgres::*;
pub use pubsub::*;
pub use reverse_private_endpoints::*;
pub use schema_discovery::*;

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
    /// TTL SQL expression. An empty value omits TTL configuration.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ttl: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
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

/// `ClickPipeMutateDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateDestination {
    // The spec describes `columns`, `managedTable`, `table`, and
    // `tableDefinition` as "Required field for all pipe types except database
    // pipes (Postgres, MySQL, BigQuery)" — all four must be omitted entirely
    // for database pipes, so none of them is in the schema's `required[]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickPipeDestinationColumn>>,
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

/// `ClickPipePatchDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchDestination {
    pub columns: Vec<ClickPipeDestinationColumn>,
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
    #[serde(rename = "validateSamples", skip_serializing_if = "Option::is_none")]
    pub validate_samples: Option<bool>,
}

/// `ClickPipePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostRequest {
    pub destination: ClickPipeMutateDestination,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none")]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
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
    /// Create stopped and start ingestion later; unsupported for database ClickPipes.
    #[serde(rename = "startPaused", skip_serializing_if = "Option::is_none")]
    pub start_paused: Option<bool>,
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
    // The control plane accepted an object-storage create with this key
    // omitted on 2026-09-05. Preserve that absence rather than inventing false.
    #[serde(rename = "validateSamples", skip_serializing_if = "Option::is_none")]
    pub validate_samples: Option<bool>,
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
    // The control plane rejected this key on an object-storage create on
    // 2026-09-05, even when false. It must be absent outside Kafka creates.
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

/// `MskIamUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MskIamUser {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "secretKey")]
    pub secret_key: String,
}

/// `PLAIN` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PLAIN {
    pub password: String,
    pub username: String,
}

/// `ServiceAccount` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "serviceAccountFile")]
    pub service_account_file: String,
}

/// `ClickPipesGcpWorkloadIdentityContext` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesGcpWorkloadIdentityContext {
    #[serde(rename = "supported", skip_serializing_if = "Option::is_none")]
    pub supported: Option<bool>,
    #[serde(rename = "ready", skip_serializing_if = "Option::is_none")]
    pub ready: Option<bool>,
    #[serde(rename = "principal", skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
}

/// `ClickPipesServiceContext` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesServiceContext {
    #[serde(
        rename = "gcpWorkloadIdentity",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_workload_identity: Option<ClickPipesGcpWorkloadIdentityContext>,
}
