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
    #[serde(rename = "infra-terminating")]
    Infra_terminating,
    #[serde(rename = "infra-degraded")]
    Infra_degraded,
    #[serde(rename = "infra-upgrading")]
    Infra_upgrading,
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
            Self::Infra_terminating => write!(f, "infra-terminating"),
            Self::Infra_degraded => write!(f, "infra-degraded"),
            Self::Infra_upgrading => write!(f, "infra-upgrading"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructurePostRequest.regionId`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ByocInfrastructurePostRequestRegionid {
    #[serde(rename = "ap-northeast-1")]
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

/// Allowed availability-zone suffixes for BYOC infrastructure.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocAvailabilityZoneSuffix {
    #[serde(rename = "a")]
    #[default]
    A,
    #[serde(rename = "b")]
    B,
    #[serde(rename = "c")]
    C,
    #[serde(rename = "d")]
    D,
    #[serde(rename = "e")]
    E,
    #[serde(rename = "f")]
    F,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocAvailabilityZoneSuffix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "a"),
            Self::B => write!(f, "b"),
            Self::C => write!(f, "c"),
            Self::D => write!(f, "d"),
            Self::E => write!(f, "e"),
            Self::F => write!(f, "f"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructureDetails.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructureDetailsCloudprovider {
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

impl std::fmt::Display for ByocInfrastructureDetailsCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructureDetails.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructureDetailsRegionid {
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

impl std::fmt::Display for ByocInfrastructureDetailsRegionid {
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

/// Inline enum for `ByocInfrastructureDetails.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructureDetailsState {
    #[serde(rename = "infra-provisioning")]
    #[default]
    Infra_provisioning,
    #[serde(rename = "infra-terminated")]
    Infra_terminated,
    #[serde(rename = "infra-terminating")]
    Infra_terminating,
    #[serde(rename = "infra-ready")]
    Infra_ready,
    #[serde(rename = "infra-degraded")]
    Infra_degraded,
    #[serde(rename = "infra-upgrading")]
    Infra_upgrading,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocInfrastructureDetailsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infra_provisioning => write!(f, "infra-provisioning"),
            Self::Infra_terminated => write!(f, "infra-terminated"),
            Self::Infra_terminating => write!(f, "infra-terminating"),
            Self::Infra_ready => write!(f, "infra-ready"),
            Self::Infra_degraded => write!(f, "infra-degraded"),
            Self::Infra_upgrading => write!(f, "infra-upgrading"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructureProgress.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructureProgressStatus {
    #[serde(rename = "not_started")]
    #[default]
    NotStarted,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "failed")]
    Failed,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocInfrastructureProgressStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not_started"),
            Self::Pending => write!(f, "pending"),
            Self::Waiting => write!(f, "waiting"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Ready => write!(f, "ready"),
            Self::Failed => write!(f, "failed"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructureProgressStage.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructureProgressStageStatus {
    #[serde(rename = "not_started")]
    #[default]
    NotStarted,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "failed")]
    Failed,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocInfrastructureProgressStageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not_started"),
            Self::Pending => write!(f, "pending"),
            Self::Waiting => write!(f, "waiting"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Ready => write!(f, "ready"),
            Self::Failed => write!(f, "failed"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructureValidatePostRequest.regionId`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ByocInfrastructureValidatePostRequestRegionid {
    #[serde(rename = "ap-northeast-1")]
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

impl std::fmt::Display for ByocInfrastructureValidatePostRequestRegionid {
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

/// Inline enum for `ByocInfrastructureValidation.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructureValidationCloudprovider {
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

impl std::fmt::Display for ByocInfrastructureValidationCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}
/// `ByocConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConfig {
    /// How ClickHouse reaches the infrastructure: `standard` through a
    /// management role in your account, or `restricted` through a connector
    /// you install in a cluster you own. An open set on the wire, so it stays a
    /// string.
    #[serde(rename = "accessModel", skip_serializing_if = "Option::is_none")]
    pub access_model: Option<String>,
    #[serde(rename = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[cfg(feature = "deprecated-fields")]
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
    #[serde(rename = "enablePrivateLink", skip_serializing_if = "Option::is_none")]
    pub enable_private_link: Option<bool>,
    #[serde(
        rename = "enablePrivateLoadBalancer",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_private_load_balancer: Option<bool>,
    #[serde(
        rename = "enablePublicLoadBalancer",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_public_load_balancer: Option<bool>,
    /// GCP BYO-VPC only: Private Service Connect NAT subnet name; only accepted with
    /// `enable_private_link: Some(true)`.
    #[serde(rename = "gcpPscSubnetId", skip_serializing_if = "Option::is_none")]
    pub gcp_psc_subnet_id: Option<String>,
    /// Replaces the infrastructure's additional tags as a whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<ByocInfrastructureTags>,
}

/// `ByocInfrastructurePrivateEndpointConfig` from the ClickHouse Cloud API.
///
/// Private link endpoint service details of a BYOC infrastructure.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePrivateEndpointConfig {
    #[serde(rename = "endpointName", skip_serializing_if = "Option::is_none")]
    pub endpoint_name: Option<String>,
    #[serde(rename = "privateDnsHostname", skip_serializing_if = "Option::is_none")]
    pub private_dns_hostname: Option<String>,
}

/// `ByocInfrastructureTagsResponse` from the ClickHouse Cloud API.
///
/// Additional tags currently applied to the cloud resources of a BYOC infrastructure.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureTagsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<ByocInfrastructureTags>,
}

/// `ByocInfrastructurePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ByocInfrastructurePostRequest {
    /// How ClickHouse reaches the infrastructure: `standard` (the server
    /// default) or `restricted`, which is AWS only and requires
    /// `restricted_facts`. An open set on the wire, so it stays a string.
    #[serde(rename = "accessModel", skip_serializing_if = "Option::is_none")]
    pub access_model: Option<String>,
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(
        rename = "availabilityZoneSuffixes",
        skip_serializing_if = "Option::is_none"
    )]
    pub availability_zone_suffixes: Option<Vec<ByocAvailabilityZoneSuffix>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(
        rename = "gcpPodCidrRangeNames",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_pod_cidr_range_names: Option<Vec<String>>,
    #[serde(
        rename = "gcpSharedVpcHostProjectId",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_shared_vpc_host_project_id: Option<String>,
    #[serde(rename = "privateSubnetIds", skip_serializing_if = "Option::is_none")]
    pub private_subnet_ids: Option<Vec<String>>,
    #[serde(rename = "publicSubnetIds", skip_serializing_if = "Option::is_none")]
    pub public_subnet_ids: Option<Vec<String>>,
    #[serde(rename = "regionId")]
    pub region_id: ByocInfrastructurePostRequestRegionid,
    /// Facts about the cluster you own; required with `access_model`
    /// `restricted`.
    #[serde(rename = "restrictedFacts", skip_serializing_if = "Option::is_none")]
    pub restricted_facts: Option<ByocRestrictedFacts>,
    #[serde(
        rename = "servicePrincipalClientId",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_principal_client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<ByocInfrastructureTags>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(rename = "vpcCidrRange", skip_serializing_if = "Option::is_none")]
    pub vpc_cidr_range: Option<String>,
    #[serde(rename = "vpcId", skip_serializing_if = "Option::is_none")]
    pub vpc_id: Option<String>,
}

