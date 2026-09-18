use serde::{Deserialize, Serialize};
/// Inline enum for `AssignedRole.roleType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AssignedRoleRoletype {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AssignedRoleRoletype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `AssignedRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssignedRole {
    #[serde(rename = "roleId", skip_serializing_if = "Option::is_none")]
    pub role_id: Option<uuid::Uuid>,
    #[serde(rename = "roleName", skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
    #[serde(rename = "roleType", skip_serializing_if = "Option::is_none")]
    pub role_type: Option<AssignedRoleRoletype>,
}

/// `IpAccessListEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: String,
}

/// `IpAccessListEntry` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`IpAccessListEntry`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListEntryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// `IpAccessListPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListPatch {
    pub add: Vec<IpAccessListEntry>,
    pub remove: Vec<IpAccessListEntry>,
}

/// `License` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct License {
    #[serde(rename = "environmentFingerprint")]
    pub environment_fingerprint: String,
    pub expiration: String,
    pub id: String,
    pub memory: String,
    pub name: String,
}

/// `ResourceTagsV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceTagsV1 {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ResourceTagsV1` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ResourceTagsV1`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing. Writing a fetched tag back to the API goes through
/// `TryFrom<ResourceTagsV1Response>` (see [`crate::convert`]), because a tag
/// without a key cannot be sent.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceTagsV1Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Standard API response wrapper.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(from = "crate::serde_helpers::ApiResponseWire<T>")]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
