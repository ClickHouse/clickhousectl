//! Typed models for ClickHouse Cloud API schemas.
//!
//! Auto-generated from the OpenAPI specification.

use serde::{Deserialize, Serialize};

/// `pgHaType` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgHaType {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "async")]
    Async,
    #[serde(rename = "sync")]
    Sync,
}

/// `pgProvider` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
}

/// `pgSize` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgSize {
    #[serde(rename = "c6gd.medium")]
    #[default]
    C6gd_medium,
    #[serde(rename = "c6gd.large")]
    C6gd_large,
    #[serde(rename = "c6gd.xlarge")]
    C6gd_xlarge,
    #[serde(rename = "c6gd.2xlarge")]
    C6gd_2xlarge,
    #[serde(rename = "c6gd.4xlarge")]
    C6gd_4xlarge,
    #[serde(rename = "c6gd.8xlarge")]
    C6gd_8xlarge,
    #[serde(rename = "c6gd.12xlarge")]
    C6gd_12xlarge,
    #[serde(rename = "c6gd.16xlarge")]
    C6gd_16xlarge,
    #[serde(rename = "c6gd.metal")]
    C6gd_metal,
    #[serde(rename = "i7i.large")]
    I7i_large,
    #[serde(rename = "i7i.xlarge")]
    I7i_xlarge,
    #[serde(rename = "i7i.2xlarge")]
    I7i_2xlarge,
    #[serde(rename = "i7i.4xlarge")]
    I7i_4xlarge,
    #[serde(rename = "i7i.8xlarge")]
    I7i_8xlarge,
    #[serde(rename = "i7i.12xlarge")]
    I7i_12xlarge,
    #[serde(rename = "i7i.16xlarge")]
    I7i_16xlarge,
    #[serde(rename = "i7i.24xlarge")]
    I7i_24xlarge,
    #[serde(rename = "i7i.metal-24xl")]
    I7i_metal_24xl,
    #[serde(rename = "i7i.48xlarge")]
    I7i_48xlarge,
    #[serde(rename = "i7i.metal-48xl")]
    I7i_metal_48xl,
    #[serde(rename = "i7ie.large")]
    I7ie_large,
    #[serde(rename = "i7ie.xlarge")]
    I7ie_xlarge,
    #[serde(rename = "i7ie.2xlarge")]
    I7ie_2xlarge,
    #[serde(rename = "i7ie.3xlarge")]
    I7ie_3xlarge,
    #[serde(rename = "i7ie.6xlarge")]
    I7ie_6xlarge,
    #[serde(rename = "i7ie.12xlarge")]
    I7ie_12xlarge,
    #[serde(rename = "i7ie.18xlarge")]
    I7ie_18xlarge,
    #[serde(rename = "i7ie.24xlarge")]
    I7ie_24xlarge,
    #[serde(rename = "i7ie.metal-24xl")]
    I7ie_metal_24xl,
    #[serde(rename = "i7ie.48xlarge")]
    I7ie_48xlarge,
    #[serde(rename = "i7ie.metal-48xl")]
    I7ie_metal_48xl,
    #[serde(rename = "i8g.large")]
    I8g_large,
    #[serde(rename = "i8g.xlarge")]
    I8g_xlarge,
    #[serde(rename = "i8g.2xlarge")]
    I8g_2xlarge,
    #[serde(rename = "i8g.4xlarge")]
    I8g_4xlarge,
    #[serde(rename = "i8g.8xlarge")]
    I8g_8xlarge,
    #[serde(rename = "i8g.12xlarge")]
    I8g_12xlarge,
    #[serde(rename = "i8g.16xlarge")]
    I8g_16xlarge,
    #[serde(rename = "i8g.24xlarge")]
    I8g_24xlarge,
    #[serde(rename = "i8g.metal-24xl")]
    I8g_metal_24xl,
    #[serde(rename = "i8g.48xlarge")]
    I8g_48xlarge,
    #[serde(rename = "i8ge.large")]
    I8ge_large,
    #[serde(rename = "i8ge.xlarge")]
    I8ge_xlarge,
    #[serde(rename = "i8ge.2xlarge")]
    I8ge_2xlarge,
    #[serde(rename = "i8ge.3xlarge")]
    I8ge_3xlarge,
    #[serde(rename = "i8ge.6xlarge")]
    I8ge_6xlarge,
    #[serde(rename = "i8ge.12xlarge")]
    I8ge_12xlarge,
    #[serde(rename = "i8ge.18xlarge")]
    I8ge_18xlarge,
    #[serde(rename = "i8ge.24xlarge")]
    I8ge_24xlarge,
    #[serde(rename = "i8ge.metal-24xl")]
    I8ge_metal_24xl,
    #[serde(rename = "i8ge.48xlarge")]
    I8ge_48xlarge,
    #[serde(rename = "i8ge.metal-48xl")]
    I8ge_metal_48xl,
    #[serde(rename = "m6a.large")]
    M6a_large,
    #[serde(rename = "m6a.xlarge")]
    M6a_xlarge,
    #[serde(rename = "m6a.2xlarge")]
    M6a_2xlarge,
    #[serde(rename = "m6a.4xlarge")]
    M6a_4xlarge,
    #[serde(rename = "m6a.8xlarge")]
    M6a_8xlarge,
    #[serde(rename = "m6a.12xlarge")]
    M6a_12xlarge,
    #[serde(rename = "m6a.16xlarge")]
    M6a_16xlarge,
    #[serde(rename = "m6a.24xlarge")]
    M6a_24xlarge,
    #[serde(rename = "m6a.32xlarge")]
    M6a_32xlarge,
    #[serde(rename = "m6a.48xlarge")]
    M6a_48xlarge,
    #[serde(rename = "m6a.metal")]
    M6a_metal,
    #[serde(rename = "m6gd.medium")]
    M6gd_medium,
    #[serde(rename = "m6gd.large")]
    M6gd_large,
    #[serde(rename = "m6gd.xlarge")]
    M6gd_xlarge,
    #[serde(rename = "m6gd.2xlarge")]
    M6gd_2xlarge,
    #[serde(rename = "m6gd.4xlarge")]
    M6gd_4xlarge,
    #[serde(rename = "m6gd.8xlarge")]
    M6gd_8xlarge,
    #[serde(rename = "m6gd.12xlarge")]
    M6gd_12xlarge,
    #[serde(rename = "m6gd.16xlarge")]
    M6gd_16xlarge,
    #[serde(rename = "m6gd.metal")]
    M6gd_metal,
    #[serde(rename = "m6id.large")]
    M6id_large,
    #[serde(rename = "m6id.xlarge")]
    M6id_xlarge,
    #[serde(rename = "m6id.2xlarge")]
    M6id_2xlarge,
    #[serde(rename = "m6id.4xlarge")]
    M6id_4xlarge,
    #[serde(rename = "m6id.8xlarge")]
    M6id_8xlarge,
    #[serde(rename = "m6id.12xlarge")]
    M6id_12xlarge,
    #[serde(rename = "m6id.16xlarge")]
    M6id_16xlarge,
    #[serde(rename = "m6id.24xlarge")]
    M6id_24xlarge,
    #[serde(rename = "m6id.32xlarge")]
    M6id_32xlarge,
    #[serde(rename = "m6id.metal")]
    M6id_metal,
    #[serde(rename = "m7a.medium")]
    M7a_medium,
    #[serde(rename = "m7a.large")]
    M7a_large,
    #[serde(rename = "m7a.xlarge")]
    M7a_xlarge,
    #[serde(rename = "m7a.2xlarge")]
    M7a_2xlarge,
    #[serde(rename = "m7a.4xlarge")]
    M7a_4xlarge,
    #[serde(rename = "m7a.8xlarge")]
    M7a_8xlarge,
    #[serde(rename = "m7a.12xlarge")]
    M7a_12xlarge,
    #[serde(rename = "m7a.16xlarge")]
    M7a_16xlarge,
    #[serde(rename = "m7a.24xlarge")]
    M7a_24xlarge,
    #[serde(rename = "m7a.32xlarge")]
    M7a_32xlarge,
    #[serde(rename = "m7a.48xlarge")]
    M7a_48xlarge,
    #[serde(rename = "m7a.metal-48xl")]
    M7a_metal_48xl,
    #[serde(rename = "m7i.large")]
    M7i_large,
    #[serde(rename = "m7i.xlarge")]
    M7i_xlarge,
    #[serde(rename = "m7i.2xlarge")]
    M7i_2xlarge,
    #[serde(rename = "m7i.4xlarge")]
    M7i_4xlarge,
    #[serde(rename = "m7i.8xlarge")]
    M7i_8xlarge,
    #[serde(rename = "m7i.12xlarge")]
    M7i_12xlarge,
    #[serde(rename = "m7i.16xlarge")]
    M7i_16xlarge,
    #[serde(rename = "m7i.24xlarge")]
    M7i_24xlarge,
    #[serde(rename = "m7i.metal-24xl")]
    M7i_metal_24xl,
    #[serde(rename = "m7i.48xlarge")]
    M7i_48xlarge,
    #[serde(rename = "m7i.metal-48xl")]
    M7i_metal_48xl,
    #[serde(rename = "m8gd.medium")]
    M8gd_medium,
    #[serde(rename = "m8gd.large")]
    M8gd_large,
    #[serde(rename = "m8gd.xlarge")]
    M8gd_xlarge,
    #[serde(rename = "m8gd.2xlarge")]
    M8gd_2xlarge,
    #[serde(rename = "m8gd.4xlarge")]
    M8gd_4xlarge,
    #[serde(rename = "m8gd.8xlarge")]
    M8gd_8xlarge,
    #[serde(rename = "m8gd.12xlarge")]
    M8gd_12xlarge,
    #[serde(rename = "m8gd.16xlarge")]
    M8gd_16xlarge,
    #[serde(rename = "m8gd.24xlarge")]
    M8gd_24xlarge,
    #[serde(rename = "m8gd.metal-24xl")]
    M8gd_metal_24xl,
    #[serde(rename = "m8gd.48xlarge")]
    M8gd_48xlarge,
    #[serde(rename = "m8gd.metal-48xl")]
    M8gd_metal_48xl,
    #[serde(rename = "r6gd.medium")]
    R6gd_medium,
    #[serde(rename = "r6gd.large")]
    R6gd_large,
    #[serde(rename = "r6gd.xlarge")]
    R6gd_xlarge,
    #[serde(rename = "r6gd.2xlarge")]
    R6gd_2xlarge,
    #[serde(rename = "r6gd.4xlarge")]
    R6gd_4xlarge,
    #[serde(rename = "r6gd.8xlarge")]
    R6gd_8xlarge,
    #[serde(rename = "r6gd.12xlarge")]
    R6gd_12xlarge,
    #[serde(rename = "r6gd.16xlarge")]
    R6gd_16xlarge,
    #[serde(rename = "r6gd.metal")]
    R6gd_metal,
    #[serde(rename = "r6id.large")]
    R6id_large,
    #[serde(rename = "r6id.xlarge")]
    R6id_xlarge,
    #[serde(rename = "r6id.2xlarge")]
    R6id_2xlarge,
    #[serde(rename = "r6id.4xlarge")]
    R6id_4xlarge,
    #[serde(rename = "r6id.8xlarge")]
    R6id_8xlarge,
    #[serde(rename = "r6id.12xlarge")]
    R6id_12xlarge,
    #[serde(rename = "r6id.16xlarge")]
    R6id_16xlarge,
    #[serde(rename = "r6id.24xlarge")]
    R6id_24xlarge,
    #[serde(rename = "r6id.32xlarge")]
    R6id_32xlarge,
    #[serde(rename = "r6id.metal")]
    R6id_metal,
    #[serde(rename = "r8gd.medium")]
    R8gd_medium,
    #[serde(rename = "r8gd.large")]
    R8gd_large,
    #[serde(rename = "r8gd.xlarge")]
    R8gd_xlarge,
    #[serde(rename = "r8gd.2xlarge")]
    R8gd_2xlarge,
    #[serde(rename = "r8gd.4xlarge")]
    R8gd_4xlarge,
    #[serde(rename = "r8gd.8xlarge")]
    R8gd_8xlarge,
    #[serde(rename = "r8gd.12xlarge")]
    R8gd_12xlarge,
    #[serde(rename = "r8gd.16xlarge")]
    R8gd_16xlarge,
    #[serde(rename = "r8gd.24xlarge")]
    R8gd_24xlarge,
    #[serde(rename = "r8gd.metal-24xl")]
    R8gd_metal_24xl,
    #[serde(rename = "r8gd.48xlarge")]
    R8gd_48xlarge,
    #[serde(rename = "r8gd.metal-48xl")]
    R8gd_metal_48xl,
}

