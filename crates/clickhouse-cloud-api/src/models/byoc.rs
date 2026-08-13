use serde::{Deserialize, Serialize};
/// Inline enum for `ByocConfig.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigCloudprovider {
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

impl std::fmt::Display for ByocConfigCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigRegionid {
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

impl std::fmt::Display for ByocConfigRegionid {
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

/// Inline enum for `ByocConfig.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigState {
    #[serde(rename = "infra-ready")]
    #[default]
    Infra_ready,
    #[serde(rename = "infra-provisioning")]
    Infra_provisioning,
    #[serde(rename = "infra-terminated")]
    Infra_terminated,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infra_ready => write!(f, "infra-ready"),
            Self::Infra_provisioning => write!(f, "infra-provisioning"),
            Self::Infra_terminated => write!(f, "infra-terminated"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructurePostRequest.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructurePostRequestRegionid {
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

impl std::fmt::Display for ByocInfrastructurePostRequestRegionid {
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

/// `ByocConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConfig {
    #[serde(rename = "accountName", skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<ByocConfigCloudprovider>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "regionId", skip_serializing_if = "Option::is_none")]
    pub region_id: Option<ByocConfigRegionid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ByocConfigState>,
}

/// `ByocInfrastructurePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePatchRequest {
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// `ByocInfrastructurePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePostRequest {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "availabilityZoneSuffixes")]
    pub availability_zone_suffixes: Vec<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "regionId")]
    pub region_id: ByocInfrastructurePostRequestRegionid,
    #[serde(rename = "vpcCidrRange")]
    pub vpc_cidr_range: String,
}
