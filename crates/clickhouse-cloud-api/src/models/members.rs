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

/// `NotificationSetting` from the ClickHouse Cloud API (Beta).
///
/// One member-level override of a notification's delivery. `type`,
/// `resource_type`, `channel_type` (`ui`, `email`) and `effect` (`enabled`,
/// `disabled`) are open sets on the wire, so they stay strings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NotificationSetting {
    #[serde(rename = "channelType", skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<String>,
    #[serde(rename = "effect", skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    /// Organization ID for settings that apply to the organization itself.
    #[serde(rename = "resourceId", skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(rename = "resourceType", skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// `NotificationSettingRule` from the ClickHouse Cloud API (Beta).
///
/// `effect` is `enabled`, `disabled` or `inherit`; `inherit` removes the setting
/// so the role default applies again. Every resource must be named: there are no
/// wildcards. Omitting `channels` applies the rule to every channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationSettingRule {
    #[serde(rename = "channels", skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
    #[serde(rename = "effect")]
    pub effect: String,
    #[serde(rename = "notifications")]
    pub notifications: Vec<String>,
    #[serde(rename = "resources")]
    pub resources: Vec<String>,
}

/// `MemberNotificationSettingsPatchRequest` from the ClickHouse Cloud API (Beta).
///
/// The server requires `rules` (1 to 100), applied in order.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberNotificationSettingsPatchRequest {
    #[serde(rename = "rules", skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<NotificationSettingRule>>,
}

/// `MemberNotificationSettingsPatchResponse` from the ClickHouse Cloud API (Beta).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberNotificationSettingsPatchResponse {
    /// Settings for what the request addressed, after the change. A setting set
    /// to `inherit` is absent.
    #[serde(rename = "settings", skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<NotificationSetting>>,
}
