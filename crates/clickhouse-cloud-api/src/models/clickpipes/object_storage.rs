use super::MskIamUser;
use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipeObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
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
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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

/// Inline enum for `ClickPipePatchObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
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
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
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
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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
