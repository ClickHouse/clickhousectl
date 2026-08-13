use serde::{Deserialize, Serialize};
/// Inline enum for `RBACPolicy.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyAllowdeny {
    #[default]
    ALLOW,
    DENY,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyAllowdeny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALLOW => write!(f, "ALLOW"),
            Self::DENY => write!(f, "DENY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicyCreateRequest.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyCreateRequestAllowdeny {
    #[default]
    ALLOW,
    DENY,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyCreateRequestAllowdeny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALLOW => write!(f, "ALLOW"),
            Self::DENY => write!(f, "DENY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicyTags.roleV2`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyTagsRolev2 {
    #[serde(rename = "sql-console-readonly")]
    #[default]
    Sql_console_readonly,
    #[serde(rename = "sql-console-admin")]
    Sql_console_admin,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyTagsRolev2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql_console_readonly => write!(f, "sql-console-readonly"),
            Self::Sql_console_admin => write!(f, "sql-console-admin"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACRole.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACRoleType {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACRoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `RBACPolicy` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicy {
    #[serde(rename = "allowDeny", skip_serializing_if = "Option::is_none")]
    pub allow_deny: Option<RBACPolicyAllowdeny>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<String>>,
    #[serde(rename = "roleId", skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RBACPolicyTagsResponse>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// `RBACPolicyCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyCreateRequest {
    #[serde(rename = "allowDeny")]
    pub allow_deny: RBACPolicyCreateRequestAllowdeny,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RBACPolicyTags>,
}

/// `RBACPolicyTags` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyTags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grants: Option<Vec<String>>,
    #[serde(rename = "roleV2", skip_serializing_if = "Option::is_none")]
    pub role_v2: Option<RBACPolicyTagsRolev2>,
}

/// `RBACPolicyTags` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`RBACPolicyTags`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyTagsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grants: Option<Vec<String>>,
    #[serde(rename = "roleV2", skip_serializing_if = "Option::is_none")]
    pub role_v2: Option<RBACPolicyTagsRolev2>,
}

/// `RBACRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACRole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actors: Option<Vec<String>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "ownerId", skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<Vec<RBACPolicy>>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<RBACRoleType>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `RoleCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleCreateRequest {
    pub actors: Vec<String>,
    pub name: String,
    pub policies: Vec<RBACPolicyCreateRequest>,
}

/// `RoleUpdateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    pub actors: Vec<String>,
    pub name: String,
    pub policies: Vec<RBACPolicyCreateRequest>,
}
