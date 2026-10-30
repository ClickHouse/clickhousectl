use super::ServiceAccount;
use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipePatchPubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchPubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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

/// Inline enum for `ClickPipePubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
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

/// `ClickPipePostPubSubSource` from the ClickHouse Cloud API, selected by `authentication`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickPipePostPubSubSource {
    ClickPipePostPubSubServiceAccountSource(ClickPipePostPubSubServiceAccountSource),
    ClickPipePostPubSubWorkloadIdentitySource(ClickPipePostPubSubWorkloadIdentitySource),
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickPipePostPubSubSource, "authentication" {
        "SERVICE_ACCOUNT" => ClickPipePostPubSubServiceAccountSource,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY" => ClickPipePostPubSubWorkloadIdentitySource,
    }
}
impl Default for ClickPipePostPubSubSource {
    fn default() -> Self {
        Self::ClickPipePostPubSubServiceAccountSource(
            ClickPipePostPubSubServiceAccountSource::default(),
        )
    }
}
impl std::fmt::Display for ClickPipePostPubSubSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickPipePostPubSubServiceAccountSource(_) => {
                write!(f, "ClickPipePostPubSubServiceAccountSource")
            }
            Self::ClickPipePostPubSubWorkloadIdentitySource(_) => {
                write!(f, "ClickPipePostPubSubWorkloadIdentitySource")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
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

/// `ClickPipePostPubSubServiceAccountSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostPubSubServiceAccountSource {
    #[serde(rename = "format")]
    pub format: ClickPipePostPubSubSourceFormat,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "topic")]
    pub topic: String,
    #[serde(rename = "authentication")]
    pub authentication: ClickPipePostPubSubSourceAuthentication,
    #[serde(rename = "seekType")]
    pub seek_type: ClickPipePostPubSubSourceSeektype,
    #[serde(rename = "seekTimestamp", skip_serializing_if = "Option::is_none")]
    pub seek_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "filter", skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "enableOrdering", skip_serializing_if = "Option::is_none")]
    pub enable_ordering: Option<bool>,
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    #[serde(rename = "serviceAccountKey")]
    pub service_account_key: ServiceAccount,
}

/// Values of `ClickPipePostPubSubWorkloadIdentitySourceAuthentication` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubWorkloadIdentitySourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    #[default]
    ServiceAccountWorkloadIdentity,
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for ClickPipePostPubSubWorkloadIdentitySourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipePostPubSubWorkloadIdentitySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostPubSubWorkloadIdentitySource {
    #[serde(rename = "format")]
    pub format: ClickPipePostPubSubSourceFormat,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "topic")]
    pub topic: String,
    #[serde(rename = "authentication")]
    pub authentication: ClickPipePostPubSubWorkloadIdentitySourceAuthentication,
    #[serde(rename = "seekType")]
    pub seek_type: ClickPipePostPubSubSourceSeektype,
    #[serde(rename = "seekTimestamp", skip_serializing_if = "Option::is_none")]
    pub seek_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "filter", skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "enableOrdering", skip_serializing_if = "Option::is_none")]
    pub enable_ordering: Option<bool>,
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
}

impl From<ClickPipePostPubSubServiceAccountSource> for ClickPipePostPubSubSource {
    fn from(value: ClickPipePostPubSubServiceAccountSource) -> Self {
        Self::ClickPipePostPubSubServiceAccountSource(value)
    }
}

impl From<ClickPipePostPubSubWorkloadIdentitySource> for ClickPipePostPubSubSource {
    fn from(value: ClickPipePostPubSubWorkloadIdentitySource) -> Self {
        Self::ClickPipePostPubSubWorkloadIdentitySource(value)
    }
}
