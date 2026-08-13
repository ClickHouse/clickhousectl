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
    #[serde(rename = "targetKeyId", skip_serializing_if = "Option::is_none")]
    pub target_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ActivityType>,
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}
