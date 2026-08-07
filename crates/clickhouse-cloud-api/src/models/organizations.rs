use super::{ByocConfig, OrganizationPrivateEndpoint, OrganizationPrivateEndpointsPatch};
use serde::{Deserialize, Serialize};
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