/// `pgStateProperty` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgStateProperty {
    #[serde(rename = "creating")]
    #[default]
    Creating,
    #[serde(rename = "restarting")]
    Restarting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "replaying_wal")]
    Replaying_wal,
    #[serde(rename = "restoring_backup")]
    Restoring_backup,
    #[serde(rename = "finalizing_restore")]
    Finalizing_restore,
    #[serde(rename = "unavailable")]
    Unavailable,
    #[serde(rename = "deleting")]
    Deleting,
}

/// `pgVersion` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgVersion {
    #[serde(rename = "18")]
    #[default]
    _18,
    #[serde(rename = "17")]
    _17,
    #[serde(rename = "16")]
    _16,
}

/// Inline enum for `Activity.actorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityActortype {
    #[serde(rename = "user")]
    #[default]
    User,
    #[serde(rename = "support")]
    Support,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "api")]
    Api,
}

/// Inline enum for `Activity.keyUpdateType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityKeyupdatetype {
    #[serde(rename = "created")]
    #[default]
    Created,
    #[serde(rename = "deleted")]
    Deleted,
    #[serde(rename = "name-changed")]
    Name_changed,
    #[serde(rename = "role-changed")]
    Role_changed,
    #[serde(rename = "state-changed")]
    State_changed,
    #[serde(rename = "date-changed")]
    Date_changed,
    #[serde(rename = "ip-access-list-changed")]
    Ip_access_list_changed,
    #[serde(rename = "org-role-changed")]
    Org_role_changed,
    #[serde(rename = "default-service-role-changed")]
    Default_service_role_changed,
    #[serde(rename = "service-role-changed")]
    Service_role_changed,
    #[serde(rename = "roles-v2-changed")]
    Roles_v2_changed,
}

/// Inline enum for `Activity.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityType {
    #[serde(rename = "create_organization")]
    #[default]
    Create_organization,
    #[serde(rename = "organization_update_name")]
    Organization_update_name,
    #[serde(rename = "transfer_service_in")]
    Transfer_service_in,
    #[serde(rename = "transfer_service_out")]
    Transfer_service_out,
    #[serde(rename = "save_payment_method")]
    Save_payment_method,
    #[serde(rename = "marketplace_subscription")]
    Marketplace_subscription,
    #[serde(rename = "migrate_marketplace_billing_details_in")]
    Migrate_marketplace_billing_details_in,
    #[serde(rename = "migrate_marketplace_billing_details_out")]
    Migrate_marketplace_billing_details_out,
    #[serde(rename = "organization_update_tier")]
    Organization_update_tier,
    #[serde(rename = "organization_invite_create")]
    Organization_invite_create,
    #[serde(rename = "organization_invite_delete")]
    Organization_invite_delete,
    #[serde(rename = "organization_member_join")]
    Organization_member_join,
    #[serde(rename = "organization_member_add")]
    Organization_member_add,
    #[serde(rename = "organization_member_leave")]
    Organization_member_leave,
    #[serde(rename = "organization_member_delete")]
    Organization_member_delete,
    #[serde(rename = "organization_member_update_role")]
    Organization_member_update_role,
    #[serde(rename = "organization_member_update_mfa_method")]
    Organization_member_update_mfa_method,
    #[serde(rename = "user_login")]
    User_login,
    #[serde(rename = "user_login_failed")]
    User_login_failed,
    #[serde(rename = "user_logout")]
    User_logout,
    #[serde(rename = "key_create")]
    Key_create,
    #[serde(rename = "key_delete")]
    Key_delete,
    #[serde(rename = "openapi_key_update")]
    Openapi_key_update,
    #[serde(rename = "service_create")]
    Service_create,
    #[serde(rename = "service_start")]
    Service_start,
    #[serde(rename = "service_stop")]
    Service_stop,
    #[serde(rename = "service_awaken")]
    Service_awaken,
    #[serde(rename = "service_idle")]
    Service_idle,
    #[serde(rename = "service_running")]
    Service_running,
    #[serde(rename = "service_partially_running")]
    Service_partially_running,
    #[serde(rename = "service_delete")]
    Service_delete,
    #[serde(rename = "service_update_name")]
    Service_update_name,
    #[serde(rename = "service_update_ip_access_list")]
    Service_update_ip_access_list,
    #[serde(rename = "service_update_autoscaling_memory")]
    Service_update_autoscaling_memory,
    #[serde(rename = "service_update_autoscaling_idling")]
    Service_update_autoscaling_idling,
    #[serde(rename = "service_update_password")]
    Service_update_password,
    #[serde(rename = "service_update_autoscaling_replicas")]
    Service_update_autoscaling_replicas,
    #[serde(rename = "service_update_max_allowable_replicas")]
    Service_update_max_allowable_replicas,
    #[serde(rename = "service_update_backup_configuration")]
    Service_update_backup_configuration,
    #[serde(rename = "service_restore_backup")]
    Service_restore_backup,
    #[serde(rename = "service_update_release_channel")]
    Service_update_release_channel,
    #[serde(rename = "service_update_gpt_usage_consent")]
    Service_update_gpt_usage_consent,
    #[serde(rename = "service_update_private_endpoints")]
    Service_update_private_endpoints,
    #[serde(rename = "service_import_to_organization")]
    Service_import_to_organization,
    #[serde(rename = "service_export_from_organization")]
    Service_export_from_organization,
    #[serde(rename = "service_maintenance_start")]
    Service_maintenance_start,
    #[serde(rename = "service_maintenance_end")]
    Service_maintenance_end,
    #[serde(rename = "service_update_core_dump")]
    Service_update_core_dump,
    #[serde(rename = "backup_delete")]
    Backup_delete,
}

/// Inline enum for `ApiKey.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

/// Inline enum for `ApiKeyPatchRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPatchRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

/// Inline enum for `ApiKeyPostRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPostRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

/// Inline enum for `AssignedRole.roleType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AssignedRoleRoletype {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
}

/// Inline enum for `AwsBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketBucketprovider {
    #[default]
    AWS,
}

/// Inline enum for `AwsBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AWS,
}

/// Inline enum for `AwsBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AWS,
}

/// Inline enum for `AwsBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPropertiesBucketprovider {
    #[default]
    AWS,
}

/// Inline enum for `AzureBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketBucketprovider {
    #[default]
    AZURE,
}

/// Inline enum for `AzureBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AZURE,
}

/// Inline enum for `AzureBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AZURE,
}

/// Inline enum for `AzureBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPropertiesBucketprovider {
    #[default]
    AZURE,
}

/// Inline enum for `Backup.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupStatus {
    #[serde(rename = "done")]
    #[default]
    Done,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "in_progress")]
    In_progress,
}

/// Inline enum for `Backup.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupType {
    #[serde(rename = "full")]
    #[default]
    Full,
    #[serde(rename = "incremental")]
    Incremental,
}

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
}

/// Inline enum for `ClickPipe.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeState {
    #[default]
    Unknown,
    Provisioning,
    Running,
    Stopping,
    Stopped,
    Failed,
    Completed,
    InternalError,
    Setup,
    Snapshot,
    Paused,
    Pausing,
    Modifying,
    Resync,
}

/// Inline enum for `ClickPipeBigQueryPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeSettingsReplicationmode {
    #[serde(rename = "snapshot")]
    #[default]
    Snapshot,
}

/// Inline enum for `ClickPipeBigQueryPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipeDestinationTableEngine.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeDestinationTableEngineType {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    SummingMergeTree,
    Null,
}

/// Inline enum for `ClickPipeKafkaOffset.strategy`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaOffsetStrategy {
    #[serde(rename = "from_beginning")]
    #[default]
    From_beginning,
    #[serde(rename = "from_latest")]
    From_latest,
    #[serde(rename = "from_timestamp")]
    From_timestamp,
}

/// Inline enum for `ClickPipeKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
}

/// Inline enum for `ClickPipeKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
}

/// Inline enum for `ClickPipeKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
}

/// Inline enum for `ClickPipeKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
}

/// Inline enum for `ClickPipeKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
}

/// Inline enum for `ClickPipeKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
}

/// Inline enum for `ClickPipeKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
}

/// Inline enum for `ClickPipeMongoDBPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
}

/// Inline enum for `ClickPipeMongoDBPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipeMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
}

/// Inline enum for `ClickPipeMutateKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
}

/// Inline enum for `ClickPipeMutateMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
}

/// Inline enum for `ClickPipeMutateMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
}

/// Inline enum for `ClickPipeMutateMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
}

/// Inline enum for `ClickPipeMutatePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
}

/// Inline enum for `ClickPipeMutatePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMechanism`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmechanism {
    #[default]
    GTID,
    FILE_POS,
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
}

/// Inline enum for `ClickPipeMySQLPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipeMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
}

/// Inline enum for `ClickPipeMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
}

/// Inline enum for `ClickPipeObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
}

/// Inline enum for `ClickPipeObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
}

/// Inline enum for `ClickPipeObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
}

/// Inline enum for `ClickPipeObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
}

/// Inline enum for `ClickPipePatchKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
}

/// Inline enum for `ClickPipePatchKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
}

/// Inline enum for `ClickPipePatchMongoDBPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipePatchMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
}

/// Inline enum for `ClickPipePatchMySQLPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipePatchMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
}

/// Inline enum for `ClickPipePatchObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
}

/// Inline enum for `ClickPipePatchPostgresPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipePostKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
}

/// Inline enum for `ClickPipePostKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
}

/// Inline enum for `ClickPipePostKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
}

/// Inline enum for `ClickPipePostKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
}

/// Inline enum for `ClickPipePostKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
}

/// Inline enum for `ClickPipePostKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
}

/// Inline enum for `ClickPipePostObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
}

/// Inline enum for `ClickPipePostObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
}

/// Inline enum for `ClickPipePostObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
}

/// Inline enum for `ClickPipePostObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
}

/// Inline enum for `ClickPipePostgresPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
}

/// Inline enum for `ClickPipePostgresPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
}

/// Inline enum for `ClickPipePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
}

/// Inline enum for `ClickPipePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
}

/// Inline enum for `ClickPipeStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "resync")]
    Resync,
}

/// Inline enum for `ClickStackAlertChannelEmail.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelEmailType {
    #[serde(rename = "webhook")]
    #[default]
    Webhook,
    #[serde(rename = "email")]
    Email,
}

/// Inline enum for `ClickStackAlertChannelWebhook.severity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookSeverity {
    #[serde(rename = "critical")]
    #[default]
    Critical,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
}

/// Inline enum for `ClickStackAlertChannelWebhook.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookType {
    #[serde(rename = "webhook")]
    #[default]
    Webhook,
    #[serde(rename = "email")]
    Email,
}

