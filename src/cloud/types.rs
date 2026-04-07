use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

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
                    _ => Err(format!("invalid {}: {}", stringify!($name), s)),
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

string_enum! {
    pub enum CloudProvider {
        Aws => "aws",
        Gcp => "gcp",
        Azure => "azure",
    }
}

string_enum! {
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

string_enum! {
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

string_enum! {
    pub enum AssignedRoleType {
        System => "system",
        Custom => "custom",
    }
}

string_enum! {
    pub enum OrganizationRole {
        Admin => "admin",
        Developer => "developer",
    }
}

string_enum! {
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

string_enum! {
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

string_enum! {
    pub enum ReleaseChannel {
        Slow => "slow",
        Default => "default",
        Fast => "fast",
    }
}

string_enum! {
    pub enum ServiceProfile {
        V1Default => "v1-default",
        V1HighmemXs => "v1-highmem-xs",
        V1HighmemS => "v1-highmem-s",
        V1HighmemM => "v1-highmem-m",
        V1HighmemL => "v1-highmem-l",
        V1HighmemXl => "v1-highmem-xl",
    }
}

string_enum! {
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

string_enum! {
    pub enum ApiKeyState {
        Enabled => "enabled",
        Disabled => "disabled",
    }
}

string_enum! {
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

string_enum! {
    pub enum ActivityActorType {
        User => "user",
        Support => "support",
        System => "system",
        Api => "api",
    }
}

string_enum! {
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

string_enum! {
    pub enum BackupStatus {
        Done => "done",
        Error => "error",
        InProgress => "in_progress",
    }
}

string_enum! {
    pub enum BackupType {
        Full => "full",
        Incremental => "incremental",
    }
}

string_enum! {
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

/// ClickPipe
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickPipe {
    pub id: String,
    pub name: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Create ClickPipe request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClickPipeRequest {
    pub name: String,
    pub source: CreateClickPipeSource,
    pub destination: ClickPipeDestination,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClickPipeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<KafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<KinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<PostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<MySQLSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<MongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery: Option<BigQuerySource>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageSource {
    #[serde(rename = "type")]
    pub storage_type: String,
    pub format: String,
    pub url: String,
    pub compression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<ObjectStorageAccessKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageAccessKey {
    pub access_key_id: String,
    pub secret_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickPipeDestination {
    pub database: String,
    pub table: String,
    pub managed_table: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_definition: Option<ClickPipeTableDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickPipeDestinationColumn>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickPipeTableDefinition {
    pub engine: ClickPipeTableEngine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting_key: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickPipeTableEngine {
    #[serde(rename = "type")]
    pub engine_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickPipeDestinationColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaSource {
    #[serde(rename = "type")]
    pub kafka_type: String,
    pub format: String,
    pub brokers: String,
    pub topics: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<KafkaCredentials>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<ObjectStorageAccessKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<KafkaOffset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<KafkaSchemaRegistry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reverse_private_endpoint_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaOffset {
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaSchemaRegistry {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<KafkaCredentials>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KinesisSource {
    pub format: String,
    pub stream_name: String,
    pub region: String,
    pub authentication: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<ObjectStorageAccessKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterator_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSource {
    #[serde(rename = "type")]
    pub postgres_type: String,
    pub credentials: DbCredentials,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub authentication: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub settings: PostgresSettings,
    pub table_mappings: Vec<DbTableMapping>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSettings {
    pub replication_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replication_slot_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MySQLSource {
    #[serde(rename = "type")]
    pub mysql_type: String,
    pub credentials: DbCredentials,
    pub host: String,
    pub port: u16,
    pub authentication: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_cert_verification: Option<bool>,
    pub settings: MySQLSettings,
    pub table_mappings: Vec<DbTableMapping>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MySQLSettings {
    pub replication_mode: String,
    pub replication_mechanism: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoDBSource {
    pub credentials: DbCredentials,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_preference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    pub settings: MongoDBSettings,
    pub table_mappings: Vec<MongoDBTableMapping>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoDBSettings {
    pub replication_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoDBTableMapping {
    pub source_database_name: String,
    pub source_collection: String,
    pub target_table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BigQuerySource {
    pub credentials: BigQueryCredentials,
    pub snapshot_staging_path: String,
    pub settings: BigQuerySettings,
    pub table_mappings: Vec<BigQueryTableMapping>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BigQueryCredentials {
    pub service_account_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BigQuerySettings {
    pub replication_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BigQueryTableMapping {
    pub source_dataset_name: String,
    pub source_table: String,
    pub target_table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbTableMapping {
    pub source_schema_name: String,
    pub source_table: String,
    pub target_table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clickpipe_deserializes_from_api_response() {
        let json = r#"{
            "id": "abc-123",
            "name": "test-pipe",
            "state": "Running",
            "serviceId": "svc-456",
            "createdAt": "2026-04-07T10:00:00Z",
            "updatedAt": "2026-04-07T10:05:00Z"
        }"#;
        let cp: ClickPipe = serde_json::from_str(json).unwrap();
        assert_eq!(cp.id, "abc-123");
        assert_eq!(cp.name, "test-pipe");
        assert_eq!(cp.state, "Running");
        assert_eq!(cp.service_id.unwrap(), "svc-456");
    }

    #[test]
    fn clickpipe_deserializes_with_unknown_fields() {
        let json = r#"{
            "id": "abc-123",
            "name": "test-pipe",
            "state": "Running",
            "source": {"kafka": {"brokers": "localhost:9092"}},
            "destination": {"database": "default"},
            "fieldMappings": [],
            "scaling": {"replicas": 1}
        }"#;
        let cp: ClickPipe = serde_json::from_str(json).unwrap();
        assert_eq!(cp.id, "abc-123");
        assert_eq!(cp.state, "Running");
    }

    #[test]
    fn object_storage_source_serializes_to_camel_case() {
        let source = ObjectStorageSource {
            storage_type: "s3".to_string(),
            format: "JSONEachRow".to_string(),
            url: "https://bucket.s3.amazonaws.com/data/**".to_string(),
            compression: "auto".to_string(),
            is_continuous: Some(true),
            queue_url: Some("https://sqs.us-east-1.amazonaws.com/123/queue".to_string()),
            delimiter: None,
            authentication: Some("IAM_USER".to_string()),
            iam_role: None,
            access_key: Some(ObjectStorageAccessKey {
                access_key_id: "AKIA123".to_string(),
                secret_key: "secret".to_string(),
            }),
            connection_string: None,
            azure_container_name: None,
            path: None,
            service_account_key: None,
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["type"], "s3");
        assert_eq!(json["isContinuous"], true);
        assert_eq!(json["queueUrl"], "https://sqs.us-east-1.amazonaws.com/123/queue");
        assert_eq!(json["accessKey"]["accessKeyId"], "AKIA123");
        assert!(json.get("delimiter").is_none());
        assert!(json.get("connectionString").is_none());
    }

    #[test]
    fn object_storage_source_skips_none_fields() {
        let source = ObjectStorageSource {
            storage_type: "s3".to_string(),
            format: "Parquet".to_string(),
            url: "https://bucket.s3.amazonaws.com/data.parquet".to_string(),
            compression: "none".to_string(),
            is_continuous: None,
            queue_url: None,
            delimiter: None,
            authentication: None,
            iam_role: None,
            access_key: None,
            connection_string: None,
            azure_container_name: None,
            path: None,
            service_account_key: None,
        };
        let json = serde_json::to_value(&source).unwrap();
        assert!(json.get("isContinuous").is_none());
        assert!(json.get("authentication").is_none());
        assert!(json.get("iamRole").is_none());
        assert!(json.get("accessKey").is_none());
    }

    #[test]
    fn kafka_source_serializes_correctly() {
        let source = KafkaSource {
            kafka_type: "redpanda".to_string(),
            format: "JSONEachRow".to_string(),
            brokers: "broker1:9092,broker2:9092".to_string(),
            topics: "events".to_string(),
            consumer_group: Some("my-group".to_string()),
            authentication: Some("SCRAM-SHA-256".to_string()),
            credentials: Some(KafkaCredentials {
                username: "user".to_string(),
                password: "pass".to_string(),
            }),
            iam_role: None,
            access_key: None,
            offset: Some(KafkaOffset {
                strategy: "from_beginning".to_string(),
                timestamp: None,
            }),
            schema_registry: None,
            ca_certificate: Some("-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----".to_string()),
            certificate: None,
            private_key: None,
            reverse_private_endpoint_ids: vec![],
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["type"], "redpanda");
        assert_eq!(json["consumerGroup"], "my-group");
        assert_eq!(json["credentials"]["username"], "user");
        assert_eq!(json["offset"]["strategy"], "from_beginning");
        assert!(json.get("offset").unwrap().get("timestamp").is_none());
        assert!(json.get("iamRole").is_none());
        assert!(json.get("reversePrivateEndpointIds").is_none()); // empty vec skipped
    }

    #[test]
    fn kafka_source_includes_reverse_private_endpoints() {
        let source = KafkaSource {
            kafka_type: "kafka".to_string(),
            format: "Avro".to_string(),
            brokers: "broker:9092".to_string(),
            topics: "topic".to_string(),
            consumer_group: None,
            authentication: None,
            credentials: None,
            iam_role: None,
            access_key: None,
            offset: None,
            schema_registry: None,
            ca_certificate: None,
            certificate: None,
            private_key: None,
            reverse_private_endpoint_ids: vec!["rpe-123".to_string(), "rpe-456".to_string()],
        };
        let json = serde_json::to_value(&source).unwrap();
        let rpe_ids = json["reversePrivateEndpointIds"].as_array().unwrap();
        assert_eq!(rpe_ids.len(), 2);
        assert_eq!(rpe_ids[0], "rpe-123");
    }

    #[test]
    fn kinesis_source_serializes_correctly() {
        let source = KinesisSource {
            format: "JSONEachRow".to_string(),
            stream_name: "my-stream".to_string(),
            region: "us-east-1".to_string(),
            authentication: "IAM_USER".to_string(),
            iam_role: None,
            access_key: Some(ObjectStorageAccessKey {
                access_key_id: "AKIA123".to_string(),
                secret_key: "secret".to_string(),
            }),
            use_enhanced_fan_out: Some(true),
            iterator_type: Some("TRIM_HORIZON".to_string()),
            timestamp: None,
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["streamName"], "my-stream");
        assert_eq!(json["useEnhancedFanOut"], true);
        assert_eq!(json["iteratorType"], "TRIM_HORIZON");
        assert!(json.get("timestamp").is_none());
    }

    #[test]
    fn postgres_source_serializes_correctly() {
        let source = PostgresSource {
            postgres_type: "postgres".to_string(),
            credentials: DbCredentials {
                username: "pguser".to_string(),
                password: "pgpass".to_string(),
            },
            host: "db.example.com".to_string(),
            port: 5432,
            database: "mydb".to_string(),
            authentication: "basic".to_string(),
            iam_role: None,
            tls_host: Some("db.example.com".to_string()),
            ca_certificate: None,
            settings: PostgresSettings {
                replication_mode: "cdc".to_string(),
                publication_name: Some("my_pub".to_string()),
                replication_slot_name: None,
            },
            table_mappings: vec![DbTableMapping {
                source_schema_name: "public".to_string(),
                source_table: "users".to_string(),
                target_table: "public_users".to_string(),
                table_engine: Some("ReplacingMergeTree".to_string()),
            }],
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["type"], "postgres");
        assert_eq!(json["credentials"]["username"], "pguser");
        assert_eq!(json["port"], 5432);
        assert_eq!(json["settings"]["replicationMode"], "cdc");
        assert_eq!(json["settings"]["publicationName"], "my_pub");
        assert!(json["settings"].get("replicationSlotName").is_none());
        assert_eq!(json["tableMappings"][0]["sourceSchemaName"], "public");
        assert_eq!(json["tableMappings"][0]["sourceTable"], "users");
        assert_eq!(json["tableMappings"][0]["targetTable"], "public_users");
    }

    #[test]
    fn mysql_source_serializes_correctly() {
        let source = MySQLSource {
            mysql_type: "mysql".to_string(),
            credentials: DbCredentials {
                username: "root".to_string(),
                password: "pass".to_string(),
            },
            host: "mysql.example.com".to_string(),
            port: 3306,
            authentication: "basic".to_string(),
            iam_role: None,
            tls_host: None,
            ca_certificate: None,
            disable_tls: Some(true),
            skip_cert_verification: None,
            settings: MySQLSettings {
                replication_mode: "cdc".to_string(),
                replication_mechanism: "GTID".to_string(),
            },
            table_mappings: vec![],
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["type"], "mysql");
        assert_eq!(json["disableTls"], true);
        assert!(json.get("skipCertVerification").is_none());
        assert_eq!(json["settings"]["replicationMechanism"], "GTID");
    }

    #[test]
    fn mongodb_source_serializes_correctly() {
        let source = MongoDBSource {
            credentials: DbCredentials {
                username: "mongouser".to_string(),
                password: "mongopass".to_string(),
            },
            uri: "mongodb+srv://cluster.example.net/mydb".to_string(),
            read_preference: Some("secondaryPreferred".to_string()),
            tls_host: None,
            ca_certificate: None,
            disable_tls: None,
            settings: MongoDBSettings {
                replication_mode: "cdc".to_string(),
            },
            table_mappings: vec![MongoDBTableMapping {
                source_database_name: "mydb".to_string(),
                source_collection: "users".to_string(),
                target_table: "mydb_users".to_string(),
                table_engine: Some("ReplacingMergeTree".to_string()),
            }],
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["uri"], "mongodb+srv://cluster.example.net/mydb");
        assert_eq!(json["readPreference"], "secondaryPreferred");
        assert_eq!(json["tableMappings"][0]["sourceCollection"], "users");
        assert_eq!(json["tableMappings"][0]["sourceDatabaseName"], "mydb");
    }

    #[test]
    fn bigquery_source_serializes_correctly() {
        let source = BigQuerySource {
            credentials: BigQueryCredentials {
                service_account_file: "base64encodedkey".to_string(),
            },
            snapshot_staging_path: "gs://bucket/staging".to_string(),
            settings: BigQuerySettings {
                replication_mode: "snapshot".to_string(),
            },
            table_mappings: vec![BigQueryTableMapping {
                source_dataset_name: "my_dataset".to_string(),
                source_table: "my_table".to_string(),
                target_table: "my_dataset_my_table".to_string(),
                table_engine: None,
            }],
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["snapshotStagingPath"], "gs://bucket/staging");
        assert_eq!(json["credentials"]["serviceAccountFile"], "base64encodedkey");
        assert_eq!(json["tableMappings"][0]["sourceDatasetName"], "my_dataset");
        assert!(json["tableMappings"][0].get("tableEngine").is_none());
    }

    #[test]
    fn create_request_serializes_full_s3_request() {
        let request = CreateClickPipeRequest {
            name: "my-pipe".to_string(),
            source: CreateClickPipeSource {
                object_storage: Some(ObjectStorageSource {
                    storage_type: "s3".to_string(),
                    format: "JSONEachRow".to_string(),
                    url: "https://bucket.s3.amazonaws.com/**".to_string(),
                    compression: "auto".to_string(),
                    is_continuous: None,
                    queue_url: None,
                    delimiter: None,
                    authentication: None,
                    iam_role: None,
                    access_key: None,
                    connection_string: None,
                    azure_container_name: None,
                    path: None,
                    service_account_key: None,
                }),
                kafka: None,
                kinesis: None,
                postgres: None,
                mysql: None,
                mongodb: None,
                bigquery: None,
            },
            destination: ClickPipeDestination {
                database: "default".to_string(),
                table: "events".to_string(),
                managed_table: true,
                table_definition: Some(ClickPipeTableDefinition {
                    engine: ClickPipeTableEngine {
                        engine_type: "MergeTree".to_string(),
                    },
                    sorting_key: None,
                    partition_by: None,
                    primary_key: None,
                }),
                columns: Some(vec![
                    ClickPipeDestinationColumn {
                        name: "id".to_string(),
                        column_type: "Int64".to_string(),
                    },
                ]),
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["name"], "my-pipe");
        assert!(json["source"]["objectStorage"].is_object());
        assert!(json["source"].get("kafka").is_none());
        assert!(json["source"].get("kinesis").is_none());
        assert!(json["source"].get("postgres").is_none());
        assert_eq!(json["destination"]["managedTable"], true);
        assert_eq!(json["destination"]["tableDefinition"]["engine"]["type"], "MergeTree");
        assert_eq!(json["destination"]["columns"][0]["name"], "id");
        assert_eq!(json["destination"]["columns"][0]["type"], "Int64");
    }

    #[test]
    fn destination_skips_none_fields() {
        let dest = ClickPipeDestination {
            database: "default".to_string(),
            table: "".to_string(),
            managed_table: false,
            table_definition: None,
            columns: None,
        };
        let json = serde_json::to_value(&dest).unwrap();
        assert!(json.get("tableDefinition").is_none());
        assert!(json.get("columns").is_none());
        assert_eq!(json["managedTable"], false);
    }
}