use super::ServiceAccount;
use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipeBigQueryPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeSettingsReplicationmode {
    #[serde(rename = "snapshot")]
    #[default]
    Snapshot,
    #[serde(rename = "cdc")]
    Cdc,
    #[serde(rename = "cdc_only")]
    CdcOnly,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc => write!(f, "cdc"),
            Self::CdcOnly => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ClickPipeBigQueryPipeSettingsReplicationmode {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["snapshot", "cdc", "cdc_only"];
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

impl ClickPipeBigQueryPipeTableMappingTableengine {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["MergeTree", "ReplacingMergeTree", "Null"];
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

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettings {
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
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeBigQueryPipeSettingsReplicationmode,
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
    #[serde(
        rename = "pullWindowSizeSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub pull_window_size_seconds: Option<i64>,
    #[serde(
        rename = "queryCdcPullSyncParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub query_cdc_pull_sync_parallelism: Option<i64>,
    #[serde(rename = "replicationMethod", skip_serializing_if = "Option::is_none")]
    pub replication_method: Option<String>,
    /// Standard 5-field cron expression (or a descriptor such as `@hourly`) that
    /// schedules CDC syncs, evaluated in UTC unless prefixed with
    /// `CRON_TZ=<zone>`. It must not fire more than once a minute and replaces
    /// `syncIntervalSeconds`, so the two cannot both be set. Not allowed for the
    /// `snapshot` replication mode.
    #[serde(rename = "syncCron", skip_serializing_if = "Option::is_none")]
    pub sync_cron: Option<String>,
    #[serde(rename = "syncDelaySeconds", skip_serializing_if = "Option::is_none")]
    pub sync_delay_seconds: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
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
    #[serde(
        rename = "pullWindowSizeSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub pull_window_size_seconds: Option<i64>,
    #[serde(
        rename = "queryCdcPullSyncParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub query_cdc_pull_sync_parallelism: Option<i64>,
    #[serde(rename = "replicationMethod", skip_serializing_if = "Option::is_none")]
    pub replication_method: Option<String>,
    /// Standard 5-field cron expression (or a descriptor such as `@hourly`) that
    /// schedules CDC syncs, evaluated in UTC unless prefixed with
    /// `CRON_TZ=<zone>`. It must not fire more than once a minute and replaces
    /// `syncIntervalSeconds`, so the two cannot both be set. Not allowed for the
    /// `snapshot` replication mode.
    #[serde(rename = "syncCron", skip_serializing_if = "Option::is_none")]
    pub sync_cron: Option<String>,
    #[serde(rename = "syncDelaySeconds", skip_serializing_if = "Option::is_none")]
    pub sync_delay_seconds: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceDatasetName")]
    pub source_dataset_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeBigQueryPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
    #[serde(rename = "eventsFunction", skip_serializing_if = "Option::is_none")]
    pub events_function: Option<String>,
    #[serde(
        rename = "queryCdcWatermarkColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub query_cdc_watermark_column: Option<String>,
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
    #[serde(rename = "eventsFunction", skip_serializing_if = "Option::is_none")]
    pub events_function: Option<String>,
    #[serde(
        rename = "queryCdcWatermarkColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub query_cdc_watermark_column: Option<String>,
}

/// `ClickPipeBigQuerySource` from the ClickHouse Cloud API, selected by `authentication`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickPipeBigQuerySource {
    ClickPipeBigQueryServiceAccountSource(ClickPipeBigQueryServiceAccountSource),
    ClickPipeBigQueryWorkloadIdentitySource(ClickPipeBigQueryWorkloadIdentitySource),
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickPipeBigQuerySource, "authentication" {
        "SERVICE_ACCOUNT" => ClickPipeBigQueryServiceAccountSource,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY" => ClickPipeBigQueryWorkloadIdentitySource,
    }
}
impl Default for ClickPipeBigQuerySource {
    fn default() -> Self {
        // An empty response has no authentication discriminator to select an arm.
        Self::Unknown(serde_json::json!({}))
    }
}
impl std::fmt::Display for ClickPipeBigQuerySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickPipeBigQueryServiceAccountSource(_) => {
                write!(f, "ClickPipeBigQueryServiceAccountSource")
            }
            Self::ClickPipeBigQueryWorkloadIdentitySource(_) => {
                write!(f, "ClickPipeBigQueryWorkloadIdentitySource")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeMutateBigQuerySource` from the ClickHouse Cloud API, selected by `authentication`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickPipeMutateBigQuerySource {
    ClickPipePostBigQueryServiceAccountSource(ClickPipePostBigQueryServiceAccountSource),
    ClickPipePostBigQueryWorkloadIdentitySource(ClickPipePostBigQueryWorkloadIdentitySource),
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickPipeMutateBigQuerySource, "authentication" {
        "SERVICE_ACCOUNT" => ClickPipePostBigQueryServiceAccountSource,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY" => ClickPipePostBigQueryWorkloadIdentitySource,
        none unless "authentication" => ClickPipePostBigQueryServiceAccountSource,
    }
}
impl Default for ClickPipeMutateBigQuerySource {
    fn default() -> Self {
        Self::ClickPipePostBigQueryServiceAccountSource(
            ClickPipePostBigQueryServiceAccountSource::default(),
        )
    }
}
impl std::fmt::Display for ClickPipeMutateBigQuerySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickPipePostBigQueryServiceAccountSource(_) => {
                write!(f, "ClickPipePostBigQueryServiceAccountSource")
            }
            Self::ClickPipePostBigQueryWorkloadIdentitySource(_) => {
                write!(f, "ClickPipePostBigQueryWorkloadIdentitySource")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// Values of `ClickPipeBigQueryServiceAccountSourceAuthentication` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryServiceAccountSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for ClickPipeBigQueryServiceAccountSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeBigQueryServiceAccountSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryServiceAccountSource {
    #[serde(
        rename = "snapshotStagingPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_staging_path: Option<String>,
    #[serde(rename = "settings", skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeBigQueryPipeSettingsResponse>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeBigQueryPipeTableMappingResponse>>,
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeBigQueryServiceAccountSourceAuthentication>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Values of `ClickPipeBigQueryWorkloadIdentitySourceAuthentication` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryWorkloadIdentitySourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    #[default]
    ServiceAccountWorkloadIdentity,
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for ClickPipeBigQueryWorkloadIdentitySourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeBigQueryWorkloadIdentitySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryWorkloadIdentitySource {
    #[serde(
        rename = "snapshotStagingPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_staging_path: Option<String>,
    #[serde(rename = "settings", skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeBigQueryPipeSettingsResponse>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeBigQueryPipeTableMappingResponse>>,
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeBigQueryWorkloadIdentitySourceAuthentication>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Values of `ClickPipePostBigQueryServiceAccountSourceAuthentication` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostBigQueryServiceAccountSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for ClickPipePostBigQueryServiceAccountSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipePostBigQueryServiceAccountSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostBigQueryServiceAccountSource {
    #[serde(rename = "snapshotStagingPath")]
    pub snapshot_staging_path: String,
    #[serde(rename = "settings")]
    pub settings: ClickPipeBigQueryPipeSettings,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeBigQueryPipeTableMapping>,
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostBigQueryServiceAccountSourceAuthentication>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(rename = "credentials")]
    pub credentials: ServiceAccount,
}

/// Values of `ClickPipePostBigQueryWorkloadIdentitySourceAuthentication` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostBigQueryWorkloadIdentitySourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    #[default]
    ServiceAccountWorkloadIdentity,
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for ClickPipePostBigQueryWorkloadIdentitySourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipePostBigQueryWorkloadIdentitySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostBigQueryWorkloadIdentitySource {
    #[serde(rename = "snapshotStagingPath")]
    pub snapshot_staging_path: String,
    #[serde(rename = "settings")]
    pub settings: ClickPipeBigQueryPipeSettings,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeBigQueryPipeTableMapping>,
    #[serde(rename = "authentication")]
    pub authentication: ClickPipePostBigQueryWorkloadIdentitySourceAuthentication,
    #[serde(rename = "projectId")]
    pub project_id: String,
}

impl From<ClickPipePostBigQueryServiceAccountSource> for ClickPipeMutateBigQuerySource {
    fn from(value: ClickPipePostBigQueryServiceAccountSource) -> Self {
        Self::ClickPipePostBigQueryServiceAccountSource(value)
    }
}

impl From<ClickPipePostBigQueryWorkloadIdentitySource> for ClickPipeMutateBigQuerySource {
    fn from(value: ClickPipePostBigQueryWorkloadIdentitySource) -> Self {
        Self::ClickPipePostBigQueryWorkloadIdentitySource(value)
    }
}