/// Inline enum for `ClickStackAlertResponse.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
}

/// Inline enum for `ClickStackAlertResponse.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
}

/// Inline enum for `ClickStackAlertResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseState {
    #[default]
    ALERT,
    OK,
    INSUFFICIENT_DATA,
    DISABLED,
}

/// Inline enum for `ClickStackAlertResponse.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
}

/// Inline enum for `ClickStackBarBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarBuilderChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
}

/// Inline enum for `ClickStackCreateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
}

/// Inline enum for `ClickStackCreateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
}

/// Inline enum for `ClickStackCreateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
}

/// Inline enum for `ClickStackCreateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackDashboardResponse.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackDashboardResponseSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackFilter.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
}

/// Inline enum for `ClickStackFilter.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterType {
    #[default]
    QUERY_EXPRESSION,
}

/// Inline enum for `ClickStackFilterInput.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
}

/// Inline enum for `ClickStackFilterInput.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputType {
    #[default]
    QUERY_EXPRESSION,
}

/// Inline enum for `ClickStackGenericWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackGenericWebhookService {
    #[serde(rename = "generic")]
    #[default]
    Generic,
}

/// Inline enum for `ClickStackIncidentIOWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackIncidentIOWebhookService {
    #[serde(rename = "incidentio")]
    #[default]
    Incidentio,
}

/// Inline enum for `ClickStackLineBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineBuilderChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
}

/// Inline enum for `ClickStackLogSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLogSourceKind {
    #[serde(rename = "log")]
    #[default]
    Log,
}

/// Inline enum for `ClickStackMarkdownChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartConfigDisplaytype {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
}

/// Inline enum for `ClickStackMarkdownChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartSeriesType {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
}

/// Inline enum for `ClickStackMaterializedView.minGranularity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMaterializedViewMingranularity {
    #[serde(rename = "1s")]
    #[default]
    _1s,
    #[serde(rename = "15s")]
    _15s,
    #[serde(rename = "30s")]
    _30s,
    #[serde(rename = "1m")]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "2h")]
    _2h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    #[serde(rename = "2d")]
    _2d,
    #[serde(rename = "7d")]
    _7d,
    #[serde(rename = "30d")]
    _30d,
}

/// Inline enum for `ClickStackMetricSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMetricSourceKind {
    #[serde(rename = "metric")]
    #[default]
    Metric,
}

/// Inline enum for `ClickStackNumberBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberBuilderChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
}

/// Inline enum for `ClickStackNumberChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
}

/// Inline enum for `ClickStackNumberChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
}

/// Inline enum for `ClickStackNumberChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesType {
    #[serde(rename = "number")]
    #[default]
    Number,
}

/// Inline enum for `ClickStackNumberChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackNumberFormat.output`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberFormatOutput {
    #[serde(rename = "currency")]
    #[default]
    Currency,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "byte")]
    Byte,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "number")]
    Number,
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
}

/// Inline enum for `ClickStackPagerDutyAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPagerDutyAPIWebhookService {
    #[serde(rename = "pagerduty_api")]
    #[default]
    Pagerduty_api,
}

/// Inline enum for `ClickStackPieBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieBuilderChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
}

/// Inline enum for `ClickStackSavedFilterValue.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedFilterValueType {
    #[serde(rename = "sql")]
    #[default]
    Sql,
}

/// Inline enum for `ClickStackSearchChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigDisplaytype {
    #[serde(rename = "search")]
    #[default]
    Search,
}

/// Inline enum for `ClickStackSearchChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackSearchChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesType {
    #[serde(rename = "search")]
    #[default]
    Search,
}

/// Inline enum for `ClickStackSearchChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackSelectItem.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
}

/// Inline enum for `ClickStackSelectItem.level`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemLevel {
    #[serde(rename = "0.5")]
    #[default]
    _0_5,
    #[serde(rename = "0.9")]
    _0_9,
    #[serde(rename = "0.95")]
    _0_95,
    #[serde(rename = "0.99")]
    _0_99,
}

/// Inline enum for `ClickStackSelectItem.metricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemMetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
}

/// Inline enum for `ClickStackSelectItem.periodAggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemPeriodaggfn {
    #[serde(rename = "delta")]
    #[default]
    Delta,
}

/// Inline enum for `ClickStackSelectItem.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackSessionSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSessionSourceKind {
    #[serde(rename = "session")]
    #[default]
    Session,
}

/// Inline enum for `ClickStackSlackAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackAPIWebhookService {
    #[serde(rename = "slack_api")]
    #[default]
    Slack_api,
}

/// Inline enum for `ClickStackSlackWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackWebhookService {
    #[serde(rename = "slack")]
    #[default]
    Slack,
}

/// Inline enum for `ClickStackTableBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableBuilderChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
}

/// Inline enum for `ClickStackTableChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
}

/// Inline enum for `ClickStackTableChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
}

/// Inline enum for `ClickStackTableChartSeries.sortOrder`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesSortorder {
    #[serde(rename = "desc")]
    #[default]
    Desc,
    #[serde(rename = "asc")]
    Asc,
}

/// Inline enum for `ClickStackTableChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesType {
    #[serde(rename = "table")]
    #[default]
    Table,
}

/// Inline enum for `ClickStackTableChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
}

/// Inline enum for `ClickStackTimeChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
}

/// Inline enum for `ClickStackTimeChartSeries.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    #[serde(rename = "line")]
    Line,
}

/// Inline enum for `ClickStackTimeChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
}

/// Inline enum for `ClickStackTimeChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesType {
    #[serde(rename = "time")]
    #[default]
    Time,
}

/// Inline enum for `ClickStackTimeChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `ClickStackTraceSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTraceSourceKind {
    #[serde(rename = "trace")]
    #[default]
    Trace,
}

/// Inline enum for `ClickStackUpdateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
}

/// Inline enum for `ClickStackUpdateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
}

/// Inline enum for `ClickStackUpdateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
}

/// Inline enum for `ClickStackUpdateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
}

/// Inline enum for `CreateReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
}

/// Inline enum for `CreateReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
}

/// Inline enum for `GcpBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketBucketprovider {
    #[default]
    GCP,
}

/// Inline enum for `GcpBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    GCP,
}

/// Inline enum for `GcpBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPostRequestV1Bucketprovider {
    #[default]
    GCP,
}

/// Inline enum for `GcpBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPropertiesBucketprovider {
    #[default]
    GCP,
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
}

/// Inline enum for `Invitation.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
}

/// Inline enum for `InvitationPostRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationPostRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
}

/// Inline enum for `Member.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
}

/// Inline enum for `MemberPatchRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberPatchRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
}

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
}

/// Inline enum for `PostgresServiceSetState.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PostgresServiceSetStateCommand {
    #[serde(rename = "restart")]
    #[default]
    Restart,
    #[serde(rename = "promote")]
    Promote,
    #[serde(rename = "switchover")]
    Switchover,
}

/// Inline enum for `RBACPolicy.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyAllowdeny {
    #[default]
    ALLOW,
    DENY,
}

/// Inline enum for `RBACPolicyCreateRequest.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyCreateRequestAllowdeny {
    #[default]
    ALLOW,
    DENY,
}

/// Inline enum for `RBACPolicyTags.roleV2`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyTagsRolev2 {
    #[serde(rename = "sql-console-readonly")]
    #[default]
    Sql_console_readonly,
    #[serde(rename = "sql-console-admin")]
    Sql_console_admin,
}

/// Inline enum for `RBACRole.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACRoleType {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
}

/// Inline enum for `ReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
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
}

/// Inline enum for `ReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
}

/// Inline enum for `ScimPatchOperation.op`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ScimPatchOperationOp {
    #[serde(rename = "add")]
    #[default]
    Add,
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "remove")]
    Remove,
}

/// Inline enum for `Service.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
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
}

/// Inline enum for `ServiceEndpointChange.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointChangeProtocol {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
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
}

/// Inline enum for `ServicePostRequest.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
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
}

/// Inline enum for `ServiceScalingPatchResponse.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
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
}

/// Inline enum for `pgConfig.default_transaction_isolation`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigDefaultTransactionIsolation {
    #[serde(rename = "read committed")]
    #[default]
    Read_committed,
    #[serde(rename = "repeatable read")]
    Repeatable_read,
    #[serde(rename = "serializable")]
    Serializable,
}

/// Inline enum for `pgConfig.wal_compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigWalCompression {
    #[serde(rename = "off")]
    #[default]
    Off,
    #[serde(rename = "on")]
    On,
    #[serde(rename = "lz4")]
    Lz4,
    #[serde(rename = "zstd")]
    Zstd,
}

/// `BackupBucket` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BackupBucket {
    AwsBackupBucket(AwsBackupBucket),
    GcpBackupBucket(GcpBackupBucket),
    AzureBackupBucket(AzureBackupBucket),
}

/// `BackupBucketPatchRequest` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BackupBucketPatchRequest {
    AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1),
    GcpBackupBucketPatchRequestV1(GcpBackupBucketPatchRequestV1),
    AzureBackupBucketPatchRequestV1(AzureBackupBucketPatchRequestV1),
}

/// `BackupBucketPostRequest` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BackupBucketPostRequest {
    AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1),
    GcpBackupBucketPostRequestV1(GcpBackupBucketPostRequestV1),
    AzureBackupBucketPostRequestV1(AzureBackupBucketPostRequestV1),
}

/// `BackupBucketProperties` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BackupBucketProperties {
    AwsBackupBucketProperties(AwsBackupBucketProperties),
    GcpBackupBucketProperties(GcpBackupBucketProperties),
    AzureBackupBucketProperties(AzureBackupBucketProperties),
}

/// `ClickStackAlertChannel` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackAlertChannel {
    ClickStackAlertChannelEmail(ClickStackAlertChannelEmail),
    ClickStackAlertChannelWebhook(ClickStackAlertChannelWebhook),
}

/// `ClickStackBarChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackBarChartConfig {
    ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig),
    ClickStackBarRawSqlChartConfig(ClickStackBarRawSqlChartConfig),
}

/// `ClickStackDashboardChartSeries` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackDashboardChartSeries {
    ClickStackTimeChartSeries(ClickStackTimeChartSeries),
    ClickStackTableChartSeries(ClickStackTableChartSeries),
    ClickStackNumberChartSeries(ClickStackNumberChartSeries),
    ClickStackSearchChartSeries(ClickStackSearchChartSeries),
    ClickStackMarkdownChartSeries(ClickStackMarkdownChartSeries),
}

/// `ClickStackLineChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackLineChartConfig {
    ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig),
    ClickStackLineRawSqlChartConfig(ClickStackLineRawSqlChartConfig),
}

/// `ClickStackNumberChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackNumberChartConfig {
    ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig),
    ClickStackNumberRawSqlChartConfig(ClickStackNumberRawSqlChartConfig),
}

/// `ClickStackPieChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackPieChartConfig {
    ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig),
    ClickStackPieRawSqlChartConfig(ClickStackPieRawSqlChartConfig),
}

/// `ClickStackSource` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackSource {
    ClickStackLogSource(ClickStackLogSource),
    ClickStackTraceSource(ClickStackTraceSource),
    ClickStackMetricSource(ClickStackMetricSource),
    ClickStackSessionSource(ClickStackSessionSource),
}

/// `ClickStackTableChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackTableChartConfig {
    ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig),
    ClickStackTableRawSqlChartConfig(ClickStackTableRawSqlChartConfig),
}

