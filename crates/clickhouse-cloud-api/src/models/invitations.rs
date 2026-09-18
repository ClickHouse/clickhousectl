use super::AssignedRole;
use serde::{Deserialize, Serialize};
/// Inline enum for `Invitation.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InvitationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InvitationPostRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationPostRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InvitationPostRequestRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `Invitation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Invitation {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<InvitationRole>,
}

/// `InvitationPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InvitationPostRequest {
    #[serde(rename = "assignedRoleIds")]
    pub assigned_role_ids: Vec<String>,
    pub email: String,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<InvitationPostRequestRole>,
}
