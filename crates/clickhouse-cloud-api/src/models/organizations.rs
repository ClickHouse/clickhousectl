use super::{ByocConfig, OrganizationPrivateEndpoint, OrganizationPrivateEndpointsPatch};
use serde::{Deserialize, Serialize};

/// `ActiveBalance` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ActiveBalance {
    #[serde(rename = "amountSpent", skip_serializing_if = "Option::is_none")]
    pub amount_spent: Option<f64>,
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(
        rename = "remainingPrepaidCredits",
        skip_serializing_if = "Option::is_none"
    )]
    pub remaining_prepaid_credits: Option<f64>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "totalAmount", skip_serializing_if = "Option::is_none")]
    pub total_amount: Option<f64>,
}

/// `ActiveBalances` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ActiveBalances {
    #[serde(rename = "prepaidBalances", skip_serializing_if = "Option::is_none")]
    pub prepaid_balances: Option<Vec<ActiveBalance>>,
    #[serde(
        rename = "totalRemainingPrepaidCredits",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_remaining_prepaid_credits: Option<f64>,
}

/// `PrometheusDiscoveryLabels` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrometheusDiscoveryLabels {
    #[serde(
        rename = "__meta_clickhouse_provider",
        skip_serializing_if = "Option::is_none"
    )]
    pub meta_clickhouse_provider: Option<String>,
    #[serde(
        rename = "__meta_clickhouse_region",
        skip_serializing_if = "Option::is_none"
    )]
    pub meta_clickhouse_region: Option<String>,
    #[serde(rename = "__metrics_path__", skip_serializing_if = "Option::is_none")]
    pub metrics_path: Option<String>,
    #[serde(
        rename = "__param_filtered_metrics",
        skip_serializing_if = "Option::is_none"
    )]
    pub param_filtered_metrics: Option<String>,
    #[serde(rename = "__scheme__", skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(
        rename = "clickhouse_discovery_service_name",
        skip_serializing_if = "Option::is_none"
    )]
    pub clickhouse_discovery_service_name: Option<String>,
    #[serde(rename = "clickhouse_org_id", skip_serializing_if = "Option::is_none")]
    pub clickhouse_org_id: Option<uuid::Uuid>,
    #[serde(
        rename = "clickhouse_service_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub clickhouse_service_id: Option<uuid::Uuid>,
    /// Labels the spec allows beyond the named ones (`additionalProperties`).
    #[serde(
        flatten,
        deserialize_with = "crate::serde_helpers::deserialize_flattened_string_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_labels: Option<std::collections::BTreeMap<String, String>>,
}

/// `PrometheusDiscoveryTargetGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrometheusDiscoveryTargetGroup {
    #[serde(rename = "labels", skip_serializing_if = "Option::is_none")]
    pub labels: Option<PrometheusDiscoveryLabels>,
    #[serde(rename = "targets", skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

/// `Organization` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Organization {
    #[serde(rename = "byocConfig", skip_serializing_if = "Option::is_none")]
    pub byoc_config: Option<Vec<ByocConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<OrganizationCapabilities>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none")]
    pub private_endpoints: Option<Vec<OrganizationPrivateEndpoint>>,
}

/// `OrganizationCapabilities` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshots: Option<bool>,
}

/// `OrganizationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none")]
    pub private_endpoints: Option<OrganizationPrivateEndpointsPatch>,
}

/// Values of `CreditBalanceType` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreditBalanceType {
    #[serde(rename = "prepaid")]
    #[default]
    Prepaid,
    #[serde(rename = "trial")]
    Trial,
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for CreditBalanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepaid => write!(f, "prepaid"),
            Self::Trial => write!(f, "trial"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// Values of `NotificationType.severity` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum NotificationTypeSeverity {
    #[serde(rename = "info")]
    #[default]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}
impl std::fmt::Display for NotificationTypeSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// A retained organization notification (Beta).
///
/// Notification types, severities and resource kinds remain open strings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Notification {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "severity", skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(rename = "resourceType", skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(rename = "resourceId", skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(rename = "resourceName", skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "title", skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `NotificationType` from the ClickHouse Cloud API.
///
/// A notification type the organization is entitled to. `category` and
/// `resource_type` are open sets on the wire, so they stay strings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NotificationType {
    /// Stable identifier of the notification type.
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "title", skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "category", skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(rename = "severity", skip_serializing_if = "Option::is_none")]
    pub severity: Option<NotificationTypeSeverity>,
    #[serde(rename = "resourceType", skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    /// Whether the per-role delivery default can be changed.
    #[serde(rename = "isConfigurable", skip_serializing_if = "Option::is_none")]
    pub is_configurable: Option<bool>,
    /// `false` when the caller lacks the permission this type requires.
    #[serde(rename = "isAvailable", skip_serializing_if = "Option::is_none")]
    pub is_available: Option<bool>,
}

/// `CreditBalance` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreditBalance {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<CreditBalanceType>,
    #[serde(rename = "remainingCredits", skip_serializing_if = "Option::is_none")]
    pub remaining_credits: Option<f64>,
    #[serde(rename = "totalAmount", skip_serializing_if = "Option::is_none")]
    pub total_amount: Option<f64>,
    #[serde(rename = "amountSpent", skip_serializing_if = "Option::is_none")]
    pub amount_spent: Option<f64>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// `CreditBalances` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreditBalances {
    #[serde(
        rename = "totalRemainingCredits",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_remaining_credits: Option<f64>,
    #[serde(rename = "balances", skip_serializing_if = "Option::is_none")]
    pub balances: Option<Vec<CreditBalance>>,
}