/// `ClickStackTileConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackTileConfig {
    ClickStackLineChartConfig(ClickStackLineChartConfig),
    ClickStackBarChartConfig(ClickStackBarChartConfig),
    ClickStackTableChartConfig(ClickStackTableChartConfig),
    ClickStackNumberChartConfig(ClickStackNumberChartConfig),
    ClickStackPieChartConfig(ClickStackPieChartConfig),
    ClickStackSearchChartConfig(ClickStackSearchChartConfig),
    ClickStackMarkdownChartConfig(ClickStackMarkdownChartConfig),
}

/// `ClickStackWebhook` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackWebhook {
    ClickStackSlackWebhook(ClickStackSlackWebhook),
    ClickStackIncidentIOWebhook(ClickStackIncidentIOWebhook),
    ClickStackGenericWebhook(ClickStackGenericWebhook),
    ClickStackSlackAPIWebhook(ClickStackSlackAPIWebhook),
    ClickStackPagerDutyAPIWebhook(ClickStackPagerDutyAPIWebhook),
}

/// Type alias for `pgCreatedAtProperty`.
pub type PgCreatedAtProperty = chrono::DateTime<chrono::Utc>;

/// Type alias for `pgIdProperty`.
pub type PgIdProperty = uuid::Uuid;

/// Type alias for `pgIsPrimaryProperty`.
pub type PgIsPrimaryProperty = bool;

/// Type alias for `pgNameProperty`.
pub type PgNameProperty = String;

/// Type alias for `pgPassword`.
pub type PgPassword = String;

/// Type alias for `pgPitrRestoreTargetProperty`.
pub type PgPitrRestoreTargetProperty = chrono::DateTime<chrono::Utc>;

/// Type alias for `pgRegion`.
pub type PgRegion = String;

/// Type alias for `pgStorageSize`.
pub type PgStorageSize = i64;

/// Type alias for `pgTags`.
pub type PgTags = Vec<ResourceTagsV1>;

/// `Activity` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "actorDetails", skip_serializing_if = "Option::is_none", default)]
    pub actor_details: Option<String>,
    #[serde(rename = "actorId", skip_serializing_if = "Option::is_none", default)]
    pub actor_id: Option<String>,
    #[serde(rename = "actorIpAddress", skip_serializing_if = "Option::is_none", default)]
    pub actor_ip_address: Option<String>,
    #[serde(rename = "actorType", skip_serializing_if = "Option::is_none", default)]
    pub actor_type: Option<ActivityActortype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(rename = "keyUpdateType", skip_serializing_if = "Option::is_none", default)]
    pub key_update_type: Option<ActivityKeyupdatetype>,
    #[serde(rename = "organizationId", skip_serializing_if = "Option::is_none", default)]
    pub organization_id: Option<String>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none", default)]
    pub service_id: Option<String>,
    #[serde(rename = "targetKeyId", skip_serializing_if = "Option::is_none", default)]
    pub target_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ActivityType>,
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none", default)]
    pub user_agent: Option<String>,
}

/// `ApiKey` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKey {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none", default)]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(rename = "keySuffix", skip_serializing_if = "Option::is_none", default)]
    pub key_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ApiKeyState>,
    #[serde(rename = "usedAt", skip_serializing_if = "Option::is_none", default)]
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ApiKeyHashData` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyHashData {
    #[serde(rename = "keyIdHash", skip_serializing_if = "Option::is_none", default)]
    pub key_id_hash: Option<String>,
    #[serde(rename = "keyIdSuffix", skip_serializing_if = "Option::is_none", default)]
    pub key_id_suffix: Option<String>,
    #[serde(rename = "keySecretHash", skip_serializing_if = "Option::is_none", default)]
    pub key_secret_hash: Option<String>,
}

/// `ApiKeyPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none", default)]
    pub assigned_role_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ApiKeyPatchRequestState>,
}

/// `ApiKeyPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none", default)]
    pub assigned_role_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "hashData", skip_serializing_if = "Option::is_none", default)]
    pub hash_data: Option<ApiKeyHashData>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ApiKeyPostRequestState>,
}

/// `ApiKeyPostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub key: Option<ApiKey>,
    #[serde(rename = "keyId", skip_serializing_if = "Option::is_none", default)]
    pub key_id: Option<String>,
    #[serde(rename = "keySecret", skip_serializing_if = "Option::is_none", default)]
    pub key_secret: Option<String>,
}

/// `AssignedRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssignedRole {
    #[serde(rename = "roleId", skip_serializing_if = "Option::is_none", default)]
    pub role_id: Option<uuid::Uuid>,
    #[serde(rename = "roleName", skip_serializing_if = "Option::is_none", default)]
    pub role_name: Option<String>,
    #[serde(rename = "roleType", skip_serializing_if = "Option::is_none", default)]
    pub role_type: Option<AssignedRoleRoletype>,
}

/// `AwsBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucket {
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AwsBackupBucketBucketprovider>,
    #[serde(rename = "iamRoleArn", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_arn: Option<String>,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_session_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
}

/// `AwsBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AwsBackupBucketPatchRequestV1Bucketprovider>,
    #[serde(rename = "iamRoleArn", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_arn: Option<String>,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_session_name: Option<String>,
}

/// `AwsBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPostRequestV1 {
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AwsBackupBucketPostRequestV1Bucketprovider>,
    #[serde(rename = "iamRoleArn", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_arn: Option<String>,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_session_name: Option<String>,
}

/// `AwsBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketProperties {
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AwsBackupBucketPropertiesBucketprovider>,
    #[serde(rename = "iamRoleArn", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_arn: Option<String>,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_session_name: Option<String>,
}

/// `AzureBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucket {
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AzureBackupBucketBucketprovider>,
    #[serde(rename = "containerName", skip_serializing_if = "Option::is_none", default)]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
}

/// `AzureBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AzureBackupBucketPatchRequestV1Bucketprovider>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(rename = "containerName", skip_serializing_if = "Option::is_none", default)]
    pub container_name: Option<String>,
}

/// `AzureBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPostRequestV1 {
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AzureBackupBucketPostRequestV1Bucketprovider>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(rename = "containerName", skip_serializing_if = "Option::is_none", default)]
    pub container_name: Option<String>,
}

/// `AzureBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketProperties {
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<AzureBackupBucketPropertiesBucketprovider>,
    #[serde(rename = "containerName", skip_serializing_if = "Option::is_none", default)]
    pub container_name: Option<String>,
}

/// `AzureEventHub` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureEventHub {
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
}

/// `Backup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Backup {
    #[serde(rename = "backupName", skip_serializing_if = "Option::is_none", default)]
    pub backup_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bucket: Option<serde_json::Value>,
    #[serde(rename = "durationInSeconds", skip_serializing_if = "Option::is_none", default)]
    pub duration_in_seconds: Option<f64>,
    #[serde(rename = "finishedAt", skip_serializing_if = "Option::is_none", default)]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none", default)]
    pub service_id: Option<String>,
    #[serde(rename = "sizeInBytes", skip_serializing_if = "Option::is_none", default)]
    pub size_in_bytes: Option<f64>,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none", default)]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<BackupStatus>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<BackupType>,
}

/// `BackupConfiguration` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfiguration {
    #[serde(rename = "backupPeriodInHours", skip_serializing_if = "Option::is_none", default)]
    pub backup_period_in_hours: Option<f64>,
    #[serde(rename = "backupRetentionPeriodInHours", skip_serializing_if = "Option::is_none", default)]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none", default)]
    pub backup_start_time: Option<String>,
}

/// `BackupConfigurationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfigurationPatchRequest {
    #[serde(rename = "backupPeriodInHours", skip_serializing_if = "Option::is_none", default)]
    pub backup_period_in_hours: Option<f64>,
    #[serde(rename = "backupRetentionPeriodInHours", skip_serializing_if = "Option::is_none", default)]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none", default)]
    pub backup_start_time: Option<String>,
}

/// `BasePostgresService` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BasePostgresService {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<PgNameProperty>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<PgSize>,
    #[serde(rename = "storageSize", skip_serializing_if = "Option::is_none", default)]
    pub storage_size: Option<PgStorageSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `ByocConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConfig {
    #[serde(rename = "accountName", skip_serializing_if = "Option::is_none", default)]
    pub account_name: Option<String>,
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none", default)]
    pub cloud_provider: Option<ByocConfigCloudprovider>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(rename = "regionId", skip_serializing_if = "Option::is_none", default)]
    pub region_id: Option<ByocConfigRegionid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ByocConfigState>,
}

/// `ByocInfrastructurePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePatchRequest {
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
}

/// `ByocInfrastructurePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePostRequest {
    #[serde(rename = "accountId", skip_serializing_if = "Option::is_none", default)]
    pub account_id: Option<String>,
    #[serde(rename = "availabilityZoneSuffixes", skip_serializing_if = "Option::is_none", default)]
    pub availability_zone_suffixes: Option<Vec<String>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    #[serde(rename = "regionId", skip_serializing_if = "Option::is_none", default)]
    pub region_id: Option<ByocInfrastructurePostRequestRegionid>,
    #[serde(rename = "vpcCidrRange", skip_serializing_if = "Option::is_none", default)]
    pub vpc_cidr_range: Option<String>,
}

/// `ClickPipe` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipe {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination: Option<ClickPipeDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none", default)]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaling: Option<ClickPipeScaling>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none", default)]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeSettings>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickPipeSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ClickPipeState>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettings {
    #[serde(rename = "allowNullableColumns", skip_serializing_if = "Option::is_none", default)]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "initialLoadParallelism", skip_serializing_if = "Option::is_none", default)]
    pub initial_load_parallelism: Option<f64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none", default)]
    pub replication_mode: Option<ClickPipeBigQueryPipeSettingsReplicationmode>,
    #[serde(rename = "snapshotNumRowsPerPartition", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_num_rows_per_partition: Option<f64>,
    #[serde(rename = "snapshotNumberOfParallelTables", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_number_of_parallel_tables: Option<f64>,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none", default)]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none", default)]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceDatasetName", skip_serializing_if = "Option::is_none", default)]
    pub source_dataset_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none", default)]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipeBigQueryPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none", default)]
    pub target_table: Option<String>,
    #[serde(rename = "useCustomSortingKey", skip_serializing_if = "Option::is_none", default)]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQuerySource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeBigQueryPipeSettings>,
    #[serde(rename = "snapshotStagingPath", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_staging_path: Option<String>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings: Option<Vec<ClickPipeBigQueryPipeTableMapping>>,
}

/// `ClickPipeDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestination {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub columns: Option<Vec<ClickPipeDestinationColumn>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database: Option<String>,
    #[serde(rename = "managedTable", skip_serializing_if = "Option::is_none", default)]
    pub managed_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub table: Option<String>,
    #[serde(rename = "tableDefinition", skip_serializing_if = "Option::is_none", default)]
    pub table_definition: Option<ClickPipeDestinationTableDefinition>,
}

/// `ClickPipeDestinationColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationColumn {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
}

/// `ClickPipeDestinationTableDefinition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableDefinition {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub engine: Option<ClickPipeDestinationTableEngine>,
    #[serde(rename = "partitionBy", skip_serializing_if = "Option::is_none", default)]
    pub partition_by: Option<String>,
    #[serde(rename = "primaryKey", skip_serializing_if = "Option::is_none", default)]
    pub primary_key: Option<String>,
    #[serde(rename = "sortingKey", skip_serializing_if = "Option::is_none", default)]
    pub sorting_key: Option<Vec<String>>,
}

/// `ClickPipeDestinationTableEngine` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableEngine {
    #[serde(rename = "columnIds", skip_serializing_if = "Option::is_none", default)]
    pub column_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeDestinationTableEngineType>,
    #[serde(rename = "versionColumnId", skip_serializing_if = "Option::is_none", default)]
    pub version_column_id: Option<String>,
}