/// `ByocInfrastructureValidatePostRequest` from the ClickHouse Cloud API.
///
/// Preflight-checks cloud account permissions for a BYOC infrastructure without creating it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ByocInfrastructureValidatePostRequest {
    /// How ClickHouse reaches the infrastructure: `standard` (the server
    /// default) or `restricted`, which is AWS only and requires
    /// `restricted_facts`. An open set on the wire, so it stays a string.
    #[serde(rename = "accessModel", skip_serializing_if = "Option::is_none")]
    pub access_model: Option<String>,
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(
        rename = "availabilityZoneSuffixes",
        skip_serializing_if = "Option::is_none"
    )]
    pub availability_zone_suffixes: Option<Vec<ByocAvailabilityZoneSuffix>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(
        rename = "gcpPodCidrRangeNames",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_pod_cidr_range_names: Option<Vec<String>>,
    #[serde(
        rename = "gcpSharedVpcHostProjectId",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_shared_vpc_host_project_id: Option<String>,
    #[serde(rename = "privateSubnetIds", skip_serializing_if = "Option::is_none")]
    pub private_subnet_ids: Option<Vec<String>>,
    #[serde(rename = "publicSubnetIds", skip_serializing_if = "Option::is_none")]
    pub public_subnet_ids: Option<Vec<String>>,
    #[serde(rename = "regionId")]
    pub region_id: ByocInfrastructureValidatePostRequestRegionid,
    /// Facts about the cluster you own; required with `access_model`
    /// `restricted`.
    #[serde(rename = "restrictedFacts", skip_serializing_if = "Option::is_none")]
    pub restricted_facts: Option<ByocRestrictedFacts>,
    #[serde(
        rename = "servicePrincipalClientId",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_principal_client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<ByocInfrastructureTags>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(rename = "vpcCidrRange", skip_serializing_if = "Option::is_none")]
    pub vpc_cidr_range: Option<String>,
    #[serde(rename = "vpcId", skip_serializing_if = "Option::is_none")]
    pub vpc_id: Option<String>,
}

