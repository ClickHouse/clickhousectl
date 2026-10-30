use super::PLAIN;
use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipeMutateMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl ClickPipeMutateMySQLSourceAuthentication {
    pub const VALUES: &'static [&'static str] =
        &["basic", "IAM_ROLE", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"];
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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
    #[serde(rename = "cloudsqlmysql")]
    Cloudsqlmysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl ClickPipeMutateMySQLSourceType {
    pub const VALUES: &'static [&'static str] = &[
        "mysql",
        "rdsmysql",
        "auroramysql",
        "cloudsqlmysql",
        "mariadb",
        "rdsmariadb",
    ];
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Cloudsqlmysql => write!(f, "cloudsqlmysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
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

impl ClickPipeMySQLPipeSettingsReplicationmechanism {
    pub const VALUES: &'static [&'static str] = &["GTID", "FILE_POS"];
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

impl ClickPipeMySQLPipeSettingsReplicationmode {
    pub const VALUES: &'static [&'static str] = &["cdc", "snapshot", "cdc_only"];
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

impl ClickPipeMySQLPipeTableMappingTableengine {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["MergeTree", "ReplacingMergeTree", "Null"];
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
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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
    #[serde(rename = "cloudsqlmysql")]
    Cloudsqlmysql,
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
            Self::Cloudsqlmysql => write!(f, "cloudsqlmysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
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
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
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
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
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
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
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

/// `ClickPipePatchMySQLPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeRemoveTableMapping {
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
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
