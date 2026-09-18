use super::AssignedRole;
use serde::{Deserialize, Serialize};
/// Inline enum for `Member.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `MemberPatchRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberPatchRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for MemberPatchRequestRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `Member` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Member {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "joinedAt", skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberRole>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// `MemberPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<String>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberPatchRequestRole>,
}