/// Cloud resource tags applied to BYOC infrastructure (at most 50 entries).
///
/// Keys are open-ended, so every tag name is preserved.
pub type ByocInfrastructureTags = std::collections::BTreeMap<String, String>;

/// `ByocInfrastructureDetails` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureDetails {
    /// Access model of the infrastructure: `standard` or `restricted`. With
    /// `restricted`, private link, load balancer and VPC fields are false or
    /// absent. An open set on the wire, so it stays a string.
    #[serde(rename = "accessModel", skip_serializing_if = "Option::is_none")]
    pub access_model: Option<String>,
    #[serde(rename = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(rename = "byoVpcId", skip_serializing_if = "Option::is_none")]
    pub byo_vpc_id: Option<String>,
    #[serde(
        rename = "byoVpcPodCidrRangeNames",
        skip_serializing_if = "Option::is_none"
    )]
    pub byo_vpc_pod_cidr_range_names: Option<Vec<String>>,
    #[serde(
        rename = "byoVpcPrivateSubnetIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub byo_vpc_private_subnet_ids: Option<Vec<String>>,
    #[serde(
        rename = "byoVpcSharedVpcHostProjectId",
        skip_serializing_if = "Option::is_none"
    )]
    pub byo_vpc_shared_vpc_host_project_id: Option<String>,
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<ByocInfrastructureDetailsCloudprovider>,
    /// State of the connector a `restricted` infrastructure is reached through.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<ByocConnector>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "enablePrivateLink", skip_serializing_if = "Option::is_none")]
    pub enable_private_link: Option<bool>,
    #[serde(
        rename = "enablePrivateLoadBalancer",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_private_load_balancer: Option<bool>,
    #[serde(
        rename = "enablePublicLoadBalancer",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_public_load_balancer: Option<bool>,
    #[serde(rename = "gcpPscSubnetId", skip_serializing_if = "Option::is_none")]
    pub gcp_psc_subnet_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isByoVpc", skip_serializing_if = "Option::is_none")]
    pub is_byo_vpc: Option<bool>,
    #[serde(rename = "regionId", skip_serializing_if = "Option::is_none")]
    pub region_id: Option<ByocInfrastructureDetailsRegionid>,
    /// Facts the `restricted` infrastructure was registered with.
    #[serde(rename = "restrictedFacts", skip_serializing_if = "Option::is_none")]
    pub restricted_facts: Option<ByocRestrictedFactsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ByocInfrastructureDetailsState>,
    #[serde(
        rename = "vpcAvailabilityZoneList",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_availability_zone_list: Option<Vec<String>>,
    #[serde(rename = "vpcCidrRange", skip_serializing_if = "Option::is_none")]
    pub vpc_cidr_range: Option<String>,
}

/// `ByocInfrastructureProgress` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureProgress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<ByocInfrastructureProgressStage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ByocInfrastructureProgressStatus>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<jiff::Timestamp>,
}

/// `ByocInfrastructureProgressStage` from the ClickHouse Cloud API.
///
/// Stages nest recursively through `sub_stages`; leaf entries are individual cloud resources.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureProgressStage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ByocInfrastructureProgressStageStatus>,
    #[serde(rename = "subStages", skip_serializing_if = "Option::is_none")]
    pub sub_stages: Option<Vec<ByocInfrastructureProgressStage>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<jiff::Timestamp>,
}

/// `ByocInfrastructureValidation` from the ClickHouse Cloud API.
///
/// When `supported` is `false`, an empty `checks` list means nothing was verified.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureValidation {
    #[serde(rename = "allPassed", skip_serializing_if = "Option::is_none")]
    pub all_passed: Option<bool>,
    #[serde(rename = "anyPassed", skip_serializing_if = "Option::is_none")]
    pub any_passed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<ByocInfrastructureValidationCheck>>,
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<ByocInfrastructureValidationCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported: Option<bool>,
}

