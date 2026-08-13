use super::{
    IpAccessListEntry, IpAccessListEntryResponse, IpAccessListPatch, ResourceTagsV1,
    ResourceTagsV1Response,
};
use serde::{Deserialize, Serialize};

/// `autoscalingMode` enum from the ClickHouse Cloud API.
///
/// Used by `Service`, `ServicePostRequest`, `ServiceReplicaScalingPatchRequest`,
/// `ServiceScalingPatchResponse`, `ScalingScheduleBaseConfig`,
/// `ScalingScheduleEntry`, and `ScalingScheduleEntryRequest`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AutoscalingMode {
    #[serde(rename = "vertical")]
    #[default]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AutoscalingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "vertical"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl AutoscalingMode {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["vertical", "horizontal"];
}

/// Inline enum for `CurrentScaling.effectiveAutoscalingMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CurrentScalingEffectiveautoscalingmode {
    #[serde(rename = "vertical")]
    #[default]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CurrentScalingEffectiveautoscalingmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "vertical"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InstancePrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InstancePrivateEndpointCloudprovider {
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

impl std::fmt::Display for InstancePrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InstancePrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InstancePrivateEndpointRegion {
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

impl std::fmt::Display for InstancePrivateEndpointRegion {
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

/// Inline enum for `Service.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceRegion {
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

impl std::fmt::Display for ServiceRegion {
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

/// Inline enum for `Service.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceState {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "terminating")]
    Terminating,
    #[serde(rename = "softdeleting")]
    Softdeleting,
    #[serde(rename = "awaking")]
    Awaking,
    #[serde(rename = "partially_running")]
    Partially_running,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "terminated")]
    Terminated,
    #[serde(rename = "softdeleted")]
    Softdeleted,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "idle")]
    Idle,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminating => write!(f, "terminating"),
            Self::Softdeleting => write!(f, "softdeleting"),
            Self::Awaking => write!(f, "awaking"),
            Self::Partially_running => write!(f, "partially_running"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
            Self::Softdeleted => write!(f, "softdeleted"),
            Self::Degraded => write!(f, "degraded"),
            Self::Failed => write!(f, "failed"),
            Self::Idle => write!(f, "idle"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => {
                write!(f, "dedicated_standard_n2d_standard_4")
            }
            Self::Dedicated_standard_n2d_standard_8 => {
                write!(f, "dedicated_standard_n2d_standard_8")
            }
            Self::Dedicated_standard_n2d_standard_32 => {
                write!(f, "dedicated_standard_n2d_standard_32")
            }
            Self::Dedicated_standard_n2d_standard_128 => {
                write!(f, "dedicated_standard_n2d_standard_128")
            }
            Self::Dedicated_standard_n2d_standard_32_16SSD => {
                write!(f, "dedicated_standard_n2d_standard_32_16SSD")
            }
            Self::Dedicated_standard_n2d_standard_64_24SSD => {
                write!(f, "dedicated_standard_n2d_standard_64_24SSD")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceEndpoint.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointProtocol {
    #[serde(rename = "https")]
    #[default]
    Https,
    #[serde(rename = "nativesecure")]
    Nativesecure,
    #[serde(rename = "mysql")]
    Mysql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceEndpointProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Https => write!(f, "https"),
            Self::Nativesecure => write!(f, "nativesecure"),
            Self::Mysql => write!(f, "mysql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceEndpointChange.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointChangeProtocol {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceEndpointChangeProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePatchRequest.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePatchRequestReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePatchRequestReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePatchRequestReleasechannel {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["slow", "default", "fast"];
}

/// Inline enum for `ServicePostRequest.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestCompliancetype {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["hipaa", "pci"];
}

/// Inline enum for `ServicePostRequest.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestProfile {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &[
        "v1-default",
        "v1-highmem-xs",
        "v1-highmem-s",
        "v1-highmem-m",
        "v1-highmem-l",
        "v1-highmem-xl",
    ];
}

/// Inline enum for `ServicePostRequest.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestProvider {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["aws", "gcp", "azure"];
}

/// Inline enum for `ServicePostRequest.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestRegion {
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

