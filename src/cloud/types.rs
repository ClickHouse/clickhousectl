use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

/// Strict string enum — rejects unknown values during deserialization.
/// Use for enums that represent user input or CLI-only values.
macro_rules! string_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $(
                #[serde(rename = $value)]
                $variant,
            )+
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let value = match self {
                    $(Self::$variant => $value,)+
                };
                f.write_str(value)
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                match self {
                    $(Self::$variant => $value,)+
                }
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(format!(
                        "unknown value '{}', expected one of: {}",
                        s,
                        [$($value),+].join(", ")
                    )),
                }
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.deref() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.deref()
            }
        }
    };
}

/// Flexible string enum — accepts unknown values from API responses.
/// Use for enums that appear in API response types where the server may
/// return new values the CLI doesn't know about yet.
macro_rules! flexible_string_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            /// Unknown value returned by the API that this CLI version doesn't recognize.
            Other(String),
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                match self {
                    $(Self::$variant => serializer.serialize_str($value),)+
                    Self::Other(s) => serializer.serialize_str(s),
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Ok(match s.as_str() {
                    $($value => Self::$variant,)+
                    _ => Self::Other(s),
                })
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$variant => f.write_str($value),)+
                    Self::Other(s) => f.write_str(s),
                }
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Other(s) => s.as_str(),
                }
            }
        }

        impl $name {
            /// Returns the list of known string values for this enum.
            pub fn known_values() -> &'static [&'static str] {
                &[$($value),+]
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(format!(
                        "unknown value '{}', expected one of: {}",
                        s,
                        [$($value),+].join(", ")
                    )),
                }
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.deref() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.deref()
            }
        }
    };
}

/// Standard API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub result: Option<T>,
    #[allow(dead_code)]
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[allow(dead_code)]
    pub code: Option<String>,
    pub message: String,
}

/// Delete service success payload returned directly by the API without a result wrapper.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub status: f64,
    pub request_id: String,
}

// =============================================================================
// Shared helper types
// =============================================================================

flexible_string_enum! {
    pub enum CloudProvider {
        Aws => "aws",
        Gcp => "gcp",
        Azure => "azure",
    }
}

flexible_string_enum! {
    pub enum CloudRegion {
        ApNortheast1 => "ap-northeast-1",
        ApNortheast2 => "ap-northeast-2",
        ApSouth1 => "ap-south-1",
        ApSoutheast1 => "ap-southeast-1",
        ApSoutheast2 => "ap-southeast-2",
        EuCentral1 => "eu-central-1",
        EuWest1 => "eu-west-1",
        EuWest2 => "eu-west-2",
        IlCentral1 => "il-central-1",
        UsEast1 => "us-east-1",
        UsEast2 => "us-east-2",
        UsWest2 => "us-west-2",
        UsEast1Gcp => "us-east1",
        UsCentral1 => "us-central1",
        EuropeWest4 => "europe-west4",
        AsiaSoutheast1 => "asia-southeast1",
        AsiaNortheast1 => "asia-northeast1",
        EastUs => "eastus",
        EastUs2 => "eastus2",
        WestUs3 => "westus3",
        GermanyWestCentral => "germanywestcentral",
        CentralUs => "centralus",
    }
}

flexible_string_enum! {
    pub enum ServiceEndpointProtocol {
        Https => "https",
        NativeSecure => "nativesecure",
        Mysql => "mysql",
    }
}

string_enum! {
    pub enum ServiceToggleableEndpointProtocol {
        Mysql => "mysql",
    }
}

flexible_string_enum! {
    pub enum AssignedRoleType {
        System => "system",
        Custom => "custom",
    }
}

flexible_string_enum! {
    pub enum OrganizationRole {
        Admin => "admin",
        Developer => "developer",
    }
}

flexible_string_enum! {
    pub enum ServiceTier {
        Development => "development",
        Production => "production",
        DedicatedHighMem => "dedicated_high_mem",
        DedicatedHighCpu => "dedicated_high_cpu",
        DedicatedStandard => "dedicated_standard",
        DedicatedStandardN2dStandard4 => "dedicated_standard_n2d_standard_4",
        DedicatedStandardN2dStandard8 => "dedicated_standard_n2d_standard_8",
        DedicatedStandardN2dStandard32 => "dedicated_standard_n2d_standard_32",
        DedicatedStandardN2dStandard128 => "dedicated_standard_n2d_standard_128",
        DedicatedStandardN2dStandard3216Ssd => "dedicated_standard_n2d_standard_32_16SSD",
        DedicatedStandardN2dStandard6424Ssd => "dedicated_standard_n2d_standard_64_24SSD",
    }
}

