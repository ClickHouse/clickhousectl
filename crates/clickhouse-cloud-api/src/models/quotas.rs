use serde::{Deserialize, Serialize};
/// Inline enum for `OrganizationQuota.quotaCode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationQuotaQuotacode {
    #[serde(rename = "services-per-organization")]
    #[default]
    Services_per_organization,
    #[serde(rename = "postgres-services-per-organization")]
    Postgres_services_per_organization,
    #[serde(rename = "replicas-per-warehouse")]
    Replicas_per_warehouse,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationQuotaQuotacode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Services_per_organization => write!(f, "services-per-organization"),
            Self::Postgres_services_per_organization => {
                write!(f, "postgres-services-per-organization")
            }
            Self::Replicas_per_warehouse => write!(f, "replicas-per-warehouse"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationQuota.scope`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationQuotaScope {
    #[serde(rename = "organization")]
    #[default]
    Organization,
    #[serde(rename = "warehouse")]
    Warehouse,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationQuotaScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Organization => write!(f, "organization"),
            Self::Warehouse => write!(f, "warehouse"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `OrganizationQuota` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationQuota {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "quotaCode", skip_serializing_if = "Option::is_none")]
    pub quota_code: Option<OrganizationQuotaQuotacode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<OrganizationQuotaScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}