/// `ClickPipeFieldMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeFieldMapping {
    #[serde(rename = "destinationField", skip_serializing_if = "Option::is_none", default)]
    pub destination_field: Option<String>,
    #[serde(rename = "sourceField", skip_serializing_if = "Option::is_none", default)]
    pub source_field: Option<String>,
}

/// `ClickPipeKafkaOffset` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaOffset {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub strategy: Option<ClickPipeKafkaOffsetStrategy>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<String>,
}

/// `ClickPipeKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeKafkaSchemaRegistryAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickPipeKafkaSchemaRegistryCredentials` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistryCredentials {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
}

/// `ClickPipeKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeKafkaSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub brokers: Option<String>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none", default)]
    pub consumer_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<ClickPipeKafkaSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<ClickPipeKafkaOffset>,
    #[serde(rename = "reversePrivateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub reverse_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none", default)]
    pub schema_registry: Option<ClickPipeKafkaSchemaRegistry>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub topics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeKafkaSourceType>,
}

/// `ClickPipeKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKinesisSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeKinesisSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<ClickPipeKinesisSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType", skip_serializing_if = "Option::is_none", default)]
    pub iterator_type: Option<ClickPipeKinesisSourceIteratortype>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<String>,
    #[serde(rename = "streamName", skip_serializing_if = "Option::is_none", default)]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none", default)]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettings {
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none", default)]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMongoDBPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(rename = "snapshotNumberOfParallelTables", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useJsonNativeFormat", skip_serializing_if = "Option::is_none", default)]
    pub use_json_native_format: Option<bool>,
}

/// `ClickPipeMongoDBPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: String,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
}

/// `ClickPipeMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipeMongoDBSourceReadpreference,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeMongoDBPipeSettings>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings: Option<Vec<ClickPipeMongoDBPipeTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    pub uri: String,
}

/// `ClickPipeMutateBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateBigQuerySource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<ServiceAccount>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeBigQueryPipeSettings>,
    #[serde(rename = "snapshotStagingPath", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_staging_path: Option<String>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings: Option<Vec<ClickPipeBigQueryPipeTableMapping>>,
}

/// `ClickPipeMutateDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateDestination {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub columns: Option<Vec<ClickPipeDestinationColumn>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database: Option<String>,
    #[serde(rename = "managedTable", skip_serializing_if = "Option::is_none", default)]
    pub managed_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub table: Option<String>,
    #[serde(rename = "tableDefinition", skip_serializing_if = "Option::is_none", default)]
    pub table_definition: Option<ClickPipeDestinationTableDefinition>,
}

/// `ClickPipeMutateKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateKafkaSchemaRegistry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeMutateKafkaSchemaRegistryAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<ClickPipeKafkaSchemaRegistryCredentials>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickPipeMutateMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipeMutateMongoDBSourceReadpreference,
    pub settings: ClickPipeMongoDBPipeSettings,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMongoDBPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    pub uri: String,
}

/// `ClickPipeMutateMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeMutateMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    pub host: String,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    pub port: i64,
    pub settings: ClickPipeMySQLPipeSettings,
    #[serde(rename = "skipCertVerification", skip_serializing_if = "Option::is_none", default)]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMySQLPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeMutateMySQLSourceType>,
}

/// `ClickPipeMutatePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutatePostgresSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeMutatePostgresSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePostgresPipeSettings>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings: Option<Vec<ClickPipePostgresPipeTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeMutatePostgresSourceType>,
}

/// `ClickPipeMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeSettings {
    #[serde(rename = "allowNullableColumns", skip_serializing_if = "Option::is_none", default)]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none", default)]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "initialLoadParallelism", skip_serializing_if = "Option::is_none", default)]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMechanism", skip_serializing_if = "Option::is_none", default)]
    pub replication_mechanism: Option<ClickPipeMySQLPipeSettingsReplicationmechanism>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMySQLPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(rename = "snapshotNumberOfParallelTables", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none", default)]
    pub use_compression: Option<bool>,
}

/// `ClickPipeMySQLPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none", default)]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none", default)]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipeMySQLPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey", skip_serializing_if = "Option::is_none", default)]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    pub host: String,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    pub port: i64,
    pub settings: ClickPipeMySQLPipeSettings,
    #[serde(rename = "skipCertVerification", skip_serializing_if = "Option::is_none", default)]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMySQLPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeMySQLSourceType>,
}

/// `ClickPipeObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeObjectStorageSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none", default)]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compression: Option<ClickPipeObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<ClickPipeObjectStorageSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none", default)]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none", default)]
    pub queue_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeObjectStorageSourceType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickPipePatchDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchDestination {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub columns: Option<Vec<ClickPipeDestinationColumn>>,
}

/// `ClickPipePatchKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchKafkaSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<serde_json::Value>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "reversePrivateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub reverse_private_endpoint_ids: Option<Vec<String>>,
}

/// `ClickPipePatchKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchKinesisSourceAuthentication>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
}

/// `ClickPipePatchMongoDBPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeRemoveTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePatchMongoDBPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipePatchMongoDBSourceReadpreference,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePatchMongoDBPipeSettings>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_add: Option<Vec<ClickPipeMongoDBPipeTableMapping>>,
    #[serde(rename = "tableMappingsToRemove", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMongoDBPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    pub uri: Option<String>,
}

/// `ClickPipePatchMySQLPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeRemoveTableMapping {
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePatchMySQLPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none", default)]
    pub use_compression: Option<bool>,
}

/// `ClickPipePatchMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePatchMySQLPipeSettings>,
    #[serde(rename = "skipCertVerification", skip_serializing_if = "Option::is_none", default)]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_add: Option<Vec<ClickPipeMySQLPipeTableMapping>>,
    #[serde(rename = "tableMappingsToRemove", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMySQLPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none", default)]
    pub azure_container_name: Option<String>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none", default)]
    pub service_account_key: Option<String>,
}

/// `ClickPipePatchPostgresPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeRemoveTableMapping {
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none", default)]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none", default)]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePatchPostgresPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none", default)]
    pub target_table: Option<String>,
}

/// `ClickPipePatchPostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchPostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePatchPostgresPipeSettings>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_add: Option<Vec<ClickPipePostgresPipeTableMapping>>,
    #[serde(rename = "tableMappingsToRemove", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchPostgresPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination: Option<ClickPipePatchDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none", default)]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeSettings>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickPipePatchSource>,
}

/// `ClickPipePatchSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kafka: Option<ClickPipePatchKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kinesis: Option<ClickPipePatchKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mongodb: Option<ClickPipePatchMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mysql: Option<ClickPipePatchMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none", default)]
    pub object_storage: Option<ClickPipePatchObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub postgres: Option<ClickPipePatchPostgresSource>,
    #[serde(rename = "validateSamples", skip_serializing_if = "Option::is_none", default)]
    pub validate_samples: Option<bool>,
}

/// `ClickPipePostKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePostKafkaSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub brokers: Option<String>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none", default)]
    pub consumer_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<ClickPipePostKafkaSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<ClickPipeKafkaOffset>,
    #[serde(rename = "reversePrivateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub reverse_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none", default)]
    pub schema_registry: Option<ClickPipeMutateKafkaSchemaRegistry>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub topics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipePostKafkaSourceType>,
}

/// `ClickPipePostKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePostKinesisSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<ClickPipePostKinesisSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType", skip_serializing_if = "Option::is_none", default)]
    pub iterator_type: Option<ClickPipePostKinesisSourceIteratortype>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<String>,
    #[serde(rename = "streamName", skip_serializing_if = "Option::is_none", default)]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none", default)]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipePostObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePostObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none", default)]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compression: Option<ClickPipePostObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<ClickPipePostObjectStorageSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none", default)]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none", default)]
    pub queue_url: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none", default)]
    pub service_account_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipePostObjectStorageSourceType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickPipePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination: Option<ClickPipeMutateDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none", default)]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scaling: Option<ClickPipeScaling>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeSettings>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickPipePostSource>,
}

/// `ClickPipePostSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bigquery: Option<ClickPipeMutateBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kafka: Option<ClickPipePostKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kinesis: Option<ClickPipePostKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mongodb: Option<ClickPipeMutateMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mysql: Option<ClickPipeMutateMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none", default)]
    pub object_storage: Option<ClickPipePostObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub postgres: Option<ClickPipeMutatePostgresSource>,
    #[serde(rename = "validateSamples", skip_serializing_if = "Option::is_none", default)]
    pub validate_samples: Option<bool>,
}

/// `ClickPipePostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeSettings {
    #[serde(rename = "allowNullableColumns", skip_serializing_if = "Option::is_none", default)]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none", default)]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "enableFailoverSlots", skip_serializing_if = "Option::is_none", default)]
    pub enable_failover_slots: Option<bool>,
    #[serde(rename = "initialLoadParallelism", skip_serializing_if = "Option::is_none", default)]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "publicationName", skip_serializing_if = "Option::is_none", default)]
    pub publication_name: Option<String>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none", default)]
    pub replication_mode: Option<ClickPipePostgresPipeSettingsReplicationmode>,
    #[serde(rename = "replicationSlotName", skip_serializing_if = "Option::is_none", default)]
    pub replication_slot_name: Option<String>,
    #[serde(rename = "snapshotNumRowsPerPartition", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(rename = "snapshotNumberOfParallelTables", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePostgresPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none", default)]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none", default)]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none", default)]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none", default)]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePostgresPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none", default)]
    pub target_table: Option<String>,
    #[serde(rename = "useCustomSortingKey", skip_serializing_if = "Option::is_none", default)]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePostgresSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePostgresPipeSettings>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings: Option<Vec<ClickPipePostgresPipeTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipePostgresSourceType>,
}

/// `ClickPipeScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScaling {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub concurrency: Option<i64>,
    #[serde(rename = "replicaCpuMillicores", skip_serializing_if = "Option::is_none", default)]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub replicas: Option<i64>,
}

/// `ClickPipeScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScalingPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub concurrency: Option<i64>,
    #[serde(rename = "replicaCpuMillicores", skip_serializing_if = "Option::is_none", default)]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub replicas: Option<i64>,
}

/// `ClickPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettings {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSettingsPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettingsPutRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bigquery: Option<ClickPipeBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kafka: Option<ClickPipeKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kinesis: Option<ClickPipeKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mongodb: Option<ClickPipeMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mysql: Option<ClickPipeMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none", default)]
    pub object_storage: Option<ClickPipeObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub postgres: Option<ClickPipePostgresSource>,
}

/// `ClickPipeStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command: Option<ClickPipeStatePatchRequestCommand>,
}

/// `ClickPipesCdcScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScaling {
    #[serde(rename = "replicaCpuMillicores", skip_serializing_if = "Option::is_none", default)]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub replica_memory_gb: Option<f64>,
}

/// `ClickPipesCdcScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScalingPatchRequest {
    #[serde(rename = "replicaCpuMillicores", skip_serializing_if = "Option::is_none", default)]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub replica_memory_gb: Option<f64>,
}

/// `ClickStackAggregatedColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAggregatedColumn {
    #[serde(rename = "aggFn")]
    pub agg_fn: String,
    #[serde(rename = "mvColumn")]
    pub mv_column: String,
    #[serde(rename = "sourceColumn", skip_serializing_if = "Option::is_none", default)]
    pub source_column: Option<String>,
}

/// `ClickStackAlertChannelEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelEmail {
    #[serde(rename = "emailRecipients")]
    pub email_recipients: Vec<String>,
    pub r#type: ClickStackAlertChannelEmailType,
}

