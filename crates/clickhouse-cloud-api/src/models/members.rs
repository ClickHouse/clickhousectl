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

/// `NotificationSetting` from the ClickHouse Cloud API.
///
/// One notification setting an organization member has explicitly chosen.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NotificationSetting {
    /// Currently `ui` or `email`.
    #[serde(rename = "channelType", skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<String>,
    /// Currently `enabled` or `disabled`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    /// ID of the resource, of the kind `resource_type` names; the organization
    /// ID for organization-wide notifications such as billing.
    #[serde(rename = "resourceId", skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    /// Currently `service`, `organization`, `clickpipe` or `postgres`; new
    /// values may be added.
    #[serde(rename = "resourceType", skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    /// Notification type name, as listed by `notification_types_get_list`; an
    /// open set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// `NotificationSettingRule` from the ClickHouse Cloud API.
///
/// Applies one effect to every combination of the named notification types,
/// resources and channels.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NotificationSettingRule {
    /// `ui` or `email`; `None` applies the rule to every channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
    /// `enabled`, `disabled` or `inherit`. `inherit` removes the stored setting,
    /// so the notification follows the platform default for the member's role.
    pub effect: String,
    /// Notification type names; at least one.
    pub notifications: Vec<String>,
    /// Resource IDs; at least one. There are no wildcards, and a rule covers no
    /// resource created afterwards.
    pub resources: Vec<String>,
}

/// `MemberNotificationSettingsPatchRequest` from the ClickHouse Cloud API.
///
/// The server requires `rules`: 1 to 100 rules, addressing at most 1000
/// settings in total, applied in order. The request applies completely or not
/// at all.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberNotificationSettingsPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<NotificationSettingRule>>,
}

/// `MemberNotificationSettingsPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberNotificationSettingsPatchResponse {
    /// The member's settings for what the request addressed, after the change.
    /// A setting set to `inherit` is absent because it no longer exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<NotificationSetting>>,
}
