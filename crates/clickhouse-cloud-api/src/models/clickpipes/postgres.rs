use super::PLAIN;
use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipeMutatePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceAuthentication {
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

impl std::fmt::Display for ClickPipeMutatePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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

/// `ClickPipeMutatePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutatePostgresSource {
    pub authentication: ClickPipeMutatePostgresSourceAuthentication,
    // caCertificate is `undefinedOr(isValidPEMCertificate)` server-side — sending
    // `""` (the bare-String default) fails PEM validation. Modeled as
    // `Option<String>` so callers can omit it.
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    // credentials is the basic-auth username/password pair. The schema carries
    // no `required[]`, so requiredness resolves from the description heuristic
    // and the field reads as required, but IAM_ROLE authentication has no
    // username or password: the role ARN in `iamRole` is the whole credential.
    // Modeled as `Option<PLAIN>` so an IAM_ROLE request can omit the object
    // entirely instead of sending an empty `{"username":"","password":""}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePatchPostgresPipeSettings>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none")]
    pub table_mappings_to_add: Option<Vec<ClickPipePostgresPipeTableMapping>>,
    #[serde(
        rename = "tableMappingsToRemove",
        skip_serializing_if = "Option::is_none"
    )]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchPostgresPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
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
