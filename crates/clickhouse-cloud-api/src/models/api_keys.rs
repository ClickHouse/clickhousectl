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
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct ApiKeyPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<uuid::Uuid>>,
    /// `None` preserves the expiry, `Some(Some(time))` sets it, and `Some(None)` clears it.
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
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

// Derived nested-Option deserialization loses the distinction between an omitted
// field and explicit null. Preserve key presence without inventing field defaults.
impl<'de> Deserialize<'de> for ApiKeyPatchRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut fields = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        fn take<T: serde::de::DeserializeOwned, E: serde::de::Error>(
            fields: &mut serde_json::Map<String, serde_json::Value>,
            name: &str,
        ) -> Result<Option<T>, E> {
            fields
                .remove(name)
                .map(serde_json::from_value)
                .transpose()
                .map_err(E::custom)
        }

        Ok(Self {
            assigned_role_ids: take::<Option<_>, D::Error>(&mut fields, "assignedRoleIds")?
                .flatten(),
            expire_at: take::<Option<_>, D::Error>(&mut fields, "expireAt")?,
            ip_access_list: take::<Option<_>, D::Error>(&mut fields, "ipAccessList")?.flatten(),
            name: take::<Option<_>, D::Error>(&mut fields, "name")?.flatten(),
            #[cfg(feature = "deprecated-fields")]
            roles: take::<Option<_>, D::Error>(&mut fields, "roles")?.flatten(),
            state: take::<Option<_>, D::Error>(&mut fields, "state")?.flatten(),
        })
    }
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