/// `ClickStackAlertChannelWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelWebhook {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub severity: Option<ClickStackAlertChannelWebhookSeverity>,
    #[serde(rename = "slackChannelId", skip_serializing_if = "Option::is_none", default)]
    pub slack_channel_id: Option<String>,
    pub r#type: ClickStackAlertChannelWebhookType,
    #[serde(rename = "webhookId")]
    pub webhook_id: String,
    #[serde(rename = "webhookService", skip_serializing_if = "Option::is_none", default)]
    pub webhook_service: Option<String>,
}

/// `ClickStackAlertResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub channel: Option<ClickStackAlertChannel>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none", default)]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub interval: Option<ClickStackAlertResponseInterval>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none", default)]
    pub saved_search_id: Option<String>,
    #[serde(rename = "scheduleOffsetMinutes", skip_serializing_if = "Option::is_none", default)]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none", default)]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub silenced: Option<ClickStackAlertSilenced>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickStackAlertResponseSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ClickStackAlertResponseState>,
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none", default)]
    pub team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub threshold: Option<f64>,
    #[serde(rename = "thresholdType", skip_serializing_if = "Option::is_none", default)]
    pub threshold_type: Option<ClickStackAlertResponseThresholdtype>,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none", default)]
    pub tile_id: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackAlertSilenced` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertSilenced {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackBarBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarBuilderChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackBarRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarRawSqlChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackBarRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackCreateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateAlertRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub channel: Option<ClickStackAlertChannel>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none", default)]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub interval: Option<ClickStackCreateAlertRequestInterval>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none", default)]
    pub saved_search_id: Option<String>,
    #[serde(rename = "scheduleOffsetMinutes", skip_serializing_if = "Option::is_none", default)]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none", default)]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickStackCreateAlertRequestSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub threshold: Option<f64>,
    #[serde(rename = "thresholdType", skip_serializing_if = "Option::is_none", default)]
    pub threshold_type: Option<ClickStackCreateAlertRequestThresholdtype>,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none", default)]
    pub tile_id: Option<String>,
}

/// `ClickStackCreateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filters: Option<Vec<ClickStackFilterInput>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none", default)]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none", default)]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none", default)]
    pub saved_query_language: Option<ClickStackCreateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `ClickStackDashboardResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filters: Option<Vec<ClickStackFilter>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none", default)]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none", default)]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none", default)]
    pub saved_query_language: Option<ClickStackDashboardResponseSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tiles: Option<Vec<ClickStackTileOutput>>,
}

/// `ClickStackFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilter {
    pub expression: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none", default)]
    pub source_metric_type: Option<ClickStackFilterSourcemetrictype>,
    pub r#type: ClickStackFilterType,
}

/// `ClickStackFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterInput {
    pub expression: String,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none", default)]
    pub source_metric_type: Option<ClickStackFilterInputSourcemetrictype>,
    pub r#type: ClickStackFilterInputType,
}

/// `ClickStackFilterSettingsColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterSettingsColumn {
    pub label: String,
    pub name: String,
}

/// `ClickStackGenericWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackGenericWebhook {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub body: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackGenericWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackHighlightedAttributeExpression` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHighlightedAttributeExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(rename = "luceneExpression", skip_serializing_if = "Option::is_none", default)]
    pub lucene_expression: Option<String>,
    #[serde(rename = "sqlExpression")]
    pub sql_expression: String,
}

/// `ClickStackIncidentIOWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackIncidentIOWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackIncidentIOWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackLineBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineBuilderChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(rename = "compareToPreviousPeriod", skip_serializing_if = "Option::is_none", default)]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackLineRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineRawSqlChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "compareToPreviousPeriod", skip_serializing_if = "Option::is_none", default)]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackLineRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackLogSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSource {
    #[serde(rename = "bodyExpression", skip_serializing_if = "Option::is_none", default)]
    pub body_expression: Option<String>,
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression")]
    pub default_table_select_expression: String,
    #[serde(rename = "displayedTimestampValueExpression", skip_serializing_if = "Option::is_none", default)]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(rename = "eventAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none", default)]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(rename = "highlightedRowAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_row_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(rename = "highlightedTraceAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_trace_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    pub id: String,
    #[serde(rename = "implicitColumnExpression", skip_serializing_if = "Option::is_none", default)]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackLogSourceKind,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none", default)]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none", default)]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub resource_attributes_expression: Option<String>,
    #[serde(rename = "serviceNameExpression", skip_serializing_if = "Option::is_none", default)]
    pub service_name_expression: Option<String>,
    #[serde(rename = "severityTextExpression", skip_serializing_if = "Option::is_none", default)]
    pub severity_text_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none", default)]
    pub span_id_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none", default)]
    pub trace_id_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none", default)]
    pub trace_source_id: Option<String>,
}

/// `ClickStackMarkdownChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackMarkdownChartConfigDisplaytype,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub markdown: Option<String>,
}

/// `ClickStackMarkdownChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartSeries {
    pub content: String,
    pub r#type: ClickStackMarkdownChartSeriesType,
}

/// `ClickStackMaterializedView` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMaterializedView {
    #[serde(rename = "aggregatedColumns")]
    pub aggregated_columns: Vec<ClickStackAggregatedColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "dimensionColumns")]
    pub dimension_columns: String,
    #[serde(rename = "minDate", skip_serializing_if = "Option::is_none", default)]
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "minGranularity")]
    pub min_granularity: ClickStackMaterializedViewMingranularity,
    #[serde(rename = "tableName")]
    pub table_name: String,
    #[serde(rename = "timestampColumn")]
    pub timestamp_column: String,
}

/// `ClickStackMetricSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSource {
    pub connection: String,
    pub from: ClickStackMetricSourceFrom,
    pub id: String,
    pub kind: ClickStackMetricSourceKind,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none", default)]
    pub log_source_id: Option<String>,
    #[serde(rename = "metricTables")]
    pub metric_tables: ClickStackMetricTables,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression")]
    pub resource_attributes_expression: String,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
}

/// `ClickStackMetricSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none", default)]
    pub table_name: Option<String>,
}

/// `ClickStackMetricTables` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricTables {
    #[serde(rename = "exponential histogram", skip_serializing_if = "Option::is_none", default)]
    pub exponential_histogram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub gauge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub histogram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub summary: Option<String>,
}

/// `ClickStackNumberBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberBuilderChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackNumberChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackNumberChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none", default)]
    pub metric_data_type: Option<ClickStackNumberChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackNumberChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackNumberChartSeriesWherelanguage,
}

/// `ClickStackNumberFormat` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberFormat {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub average: Option<bool>,
    #[serde(rename = "currencySymbol", skip_serializing_if = "Option::is_none", default)]
    pub currency_symbol: Option<String>,
    #[serde(rename = "decimalBytes", skip_serializing_if = "Option::is_none", default)]
    pub decimal_bytes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mantissa: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<ClickStackNumberFormatOutput>,
    #[serde(rename = "thousandSeparated", skip_serializing_if = "Option::is_none", default)]
    pub thousand_separated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unit: Option<String>,
}

/// `ClickStackNumberRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackNumberRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackPagerDutyAPIWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPagerDutyAPIWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackPagerDutyAPIWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackPieBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackPieRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackPieRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackQuerySetting` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQuerySetting {
    pub setting: String,
    pub value: String,
}

/// `ClickStackSavedFilterValue` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedFilterValue {
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickStackSavedFilterValueType>,
}

/// `ClickStackSearchChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackSearchChartConfigDisplaytype,
    pub select: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartConfigWherelanguage,
}

/// `ClickStackSearchChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartSeries {
    pub fields: Vec<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackSearchChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartSeriesWherelanguage,
}

/// `ClickStackSelectItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSelectItem {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackSelectItemAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<ClickStackSelectItemLevel>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none", default)]
    pub metric_type: Option<ClickStackSelectItemMetrictype>,
    #[serde(rename = "periodAggFn", skip_serializing_if = "Option::is_none", default)]
    pub period_agg_fn: Option<ClickStackSelectItemPeriodaggfn>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none", default)]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none", default)]
    pub where_language: Option<ClickStackSelectItemWherelanguage>,
}

/// `ClickStackSessionSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSessionSource {
    pub connection: String,
    pub from: ClickStackSourceFrom,
    pub id: String,
    pub kind: ClickStackSessionSourceKind,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "timestampValueExpression", skip_serializing_if = "Option::is_none", default)]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceSourceId")]
    pub trace_source_id: String,
}

/// `ClickStackSlackAPIWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackAPIWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackSlackAPIWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackSlackWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackSlackWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackSourceFilterSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterSettings {
    pub columns: Vec<ClickStackFilterSettingsColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackTableBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableBuilderChartConfig {
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub having: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none", default)]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackTableChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTableChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none", default)]
    pub metric_data_type: Option<ClickStackTableChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none", default)]
    pub sort_order: Option<ClickStackTableChartSeriesSortorder>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTableChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTableChartSeriesWherelanguage,
}

/// `ClickStackTableRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackTableRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackTileInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileInput {
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<ClickStackTileConfig>,
    pub h: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub series: Option<Vec<ClickStackDashboardChartSeries>>,
    pub w: i64,
    pub x: i64,
    pub y: i64,
}

/// `ClickStackTileOutput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileOutput {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<ClickStackTileConfig>,
    pub h: i64,
    pub id: String,
    pub name: String,
    pub w: i64,
    pub x: i64,
    pub y: i64,
}

/// `ClickStackTimeChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTimeChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTimeChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none", default)]
    pub display_type: Option<ClickStackTimeChartSeriesDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none", default)]
    pub metric_data_type: Option<ClickStackTimeChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTimeChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTimeChartSeriesWherelanguage,
}

/// `ClickStackTraceSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSource {
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression", skip_serializing_if = "Option::is_none", default)]
    pub default_table_select_expression: Option<String>,
    #[serde(rename = "durationExpression")]
    pub duration_expression: String,
    #[serde(rename = "durationPrecision")]
    pub duration_precision: i64,
    #[serde(rename = "eventAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none", default)]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(rename = "highlightedRowAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_row_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(rename = "highlightedTraceAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_trace_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    pub id: String,
    #[serde(rename = "implicitColumnExpression", skip_serializing_if = "Option::is_none", default)]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackTraceSourceKind,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none", default)]
    pub log_source_id: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none", default)]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none", default)]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "parentSpanIdExpression")]
    pub parent_span_id_expression: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub resource_attributes_expression: Option<String>,
    #[serde(rename = "serviceNameExpression", skip_serializing_if = "Option::is_none", default)]
    pub service_name_expression: Option<String>,
    #[serde(rename = "sessionSourceId", skip_serializing_if = "Option::is_none", default)]
    pub session_source_id: Option<String>,
    #[serde(rename = "spanEventsValueExpression", skip_serializing_if = "Option::is_none", default)]
    pub span_events_value_expression: Option<String>,
    #[serde(rename = "spanIdExpression")]
    pub span_id_expression: String,
    #[serde(rename = "spanKindExpression")]
    pub span_kind_expression: String,
    #[serde(rename = "spanNameExpression")]
    pub span_name_expression: String,
    #[serde(rename = "statusCodeExpression", skip_serializing_if = "Option::is_none", default)]
    pub status_code_expression: Option<String>,
    #[serde(rename = "statusMessageExpression", skip_serializing_if = "Option::is_none", default)]
    pub status_message_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression")]
    pub trace_id_expression: String,
}