/// `ByocInfrastructureValidationCheck` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureValidationCheck {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `ByocCustomerAuthorizedAccess` from the ClickHouse Cloud API (Beta).
///
/// When ClickHouse personnel may access a BYOC infrastructure. `mode` is an open
/// set on the wire (currently `always`, `never` or `window`), so it stays a string.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocCustomerAuthorizedAccess {
    /// Whether the data plane allows ClickHouse personnel access at request time.
    #[serde(rename = "allowed", skip_serializing_if = "Option::is_none")]
    pub allowed: Option<bool>,
    /// Length of the access window in seconds; present only for `window` mode.
    #[serde(rename = "durationSeconds", skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i64>,
    /// `always` when the setting has never been configured.
    #[serde(rename = "mode", skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Start of the access window; present only for `window` mode.
    #[serde(rename = "startTimestamp", skip_serializing_if = "Option::is_none")]
    pub start_timestamp: Option<jiff::Timestamp>,
    /// End of the current access window; present only while access is allowed
    /// until a deadline.
    #[serde(rename = "windowEndTimestamp", skip_serializing_if = "Option::is_none")]
    pub window_end_timestamp: Option<jiff::Timestamp>,
}

/// `ByocCustomerAuthorizedAccessPatchRequest` from the ClickHouse Cloud API (Beta).
///
/// The server requires `mode` (an open set, currently `always`, `never` or
/// `window`). `start_timestamp` and `duration_seconds` (1 to 172800) are required
/// with `window` and rejected otherwise.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocCustomerAuthorizedAccessPatchRequest {
    #[serde(rename = "durationSeconds", skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i64>,
    #[serde(rename = "mode", skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(rename = "startTimestamp", skip_serializing_if = "Option::is_none")]
    pub start_timestamp: Option<jiff::Timestamp>,
}

/// `ByocRestrictedFacts` from the ClickHouse Cloud API.
///
/// Facts about the EKS cluster a `restricted` BYOC infrastructure is reached
/// through, as sent when registering or validating it. The response variant is
/// [`ByocRestrictedFactsResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ByocRestrictedFacts {
    /// At least three distinct availability zones of the region. Omitted: the
    /// region default.
    #[serde(rename = "availabilityZones", skip_serializing_if = "Option::is_none")]
    pub availability_zones: Option<Vec<String>>,
    /// S3 bucket services store backups in.
    #[serde(rename = "backupBucket")]
    pub backup_bucket: String,
    /// Name of the EKS cluster the connector runs in.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    /// S3 bucket services store data in.
    #[serde(rename = "dataBucket")]
    pub data_bucket: String,
    /// Install the ClickHouse autoscaler. Server default `false`.
    #[serde(rename = "enableAutoscaler", skip_serializing_if = "Option::is_none")]
    pub enable_autoscaler: Option<bool>,
    /// Install the monitoring components. Server default `true`.
    #[serde(rename = "enableMonitoring", skip_serializing_if = "Option::is_none")]
    pub enable_monitoring: Option<bool>,
    /// Install the volume snapshot controller. Server default `true`.
    #[serde(
        rename = "enableSnapshotController",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_snapshot_controller: Option<bool>,
    /// Direct image mode only (required there): IAM role of the connector
    /// executor, in `accountId`.
    #[serde(rename = "executorRoleArn", skip_serializing_if = "Option::is_none")]
    pub executor_role_arn: Option<String>,
    /// How the connector pulls platform images: `direct` from the ClickHouse
    /// registry, or `bundle` from `image_registry`. An open set on the wire, so
    /// it stays a string.
    #[serde(rename = "imageMode")]
    pub image_mode: String,
    /// Bundle image mode only (required there): OCI repository root holding the
    /// platform images.
    #[serde(rename = "imageRegistry", skip_serializing_if = "Option::is_none")]
    pub image_registry: Option<String>,
    /// Prefix of the namespace each service runs in. Must match the executor
    /// configuration. Omitted: `ns-`.
    #[serde(rename = "namespacePrefix", skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<String>,
    /// Direct image mode only (required there): IAM roles of the cluster nodes,
    /// in `accountId`.
    #[serde(rename = "nodeRoleArns", skip_serializing_if = "Option::is_none")]
    pub node_role_arns: Option<Vec<String>>,
    /// Node group the platform components run on. Omitted: `system`.
    #[serde(rename = "platformNodeGroup", skip_serializing_if = "Option::is_none")]
    pub platform_node_group: Option<String>,
    /// Remote-write URL of the connector scraper. Required unless
    /// `enable_monitoring` is `false`.
    #[serde(rename = "scraperEndpoint", skip_serializing_if = "Option::is_none")]
    pub scraper_endpoint: Option<String>,
    /// IAM role services assume to reach the buckets, in `accountId`.
    #[serde(rename = "storageRoleArn")]
    pub storage_role_arn: String,
}

