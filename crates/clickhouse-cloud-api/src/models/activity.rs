use serde::{Deserialize, Serialize};
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
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityActortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Support => write!(f, "support"),
            Self::System => write!(f, "system"),
            Self::Api => write!(f, "api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
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
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityKeyupdatetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Deleted => write!(f, "deleted"),
            Self::Name_changed => write!(f, "name-changed"),
            Self::Role_changed => write!(f, "role-changed"),
            Self::State_changed => write!(f, "state-changed"),
            Self::Date_changed => write!(f, "date-changed"),
            Self::Ip_access_list_changed => write!(f, "ip-access-list-changed"),
            Self::Org_role_changed => write!(f, "org-role-changed"),
            Self::Default_service_role_changed => write!(f, "default-service-role-changed"),
            Self::Service_role_changed => write!(f, "service-role-changed"),
            Self::Roles_v2_changed => write!(f, "roles-v2-changed"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
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
    #[serde(rename = "organization_member_update_roles")]
    Organization_member_update_roles,
    #[serde(rename = "organization_member_update_mfa_method")]
    Organization_member_update_mfa_method,
    #[serde(rename = "organization_saml_connection_create")]
    Organization_saml_connection_create,
    #[serde(rename = "organization_saml_connection_update")]
    Organization_saml_connection_update,
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
    #[serde(rename = "service_update_snapshot_configuration")]
    Service_update_snapshot_configuration,
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
    #[serde(rename = "backup_bucket_archive")]
    Backup_bucket_archive,
    #[serde(rename = "backup_bucket_create")]
    Backup_bucket_create,
    #[serde(rename = "backup_bucket_delete")]
    Backup_bucket_delete,
    #[serde(rename = "backup_bucket_update")]
    Backup_bucket_update,
    #[serde(rename = "datadog_integration_create")]
    Datadog_integration_create,
    #[serde(rename = "datadog_integration_delete")]
    Datadog_integration_delete,
    #[serde(rename = "delete_organization")]
    Delete_organization,
    #[serde(rename = "organization_member_remove_roles")]
    Organization_member_remove_roles,
    #[serde(rename = "organization_saml_connection_delete")]
    Organization_saml_connection_delete,
    #[serde(rename = "organization_update_core_dumps")]
    Organization_update_core_dumps,
    #[serde(rename = "organization_update_hipaa_status")]
    Organization_update_hipaa_status,
    #[serde(rename = "organization_update_pci_compliance")]
    Organization_update_pci_compliance,
    #[serde(rename = "organization_update_private_endpoints")]
    Organization_update_private_endpoints,
    #[serde(rename = "organization_update_spend_alert")]
    Organization_update_spend_alert,
    #[serde(rename = "promo_code_claim")]
    Promo_code_claim,
    #[serde(rename = "role_create")]
    Role_create,
    #[serde(rename = "role_delete")]
    Role_delete,
    #[serde(rename = "role_resources_delete")]
    Role_resources_delete,
    #[serde(rename = "role_update")]
    Role_update,
    #[serde(rename = "schema_advisor_approve_plan")]
    Schema_advisor_approve_plan,
    #[serde(rename = "schema_advisor_drop_sandbox")]
    Schema_advisor_drop_sandbox,
    #[serde(rename = "schema_advisor_exchange_tables")]
    Schema_advisor_exchange_tables,
    #[serde(rename = "schema_advisor_generate_plan")]
    Schema_advisor_generate_plan,
    #[serde(rename = "schema_advisor_run_benchmark")]
    Schema_advisor_run_benchmark,
    #[serde(rename = "schema_advisor_seed")]
    Schema_advisor_seed,
    #[serde(rename = "schema_advisor_start_benchmark")]
    Schema_advisor_start_benchmark,
    #[serde(rename = "schema_advisor_start_deployment")]
    Schema_advisor_start_deployment,
    #[serde(rename = "schema_advisor_start_promotion")]
    Schema_advisor_start_promotion,
    #[serde(rename = "scim_group_create")]
    Scim_group_create,
    #[serde(rename = "scim_group_delete")]
    Scim_group_delete,
    #[serde(rename = "scim_group_update")]
    Scim_group_update,
    #[serde(rename = "scim_user_profile_update")]
    Scim_user_profile_update,
    #[serde(rename = "service_delete_upgrade_window")]
    Service_delete_upgrade_window,
    #[serde(rename = "service_encryption_key_check_failed")]
    Service_encryption_key_check_failed,
    #[serde(rename = "service_encryption_key_rotated")]
    Service_encryption_key_rotated,
    #[serde(rename = "service_encryption_key_rotation_failed")]
    Service_encryption_key_rotation_failed,
    #[serde(rename = "service_mcp_disabled")]
    Service_mcp_disabled,
    #[serde(rename = "service_mcp_enabled")]
    Service_mcp_enabled,
    #[serde(rename = "service_restart_encryption_key_rotation")]
    Service_restart_encryption_key_rotation,
    #[serde(rename = "service_scaled_down_for_tier_change")]
    Service_scaled_down_for_tier_change,
    #[serde(rename = "service_stop_encryption_key_inaccessible")]
    Service_stop_encryption_key_inaccessible,
    #[serde(rename = "service_trigger_failover")]
    Service_trigger_failover,
    #[serde(rename = "service_trigger_recovery")]
    Service_trigger_recovery,
    #[serde(rename = "service_update_autoscaling_schedule")]
    Service_update_autoscaling_schedule,
    #[serde(rename = "service_update_collector_ip_access_list")]
    Service_update_collector_ip_access_list,
    #[serde(rename = "service_update_direct_connection")]
    Service_update_direct_connection,
    #[serde(rename = "service_update_mysql_interface")]
    Service_update_mysql_interface,
    #[serde(rename = "service_update_query_endpoints")]
    Service_update_query_endpoints,
    #[serde(rename = "service_update_sql_console_jwt_auth")]
    Service_update_sql_console_jwt_auth,
    #[serde(rename = "service_update_upgrade_window")]
    Service_update_upgrade_window,
    #[serde(rename = "service_upgrade")]
    Service_upgrade,
    #[serde(rename = "transfer_credits_in")]
    Transfer_credits_in,
    #[serde(rename = "transfer_credits_out")]
    Transfer_credits_out,
    #[serde(rename = "udf_attach")]
    Udf_attach,
    #[serde(rename = "udf_create")]
    Udf_create,
    #[serde(rename = "udf_delete")]
    Udf_delete,
    #[serde(rename = "udf_detach")]
    Udf_detach,
    #[serde(rename = "udf_rebuild")]
    Udf_rebuild,
    #[serde(rename = "udf_redeploy")]
    Udf_redeploy,
    #[serde(rename = "udf_update")]
    Udf_update,
    #[serde(rename = "udf_update_services")]
    Udf_update_services,
    #[serde(rename = "udf_version_create")]
    Udf_version_create,
    #[serde(rename = "udf_version_delete")]
    Udf_version_delete,
    #[serde(rename = "warehouse_update_name")]
    Warehouse_update_name,
    #[serde(rename = "warehouse_update_release_channel")]
    Warehouse_update_release_channel,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create_organization => write!(f, "create_organization"),
            Self::Organization_update_name => write!(f, "organization_update_name"),
            Self::Transfer_service_in => write!(f, "transfer_service_in"),
            Self::Transfer_service_out => write!(f, "transfer_service_out"),
            Self::Save_payment_method => write!(f, "save_payment_method"),
            Self::Marketplace_subscription => write!(f, "marketplace_subscription"),
            Self::Migrate_marketplace_billing_details_in => {
                write!(f, "migrate_marketplace_billing_details_in")
            }
            Self::Migrate_marketplace_billing_details_out => {
                write!(f, "migrate_marketplace_billing_details_out")
            }
            Self::Organization_update_tier => write!(f, "organization_update_tier"),
            Self::Organization_invite_create => write!(f, "organization_invite_create"),
            Self::Organization_invite_delete => write!(f, "organization_invite_delete"),
            Self::Organization_member_join => write!(f, "organization_member_join"),
            Self::Organization_member_add => write!(f, "organization_member_add"),
            Self::Organization_member_leave => write!(f, "organization_member_leave"),
            Self::Organization_member_delete => write!(f, "organization_member_delete"),
            Self::Organization_member_update_role => write!(f, "organization_member_update_role"),
            Self::Organization_member_update_roles => {
                write!(f, "organization_member_update_roles")
            }
            Self::Organization_member_update_mfa_method => {
                write!(f, "organization_member_update_mfa_method")
            }
            Self::Organization_saml_connection_create => {
                write!(f, "organization_saml_connection_create")
            }
            Self::Organization_saml_connection_update => {
                write!(f, "organization_saml_connection_update")
            }
            Self::User_login => write!(f, "user_login"),
            Self::User_login_failed => write!(f, "user_login_failed"),
            Self::User_logout => write!(f, "user_logout"),
            Self::Key_create => write!(f, "key_create"),
            Self::Key_delete => write!(f, "key_delete"),
            Self::Openapi_key_update => write!(f, "openapi_key_update"),
            Self::Service_create => write!(f, "service_create"),
            Self::Service_start => write!(f, "service_start"),
            Self::Service_stop => write!(f, "service_stop"),
            Self::Service_awaken => write!(f, "service_awaken"),
            Self::Service_idle => write!(f, "service_idle"),
            Self::Service_running => write!(f, "service_running"),
            Self::Service_partially_running => write!(f, "service_partially_running"),
            Self::Service_delete => write!(f, "service_delete"),
            Self::Service_update_name => write!(f, "service_update_name"),
            Self::Service_update_ip_access_list => write!(f, "service_update_ip_access_list"),
            Self::Service_update_autoscaling_memory => {
                write!(f, "service_update_autoscaling_memory")
            }
            Self::Service_update_autoscaling_idling => {
                write!(f, "service_update_autoscaling_idling")
            }
            Self::Service_update_password => write!(f, "service_update_password"),
            Self::Service_update_autoscaling_replicas => {
                write!(f, "service_update_autoscaling_replicas")
            }
            Self::Service_update_max_allowable_replicas => {
                write!(f, "service_update_max_allowable_replicas")
            }
            Self::Service_update_backup_configuration => {
                write!(f, "service_update_backup_configuration")
            }
            Self::Service_update_snapshot_configuration => {
                write!(f, "service_update_snapshot_configuration")
            }
            Self::Service_restore_backup => write!(f, "service_restore_backup"),
            Self::Service_update_release_channel => write!(f, "service_update_release_channel"),
            Self::Service_update_gpt_usage_consent => write!(f, "service_update_gpt_usage_consent"),
            Self::Service_update_private_endpoints => write!(f, "service_update_private_endpoints"),
            Self::Service_import_to_organization => write!(f, "service_import_to_organization"),
            Self::Service_export_from_organization => write!(f, "service_export_from_organization"),
            Self::Service_maintenance_start => write!(f, "service_maintenance_start"),
            Self::Service_maintenance_end => write!(f, "service_maintenance_end"),
            Self::Service_update_core_dump => write!(f, "service_update_core_dump"),
            Self::Backup_delete => write!(f, "backup_delete"),
            Self::Backup_bucket_archive => write!(f, "backup_bucket_archive"),
            Self::Backup_bucket_create => write!(f, "backup_bucket_create"),
            Self::Backup_bucket_delete => write!(f, "backup_bucket_delete"),
            Self::Backup_bucket_update => write!(f, "backup_bucket_update"),
            Self::Datadog_integration_create => write!(f, "datadog_integration_create"),
            Self::Datadog_integration_delete => write!(f, "datadog_integration_delete"),
            Self::Delete_organization => write!(f, "delete_organization"),
            Self::Organization_member_remove_roles => {
                write!(f, "organization_member_remove_roles")
            }
            Self::Organization_saml_connection_delete => {
                write!(f, "organization_saml_connection_delete")
            }
            Self::Organization_update_core_dumps => {
                write!(f, "organization_update_core_dumps")
            }
            Self::Organization_update_hipaa_status => {
                write!(f, "organization_update_hipaa_status")
            }
            Self::Organization_update_pci_compliance => {
                write!(f, "organization_update_pci_compliance")
            }
            Self::Organization_update_private_endpoints => {
                write!(f, "organization_update_private_endpoints")
            }
            Self::Organization_update_spend_alert => {
                write!(f, "organization_update_spend_alert")
            }
            Self::Promo_code_claim => write!(f, "promo_code_claim"),
            Self::Role_create => write!(f, "role_create"),
            Self::Role_delete => write!(f, "role_delete"),
            Self::Role_resources_delete => write!(f, "role_resources_delete"),
            Self::Role_update => write!(f, "role_update"),
            Self::Schema_advisor_approve_plan => write!(f, "schema_advisor_approve_plan"),
            Self::Schema_advisor_drop_sandbox => write!(f, "schema_advisor_drop_sandbox"),
            Self::Schema_advisor_exchange_tables => {
                write!(f, "schema_advisor_exchange_tables")
            }
            Self::Schema_advisor_generate_plan => write!(f, "schema_advisor_generate_plan"),
            Self::Schema_advisor_run_benchmark => {
                write!(f, "schema_advisor_run_benchmark")
            }
            Self::Schema_advisor_seed => write!(f, "schema_advisor_seed"),
            Self::Schema_advisor_start_benchmark => {
                write!(f, "schema_advisor_start_benchmark")
            }
            Self::Schema_advisor_start_deployment => {
                write!(f, "schema_advisor_start_deployment")
            }
            Self::Schema_advisor_start_promotion => {
                write!(f, "schema_advisor_start_promotion")
            }
            Self::Scim_group_create => write!(f, "scim_group_create"),
            Self::Scim_group_delete => write!(f, "scim_group_delete"),
            Self::Scim_group_update => write!(f, "scim_group_update"),
            Self::Scim_user_profile_update => write!(f, "scim_user_profile_update"),
            Self::Service_delete_upgrade_window => {
                write!(f, "service_delete_upgrade_window")
            }
            Self::Service_encryption_key_check_failed => {
                write!(f, "service_encryption_key_check_failed")
            }
            Self::Service_encryption_key_rotated => {
                write!(f, "service_encryption_key_rotated")
            }
            Self::Service_encryption_key_rotation_failed => {
                write!(f, "service_encryption_key_rotation_failed")
            }
            Self::Service_mcp_disabled => write!(f, "service_mcp_disabled"),
            Self::Service_mcp_enabled => write!(f, "service_mcp_enabled"),
            Self::Service_restart_encryption_key_rotation => {
                write!(f, "service_restart_encryption_key_rotation")
            }
            Self::Service_scaled_down_for_tier_change => {
                write!(f, "service_scaled_down_for_tier_change")
            }
            Self::Service_stop_encryption_key_inaccessible => {
                write!(f, "service_stop_encryption_key_inaccessible")
            }
            Self::Service_trigger_failover => write!(f, "service_trigger_failover"),
            Self::Service_trigger_recovery => write!(f, "service_trigger_recovery"),
            Self::Service_update_autoscaling_schedule => {
                write!(f, "service_update_autoscaling_schedule")
            }
            Self::Service_update_collector_ip_access_list => {
                write!(f, "service_update_collector_ip_access_list")
            }
            Self::Service_update_direct_connection => {
                write!(f, "service_update_direct_connection")
            }
            Self::Service_update_mysql_interface => {
                write!(f, "service_update_mysql_interface")
            }
            Self::Service_update_query_endpoints => {
                write!(f, "service_update_query_endpoints")
            }
            Self::Service_update_sql_console_jwt_auth => {
                write!(f, "service_update_sql_console_jwt_auth")
            }
            Self::Service_update_upgrade_window => {
                write!(f, "service_update_upgrade_window")
            }
            Self::Service_upgrade => write!(f, "service_upgrade"),
            Self::Transfer_credits_in => write!(f, "transfer_credits_in"),
            Self::Transfer_credits_out => write!(f, "transfer_credits_out"),
            Self::Udf_attach => write!(f, "udf_attach"),
            Self::Udf_create => write!(f, "udf_create"),
            Self::Udf_delete => write!(f, "udf_delete"),
            Self::Udf_detach => write!(f, "udf_detach"),
            Self::Udf_rebuild => write!(f, "udf_rebuild"),
            Self::Udf_redeploy => write!(f, "udf_redeploy"),
            Self::Udf_update => write!(f, "udf_update"),
            Self::Udf_update_services => write!(f, "udf_update_services"),
            Self::Udf_version_create => write!(f, "udf_version_create"),
            Self::Udf_version_delete => write!(f, "udf_version_delete"),
            Self::Warehouse_update_name => write!(f, "warehouse_update_name"),
            Self::Warehouse_update_release_channel => {
                write!(f, "warehouse_update_release_channel")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `Activity` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "actorDetails", skip_serializing_if = "Option::is_none")]
    pub actor_details: Option<String>,
    #[serde(rename = "actorId", skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(rename = "actorIpAddress", skip_serializing_if = "Option::is_none")]
    pub actor_ip_address: Option<String>,
    #[serde(rename = "actorType", skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<ActivityActortype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "keyUpdateType", skip_serializing_if = "Option::is_none")]
    pub key_update_type: Option<ActivityKeyupdatetype>,
    #[serde(rename = "organizationId", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "targetActorIds", skip_serializing_if = "Option::is_none")]
    pub target_actor_ids: Option<Vec<String>>,
    #[serde(rename = "targetKeyId", skip_serializing_if = "Option::is_none")]
    pub target_key_id: Option<String>,
    #[serde(rename = "targetResourceIds", skip_serializing_if = "Option::is_none")]
    pub target_resource_ids: Option<Vec<String>>,
    #[serde(rename = "targetRoleIds", skip_serializing_if = "Option::is_none")]
    pub target_role_ids: Option<Vec<String>>,
    #[serde(rename = "targetRoleNames", skip_serializing_if = "Option::is_none")]
    pub target_role_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ActivityType>,
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}
