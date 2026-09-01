use serde::{Deserialize, Serialize};

/// Inline enum for `AwsBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketBucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPropertiesBucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketBucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPropertiesBucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Backup.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupStatus {
    #[serde(rename = "done")]
    #[default]
    Done,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "in_progress")]
    In_progress,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done => write!(f, "done"),
            Self::Error => write!(f, "error"),
            Self::In_progress => write!(f, "in_progress"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Backup.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupType {
    #[serde(rename = "full")]
    #[default]
    Full,
    #[serde(rename = "incremental")]
    Incremental,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Incremental => write!(f, "incremental"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketBucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPostRequestV1Bucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPropertiesBucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucket` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucket {
    AwsBackupBucket(AwsBackupBucket),
    GcpBackupBucket(GcpBackupBucket),
    AzureBackupBucket(AzureBackupBucket),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucket, "bucketProvider" {
        "AWS" => AwsBackupBucket,
        "GCP" => GcpBackupBucket,
        "AZURE" => AzureBackupBucket,
    }
}

impl std::fmt::Display for BackupBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucket(_) => write!(f, "AwsBackupBucket"),
            Self::GcpBackupBucket(_) => write!(f, "GcpBackupBucket"),
            Self::AzureBackupBucket(_) => write!(f, "AzureBackupBucket"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketPatchRequest` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketPatchRequest {
    AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1),
    GcpBackupBucketPatchRequestV1(GcpBackupBucketPatchRequestV1),
    AzureBackupBucketPatchRequestV1(AzureBackupBucketPatchRequestV1),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucketPatchRequest, "bucketProvider" {
        "AWS" => AwsBackupBucketPatchRequestV1,
        "GCP" => GcpBackupBucketPatchRequestV1,
        "AZURE" => AzureBackupBucketPatchRequestV1,
    }
}

impl std::fmt::Display for BackupBucketPatchRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketPatchRequestV1(_) => write!(f, "AwsBackupBucketPatchRequestV1"),
            Self::GcpBackupBucketPatchRequestV1(_) => write!(f, "GcpBackupBucketPatchRequestV1"),
            Self::AzureBackupBucketPatchRequestV1(_) => {
                write!(f, "AzureBackupBucketPatchRequestV1")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketPostRequest` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketPostRequest {
    AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1),
    GcpBackupBucketPostRequestV1(GcpBackupBucketPostRequestV1),
    AzureBackupBucketPostRequestV1(AzureBackupBucketPostRequestV1),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucketPostRequest, "bucketProvider" {
        "AWS" => AwsBackupBucketPostRequestV1,
        "GCP" => GcpBackupBucketPostRequestV1,
        "AZURE" => AzureBackupBucketPostRequestV1,
    }
}

impl std::fmt::Display for BackupBucketPostRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketPostRequestV1(_) => write!(f, "AwsBackupBucketPostRequestV1"),
            Self::GcpBackupBucketPostRequestV1(_) => write!(f, "GcpBackupBucketPostRequestV1"),
            Self::AzureBackupBucketPostRequestV1(_) => write!(f, "AzureBackupBucketPostRequestV1"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketProperties` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketProperties {
    AwsBackupBucketProperties(AwsBackupBucketProperties),
    GcpBackupBucketProperties(GcpBackupBucketProperties),
    AzureBackupBucketProperties(AzureBackupBucketProperties),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucketProperties, "bucketProvider" {
        "AWS" => AwsBackupBucketProperties,
        "GCP" => GcpBackupBucketProperties,
        "AZURE" => AzureBackupBucketProperties,
    }
}

impl std::fmt::Display for BackupBucketProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketProperties(_) => write!(f, "AwsBackupBucketProperties"),
            Self::GcpBackupBucketProperties(_) => write!(f, "GcpBackupBucketProperties"),
            Self::AzureBackupBucketProperties(_) => write!(f, "AzureBackupBucketProperties"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `AwsBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucket {
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none")]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none")]
    pub bucket_provider: Option<AwsBackupBucketBucketprovider>,
    #[serde(rename = "iamRoleArn", skip_serializing_if = "Option::is_none")]
    pub iam_role_arn: Option<String>,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
}

/// `AwsBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AwsBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "iamRoleArn")]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
}

/// `AwsBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPostRequestV1 {
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AwsBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "iamRoleArn")]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName")]
    pub iam_role_session_name: String,
}