impl std::fmt::Display for ServicePostRequestRegion {
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

impl ServicePostRequestRegion {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &[
        "ap-northeast-1",
        "ap-northeast-2",
        "ap-south-1",
        "ap-southeast-1",
        "ap-southeast-2",
        "ca-central-1",
        "eu-central-1",
        "eu-west-1",
        "eu-west-2",
        "il-central-1",
        "us-east-1",
        "us-east-2",
        "us-west-2",
        "us-east1",
        "us-central1",
        "europe-west2",
        "europe-west4",
        "asia-southeast1",
        "asia-northeast1",
        "eastus",
        "eastus2",
        "westus3",
        "germanywestcentral",
        "centralus",
    ];
}

/// Inline enum for `ServicePostRequest.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestReleasechannel {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["slow", "default", "fast"];
}

/// Inline enum for `ServicePostRequest.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => {
                write!(f, "dedicated_standard_n2d_standard_4")
            }
            Self::Dedicated_standard_n2d_standard_8 => {
                write!(f, "dedicated_standard_n2d_standard_8")
            }
            Self::Dedicated_standard_n2d_standard_32 => {
                write!(f, "dedicated_standard_n2d_standard_32")
            }
            Self::Dedicated_standard_n2d_standard_128 => {
                write!(f, "dedicated_standard_n2d_standard_128")
            }
            Self::Dedicated_standard_n2d_standard_32_16SSD => {
                write!(f, "dedicated_standard_n2d_standard_32_16SSD")
            }
            Self::Dedicated_standard_n2d_standard_64_24SSD => {
                write!(f, "dedicated_standard_n2d_standard_64_24SSD")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseRegion {
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

impl std::fmt::Display for ServiceScalingPatchResponseRegion {
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

/// Inline enum for `ServiceScalingPatchResponse.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseState {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "terminating")]
    Terminating,
    #[serde(rename = "softdeleting")]
    Softdeleting,
    #[serde(rename = "awaking")]
    Awaking,
    #[serde(rename = "partially_running")]
    Partially_running,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "terminated")]
    Terminated,
    #[serde(rename = "softdeleted")]
    Softdeleted,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "idle")]
    Idle,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminating => write!(f, "terminating"),
            Self::Softdeleting => write!(f, "softdeleting"),
            Self::Awaking => write!(f, "awaking"),
            Self::Partially_running => write!(f, "partially_running"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
            Self::Softdeleted => write!(f, "softdeleted"),
            Self::Degraded => write!(f, "degraded"),
            Self::Failed => write!(f, "failed"),
            Self::Idle => write!(f, "idle"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => {
                write!(f, "dedicated_standard_n2d_standard_4")
            }
            Self::Dedicated_standard_n2d_standard_8 => {
                write!(f, "dedicated_standard_n2d_standard_8")
            }
            Self::Dedicated_standard_n2d_standard_32 => {
                write!(f, "dedicated_standard_n2d_standard_32")
            }
            Self::Dedicated_standard_n2d_standard_128 => {
                write!(f, "dedicated_standard_n2d_standard_128")
            }
            Self::Dedicated_standard_n2d_standard_32_16SSD => {
                write!(f, "dedicated_standard_n2d_standard_32_16SSD")
            }
            Self::Dedicated_standard_n2d_standard_64_24SSD => {
                write!(f, "dedicated_standard_n2d_standard_64_24SSD")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "awake")]
    Awake,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceStatePatchRequestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Awake => write!(f, "awake"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `UsageCostRecord.entityType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UsageCostRecordEntitytype {
    #[serde(rename = "datawarehouse")]
    #[default]
    Datawarehouse,
    #[serde(rename = "service")]
    Service,
    #[serde(rename = "clickpipe")]
    Clickpipe,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UsageCostRecordEntitytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Datawarehouse => write!(f, "datawarehouse"),
            Self::Service => write!(f, "service"),
            Self::Clickpipe => write!(f, "clickpipe"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `CurrentScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CurrentScaling {
    #[serde(rename = "activeEntryId", skip_serializing_if = "Option::is_none")]
    pub active_entry_id: Option<uuid::Uuid>,
    #[serde(
        rename = "effectiveAutoscalingMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_autoscaling_mode: Option<CurrentScalingEffectiveautoscalingmode>,
    #[serde(
        rename = "effectiveIdleScaling",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_idle_scaling: Option<bool>,
    #[serde(
        rename = "effectiveIdleTimeoutMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_idle_timeout_minutes: Option<i64>,
    #[serde(
        rename = "effectiveMaxReplicaMemoryGb",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_max_replica_memory_gb: Option<f64>,
    #[serde(
        rename = "effectiveMaxReplicas",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_max_replicas: Option<i64>,
    #[serde(
        rename = "effectiveMinReplicaMemoryGb",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_min_replica_memory_gb: Option<f64>,
    #[serde(
        rename = "effectiveMinReplicas",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_min_replicas: Option<i64>,
}

/// `InstancePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<InstancePrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<InstancePrivateEndpointRegion>,
}

/// `InstancePrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpointsPatch {
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

/// `InstanceServiceQueryApiEndpointsPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceServiceQueryApiEndpointsPostRequest {
    #[serde(rename = "allowedOrigins")]
    pub allowed_origins: String,
    #[serde(rename = "openApiKeys")]
    pub open_api_keys: Vec<String>,
    pub roles: Vec<String>,
}

