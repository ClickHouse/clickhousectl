use super::{AssignedRole, IpAccessListEntry, IpAccessListEntryResponse};
use serde::{Deserialize, Serialize};
/// Inline enum for `ApiKey.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKeyPatchRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPatchRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyPatchRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKeyPostRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPostRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyPostRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ApiKey` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKey {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntryResponse>>,
    #[serde(rename = "keySuffix", skip_serializing_if = "Option::is_none")]
    pub key_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ApiKeyState>,
    #[serde(rename = "usedAt", skip_serializing_if = "Option::is_none")]
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ApiKeyHashData` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyHashData {
    #[serde(rename = "keyIdHash")]
    pub key_id_hash: String,
    #[serde(rename = "keyIdSuffix")]
    pub key_id_suffix: String,
    #[serde(rename = "keySecretHash")]
    pub key_secret_hash: String,
}

/// `ApiKeyPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ApiKeyPatchRequestState>,
}

/// `ApiKeyPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostRequest {
    #[serde(rename = "assignedRoleIds")]
    pub assigned_role_ids: Vec<uuid::Uuid>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "hashData", skip_serializing_if = "Option::is_none")]
    pub hash_data: Option<ApiKeyHashData>,
    #[serde(rename = "ipAccessList")]
    pub ip_access_list: Vec<IpAccessListEntry>,
    pub name: String,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    pub state: ApiKeyPostRequestState,
}

/// `ApiKeyPostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<ApiKey>,
    #[serde(rename = "keyId", skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(rename = "keySecret", skip_serializing_if = "Option::is_none")]
    pub key_secret: Option<String>,
}
