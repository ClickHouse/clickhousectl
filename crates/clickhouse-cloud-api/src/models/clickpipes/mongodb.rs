use super::PLAIN;
use serde::{Deserialize, Serialize};

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

impl ClickPipeMongoDBPipeSettingsReplicationmode {
    pub const VALUES: &'static [&'static str] = &["cdc", "snapshot", "cdc_only"];
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

impl ClickPipeMongoDBPipeTableMappingTableengine {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["MergeTree", "ReplacingMergeTree", "Null"];
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

impl ClickPipeMutateMongoDBSourceReadpreference {
    pub const VALUES: &'static [&'static str] = &[
        "primary",
        "primaryPreferred",
        "secondary",
        "secondaryPreferred",
        "nearest",
    ];
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

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettings {
    /// Parallel workers per collection during the initial snapshot.
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
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
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
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
    #[serde(rename = "mongoConfig", skip_serializing_if = "Option::is_none")]
    pub mongo_config: Option<ClickPipeMongoDBTableConfig>,
    #[serde(rename = "sourceCollection")]
    pub source_collection: String,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: String,
    #[serde(
        rename = "structuredIngestion",
        skip_serializing_if = "Option::is_none"
    )]
    pub structured_ingestion: Option<ClickPipeMongoDBStructuredIngestion>,
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
    #[serde(rename = "mongoConfig", skip_serializing_if = "Option::is_none")]
    pub mongo_config: Option<ClickPipeMongoDBTableConfigResponse>,
    #[serde(rename = "sourceCollection", skip_serializing_if = "Option::is_none")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName", skip_serializing_if = "Option::is_none")]
    pub source_database_name: Option<String>,
    #[serde(
        rename = "structuredIngestion",
        skip_serializing_if = "Option::is_none"
    )]
    pub structured_ingestion: Option<ClickPipeMongoDBStructuredIngestionResponse>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
}

/// `ClickPipeMongoDBTableConfig` from the ClickHouse Cloud API: per-collection
/// MongoDB replication options.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBTableConfig {
    /// Persist the full document as it was right before it was deleted,
    /// instead of only its `_id`. Requires `changeStreamPreAndPostImages` on
    /// the source collection.
    #[serde(rename = "persistDeletedDocuments")]
    pub persist_deleted_documents: bool,
}

/// `ClickPipeMongoDBTableConfig` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMongoDBTableConfig`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBTableConfigResponse {
    #[serde(
        rename = "persistDeletedDocuments",
        skip_serializing_if = "Option::is_none"
    )]
    pub persist_deleted_documents: Option<bool>,
}

/// `ClickPipeMongoDBStructuredIngestion` from the ClickHouse Cloud API.
///
/// Replicates a collection into typed ClickHouse columns instead of a single
/// JSON document column.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBStructuredIngestion {
    #[serde(rename = "columns", skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickPipeMongoDBPipeColumn>>,
    #[serde(
        rename = "dropUnexpectedValues",
        skip_serializing_if = "Option::is_none"
    )]
    pub drop_unexpected_values: Option<bool>,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
}

/// `ClickPipeMongoDBStructuredIngestion` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeMongoDBStructuredIngestion`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBStructuredIngestionResponse {
    #[serde(rename = "columns", skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickPipeMongoDBPipeColumnResponse>>,
    #[serde(
        rename = "dropUnexpectedValues",
        skip_serializing_if = "Option::is_none"
    )]
    pub drop_unexpected_values: Option<bool>,
    #[serde(rename = "enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
}

/// `ClickPipeMongoDBPipeColumn` from the ClickHouse Cloud API: a top-level
/// document field and the ClickHouse type it is replicated as.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeColumn {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// `ClickPipeMongoDBPipeColumn` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMongoDBPipeColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeColumnResponse {
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
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