/// `InstanceTagsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceTagsPatch {
    pub add: Vec<ResourceTagsV1>,
    pub remove: Vec<ResourceTagsV1>,
}

/// `PrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", skip_serializing_if = "Option::is_none")]
    pub endpoint_service_id: Option<String>,
    #[serde(rename = "privateDnsHostname", skip_serializing_if = "Option::is_none")]
    pub private_dns_hostname: Option<String>,
}

/// `ScalingSchedule` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingSchedule {
    #[serde(rename = "activeEntryId", skip_serializing_if = "Option::is_none")]
    pub active_entry_id: Option<uuid::Uuid>,
    #[serde(rename = "baseConfig", skip_serializing_if = "Option::is_none")]
    pub base_config: Option<ScalingScheduleBaseConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<ScalingScheduleEntry>>,
}

/// `ScalingScheduleBaseConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingScheduleBaseConfig {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<i64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
}

/// `ScalingScheduleEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingScheduleEntry {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "endHourUtc", skip_serializing_if = "Option::is_none")]
    pub end_hour_utc: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<i64>,
    #[serde(rename = "isActiveNow", skip_serializing_if = "Option::is_none")]
    pub is_active_now: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "startHourUtc", skip_serializing_if = "Option::is_none")]
    pub start_hour_utc: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<i64>>,
}

/// `ScalingScheduleEntryRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingScheduleEntryRequest {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "endHourUtc")]
    pub end_hour_utc: i64,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<i64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    pub name: String,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[serde(rename = "startHourUtc")]
    pub start_hour_utc: i64,
    pub weekdays: Vec<i64>,
}

/// `ScalingSchedulePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingSchedulePostRequest {
    pub entries: Vec<ScalingScheduleEntryRequest>,
}

/// `ServicPrivateEndpointePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicPrivateEndpointePostRequest {
    pub description: String,
    pub id: String,
}

/// `Service` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Service {
    #[serde(
        rename = "availablePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(rename = "clickhouseVersion", skip_serializing_if = "Option::is_none")]
    pub clickhouse_version: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ServiceCompliancetype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "currentScaling", skip_serializing_if = "Option::is_none")]
    pub current_scaling: Option<CurrentScaling>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(
        rename = "encryptionAssumedRoleIdentifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpoint>>,
    #[serde(
        rename = "hasTransparentDataEncryption",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntryResponse>>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ServiceProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<ServiceRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServiceReleasechannel>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(rename = "scalingSchedule", skip_serializing_if = "Option::is_none")]
    pub scaling_schedule: Option<ScalingSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ServiceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTagsV1Response>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServiceTier>,
    #[serde(
        rename = "transparentDataEncryptionKeyId",
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServiceClickhouseSetting` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSetting {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ServiceClickhouseSettingSchemaEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingSchemaEntry {
    #[serde(rename = "deprecationNotice", skip_serializing_if = "Option::is_none")]
    pub deprecation_notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// `ServiceClickhouseSettingWarning` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingWarning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `ServiceClickhouseSettingsList` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<ServiceClickhouseSetting>>,
}

/// `ServiceClickhouseSettingsPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
}

/// `ServiceClickhouseSettingsPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsPatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<ServiceClickhouseSettingWarning>>,
}

/// `ServiceClickhouseSettingsSchema` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<ServiceClickhouseSettingSchemaEntry>>,
}

/// `ServiceEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(from = "crate::serde_helpers::ServiceEndpointWire")]
pub struct ServiceEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // The schema currently says `number`, but Cloud endpoints are TCP ports and
    // the API sends integral values. Accept integral float syntax from the API,
    // but keep JSON output integral for consumers.
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ServiceEndpointProtocol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// `ServiceEndpointChange` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpointChange {
    pub enabled: bool,
    pub protocol: ServiceEndpointChangeProtocol,
}