/// `ClickStackUpdateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateAlertRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub channel: Option<ClickStackAlertChannel>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none", default)]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub interval: Option<ClickStackUpdateAlertRequestInterval>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none", default)]
    pub saved_search_id: Option<String>,
    #[serde(rename = "scheduleOffsetMinutes", skip_serializing_if = "Option::is_none", default)]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none", default)]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickStackUpdateAlertRequestSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub threshold: Option<f64>,
    #[serde(rename = "thresholdType", skip_serializing_if = "Option::is_none", default)]
    pub threshold_type: Option<ClickStackUpdateAlertRequestThresholdtype>,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none", default)]
    pub tile_id: Option<String>,
}

/// `ClickStackUpdateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filters: Option<Vec<ClickStackFilter>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none", default)]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none", default)]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none", default)]
    pub saved_query_language: Option<ClickStackUpdateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `CreateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateReversePrivateEndpoint {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none", default)]
    pub msk_authentication: Option<CreateReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none", default)]
    pub msk_cluster_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<CreateReversePrivateEndpointType>,
    #[serde(rename = "vpcEndpointServiceName", skip_serializing_if = "Option::is_none", default)]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(rename = "vpcResourceConfigurationId", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(rename = "vpcResourceShareArn", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_share_arn: Option<String>,
}

/// `GcpBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucket {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none", default)]
    pub access_key_id: Option<String>,
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<GcpBackupBucketBucketprovider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
}

/// `GcpBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPatchRequestV1 {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none", default)]
    pub access_key_id: Option<String>,
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<GcpBackupBucketPatchRequestV1Bucketprovider>,
    #[serde(rename = "secretAccessKey", skip_serializing_if = "Option::is_none", default)]
    pub secret_access_key: Option<String>,
}

/// `GcpBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPostRequestV1 {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none", default)]
    pub access_key_id: Option<String>,
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<GcpBackupBucketPostRequestV1Bucketprovider>,
    #[serde(rename = "secretAccessKey", skip_serializing_if = "Option::is_none", default)]
    pub secret_access_key: Option<String>,
}

/// `GcpBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketProperties {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none", default)]
    pub access_key_id: Option<String>,
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none", default)]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none", default)]
    pub bucket_provider: Option<GcpBackupBucketPropertiesBucketprovider>,
}

/// `InstancePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none", default)]
    pub cloud_provider: Option<InstancePrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<InstancePrivateEndpointRegion>,
}

/// `InstancePrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpointsPatch {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub add: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub remove: Option<Vec<String>>,
}

/// `InstanceServiceQueryApiEndpointsPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceServiceQueryApiEndpointsPostRequest {
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none", default)]
    pub allowed_origins: Option<String>,
    #[serde(rename = "openApiKeys", skip_serializing_if = "Option::is_none", default)]
    pub open_api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
}

/// `InstanceTagsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceTagsPatch {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub add: Option<Vec<ResourceTagsV1>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub remove: Option<Vec<ResourceTagsV1>>,
}

/// `Invitation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Invitation {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none", default)]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email: Option<String>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<InvitationRole>,
}

/// `InvitationPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InvitationPostRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none", default)]
    pub assigned_role_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<InvitationPostRequestRole>,
}

/// `IpAccessListEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListEntry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<String>,
}

/// `IpAccessListPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListPatch {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub add: Option<Vec<IpAccessListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub remove: Option<Vec<IpAccessListEntry>>,
}

/// `Member` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Member {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none", default)]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email: Option<String>,
    #[serde(rename = "joinedAt", skip_serializing_if = "Option::is_none", default)]
    pub joined_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<MemberRole>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none", default)]
    pub user_id: Option<String>,
}

/// `MemberPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none", default)]
    pub assigned_role_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<MemberPatchRequestRole>,
}

/// `MskIamUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MskIamUser {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none", default)]
    pub access_key_id: Option<String>,
    #[serde(rename = "secretKey", skip_serializing_if = "Option::is_none", default)]
    pub secret_key: Option<String>,
}

/// `MutualTLS` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MutualTLS {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub certificate: Option<String>,
    #[serde(rename = "privateKey", skip_serializing_if = "Option::is_none", default)]
    pub private_key: Option<String>,
}

/// `Organization` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Organization {
    #[serde(rename = "byocConfig", skip_serializing_if = "Option::is_none", default)]
    pub byoc_config: Option<Vec<ByocConfig>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoints: Option<Vec<OrganizationPrivateEndpoint>>,
}

/// `OrganizationCloudRegionPrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationCloudRegionPrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", skip_serializing_if = "Option::is_none", default)]
    pub endpoint_service_id: Option<String>,
}

/// `OrganizationPatchPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchPrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none", default)]
    pub cloud_provider: Option<OrganizationPatchPrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<OrganizationPatchPrivateEndpointRegion>,
}

/// `OrganizationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoints: Option<OrganizationPrivateEndpointsPatch>,
}

/// `OrganizationPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none", default)]
    pub cloud_provider: Option<OrganizationPrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<OrganizationPrivateEndpointRegion>,
}

/// `OrganizationPrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpointsPatch {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub add: Option<Vec<OrganizationPatchPrivateEndpoint>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub remove: Option<Vec<OrganizationPatchPrivateEndpoint>>,
}

/// `PLAIN` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PLAIN {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
}

/// `PostgresService` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresService {
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<PgCreatedAtProperty>,
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<PgIdProperty>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none", default)]
    pub is_primary: Option<PgIsPrimaryProperty>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<PgNameProperty>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<PgSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<PgStateProperty>,
    #[serde(rename = "storageSize", skip_serializing_if = "Option::is_none", default)]
    pub storage_size: Option<PgStorageSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
}

/// `PostgresServiceListItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceListItem {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<PgCreatedAtProperty>,
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<PgIdProperty>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none", default)]
    pub is_primary: Option<PgIsPrimaryProperty>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<PgNameProperty>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<PgSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<PgStateProperty>,
    #[serde(rename = "storageSize", skip_serializing_if = "Option::is_none", default)]
    pub storage_size: Option<PgStorageSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServicePasswordResource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePasswordResource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
}

/// `PostgresServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePatchRequest {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<PgNameProperty>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<PgSize>,
    #[serde(rename = "storageSize", skip_serializing_if = "Option::is_none", default)]
    pub storage_size: Option<PgStorageSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePostRequest {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_config: Option<PgConfig>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    pub provider: PgProvider,
    pub region: PgRegion,
    pub size: PgSize,
    #[serde(rename = "storageSize")]
    pub storage_size: PgStorageSize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceReadReplicaRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceReadReplicaRequest {
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_config: Option<PgConfig>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceRestoreRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceRestoreRequest {
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_config: Option<PgConfig>,
    #[serde(rename = "restoreTarget")]
    pub restore_target: PgPitrRestoreTargetProperty,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceSetPassword` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceSetPassword {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<PgPassword>,
}

/// `PostgresServiceSetState` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceSetState {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command: Option<PostgresServiceSetStateCommand>,
}

/// `PrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", skip_serializing_if = "Option::is_none", default)]
    pub endpoint_service_id: Option<String>,
    #[serde(rename = "privateDnsHostname", skip_serializing_if = "Option::is_none", default)]
    pub private_dns_hostname: Option<String>,
}

/// `RBACPolicy` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicy {
    #[serde(rename = "allowDeny", skip_serializing_if = "Option::is_none", default)]
    pub allow_deny: Option<RBACPolicyAllowdeny>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resources: Option<Vec<String>>,
    #[serde(rename = "roleId", skip_serializing_if = "Option::is_none", default)]
    pub role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<RBACPolicyTags>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none", default)]
    pub tenant_id: Option<String>,
}

/// `RBACPolicyCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyCreateRequest {
    #[serde(rename = "allowDeny")]
    pub allow_deny: RBACPolicyCreateRequestAllowdeny,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<RBACPolicyTags>,
}

/// `RBACPolicyTags` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyTags {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub grants: Option<Vec<String>>,
    #[serde(rename = "roleV2", skip_serializing_if = "Option::is_none", default)]
    pub role_v2: Option<RBACPolicyTagsRolev2>,
}

/// `RBACRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACRole {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub actors: Option<Vec<String>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "ownerId", skip_serializing_if = "Option::is_none", default)]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub policies: Option<Vec<RBACPolicy>>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none", default)]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<RBACRoleType>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ResourceTagsV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceTagsV1 {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpoint {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(rename = "dnsNames", skip_serializing_if = "Option::is_none", default)]
    pub dns_names: Option<Vec<String>>,
    #[serde(rename = "endpointId", skip_serializing_if = "Option::is_none", default)]
    pub endpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none", default)]
    pub msk_authentication: Option<ReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none", default)]
    pub msk_cluster_arn: Option<String>,
    #[serde(rename = "privateDnsNames", skip_serializing_if = "Option::is_none", default)]
    pub private_dns_names: Option<Vec<String>>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none", default)]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<ReversePrivateEndpointStatus>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ReversePrivateEndpointType>,
    #[serde(rename = "vpcEndpointServiceName", skip_serializing_if = "Option::is_none", default)]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(rename = "vpcResourceConfigurationId", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(rename = "vpcResourceShareArn", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_share_arn: Option<String>,
}

/// `RoleCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleCreateRequest {
    pub actors: Vec<String>,
    pub name: String,
    pub policies: Vec<RBACPolicyCreateRequest>,
}

/// `RoleUpdateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub actors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub policies: Option<Vec<RBACPolicyCreateRequest>>,
}

/// `ScimEnterpriseManager` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimEnterpriseManager {
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none", default)]
    pub r#ref: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimEnterpriseUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimEnterpriseUser {
    #[serde(rename = "costCenter", skip_serializing_if = "Option::is_none", default)]
    pub cost_center: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub department: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub division: Option<String>,
    #[serde(rename = "employeeNumber", skip_serializing_if = "Option::is_none", default)]
    pub employee_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub manager: Option<ScimEnterpriseManager>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub organization: Option<String>,
}

/// `ScimGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroup {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    pub id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub members: Option<Vec<ScimGroupMember>>,
    pub meta: ScimGroupMeta,
    pub schemas: Vec<String>,
}

/// `ScimGroupListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimGroup>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimGroupMember` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupMember {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    pub value: String,
}

/// `ScimGroupMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupMeta {
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimGroupPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupPostRequest {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub members: Option<Vec<ScimGroupMember>>,
    pub schemas: Vec<String>,
}

/// `ScimGroupPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupPutRequest {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub members: Option<Vec<ScimGroupMember>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<ScimGroupMeta>,
    pub schemas: Vec<String>,
}

/// `ScimListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimUser>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimPatchOp` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimPatchOp {
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimPatchOperation>,
    pub schemas: Vec<String>,
}

/// `ScimPatchOperation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimPatchOperation {
    pub op: ScimPatchOperationOp,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUser {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<Vec<ScimUserGroup>>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    pub meta: ScimUserMeta,
    pub name: ScimUserName,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none", default)]
    pub nick_name: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none", default)]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none", default)]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none", default)]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none", default)]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none", default)]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserAddress` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserAddress {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formatted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locality: Option<String>,
    #[serde(rename = "postalCode", skip_serializing_if = "Option::is_none", default)]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<String>,
    #[serde(rename = "streetAddress", skip_serializing_if = "Option::is_none", default)]
    pub street_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
}

/// `ScimUserEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserEmail {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    pub value: String,
}

