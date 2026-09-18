use serde::{Deserialize, Serialize};
/// Inline enum for `OrganizationPatchPrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPatchPrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPatchPrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPatchPrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPatchPrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPatchPrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `OrganizationCloudRegionPrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationCloudRegionPrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", skip_serializing_if = "Option::is_none")]
    pub endpoint_service_id: Option<String>,
}

/// `OrganizationPatchPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchPrivateEndpoint {
    #[serde(rename = "cloudProvider")]
    pub cloud_provider: OrganizationPatchPrivateEndpointCloudprovider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub id: String,
    pub region: OrganizationPatchPrivateEndpointRegion,
}

/// `OrganizationPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<OrganizationPrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<OrganizationPrivateEndpointRegion>,
}

/// `OrganizationPrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpointsPatch {
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<OrganizationPatchPrivateEndpoint>>,
    pub remove: Vec<OrganizationPatchPrivateEndpoint>,
}