flexible_string_enum! {
    pub enum ServiceState {
        Starting => "starting",
        Stopping => "stopping",
        Terminating => "terminating",
        SoftDeleting => "softdeleting",
        Awaking => "awaking",
        PartiallyRunning => "partially_running",
        Provisioning => "provisioning",
        Running => "running",
        Stopped => "stopped",
        Terminated => "terminated",
        SoftDeleted => "softdeleted",
        Degraded => "degraded",
        Failed => "failed",
        Idle => "idle",
    }
}

flexible_string_enum! {
    pub enum ReleaseChannel {
        Slow => "slow",
        Default => "default",
        Fast => "fast",
    }
}

flexible_string_enum! {
    pub enum ServiceProfile {
        V1Default => "v1-default",
        V1HighmemXs => "v1-highmem-xs",
        V1HighmemS => "v1-highmem-s",
        V1HighmemM => "v1-highmem-m",
        V1HighmemL => "v1-highmem-l",
        V1HighmemXl => "v1-highmem-xl",
    }
}

flexible_string_enum! {
    pub enum ComplianceType {
        Hipaa => "hipaa",
        Pci => "pci",
    }
}

string_enum! {
    pub enum ServiceStateCommand {
        Start => "start",
        Stop => "stop",
    }
}

flexible_string_enum! {
    pub enum ApiKeyState {
        Enabled => "enabled",
        Disabled => "disabled",
    }
}

flexible_string_enum! {
    pub enum ActivityType {
        CreateOrganization => "create_organization",
        OrganizationUpdateName => "organization_update_name",
        TransferServiceIn => "transfer_service_in",
        TransferServiceOut => "transfer_service_out",
        SavePaymentMethod => "save_payment_method",
        MarketplaceSubscription => "marketplace_subscription",
        MigrateMarketplaceBillingDetailsIn => "migrate_marketplace_billing_details_in",
        MigrateMarketplaceBillingDetailsOut => "migrate_marketplace_billing_details_out",
        OrganizationUpdateTier => "organization_update_tier",
        OrganizationInviteCreate => "organization_invite_create",
        OrganizationInviteDelete => "organization_invite_delete",
        OrganizationMemberJoin => "organization_member_join",
        OrganizationMemberAdd => "organization_member_add",
        OrganizationMemberLeave => "organization_member_leave",
        OrganizationMemberDelete => "organization_member_delete",
        OrganizationMemberUpdateRole => "organization_member_update_role",
        OrganizationMemberUpdateMfaMethod => "organization_member_update_mfa_method",
        UserLogin => "user_login",
        UserLoginFailed => "user_login_failed",
        UserLogout => "user_logout",
        KeyCreate => "key_create",
        KeyDelete => "key_delete",
        OpenApiKeyUpdate => "openapi_key_update",
        ServiceCreate => "service_create",
        ServiceStart => "service_start",
        ServiceStop => "service_stop",
        ServiceAwaken => "service_awaken",
        ServiceIdle => "service_idle",
        ServiceRunning => "service_running",
        ServicePartiallyRunning => "service_partially_running",
        ServiceDelete => "service_delete",
        ServiceUpdateName => "service_update_name",
        ServiceUpdateIpAccessList => "service_update_ip_access_list",
        ServiceUpdateAutoscalingMemory => "service_update_autoscaling_memory",
        ServiceUpdateAutoscalingIdling => "service_update_autoscaling_idling",
        ServiceUpdatePassword => "service_update_password",
        ServiceUpdateAutoscalingReplicas => "service_update_autoscaling_replicas",
        ServiceUpdateMaxAllowableReplicas => "service_update_max_allowable_replicas",
        ServiceUpdateBackupConfiguration => "service_update_backup_configuration",
        ServiceRestoreBackup => "service_restore_backup",
        ServiceUpdateReleaseChannel => "service_update_release_channel",
        ServiceUpdateGptUsageConsent => "service_update_gpt_usage_consent",
        ServiceUpdatePrivateEndpoints => "service_update_private_endpoints",
        ServiceImportToOrganization => "service_import_to_organization",
        ServiceExportFromOrganization => "service_export_from_organization",
        ServiceMaintenanceStart => "service_maintenance_start",
        ServiceMaintenanceEnd => "service_maintenance_end",
        ServiceUpdateCoreDump => "service_update_core_dump",
        BackupDelete => "backup_delete",
    }
}