/// `ScimUserEntitlement` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserEntitlement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUserGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserGroup {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUserIm` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserIm {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUserMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserMeta {
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimUserName` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserName {
    #[serde(rename = "familyName", skip_serializing_if = "Option::is_none", default)]
    pub family_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formatted: Option<String>,
    #[serde(rename = "givenName", skip_serializing_if = "Option::is_none", default)]
    pub given_name: Option<String>,
    #[serde(rename = "honorificPrefix", skip_serializing_if = "Option::is_none", default)]
    pub honorific_prefix: Option<String>,
    #[serde(rename = "honorificSuffix", skip_serializing_if = "Option::is_none", default)]
    pub honorific_suffix: Option<String>,
    #[serde(rename = "middleName", skip_serializing_if = "Option::is_none", default)]
    pub middle_name: Option<String>,
}

/// `ScimUserPhoneNumber` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPhoneNumber {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUserPhoto` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPhoto {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUserPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPostRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<Vec<ScimUserGroup>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<ScimUserName>,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none", default)]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none", default)]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none", default)]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none", default)]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User", skip_serializing_if = "Option::is_none", default)]
    pub urn_ietf_params_scim_schemas_extension_enterprise_2_0_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none", default)]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none", default)]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPutRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<Vec<ScimUserGroup>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<ScimUserMeta>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<ScimUserName>,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none", default)]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none", default)]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none", default)]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none", default)]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User", skip_serializing_if = "Option::is_none", default)]
    pub urn_ietf_params_scim_schemas_extension_enterprise_2_0_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none", default)]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none", default)]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserRole {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimX509Certificate` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimX509Certificate {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ServicPrivateEndpointePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicPrivateEndpointePostRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
}

/// `Service` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Service {
    #[serde(rename = "availablePrivateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none", default)]
    pub byoc_id: Option<String>,
    #[serde(rename = "clickhouseVersion", skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_version: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none", default)]
    pub compliance_type: Option<ServiceCompliancetype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none", default)]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(rename = "encryptionAssumedRoleIdentifier", skip_serializing_if = "Option::is_none", default)]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none", default)]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", skip_serializing_if = "Option::is_none", default)]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoints: Option<Vec<ServiceEndpoint>>,
    #[serde(rename = "hasTransparentDataEncryption", skip_serializing_if = "Option::is_none", default)]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none", default)]
    pub is_primary: Option<bool>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none", default)]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub profile: Option<ServiceProfile>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<ServiceProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<ServiceRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none", default)]
    pub release_channel: Option<ServiceReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ServiceState>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<ResourceTagsV1>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tier: Option<ServiceTier>,
    #[serde(rename = "transparentDataEncryptionKeyId", skip_serializing_if = "Option::is_none", default)]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServiceAccount` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "serviceAccountFile", skip_serializing_if = "Option::is_none", default)]
    pub service_account_file: Option<String>,
}

/// `ServiceEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub port: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ServiceEndpointProtocol>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
}

/// `ServiceEndpointChange` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpointChange {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ServiceEndpointChangeProtocol>,
}

/// `ServicePasswordPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchRequest {
    #[serde(rename = "newDoubleSha1Hash", skip_serializing_if = "Option::is_none", default)]
    pub new_double_sha1_hash: Option<String>,
    #[serde(rename = "newPasswordHash", skip_serializing_if = "Option::is_none", default)]
    pub new_password_hash: Option<String>,
}

/// `ServicePasswordPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
}

/// `ServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<IpAccessListPatch>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoint_ids: Option<InstancePrivateEndpointsPatch>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none", default)]
    pub release_channel: Option<ServicePatchRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<InstanceTagsPatch>,
    #[serde(rename = "transparentDataEncryptionKeyId", skip_serializing_if = "Option::is_none", default)]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostRequest {
    #[serde(rename = "backupId", skip_serializing_if = "Option::is_none", default)]
    pub backup_id: Option<uuid::Uuid>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none", default)]
    pub byoc_id: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none", default)]
    pub compliance_type: Option<ServicePostRequestCompliancetype>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none", default)]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(rename = "encryptionAssumedRoleIdentifier", skip_serializing_if = "Option::is_none", default)]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none", default)]
    pub encryption_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(rename = "hasTransparentDataEncryption", skip_serializing_if = "Option::is_none", default)]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none", default)]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "privatePreviewTermsChecked", skip_serializing_if = "Option::is_none", default)]
    pub private_preview_terms_checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub profile: Option<ServicePostRequestProfile>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<ServicePostRequestProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<ServicePostRequestRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none", default)]
    pub release_channel: Option<ServicePostRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<ResourceTagsV1>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tier: Option<ServicePostRequestTier>,
}

/// `ServicePostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub service: Option<Service>,
}

/// `ServiceQueryAPIEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceQueryAPIEndpoint {
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none", default)]
    pub allowed_origins: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(rename = "openApiKeys", skip_serializing_if = "Option::is_none", default)]
    pub open_api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
}

/// `ServiceReplicaScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceReplicaScalingPatchRequest {
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
}

/// `ServiceScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchRequest {
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_total_memory_gb: Option<f64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
}

/// `ServiceScalingPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchResponse {
    #[serde(rename = "availablePrivateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none", default)]
    pub byoc_id: Option<String>,
    #[serde(rename = "clickhouseVersion", skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_version: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none", default)]
    pub compliance_type: Option<ServiceScalingPatchResponseCompliancetype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none", default)]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(rename = "encryptionAssumedRoleIdentifier", skip_serializing_if = "Option::is_none", default)]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none", default)]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", skip_serializing_if = "Option::is_none", default)]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoints: Option<Vec<ServiceEndpoint>>,
    #[serde(rename = "hasTransparentDataEncryption", skip_serializing_if = "Option::is_none", default)]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none", default)]
    pub is_primary: Option<bool>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none", default)]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub profile: Option<ServiceScalingPatchResponseProfile>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<ServiceScalingPatchResponseProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<ServiceScalingPatchResponseRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none", default)]
    pub release_channel: Option<ServiceScalingPatchResponseReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ServiceScalingPatchResponseState>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<ResourceTagsV1>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tier: Option<ServiceScalingPatchResponseTier>,
    #[serde(rename = "transparentDataEncryptionKeyId", skip_serializing_if = "Option::is_none", default)]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServiceStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command: Option<ServiceStatePatchRequestCommand>,
}

/// `UsageCost` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCost {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub costs: Option<Vec<UsageCostRecord>>,
    #[serde(rename = "grandTotalCHC", skip_serializing_if = "Option::is_none", default)]
    pub grand_total_chc: Option<f64>,
}

/// `UsageCostMetrics` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostMetrics {
    #[serde(rename = "backupCHC", skip_serializing_if = "Option::is_none", default)]
    pub backup_chc: Option<f64>,
    #[serde(rename = "computeCHC", skip_serializing_if = "Option::is_none", default)]
    pub compute_chc: Option<f64>,
    #[serde(rename = "dataTransferCHC", skip_serializing_if = "Option::is_none", default)]
    pub data_transfer_chc: Option<f64>,
    #[serde(rename = "initialLoadCHC", skip_serializing_if = "Option::is_none", default)]
    pub initial_load_chc: Option<f64>,
    #[serde(rename = "interRegionTier1DataTransferCHC", skip_serializing_if = "Option::is_none", default)]
    pub inter_region_tier1_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier2DataTransferCHC", skip_serializing_if = "Option::is_none", default)]
    pub inter_region_tier2_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier3DataTransferCHC", skip_serializing_if = "Option::is_none", default)]
    pub inter_region_tier3_data_transfer_chc: Option<f64>,
    #[serde(rename = "interRegionTier4DataTransferCHC", skip_serializing_if = "Option::is_none", default)]
    pub inter_region_tier4_data_transfer_chc: Option<f64>,
    #[serde(rename = "publicDataTransferCHC", skip_serializing_if = "Option::is_none", default)]
    pub public_data_transfer_chc: Option<f64>,
    #[serde(rename = "storageCHC", skip_serializing_if = "Option::is_none", default)]
    pub storage_chc: Option<f64>,
}

/// `UsageCostRecord` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostRecord {
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none", default)]
    pub data_warehouse_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub date: Option<String>,
    #[serde(rename = "entityId", skip_serializing_if = "Option::is_none", default)]
    pub entity_id: Option<uuid::Uuid>,
    #[serde(rename = "entityName", skip_serializing_if = "Option::is_none", default)]
    pub entity_name: Option<String>,
    #[serde(rename = "entityType", skip_serializing_if = "Option::is_none", default)]
    pub entity_type: Option<UsageCostRecordEntitytype>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metrics: Option<UsageCostMetrics>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none", default)]
    pub service_id: Option<uuid::Uuid>,
    #[serde(rename = "totalCHC", skip_serializing_if = "Option::is_none", default)]
    pub total_chc: Option<f64>,
}

/// `pgBouncerConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgBouncerConfig {
}

/// `pgConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgConfig {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub default_transaction_isolation: Option<PgConfigDefaultTransactionIsolation>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub effective_cache_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub effective_io_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub idle_in_transaction_session_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub idle_session_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lock_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub maintenance_work_mem: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_connections: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_parallel_maintenance_workers: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_parallel_workers: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_parallel_workers_per_gather: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_slot_wal_keep_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_wal_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_worker_processes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub min_wal_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub random_page_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub statement_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transaction_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wal_compression: Option<PgConfigWalCompression>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wal_keep_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wal_sender_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub work_mem: Option<serde_json::Value>,
}

/// `postgresInstanceConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceConfig {
    #[serde(rename = "pgBouncerConfig")]
    pub pg_bouncer_config: PgBouncerConfig,
    #[serde(rename = "pgConfig")]
    pub pg_config: PgConfig,
}

/// `postgresInstanceUpdateConfigResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceUpdateConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(rename = "pgBouncerConfig")]
    pub pg_bouncer_config: PgBouncerConfig,
    #[serde(rename = "pgConfig")]
    pub pg_config: PgConfig,
}

/// Standard API response wrapper.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "requestId")]
    pub request_id: Option<String>,
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}


impl Default for BackupBucket {
    fn default() -> Self {
        Self::AwsBackupBucket(AwsBackupBucket::default())
    }
}


impl Default for BackupBucketPatchRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1::default())
    }
}


impl Default for BackupBucketPostRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1::default())
    }
}


impl Default for BackupBucketProperties {
    fn default() -> Self {
        Self::AwsBackupBucketProperties(AwsBackupBucketProperties::default())
    }
}


impl Default for ClickStackAlertChannel {
    fn default() -> Self {
        Self::ClickStackAlertChannelEmail(ClickStackAlertChannelEmail::default())
    }
}


impl Default for ClickStackBarChartConfig {
    fn default() -> Self {
        Self::ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig::default())
    }
}


impl Default for ClickStackDashboardChartSeries {
    fn default() -> Self {
        Self::ClickStackTimeChartSeries(ClickStackTimeChartSeries::default())
    }
}


impl Default for ClickStackLineChartConfig {
    fn default() -> Self {
        Self::ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig::default())
    }
}


impl Default for ClickStackNumberChartConfig {
    fn default() -> Self {
        Self::ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig::default())
    }
}


impl Default for ClickStackPieChartConfig {
    fn default() -> Self {
        Self::ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig::default())
    }
}


impl Default for ClickStackSource {
    fn default() -> Self {
        Self::ClickStackLogSource(ClickStackLogSource::default())
    }
}


impl Default for ClickStackTableChartConfig {
    fn default() -> Self {
        Self::ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig::default())
    }
}


impl Default for ClickStackTileConfig {
    fn default() -> Self {
        Self::ClickStackLineChartConfig(ClickStackLineChartConfig::default())
    }
}


impl Default for ClickStackWebhook {
    fn default() -> Self {
        Self::ClickStackSlackWebhook(ClickStackSlackWebhook::default())
    }
}
