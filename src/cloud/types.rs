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

/// Organization
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub created_at: Option<String>,
}

/// Service (ClickHouse Cloud instance)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub region: String,
    pub state: String,
    pub tier: Option<String>,
    pub idle_scaling: Option<bool>,
    pub idle_timeout_minutes: Option<u32>,
    pub ip_access_list: Option<Vec<IpAccessEntry>>,
    pub created_at: Option<String>,
    pub endpoints: Option<Vec<Endpoint>>,
    pub min_replica_memory_gb: Option<u32>,
    pub max_replica_memory_gb: Option<u32>,
    pub num_replicas: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IpAccessEntry {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub protocol: String,
    pub host: String,
    pub port: u16,
}

/// Backup
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub service_id: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub size_in_bytes: Option<u64>,
}

/// Resource tag
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTag {
    pub key: String,
    pub value: String,
}

/// Create service request - all non-deprecated fields from OpenAPI spec
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceRequest {
    /// Name of the service (required)
    pub name: String,

    /// Cloud provider: aws, gcp, azure (required)
    pub provider: String,

    /// Service region (required)
    pub region: String,

    /// List of IP addresses allowed to access the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,

    /// Minimum memory per replica in GB (8-356, multiple of 4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<u32>,

    /// Maximum memory per replica in GB (8-356, multiple of 4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<u32>,

    /// Number of replicas (1-20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<u32>,

    /// Allow scale to zero when idle (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,

    /// Minimum idle timeout in minutes (>= 5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<u32>,

    /// Backup ID to restore from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,

    /// Release channel: slow, default, fast
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<String>,

    /// Tags for the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTag>>,

    /// Data warehouse ID (for read replicas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,

    /// Make service read-only (requires data_warehouse_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,

    /// Customer-provided disk encryption key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,

    /// Role for disk encryption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_assumed_role_identifier: Option<String>,

    /// Enable Transparent Data Encryption (enterprise only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_transparent_data_encryption: Option<bool>,

    /// BYOC region ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,

    /// Compliance type: hipaa, pci
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<String>,

    /// Custom instance profile (enterprise only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Accept private preview terms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_preview_terms_checked: Option<bool>,
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
    pub command: String, // "start" or "stop"
}

/// Update service request (PATCH /organizations/{orgId}/services/{serviceId})
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServiceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessEntry>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTag>>,
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
    pub password: String,
}

/// Service query endpoint
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_api_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
}

/// Create query endpoint request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateQueryEndpointRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_api_enabled: Option<bool>,
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

/// Create private endpoint request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePrivateEndpointRequest {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// =============================================================================
// Phase 3 — Org types
// =============================================================================

/// Update organization request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrgRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Organization Prometheus endpoint configuration
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgPrometheus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

/// Usage cost summary for an organization
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCost {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_details: Option<Vec<UsageCostDetail>>,
}

/// Individual service usage cost detail
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCostDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

// =============================================================================
// Phase 4 — Member types
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
    pub created_at: Option<String>,
}

/// Update member request (change role)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRequest {
    pub role: String,
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
    pub expires_at: Option<String>,
}

/// Create invitation request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvitationRequest {
    pub email: String,
    pub role: String,
}

// =============================================================================
// Phase 5 — API Key types
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
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Create API key request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Create API key response (includes the secret, shown only once)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    pub api_key: ApiKey,
    pub key_id: String,
    pub key_secret: String,
}

/// Update API key request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

// =============================================================================
// Phase 6 — Activity, BYOC, Backup Bucket, Backup Config, Prometheus
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

/// Create BYOC infrastructure request (ByocInfrastructurePostRequest in OpenAPI spec)
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

/// Update BYOC infrastructure request (ByocInfrastructurePatchRequest in OpenAPI spec)
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateByocRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Backup bucket configuration (oneOf: AWS, GCP, Azure variants in OpenAPI spec)
/// Uses a flat struct with optional provider-specific fields.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupBucket {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub bucket_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_path: Option<String>,
    // AWS-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
    // GCP-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    // Azure-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

/// Create backup bucket request (oneOf: provider-specific in OpenAPI spec)
/// Uses a flat struct — set fields for the target provider, leave others as None.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupBucketRequest {
    pub bucket_provider: String,
    pub bucket_path: String,
    // AWS-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
    // GCP-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    // Azure-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
}

/// Update backup bucket request (provider-specific fields required per spec)
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupBucketRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_path: Option<String>,
    // AWS-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
    // GCP-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    // Azure-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
}

/// Backup configuration (schedule and retention)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_period_days: Option<u32>,
}

/// Update backup configuration request
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_period_days: Option<u32>,
}

/// Service-level Prometheus configuration
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

/// Setup Prometheus for a service
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPrometheusRequest {}
