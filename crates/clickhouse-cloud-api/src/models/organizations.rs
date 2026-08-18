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