flexible_string_enum! {
    pub enum ActivityActorType {
        User => "user",
        Support => "support",
        System => "system",
        Api => "api",
    }
}

flexible_string_enum! {
    pub enum ActivityKeyUpdateType {
        Created => "created",
        Deleted => "deleted",
        NameChanged => "name-changed",
        RoleChanged => "role-changed",
        StateChanged => "state-changed",
        DateChanged => "date-changed",
        IpAccessListChanged => "ip-access-list-changed",
        OrgRoleChanged => "org-role-changed",
        DefaultServiceRoleChanged => "default-service-role-changed",
        ServiceRoleChanged => "service-role-changed",
        RolesV2Changed => "roles-v2-changed",
    }
}

flexible_string_enum! {
    pub enum BackupStatus {
        Done => "done",
        Error => "error",
        InProgress => "in_progress",
    }
}

flexible_string_enum! {
    pub enum BackupType {
        Full => "full",
        Incremental => "incremental",
    }
}

flexible_string_enum! {
    #[allow(clippy::enum_variant_names)]
    pub enum ByocConfigState {
        InfraReady => "infra-ready",
        InfraProvisioning => "infra-provisioning",
        InfraTerminated => "infra-terminated",
    }
}

#[allow(clippy::derivable_impls)]
impl Default for CloudProvider {
    fn default() -> Self {
        Self::Aws
    }
}

#[allow(clippy::derivable_impls)]
impl Default for CloudRegion {
    fn default() -> Self {
        Self::UsEast1
    }
}

/// Resource tag
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTag {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IpAccessEntry {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Patch-style add/remove for IP access list entries
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IpAccessListPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<IpAccessEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<IpAccessEntry>>,
}

/// Patch-style add/remove for private endpoint IDs
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstancePrivateEndpointsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
}

/// Patch-style add/remove for resource tags
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceTagsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<ResourceTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<ResourceTag>>,
}

/// Enable/disable a service endpoint protocol (e.g. mysql)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceEndpointChange {
    pub protocol: ServiceToggleableEndpointProtocol,
    pub enabled: bool,
}

/// Role assigned to an API key, member, or invitation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssignedRole {
    pub role_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_type: Option<AssignedRoleType>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ByocConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ByocConfigState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_id: Option<CloudRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<CloudProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

// =============================================================================
// Organization
// =============================================================================

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_endpoints: Option<Vec<PrivateEndpoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byoc_config: Option<Vec<ByocConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

/// Private endpoint patch for organization update
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPatchPrivateEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<CloudProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<CloudRegion>,
}

/// Patch-style add/remove for organization private endpoints
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPrivateEndpointsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<OrganizationPatchPrivateEndpoint>>,
}

/// Update organization request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrgRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_endpoints: Option<OrganizationPrivateEndpointsPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

// =============================================================================
// Service
// =============================================================================

/// Service (ClickHouse Cloud instance)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: String,
    pub name: String,
    pub provider: CloudProvider,
    pub region: CloudRegion,
    pub state: ServiceState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<Endpoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ReleaseChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent_data_encryption_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ComplianceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub protocol: ServiceEndpointProtocol,
    pub host: String,
    pub port: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// Create service request — all non-deprecated fields from OpenAPI spec
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceRequest {
    pub name: String,
    pub provider: CloudProvider,
    pub region: CloudRegion,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ReleaseChannel>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTag>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_assumed_role_identifier: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_transparent_data_encryption: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ComplianceType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceProfile>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_preview_terms_checked: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

/// Create service response includes credentials
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceResponse {
    pub service: Service,
    pub password: String,
}

/// State change request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChangeRequest {
    pub command: ServiceStateCommand,
}

/// Update service request (PATCH /organizations/{orgId}/services/{serviceId})
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServiceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<IpAccessListPatch>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<InstancePrivateEndpointsPatch>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ReleaseChannel>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent_data_encryption_key_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<InstanceTagsPatch>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

/// Replica scaling request (PATCH .../services/{serviceId}/replicaScaling)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaScalingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
}

/// Replica scaling response (ServiceScalingPatchResponse in OpenAPI spec)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceScalingPatchResponse {
    pub id: String,
    pub name: String,
    pub provider: CloudProvider,
    pub region: CloudRegion,
    pub state: ServiceState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<Endpoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ReleaseChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent_data_encryption_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ComplianceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