/// `AwsBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketProperties {
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AwsBackupBucketPropertiesBucketprovider,
    #[serde(rename = "iamRoleArn")]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName")]
    pub iam_role_session_name: String,
}

/// `AzureBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucket {
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none")]
    pub bucket_provider: Option<AzureBackupBucketBucketprovider>,
    #[serde(rename = "containerName", skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
}

/// `AzureBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AzureBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "connectionString")]
    pub connection_string: String,
    #[serde(rename = "containerName")]
    pub container_name: String,
}

/// `AzureBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPostRequestV1 {
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AzureBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "connectionString")]
    pub connection_string: String,
    #[serde(rename = "containerName")]
    pub container_name: String,
}

/// `AzureBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketProperties {
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AzureBackupBucketPropertiesBucketprovider,
    #[serde(rename = "containerName")]
    pub container_name: String,
}

/// `Backup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Backup {
    #[serde(rename = "backupName", skip_serializing_if = "Option::is_none")]
    pub backup_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<serde_json::Value>,
    #[serde(rename = "durationInSeconds", skip_serializing_if = "Option::is_none")]
    pub duration_in_seconds: Option<f64>,
    #[serde(rename = "finishedAt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "sizeInBytes", skip_serializing_if = "Option::is_none")]
    pub size_in_bytes: Option<f64>,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BackupStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<BackupType>,
}

/// `BackupConfiguration` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfiguration {
    #[serde(
        rename = "backupPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_period_in_hours: Option<f64>,
    #[serde(
        rename = "backupRetentionPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}

/// `BackupConfigurationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfigurationPatchRequest {
    #[serde(
        rename = "backupPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_period_in_hours: Option<f64>,
    #[serde(
        rename = "backupRetentionPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_retention_period_in_hours: Option<f64>,
    /// Three states, because a PATCH that can only set a value cannot undo it:
    /// `None` omits the key and leaves the stored start time alone,
    /// `Some(None)` sends an explicit `null`, and `Some(Some(time))` sends the
    /// time string. An explicit `null` is what clears a stored start time
    /// (verified against api.clickhouse.cloud on 2026-09-01: the PATCH returns
    /// 200 and the following GET no longer carries the field), which matters
    /// because the API refuses any backup period other than 24 or 48 hours
    /// while one is stored. The empty string is not an alternative: the API
    /// answers it with `BAD_REQUEST: customBackupStartTime must be a valid
    /// time (HH:00)`.
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<Option<String>>,
}

/// `GcpBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucket {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none")]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none")]
    pub bucket_provider: Option<GcpBackupBucketBucketprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
}

/// `GcpBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPatchRequestV1 {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: GcpBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
}

/// `GcpBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPostRequestV1 {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: GcpBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
}

/// `GcpBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketProperties {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: GcpBackupBucketPropertiesBucketprovider,
}

impl Default for BackupBucket {
    fn default() -> Self {
        // Every field of a response variant is `Option<T>`, so the derived
        // `AwsBackupBucket::default()` leaves `bucketProvider` absent and
        // serializes to `{}` — which deserializes back through the
        // discriminator dispatch as `Unknown`, not as this variant. Naming the
        // variant's own wire value keeps the default round-tripping.
        Self::AwsBackupBucket(AwsBackupBucket {
            bucket_provider: Some(AwsBackupBucketBucketprovider::default()),
            ..AwsBackupBucket::default()
        })
    }
}

impl Default for BackupBucketPatchRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1::default())
    }
}

impl Default for BackupBucketPostRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1::default())
    }
}

impl Default for BackupBucketProperties {
    fn default() -> Self {
        Self::AwsBackupBucketProperties(AwsBackupBucketProperties::default())
    }
}