/// `ServicePasswordPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchRequest {
    #[serde(rename = "newDoubleSha1Hash", skip_serializing_if = "Option::is_none")]
    pub new_double_sha1_hash: Option<String>,
    #[serde(rename = "newPasswordHash", skip_serializing_if = "Option::is_none")]
    pub new_password_hash: Option<String>,
}

/// `ServicePasswordPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// `ServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<IpAccessListPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<InstancePrivateEndpointsPatch>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServicePatchRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<InstanceTagsPatch>,
    #[serde(
        rename = "transparentDataEncryptionKeyId",
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostRequest {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "backupId", skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<uuid::Uuid>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ServicePostRequestCompliancetype>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(
        rename = "encryptionAssumedRoleIdentifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(
        rename = "hasTransparentDataEncryption",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList")]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    pub name: String,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(
        rename = "privatePreviewTermsChecked",
        skip_serializing_if = "Option::is_none"
    )]
    pub private_preview_terms_checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServicePostRequestProfile>,
    pub provider: ServicePostRequestProvider,
    pub region: ServicePostRequestRegion,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServicePostRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTagsV1>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServicePostRequestTier>,
}

/// `ServicePostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Service>,
}

/// `ServiceQueryAPIEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceQueryAPIEndpoint {
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "openApiKeys", skip_serializing_if = "Option::is_none")]
    pub open_api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
}

/// `ServiceReplicaScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceReplicaScalingPatchRequest {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
}

/// `ServiceScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchRequest {
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
}

/// `ServiceScalingPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchResponse {
    #[serde(
        rename = "availablePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(rename = "clickhouseVersion", skip_serializing_if = "Option::is_none")]
    pub clickhouse_version: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ServiceScalingPatchResponseCompliancetype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "currentScaling", skip_serializing_if = "Option::is_none")]
    pub current_scaling: Option<CurrentScaling>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(
        rename = "encryptionAssumedRoleIdentifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpoint>>,
    #[serde(
        rename = "hasTransparentDataEncryption",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntryResponse>>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceScalingPatchResponseProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ServiceScalingPatchResponseProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<ServiceScalingPatchResponseRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServiceScalingPatchResponseReleasechannel>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(rename = "scalingSchedule", skip_serializing_if = "Option::is_none")]
    pub scaling_schedule: Option<ScalingSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ServiceScalingPatchResponseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTagsV1Response>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServiceScalingPatchResponseTier>,
    #[serde(
        rename = "transparentDataEncryptionKeyId",
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServiceStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<ServiceStatePatchRequestCommand>,
}

/// `UpgradeWindow` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpgradeWindow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(rename = "startHourUtc", skip_serializing_if = "Option::is_none")]
    pub start_hour_utc: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekday: Option<i64>,
}

/// `UpgradeWindowPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpgradeWindowPutRequest {
    #[serde(rename = "startHourUtc")]
    pub start_hour_utc: i64,
    pub weekday: i64,
}

/// `UsageCost` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCost {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costs: Option<Vec<UsageCostRecord>>,
    #[serde(rename = "grandTotalCHC", skip_serializing_if = "Option::is_none")]
    pub grand_total_chc: Option<f64>,
}

/// `UsageCostMetrics` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostMetrics {
    #[serde(rename = "backupCHC", skip_serializing_if = "Option::is_none")]
    pub backup_chc: Option<f64>,
    #[serde(rename = "computeCHC", skip_serializing_if = "Option::is_none")]
    pub compute_chc: Option<f64>,
    #[serde(rename = "dataTransferCHC", skip_serializing_if = "Option::is_none")]
    pub data_transfer_chc: Option<f64>,
    #[serde(rename = "initialLoadCHC", skip_serializing_if = "Option::is_none")]
    pub initial_load_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier1DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier1_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier2DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier2_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier3DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier3_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier4DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier4_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "publicDataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub public_data_transfer_chc: Option<f64>,
    #[serde(rename = "storageCHC", skip_serializing_if = "Option::is_none")]
    pub storage_chc: Option<f64>,
}

/// `UsageCostRecord` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostRecord {
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "entityId", skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<uuid::Uuid>,
    #[serde(rename = "entityName", skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    #[serde(rename = "entityType", skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<UsageCostRecordEntitytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<UsageCostMetrics>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(rename = "totalCHC", skip_serializing_if = "Option::is_none")]
    pub total_chc: Option<f64>,
}