/// Password patch response (ServicePasswordPatchResponse in OpenAPI spec)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicePasswordPatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Password patch request (PATCH .../services/{serviceId}/password)
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServicePasswordPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_password_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_double_sha1_hash: Option<String>,
}

/// Service query endpoint (ServiceQueryAPIEndpoint in OpenAPI spec)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<String>,
}

/// Create query endpoint request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateQueryEndpointRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_api_keys: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<String>,
}

/// Private endpoint
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<CloudProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<CloudRegion>,
}

/// Private endpoint response (InstancePrivateEndpoint in OpenAPI spec)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancePrivateEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<CloudProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<CloudRegion>,
}

/// Private endpoint configuration (returned by GET .../privateEndpointConfig)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateEndpointConfig {
    pub endpoint_service_id: String,
    pub private_dns_hostname: String,
}

/// Create private endpoint request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePrivateEndpointRequest {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// =============================================================================
// Usage cost
// =============================================================================

/// Usage cost wrapper (returned by GET .../usageCost)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCost {
    #[serde(rename = "grandTotalCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grand_total_chc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costs: Option<Vec<UsageCostRecord>>,
}

/// Usage cost metrics
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCostMetrics {
    #[serde(rename = "storageCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_chc: Option<f64>,
    #[serde(rename = "backupCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_chc: Option<f64>,
    #[serde(rename = "computeCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_chc: Option<f64>,
    #[serde(rename = "dataTransferCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_transfer_chc: Option<f64>,
    #[serde(rename = "initialLoadCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_load_chc: Option<f64>,
    #[serde(rename = "publicDataTransferCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier1DataTransferCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inter_region_tier1_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier2DataTransferCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inter_region_tier2_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier3DataTransferCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inter_region_tier3_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier4DataTransferCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inter_region_tier4_data_transfer_chc: Option<f64>,
}

/// Usage cost record (per-entity, per-day)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCostRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<UsageCostMetrics>,
    #[serde(rename = "totalCHC")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_chc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
}

// =============================================================================
// Member types
// =============================================================================

/// Organization member
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub user_id: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<OrganizationRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
}

/// Update member request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<String>>,
}

/// Organization invitation
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invitation {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<OrganizationRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
}

/// Create invitation request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvitationRequest {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<String>>,
}

// =============================================================================
// API Key types
// =============================================================================

/// API key
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub state: ApiKeyState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
}

/// Hash data for pre-hashed API key creation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyHashData {
    pub key_id_hash: String,
    pub key_id_suffix: String,
    pub key_secret_hash: String,
}

/// Create API key request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ApiKeyState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_data: Option<ApiKeyHashData>,
}

/// Create API key response (includes the secret, shown only once)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    pub key: ApiKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_secret: Option<String>,
}

/// Update API key request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ApiKeyState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
}

// =============================================================================
// Activity, Backup, Backup Config
// =============================================================================

/// Activity log entry
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    #[serde(rename = "type")]
    pub activity_type: ActivityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<ActivityActorType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_update_type: Option<ActivityKeyUpdateType>,
}

/// Backup
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "bucketProvider")]
#[allow(clippy::upper_case_acronyms)]
pub enum BackupBucket {
    #[serde(rename = "AWS")]
    AWS {
        #[serde(rename = "bucketPath")]
        #[serde(skip_serializing_if = "Option::is_none")]
        bucket_path: Option<String>,
        #[serde(rename = "iamRoleArn")]
        #[serde(skip_serializing_if = "Option::is_none")]
        iam_role_arn: Option<String>,
        #[serde(rename = "iamRoleSessionName")]
        #[serde(skip_serializing_if = "Option::is_none")]
        iam_role_session_name: Option<String>,
    },
    #[serde(rename = "GCP")]
    GCP {
        #[serde(rename = "bucketPath")]
        #[serde(skip_serializing_if = "Option::is_none")]
        bucket_path: Option<String>,
        #[serde(rename = "accessKeyId")]
        #[serde(skip_serializing_if = "Option::is_none")]
        access_key_id: Option<String>,
    },
    #[serde(rename = "AZURE")]
    AZURE {
        #[serde(rename = "containerName")]
        #[serde(skip_serializing_if = "Option::is_none")]
        container_name: Option<String>,
    },
}

/// Backup
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    pub status: BackupStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_in_bytes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_in_seconds: Option<f64>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_type: Option<BackupType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<BackupBucket>,
}

/// Backup configuration
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_period_in_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}

/// Update backup configuration request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_period_in_hours: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_retention_period_in_hours: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}