/// `ByocRestrictedFacts` as returned by the ClickHouse Cloud API.
///
/// Response variant of [`ByocRestrictedFacts`]; convert it with `TryFrom` to
/// send the facts back.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocRestrictedFactsResponse {
    #[serde(rename = "availabilityZones", skip_serializing_if = "Option::is_none")]
    pub availability_zones: Option<Vec<String>>,
    #[serde(rename = "backupBucket", skip_serializing_if = "Option::is_none")]
    pub backup_bucket: Option<String>,
    #[serde(rename = "clusterName", skip_serializing_if = "Option::is_none")]
    pub cluster_name: Option<String>,
    #[serde(rename = "dataBucket", skip_serializing_if = "Option::is_none")]
    pub data_bucket: Option<String>,
    #[serde(rename = "enableAutoscaler", skip_serializing_if = "Option::is_none")]
    pub enable_autoscaler: Option<bool>,
    #[serde(rename = "enableMonitoring", skip_serializing_if = "Option::is_none")]
    pub enable_monitoring: Option<bool>,
    #[serde(
        rename = "enableSnapshotController",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_snapshot_controller: Option<bool>,
    #[serde(rename = "executorRoleArn", skip_serializing_if = "Option::is_none")]
    pub executor_role_arn: Option<String>,
    #[serde(rename = "imageMode", skip_serializing_if = "Option::is_none")]
    pub image_mode: Option<String>,
    #[serde(rename = "imageRegistry", skip_serializing_if = "Option::is_none")]
    pub image_registry: Option<String>,
    #[serde(rename = "namespacePrefix", skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<String>,
    #[serde(rename = "nodeRoleArns", skip_serializing_if = "Option::is_none")]
    pub node_role_arns: Option<Vec<String>>,
    #[serde(rename = "platformNodeGroup", skip_serializing_if = "Option::is_none")]
    pub platform_node_group: Option<String>,
    #[serde(rename = "scraperEndpoint", skip_serializing_if = "Option::is_none")]
    pub scraper_endpoint: Option<String>,
    #[serde(rename = "storageRoleArn", skip_serializing_if = "Option::is_none")]
    pub storage_role_arn: Option<String>,
}

/// `ByocConnector` from the ClickHouse Cloud API.
///
/// State of the connector a `restricted` BYOC infrastructure is reached
/// through. The states are open sets on the wire, so they stay strings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConnector {
    /// Components the connector reports on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ByocConnectorComponent>>,
    /// Last heartbeat of the connector; absent until it first reports.
    #[serde(rename = "lastHeartbeatAt", skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<jiff::Timestamp>,
    /// What you need to do when the platform layer waits on you.
    #[serde(rename = "platformDetail", skip_serializing_if = "Option::is_none")]
    pub platform_detail: Option<String>,
    /// State of the platform layer the connector installs.
    #[serde(rename = "platformState", skip_serializing_if = "Option::is_none")]
    pub platform_state: Option<String>,
}

/// `ByocConnectorComponent` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConnectorComponent {
    /// Last heartbeat of the component.
    #[serde(rename = "lastHeartbeatAt", skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<jiff::Timestamp>,
    /// Connector component, such as `scraper`, `executor` or `troubleshooter`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// State the component last reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Version the component last reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// `ByocInfrastructureEnrollmentTokenPostRequest` from the ClickHouse Cloud API (Beta).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureEnrollmentTokenPostRequest {
    /// Lifetime of the token in seconds: whole minutes between 3600 and 86400.
    /// Omitted: four hours.
    #[serde(rename = "ttlSeconds", skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<i64>,
}

/// `ByocInfrastructureEnrollmentToken` from the ClickHouse Cloud API (Beta).
///
/// `token` is a one-time secret the connector enrolls with; it cannot be read
/// again after this response.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructureEnrollmentToken {
    /// When the token expires.
    #[serde(rename = "expiresAt", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<jiff::Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}
