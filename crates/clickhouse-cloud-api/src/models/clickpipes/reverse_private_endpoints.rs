use serde::{Deserialize, Serialize};

/// Inline enum for `CreateReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl CreateReversePrivateEndpointMskauthentication {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["SASL_IAM", "SASL_SCRAM"];
}

/// Inline enum for `CreateReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    GCP_PSC_SERVICE_ATTACHMENT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::GCP_PSC_SERVICE_ATTACHMENT => write!(f, "GCP_PSC_SERVICE_ATTACHMENT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl CreateReversePrivateEndpointType {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &[
        "VPC_ENDPOINT_SERVICE",
        "VPC_RESOURCE",
        "MSK_MULTI_VPC",
        "GCP_PSC_SERVICE_ATTACHMENT",
    ];
}

/// Inline enum for `ReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointStatus {
    #[default]
    Unknown,
    Provisioning,
    Deleting,
    Ready,
    Failed,
    PendingAcceptance,
    Rejected,
    Expired,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ReversePrivateEndpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Deleting => write!(f, "Deleting"),
            Self::Ready => write!(f, "Ready"),
            Self::Failed => write!(f, "Failed"),
            Self::PendingAcceptance => write!(f, "PendingAcceptance"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    GCP_PSC_SERVICE_ATTACHMENT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::GCP_PSC_SERVICE_ATTACHMENT => write!(f, "GCP_PSC_SERVICE_ATTACHMENT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpointDnsTarget.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointDnsTargetKind {
    #[default]
    RESOURCE_CONFIGURATION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointDnsTargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RESOURCE_CONFIGURATION => write!(f, "RESOURCE_CONFIGURATION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `CreateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMapping>>,
    pub description: String,
    #[serde(
        rename = "gcpServiceAttachment",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_service_attachment: Option<String>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none")]
    pub msk_authentication: Option<CreateReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none")]
    pub msk_cluster_arn: Option<String>,
    pub r#type: CreateReversePrivateEndpointType,
    #[serde(
        rename = "vpcEndpointServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(
        rename = "vpcResourceConfigurationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(
        rename = "vpcResourceShareArn",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_share_arn: Option<String>,
}

/// `CustomPrivateDnsMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CustomPrivateDnsMapping {
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
    #[serde(rename = "targetId", skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

/// `CustomPrivateDnsMapping` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`CustomPrivateDnsMapping`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CustomPrivateDnsMappingResponse {
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
    #[serde(rename = "targetId", skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

/// `ReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMappingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "dnsNames", skip_serializing_if = "Option::is_none")]
    pub dns_names: Option<Vec<String>>,
    #[serde(rename = "dnsTargets", skip_serializing_if = "Option::is_none")]
    pub dns_targets: Option<Vec<ReversePrivateEndpointDnsTarget>>,
    #[serde(rename = "endpointId", skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
    #[serde(
        rename = "gcpServiceAttachment",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_service_attachment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none")]
    pub msk_authentication: Option<ReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none")]
    pub msk_cluster_arn: Option<String>,
    #[serde(rename = "privateDnsMappings", skip_serializing_if = "Option::is_none")]
    pub private_dns_mappings: Option<Vec<ReversePrivateEndpointPrivateDnsMapping>>,
    #[serde(rename = "privateDnsNames", skip_serializing_if = "Option::is_none")]
    pub private_dns_names: Option<Vec<String>>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ReversePrivateEndpointStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ReversePrivateEndpointType>,
    #[serde(
        rename = "vpcEndpointServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(
        rename = "vpcResourceConfigurationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(
        rename = "vpcResourceShareArn",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_share_arn: Option<String>,
}

/// `ReversePrivateEndpointDnsTarget` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpointDnsTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "internalDnsName", skip_serializing_if = "Option::is_none")]
    pub internal_dns_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ReversePrivateEndpointDnsTargetKind>,
}

/// `ReversePrivateEndpointPrivateDnsMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpointPrivateDnsMapping {
    #[serde(rename = "internalDnsName", skip_serializing_if = "Option::is_none")]
    pub internal_dns_name: Option<String>,
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
}

/// `UpdateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMapping>>,
}
