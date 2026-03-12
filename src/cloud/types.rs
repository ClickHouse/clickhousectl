use serde::{Deserialize, Serialize};

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

// =============================================================================
// Shared helper types
// =============================================================================

/// Resource tag
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTag {
    pub key: String,
    pub value: String,
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
    pub protocol: String,
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
    pub role_type: Option<String>,
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
    pub byoc_config: Option<Vec<ByocInfrastructure>>,
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
    pub cloud_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
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
    pub provider: String,
    pub region: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<Endpoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<u32>,
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
    pub release_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent_data_encryption_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub protocol: String,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// Create service request — all non-deprecated fields from OpenAPI spec
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceRequest {
    pub name: String,
    pub provider: String,
    pub region: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<String>,

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
    pub byoc_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

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
    pub command: String,
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
    pub release_channel: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent_data_encryption_key_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<InstanceTagsPatch>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
}

/// Replica scaling request (PATCH .../services/{serviceId}/replicaScaling)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaScalingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<u32>,
}

/// Password reset response
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetResponse {
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
    pub cloud_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
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
    pub metrics: Option<serde_json::Value>,
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
    pub role: String,
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
    pub role: String,
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
    pub state: String,
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
    pub state: Option<String>,

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
    pub state: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
}

// =============================================================================
// Activity, BYOC, Backup, Backup Config
// =============================================================================

/// Activity log entry
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    #[serde(rename = "type")]
    pub activity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,
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
    pub key_update_type: Option<String>,
}

/// Backup
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_in_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_in_seconds: Option<f64>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<serde_json::Value>,
}

/// BYOC (Bring Your Own Cloud) infrastructure (ByocConfig in OpenAPI spec)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ByocInfrastructure {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Create BYOC infrastructure request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateByocRequest {
    pub region_id: String,
    pub account_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_zone_suffixes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_cidr_range: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Update BYOC infrastructure request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateByocRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Backup configuration
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_period_in_hours: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_retention_period_in_hours: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}

/// Update backup configuration request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_period_in_hours: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_retention_period_in_hours: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}
