//! Generated from the vendored OpenAPI snapshot; do not edit by hand.
//! Regenerate with `openapi-drift-analyzer --spec <snapshot> --generate-operations <file>`.

use super::OperationMetadata;

/// `GET /v1/organizations/{organizationId}/activeBalances` (`activeBalancesGet`).
pub const ACTIVE_BALANCES_GET: OperationMetadata = OperationMetadata {
    operation_id: "activeBalancesGet",
    rust_method: "active_balances_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/activeBalances",
    required_permissions: &["control-plane:organization:view-billing"],
};

/// `GET /v1/organizations/{organizationId}/activities/{activityId}` (`activityGet`).
pub const ACTIVITY_GET: OperationMetadata = OperationMetadata {
    operation_id: "activityGet",
    rust_method: "activity_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/activities/{activityId}",
    required_permissions: &["control-plane:organization:view-activities"],
};

/// `GET /v1/organizations/{organizationId}/activities` (`activityGetList`).
pub const ACTIVITY_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "activityGetList",
    rust_method: "activity_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/activities",
    required_permissions: &["control-plane:organization:view-activities"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/backupBucket` (`backupBucketCreate`).
pub const BACKUP_BUCKET_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "backupBucketCreate",
    rust_method: "backup_bucket_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backupBucket",
    required_permissions: &["control-plane:service:manage-backups"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/backupBucket` (`backupBucketDelete`).
pub const BACKUP_BUCKET_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "backupBucketDelete",
    rust_method: "backup_bucket_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backupBucket",
    required_permissions: &["control-plane:service:manage-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/backupBucket` (`backupBucketGet`).
pub const BACKUP_BUCKET_GET: OperationMetadata = OperationMetadata {
    operation_id: "backupBucketGet",
    rust_method: "backup_bucket_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backupBucket",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/backupBucket` (`backupBucketUpdate`).
pub const BACKUP_BUCKET_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "backupBucketUpdate",
    rust_method: "backup_bucket_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backupBucket",
    required_permissions: &["control-plane:service:manage-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/backupConfiguration` (`backupConfigurationGet`).
pub const BACKUP_CONFIGURATION_GET: OperationMetadata = OperationMetadata {
    operation_id: "backupConfigurationGet",
    rust_method: "backup_configuration_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backupConfiguration",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/backupConfiguration` (`backupConfigurationUpdate`).
pub const BACKUP_CONFIGURATION_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "backupConfigurationUpdate",
    rust_method: "backup_configuration_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backupConfiguration",
    required_permissions: &["control-plane:service:manage-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/backups/{backupId}` (`backupGet`).
pub const BACKUP_GET: OperationMetadata = OperationMetadata {
    operation_id: "backupGet",
    rust_method: "backup_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backups/{backupId}",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/backups` (`backupGetList`).
pub const BACKUP_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "backupGetList",
    rust_method: "backup_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/backups",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipesCdcScaling` (`clickPipeCdcScalingGet`).
pub const CLICK_PIPE_CDC_SCALING_GET: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeCdcScalingGet",
    rust_method: "click_pipe_cdc_scaling_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesCdcScaling",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/clickpipesCdcScaling` (`clickPipeCdcScalingUpdate`).
pub const CLICK_PIPE_CDC_SCALING_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeCdcScalingUpdate",
    rust_method: "click_pipe_cdc_scaling_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesCdcScaling",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickpipes` (`clickPipeCreate`).
pub const CLICK_PIPE_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeCreate",
    rust_method: "click_pipe_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}` (`clickPipeDelete`).
pub const CLICK_PIPE_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeDelete",
    rust_method: "click_pipe_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}` (`clickPipeGet`).
pub const CLICK_PIPE_GET: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeGet",
    rust_method: "click_pipe_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipes` (`clickPipeGetList`).
pub const CLICK_PIPE_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeGetList",
    rust_method: "click_pipe_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints` (`clickPipeReversePrivateEndpointCreate`).
pub const CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeReversePrivateEndpointCreate",
    rust_method: "click_pipe_reverse_private_endpoint_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints/{reversePrivateEndpointId}` (`clickPipeReversePrivateEndpointDelete`).
pub const CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeReversePrivateEndpointDelete",
    rust_method: "click_pipe_reverse_private_endpoint_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints/{reversePrivateEndpointId}",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints/{reversePrivateEndpointId}` (`clickPipeReversePrivateEndpointGet`).
pub const CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_GET: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeReversePrivateEndpointGet",
    rust_method: "click_pipe_reverse_private_endpoint_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints/{reversePrivateEndpointId}",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints` (`clickPipeReversePrivateEndpointGetList`).
pub const CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeReversePrivateEndpointGetList",
    rust_method: "click_pipe_reverse_private_endpoint_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints/{reversePrivateEndpointId}` (`clickPipeReversePrivateEndpointUpdate`).
pub const CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeReversePrivateEndpointUpdate",
    rust_method: "click_pipe_reverse_private_endpoint_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipesReversePrivateEndpoints/{reversePrivateEndpointId}",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/scaling` (`clickPipeScalingUpdate`).
pub const CLICK_PIPE_SCALING_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeScalingUpdate",
    rust_method: "click_pipe_scaling_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/scaling",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/schemaDiscovery` (`clickPipeSchemaDiscovery`).
pub const CLICK_PIPE_SCHEMA_DISCOVERY: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeSchemaDiscovery",
    rust_method: "click_pipe_schema_discovery",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/schemaDiscovery",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/settings` (`clickPipeSettingsGet`).
pub const CLICK_PIPE_SETTINGS_GET: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeSettingsGet",
    rust_method: "click_pipe_settings_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/settings",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/settings` (`clickPipeSettingsUpdate`).
pub const CLICK_PIPE_SETTINGS_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeSettingsUpdate",
    rust_method: "click_pipe_settings_update",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/settings",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/state` (`clickPipeStateUpdate`).
pub const CLICK_PIPE_STATE_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeStateUpdate",
    rust_method: "click_pipe_state_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}/state",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}` (`clickPipeUpdate`).
pub const CLICK_PIPE_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "clickPipeUpdate",
    rust_method: "click_pipe_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/{clickPipeId}",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickpipes/context` (`clickPipesServiceContextGet`).
pub const CLICK_PIPES_SERVICE_CONTEXT_GET: OperationMetadata = OperationMetadata {
    operation_id: "clickPipesServiceContextGet",
    rust_method: "click_pipes_service_context_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickpipes/context",
    required_permissions: &["control-plane:service:manage-clickpipes"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts` (`clickStackCreateAlert`).
pub const CLICK_STACK_CREATE_ALERT: OperationMetadata = OperationMetadata {
    operation_id: "clickStackCreateAlert",
    rust_method: "click_stack_create_alert",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards` (`clickStackCreateDashboard`).
pub const CLICK_STACK_CREATE_DASHBOARD: OperationMetadata = OperationMetadata {
    operation_id: "clickStackCreateDashboard",
    rust_method: "click_stack_create_dashboard",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles` (`clickStackCreateRole`).
pub const CLICK_STACK_CREATE_ROLE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackCreateRole",
    rust_method: "click_stack_create_role",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches` (`clickStackCreateSavedSearch`).
pub const CLICK_STACK_CREATE_SAVED_SEARCH: OperationMetadata = OperationMetadata {
    operation_id: "clickStackCreateSavedSearch",
    rust_method: "click_stack_create_saved_search",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources` (`clickStackCreateSource`).
pub const CLICK_STACK_CREATE_SOURCE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackCreateSource",
    rust_method: "click_stack_create_source",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks` (`clickStackCreateWebhook`).
pub const CLICK_STACK_CREATE_WEBHOOK: OperationMetadata = OperationMetadata {
    operation_id: "clickStackCreateWebhook",
    rust_method: "click_stack_create_webhook",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts/{clickStackAlertId}` (`clickStackDeleteAlert`).
pub const CLICK_STACK_DELETE_ALERT: OperationMetadata = OperationMetadata {
    operation_id: "clickStackDeleteAlert",
    rust_method: "click_stack_delete_alert",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts/{clickStackAlertId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/{clickStackDashboardId}` (`clickStackDeleteDashboard`).
pub const CLICK_STACK_DELETE_DASHBOARD: OperationMetadata = OperationMetadata {
    operation_id: "clickStackDeleteDashboard",
    rust_method: "click_stack_delete_dashboard",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/{clickStackDashboardId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles/{clickStackRoleId}` (`clickStackDeleteRole`).
pub const CLICK_STACK_DELETE_ROLE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackDeleteRole",
    rust_method: "click_stack_delete_role",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles/{clickStackRoleId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches/{clickStackSavedSearchId}` (`clickStackDeleteSavedSearch`).
pub const CLICK_STACK_DELETE_SAVED_SEARCH: OperationMetadata = OperationMetadata {
    operation_id: "clickStackDeleteSavedSearch",
    rust_method: "click_stack_delete_saved_search",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches/{clickStackSavedSearchId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources/{clickStackSourceId}` (`clickStackDeleteSource`).
pub const CLICK_STACK_DELETE_SOURCE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackDeleteSource",
    rust_method: "click_stack_delete_source",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources/{clickStackSourceId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks/{clickStackWebhookId}` (`clickStackDeleteWebhook`).
pub const CLICK_STACK_DELETE_WEBHOOK: OperationMetadata = OperationMetadata {
    operation_id: "clickStackDeleteWebhook",
    rust_method: "click_stack_delete_webhook",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks/{clickStackWebhookId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts/{clickStackAlertId}` (`clickStackGetAlert`).
pub const CLICK_STACK_GET_ALERT: OperationMetadata = OperationMetadata {
    operation_id: "clickStackGetAlert",
    rust_method: "click_stack_get_alert",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts/{clickStackAlertId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/{clickStackDashboardId}` (`clickStackGetDashboard`).
pub const CLICK_STACK_GET_DASHBOARD: OperationMetadata = OperationMetadata {
    operation_id: "clickStackGetDashboard",
    rust_method: "click_stack_get_dashboard",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/{clickStackDashboardId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles/{clickStackRoleId}` (`clickStackGetRole`).
pub const CLICK_STACK_GET_ROLE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackGetRole",
    rust_method: "click_stack_get_role",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles/{clickStackRoleId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches/{clickStackSavedSearchId}` (`clickStackGetSavedSearch`).
pub const CLICK_STACK_GET_SAVED_SEARCH: OperationMetadata = OperationMetadata {
    operation_id: "clickStackGetSavedSearch",
    rust_method: "click_stack_get_saved_search",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches/{clickStackSavedSearchId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources/{clickStackSourceId}` (`clickStackGetSource`).
pub const CLICK_STACK_GET_SOURCE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackGetSource",
    rust_method: "click_stack_get_source",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources/{clickStackSourceId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts` (`clickStackListAlerts`).
pub const CLICK_STACK_LIST_ALERTS: OperationMetadata = OperationMetadata {
    operation_id: "clickStackListAlerts",
    rust_method: "click_stack_list_alerts",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards` (`clickStackListDashboards`).
pub const CLICK_STACK_LIST_DASHBOARDS: OperationMetadata = OperationMetadata {
    operation_id: "clickStackListDashboards",
    rust_method: "click_stack_list_dashboards",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles` (`clickStackListRoles`).
pub const CLICK_STACK_LIST_ROLES: OperationMetadata = OperationMetadata {
    operation_id: "clickStackListRoles",
    rust_method: "click_stack_list_roles",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches` (`clickStackListSavedSearches`).
pub const CLICK_STACK_LIST_SAVED_SEARCHES: OperationMetadata = OperationMetadata {
    operation_id: "clickStackListSavedSearches",
    rust_method: "click_stack_list_saved_searches",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources` (`clickStackListSources`).
pub const CLICK_STACK_LIST_SOURCES: OperationMetadata = OperationMetadata {
    operation_id: "clickStackListSources",
    rust_method: "click_stack_list_sources",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks` (`clickStackListWebhooks`).
pub const CLICK_STACK_LIST_WEBHOOKS: OperationMetadata = OperationMetadata {
    operation_id: "clickStackListWebhooks",
    rust_method: "click_stack_list_webhooks",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts/{clickStackAlertId}` (`clickStackUpdateAlert`).
pub const CLICK_STACK_UPDATE_ALERT: OperationMetadata = OperationMetadata {
    operation_id: "clickStackUpdateAlert",
    rust_method: "click_stack_update_alert",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/alerts/{clickStackAlertId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/{clickStackDashboardId}` (`clickStackUpdateDashboard`).
pub const CLICK_STACK_UPDATE_DASHBOARD: OperationMetadata = OperationMetadata {
    operation_id: "clickStackUpdateDashboard",
    rust_method: "click_stack_update_dashboard",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/{clickStackDashboardId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles/{clickStackRoleId}` (`clickStackUpdateRole`).
pub const CLICK_STACK_UPDATE_ROLE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackUpdateRole",
    rust_method: "click_stack_update_role",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/roles/{clickStackRoleId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches/{clickStackSavedSearchId}` (`clickStackUpdateSavedSearch`).
pub const CLICK_STACK_UPDATE_SAVED_SEARCH: OperationMetadata = OperationMetadata {
    operation_id: "clickStackUpdateSavedSearch",
    rust_method: "click_stack_update_saved_search",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/saved-searches/{clickStackSavedSearchId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources/{clickStackSourceId}` (`clickStackUpdateSource`).
pub const CLICK_STACK_UPDATE_SOURCE: OperationMetadata = OperationMetadata {
    operation_id: "clickStackUpdateSource",
    rust_method: "click_stack_update_source",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/sources/{clickStackSourceId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks/{clickStackWebhookId}` (`clickStackUpdateWebhook`).
pub const CLICK_STACK_UPDATE_WEBHOOK: OperationMetadata = OperationMetadata {
    operation_id: "clickStackUpdateWebhook",
    rust_method: "click_stack_update_webhook",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/webhooks/{clickStackWebhookId}",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/validate` (`clickStackValidateDashboard`).
pub const CLICK_STACK_VALIDATE_DASHBOARD: OperationMetadata = OperationMetadata {
    operation_id: "clickStackValidateDashboard",
    rust_method: "click_stack_validate_dashboard",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickstack/dashboards/validate",
    required_permissions: &["control-plane:service:manage-clickstack-api"],
};

/// `GET /v1/organizations/{organizationId}/creditBalances` (`creditBalancesGet`).
pub const CREDIT_BALANCES_GET: OperationMetadata = OperationMetadata {
    operation_id: "creditBalancesGet",
    rust_method: "credit_balances_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/creditBalances",
    required_permissions: &["control-plane:organization:view-billing"],
};

/// `POST /v1/organizations/{organizationId}/services` (`instanceCreate`).
pub const INSTANCE_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "instanceCreate",
    rust_method: "instance_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services",
    required_permissions: &["control-plane:organization:create-service"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}` (`instanceDelete`).
pub const INSTANCE_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "instanceDelete",
    rust_method: "instance_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}",
    required_permissions: &["control-plane:service:delete"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}` (`instanceGet`).
pub const INSTANCE_GET: OperationMetadata = OperationMetadata {
    operation_id: "instanceGet",
    rust_method: "instance_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}",
    required_permissions: &["control-plane:service:view"],
};

/// `GET /v1/organizations/{organizationId}/services` (`instanceGetList`).
pub const INSTANCE_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "instanceGetList",
    rust_method: "instance_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services",
    required_permissions: &[],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/password` (`instancePasswordUpdate`).
pub const INSTANCE_PASSWORD_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "instancePasswordUpdate",
    rust_method: "instance_password_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/password",
    required_permissions: &["control-plane:service:manage"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/privateEndpointConfig` (`instancePrivateEndpointConfigGet`).
pub const INSTANCE_PRIVATE_ENDPOINT_CONFIG_GET: OperationMetadata = OperationMetadata {
    operation_id: "instancePrivateEndpointConfigGet",
    rust_method: "instance_private_endpoint_config_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/privateEndpointConfig",
    required_permissions: &["control-plane:service:view-private-endpoints"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/privateEndpoint` (`instancePrivateEndpointCreate`).
pub const INSTANCE_PRIVATE_ENDPOINT_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "instancePrivateEndpointCreate",
    rust_method: "instance_private_endpoint_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/privateEndpoint",
    required_permissions: &["control-plane:service:manage-private-endpoints"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/prometheus` (`instancePrometheusGet`).
pub const INSTANCE_PROMETHEUS_GET: OperationMetadata = OperationMetadata {
    operation_id: "instancePrometheusGet",
    rust_method: "instance_prometheus_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/prometheus",
    required_permissions: &["control-plane:service:view"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/serviceQueryEndpoint` (`instanceQueryEndpointDelete`).
pub const INSTANCE_QUERY_ENDPOINT_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "instanceQueryEndpointDelete",
    rust_method: "instance_query_endpoint_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/serviceQueryEndpoint",
    required_permissions: &["control-plane:service:manage-query-api-endpoints"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/serviceQueryEndpoint` (`instanceQueryEndpointGet`).
pub const INSTANCE_QUERY_ENDPOINT_GET: OperationMetadata = OperationMetadata {
    operation_id: "instanceQueryEndpointGet",
    rust_method: "instance_query_endpoint_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/serviceQueryEndpoint",
    required_permissions: &["control-plane:service:view-query-api-endpoints"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/serviceQueryEndpoint` (`instanceQueryEndpointUpsert`).
pub const INSTANCE_QUERY_ENDPOINT_UPSERT: OperationMetadata = OperationMetadata {
    operation_id: "instanceQueryEndpointUpsert",
    rust_method: "instance_query_endpoint_upsert",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/serviceQueryEndpoint",
    required_permissions: &["control-plane:service:manage-query-api-endpoints"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/replicaScaling` (`instanceReplicaScalingUpdate`).
pub const INSTANCE_REPLICA_SCALING_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "instanceReplicaScalingUpdate",
    rust_method: "instance_replica_scaling_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/replicaScaling",
    required_permissions: &["control-plane:service:manage-scaling-config"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/scaling` (`instanceScalingUpdate`).
pub const INSTANCE_SCALING_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "instanceScalingUpdate",
    rust_method: "instance_scaling_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/scaling",
    required_permissions: &["control-plane:service:manage-scaling-config"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/state` (`instanceStateUpdate`).
pub const INSTANCE_STATE_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "instanceStateUpdate",
    rust_method: "instance_state_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/state",
    required_permissions: &[],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}` (`instanceUpdate`).
pub const INSTANCE_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "instanceUpdate",
    rust_method: "instance_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}",
    required_permissions: &["control-plane:service:manage"],
};

/// `POST /v1/organizations/{organizationId}/invitations` (`invitationCreate`).
pub const INVITATION_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "invitationCreate",
    rust_method: "invitation_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/invitations",
    required_permissions: &["control-plane:organization:manage"],
};

/// `DELETE /v1/organizations/{organizationId}/invitations/{invitationId}` (`invitationDelete`).
pub const INVITATION_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "invitationDelete",
    rust_method: "invitation_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/invitations/{invitationId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/invitations/{invitationId}` (`invitationGet`).
pub const INVITATION_GET: OperationMetadata = OperationMetadata {
    operation_id: "invitationGet",
    rust_method: "invitation_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/invitations/{invitationId}",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/invitations` (`invitationGetList`).
pub const INVITATION_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "invitationGetList",
    rust_method: "invitation_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/invitations",
    required_permissions: &["control-plane:organization:view"],
};

/// `DELETE /v1/organizations/{organizationId}/members/{userId}` (`memberDelete`).
pub const MEMBER_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "memberDelete",
    rust_method: "member_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/members/{userId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/members/{userId}` (`memberGet`).
pub const MEMBER_GET: OperationMetadata = OperationMetadata {
    operation_id: "memberGet",
    rust_method: "member_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/members/{userId}",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/members` (`memberGetList`).
pub const MEMBER_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "memberGetList",
    rust_method: "member_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/members",
    required_permissions: &["control-plane:organization:view"],
};

/// `PATCH /v1/organizations/{organizationId}/members/{userId}` (`memberUpdate`).
pub const MEMBER_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "memberUpdate",
    rust_method: "member_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/members/{userId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `POST /v1/organizations/{organizationId}/keys` (`openapiKeyCreate`).
pub const OPENAPI_KEY_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "openapiKeyCreate",
    rust_method: "openapi_key_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/keys",
    required_permissions: &["control-plane:organization:create-api-keys"],
};

/// `DELETE /v1/organizations/{organizationId}/keys/{keyId}` (`openapiKeyDelete`).
pub const OPENAPI_KEY_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "openapiKeyDelete",
    rust_method: "openapi_key_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/keys/{keyId}",
    required_permissions: &["control-plane:organization:view-api-keys"],
};

/// `GET /v1/organizations/{organizationId}/keys/{keyId}` (`openapiKeyGet`).
pub const OPENAPI_KEY_GET: OperationMetadata = OperationMetadata {
    operation_id: "openapiKeyGet",
    rust_method: "openapi_key_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/keys/{keyId}",
    required_permissions: &["control-plane:organization:view-api-keys"],
};

/// `GET /v1/organizations/{organizationId}/keys` (`openapiKeyGetList`).
pub const OPENAPI_KEY_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "openapiKeyGetList",
    rust_method: "openapi_key_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/keys",
    required_permissions: &["control-plane:organization:view-api-keys"],
};

/// `PATCH /v1/organizations/{organizationId}/keys/{keyId}` (`openapiKeyUpdate`).
pub const OPENAPI_KEY_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "openapiKeyUpdate",
    rust_method: "openapi_key_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/keys/{keyId}",
    required_permissions: &["control-plane:organization:view-api-keys"],
};

/// `POST /v1/organizations/{organizationId}/byocInfrastructure` (`organizationByocInfrastructureCreate`).
pub const ORGANIZATION_BYOC_INFRASTRUCTURE_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "organizationByocInfrastructureCreate",
    rust_method: "organization_byoc_infrastructure_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/byocInfrastructure",
    required_permissions: &["control-plane:organization:manage"],
};

/// `DELETE /v1/organizations/{organizationId}/byocInfrastructure/{byocInfrastructureId}` (`organizationByocInfrastructureDelete`).
pub const ORGANIZATION_BYOC_INFRASTRUCTURE_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "organizationByocInfrastructureDelete",
    rust_method: "organization_byoc_infrastructure_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/byocInfrastructure/{byocInfrastructureId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `PATCH /v1/organizations/{organizationId}/byocInfrastructure/{byocInfrastructureId}` (`organizationByocInfrastructureUpdate`).
pub const ORGANIZATION_BYOC_INFRASTRUCTURE_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "organizationByocInfrastructureUpdate",
    rust_method: "organization_byoc_infrastructure_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/byocInfrastructure/{byocInfrastructureId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}` (`organizationGet`).
pub const ORGANIZATION_GET: OperationMetadata = OperationMetadata {
    operation_id: "organizationGet",
    rust_method: "organization_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations` (`organizationGetList`).
pub const ORGANIZATION_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "organizationGetList",
    rust_method: "organization_get_list",
    method: "GET",
    path: "/v1/organizations",
    required_permissions: &[],
};

/// `GET /v1/organizations/{organizationId}/privateEndpointConfig` (`organizationPrivateEndpointConfigGetList`).
pub const ORGANIZATION_PRIVATE_ENDPOINT_CONFIG_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "organizationPrivateEndpointConfigGetList",
    rust_method: "organization_private_endpoint_config_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/privateEndpointConfig",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/prometheus/discovery` (`organizationPrometheusDiscoveryGet`).
pub const ORGANIZATION_PROMETHEUS_DISCOVERY_GET: OperationMetadata = OperationMetadata {
    operation_id: "organizationPrometheusDiscoveryGet",
    rust_method: "organization_prometheus_discovery_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/prometheus/discovery",
    required_permissions: &[],
};

/// `GET /v1/organizations/{organizationId}/prometheus` (`organizationPrometheusGet`).
pub const ORGANIZATION_PROMETHEUS_GET: OperationMetadata = OperationMetadata {
    operation_id: "organizationPrometheusGet",
    rust_method: "organization_prometheus_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/prometheus",
    required_permissions: &[],
};

/// `GET /v1/organizations/{organizationId}/quotas/{quotaCode}` (`organizationQuotaGet`).
pub const ORGANIZATION_QUOTA_GET: OperationMetadata = OperationMetadata {
    operation_id: "organizationQuotaGet",
    rust_method: "organization_quota_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/quotas/{quotaCode}",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/quotas` (`organizationQuotasGetList`).
pub const ORGANIZATION_QUOTAS_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "organizationQuotasGetList",
    rust_method: "organization_quotas_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/quotas",
    required_permissions: &["control-plane:organization:view"],
};

/// `DELETE /v1/organizations/{organizationId}/roles/{roleId}` (`organizationRoleDelete`).
pub const ORGANIZATION_ROLE_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "organizationRoleDelete",
    rust_method: "organization_role_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/roles/{roleId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/roles/{roleId}` (`organizationRoleGet`).
pub const ORGANIZATION_ROLE_GET: OperationMetadata = OperationMetadata {
    operation_id: "organizationRoleGet",
    rust_method: "organization_role_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/roles/{roleId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `PATCH /v1/organizations/{organizationId}/roles/{roleId}` (`organizationRolePatch`).
pub const ORGANIZATION_ROLE_PATCH: OperationMetadata = OperationMetadata {
    operation_id: "organizationRolePatch",
    rust_method: "organization_role_patch",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/roles/{roleId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `POST /v1/organizations/{organizationId}/roles` (`organizationRolePost`).
pub const ORGANIZATION_ROLE_POST: OperationMetadata = OperationMetadata {
    operation_id: "organizationRolePost",
    rust_method: "organization_role_post",
    method: "POST",
    path: "/v1/organizations/{organizationId}/roles",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/roles` (`organizationRolesGetList`).
pub const ORGANIZATION_ROLES_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "organizationRolesGetList",
    rust_method: "organization_roles_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/roles",
    required_permissions: &["control-plane:organization:manage"],
};

/// `PATCH /v1/organizations/{organizationId}` (`organizationUpdate`).
pub const ORGANIZATION_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "organizationUpdate",
    rust_method: "organization_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/config` (`postgresInstanceConfigGet`).
pub const POSTGRES_INSTANCE_CONFIG_GET: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstanceConfigGet",
    rust_method: "postgres_instance_config_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/config",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `PATCH /v1/organizations/{organizationId}/postgres/{postgresId}/config` (`postgresInstanceConfigPatch`).
pub const POSTGRES_INSTANCE_CONFIG_PATCH: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstanceConfigPatch",
    rust_method: "postgres_instance_config_patch",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/config",
    required_permissions: &["control-plane:postgres-service:manage"],
};

/// `POST /v1/organizations/{organizationId}/postgres/{postgresId}/config` (`postgresInstanceConfigPost`).
pub const POSTGRES_INSTANCE_CONFIG_POST: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstanceConfigPost",
    rust_method: "postgres_instance_config_post",
    method: "POST",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/config",
    required_permissions: &["control-plane:postgres-service:manage"],
};

/// `POST /v1/organizations/{organizationId}/postgres/{postgresId}/readReplica` (`postgresInstanceCreateReadReplica`).
pub const POSTGRES_INSTANCE_CREATE_READ_REPLICA: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstanceCreateReadReplica",
    rust_method: "postgres_instance_create_read_replica",
    method: "POST",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/readReplica",
    required_permissions: &[
        "control-plane:organization:create-service",
        "control-plane:postgres-service:manage",
    ],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/metrics` (`postgresInstanceMetricsGet`).
pub const POSTGRES_INSTANCE_METRICS_GET: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstanceMetricsGet",
    rust_method: "postgres_instance_metrics_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/metrics",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/prometheus` (`postgresInstancePrometheusGet`).
pub const POSTGRES_INSTANCE_PROMETHEUS_GET: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstancePrometheusGet",
    rust_method: "postgres_instance_prometheus_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/prometheus",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `POST /v1/organizations/{organizationId}/postgres/{postgresId}/restoredService` (`postgresInstanceRestore`).
pub const POSTGRES_INSTANCE_RESTORE: OperationMetadata = OperationMetadata {
    operation_id: "postgresInstanceRestore",
    rust_method: "postgres_instance_restore",
    method: "POST",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/restoredService",
    required_permissions: &[
        "control-plane:organization:create-service",
        "control-plane:postgres-service:manage",
    ],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/logs` (`postgresLogsGetList`).
pub const POSTGRES_LOGS_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "postgresLogsGetList",
    rust_method: "postgres_logs_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/logs",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `GET /v1/organizations/{organizationId}/postgres/prometheus` (`postgresOrgPrometheusGet`).
pub const POSTGRES_ORG_PROMETHEUS_GET: OperationMetadata = OperationMetadata {
    operation_id: "postgresOrgPrometheusGet",
    rust_method: "postgres_org_prometheus_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/prometheus",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/caCertificates` (`postgresServiceCertsGet`).
pub const POSTGRES_SERVICE_CERTS_GET: OperationMetadata = OperationMetadata {
    operation_id: "postgresServiceCertsGet",
    rust_method: "postgres_service_certs_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/caCertificates",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `POST /v1/organizations/{organizationId}/postgres` (`postgresServiceCreate`).
pub const POSTGRES_SERVICE_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "postgresServiceCreate",
    rust_method: "postgres_service_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/postgres",
    required_permissions: &["control-plane:organization:create-service"],
};

/// `DELETE /v1/organizations/{organizationId}/postgres/{postgresId}` (`postgresServiceDelete`).
pub const POSTGRES_SERVICE_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "postgresServiceDelete",
    rust_method: "postgres_service_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}",
    required_permissions: &["control-plane:postgres-service:manage"],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}` (`postgresServiceGet`).
pub const POSTGRES_SERVICE_GET: OperationMetadata = OperationMetadata {
    operation_id: "postgresServiceGet",
    rust_method: "postgres_service_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `GET /v1/organizations/{organizationId}/postgres` (`postgresServiceGetList`).
pub const POSTGRES_SERVICE_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "postgresServiceGetList",
    rust_method: "postgres_service_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `PATCH /v1/organizations/{organizationId}/postgres/{postgresId}` (`postgresServicePatch`).
pub const POSTGRES_SERVICE_PATCH: OperationMetadata = OperationMetadata {
    operation_id: "postgresServicePatch",
    rust_method: "postgres_service_patch",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}",
    required_permissions: &["control-plane:postgres-service:manage"],
};

/// `PATCH /v1/organizations/{organizationId}/postgres/{postgresId}/state` (`postgresServicePatchState`).
pub const POSTGRES_SERVICE_PATCH_STATE: OperationMetadata = OperationMetadata {
    operation_id: "postgresServicePatchState",
    rust_method: "postgres_service_patch_state",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/state",
    required_permissions: &["control-plane:postgres-service:manage"],
};

/// `PATCH /v1/organizations/{organizationId}/postgres/{postgresId}/password` (`postgresServiceSetPassword`).
pub const POSTGRES_SERVICE_SET_PASSWORD: OperationMetadata = OperationMetadata {
    operation_id: "postgresServiceSetPassword",
    rust_method: "postgres_service_set_password",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/password",
    required_permissions: &["control-plane:postgres-service:manage"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints` (`queryApiEndpointCreate`).
pub const QUERY_API_ENDPOINT_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "queryApiEndpointCreate",
    rust_method: "query_api_endpoint_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints",
    required_permissions: &["control-plane:service:manage-query-api-endpoints"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints/{endpointId}` (`queryApiEndpointDelete`).
pub const QUERY_API_ENDPOINT_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "queryApiEndpointDelete",
    rust_method: "query_api_endpoint_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints/{endpointId}",
    required_permissions: &["control-plane:service:manage-query-api-endpoints"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints/{endpointId}` (`queryApiEndpointGet`).
pub const QUERY_API_ENDPOINT_GET: OperationMetadata = OperationMetadata {
    operation_id: "queryApiEndpointGet",
    rust_method: "query_api_endpoint_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints/{endpointId}",
    required_permissions: &["control-plane:service:view-query-api-endpoints"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints` (`queryApiEndpointList`).
pub const QUERY_API_ENDPOINT_LIST: OperationMetadata = OperationMetadata {
    operation_id: "queryApiEndpointList",
    rust_method: "query_api_endpoint_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints",
    required_permissions: &["control-plane:service:view-query-api-endpoints"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints/{endpointId}` (`queryApiEndpointUpdate`).
pub const QUERY_API_ENDPOINT_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "queryApiEndpointUpdate",
    rust_method: "query_api_endpoint_update",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/query-api-endpoints/{endpointId}",
    required_permissions: &["control-plane:service:manage-query-api-endpoints"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/scalingSchedule` (`scalingScheduleDelete`).
pub const SCALING_SCHEDULE_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "scalingScheduleDelete",
    rust_method: "scaling_schedule_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/scalingSchedule",
    required_permissions: &["control-plane:service:manage-scaling-config"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/scalingSchedule` (`scalingScheduleGet`).
pub const SCALING_SCHEDULE_GET: OperationMetadata = OperationMetadata {
    operation_id: "scalingScheduleGet",
    rust_method: "scaling_schedule_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/scalingSchedule",
    required_permissions: &["control-plane:service:view"],
};

/// `POST /v1/organizations/{organizationId}/services/{serviceId}/scalingSchedule` (`scalingScheduleUpsert`).
pub const SCALING_SCHEDULE_UPSERT: OperationMetadata = OperationMetadata {
    operation_id: "scalingScheduleUpsert",
    rust_method: "scaling_schedule_upsert",
    method: "POST",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/scalingSchedule",
    required_permissions: &["control-plane:service:manage-scaling-config"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings/{settingName}` (`serviceClickhouseSettingDelete`).
pub const SERVICE_CLICKHOUSE_SETTING_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "serviceClickhouseSettingDelete",
    rust_method: "service_clickhouse_setting_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings/{settingName}",
    required_permissions: &["control-plane:service:manage"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings/{settingName}` (`serviceClickhouseSettingGet`).
pub const SERVICE_CLICKHOUSE_SETTING_GET: OperationMetadata = OperationMetadata {
    operation_id: "serviceClickhouseSettingGet",
    rust_method: "service_clickhouse_setting_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings/{settingName}",
    required_permissions: &["control-plane:service:view"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings` (`serviceClickhouseSettingsListGet`).
pub const SERVICE_CLICKHOUSE_SETTINGS_LIST_GET: OperationMetadata = OperationMetadata {
    operation_id: "serviceClickhouseSettingsListGet",
    rust_method: "service_clickhouse_settings_list_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings",
    required_permissions: &["control-plane:service:view"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings/schema` (`serviceClickhouseSettingsSchemaGet`).
pub const SERVICE_CLICKHOUSE_SETTINGS_SCHEMA_GET: OperationMetadata = OperationMetadata {
    operation_id: "serviceClickhouseSettingsSchemaGet",
    rust_method: "service_clickhouse_settings_schema_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings/schema",
    required_permissions: &["control-plane:service:view"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings` (`serviceClickhouseSettingsUpdate`).
pub const SERVICE_CLICKHOUSE_SETTINGS_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "serviceClickhouseSettingsUpdate",
    rust_method: "service_clickhouse_settings_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/clickhouseSettings",
    required_permissions: &["control-plane:service:manage"],
};

/// `GET /v1/organizations/{organizationId}/serviceProfiles` (`serviceProfilesList`).
pub const SERVICE_PROFILES_LIST: OperationMetadata = OperationMetadata {
    operation_id: "serviceProfilesList",
    rust_method: "service_profiles_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/serviceProfiles",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/slowQueryPatterns/{queryId}` (`slowQueryPatternGet`).
pub const SLOW_QUERY_PATTERN_GET: OperationMetadata = OperationMetadata {
    operation_id: "slowQueryPatternGet",
    rust_method: "slow_query_pattern_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/slowQueryPatterns/{queryId}",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `GET /v1/organizations/{organizationId}/postgres/{postgresId}/slowQueryPatterns` (`slowQueryPatternsGetList`).
pub const SLOW_QUERY_PATTERNS_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "slowQueryPatternsGetList",
    rust_method: "slow_query_patterns_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/postgres/{postgresId}/slowQueryPatterns",
    required_permissions: &["control-plane:postgres-service:view"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/snapshotConfiguration` (`snapshotConfigurationGet`).
pub const SNAPSHOT_CONFIGURATION_GET: OperationMetadata = OperationMetadata {
    operation_id: "snapshotConfigurationGet",
    rust_method: "snapshot_configuration_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/snapshotConfiguration",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `PATCH /v1/organizations/{organizationId}/services/{serviceId}/snapshotConfiguration` (`snapshotConfigurationUpdate`).
pub const SNAPSHOT_CONFIGURATION_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "snapshotConfigurationUpdate",
    rust_method: "snapshot_configuration_update",
    method: "PATCH",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/snapshotConfiguration",
    required_permissions: &["control-plane:service:manage-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/snapshots/{snapshotId}` (`snapshotGet`).
pub const SNAPSHOT_GET: OperationMetadata = OperationMetadata {
    operation_id: "snapshotGet",
    rust_method: "snapshot_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/snapshots/{snapshotId}",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/snapshots` (`snapshotGetList`).
pub const SNAPSHOT_GET_LIST: OperationMetadata = OperationMetadata {
    operation_id: "snapshotGetList",
    rust_method: "snapshot_get_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/snapshots",
    required_permissions: &["control-plane:service:view-backups"],
};

/// `PUT /v1/organizations/{organizationId}/udfs/{functionName}/attachments/{serviceId}` (`udfAttach`).
pub const UDF_ATTACH: OperationMetadata = OperationMetadata {
    operation_id: "udfAttach",
    rust_method: "udf_attach",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/attachments/{serviceId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/udfs/{functionName}/attachments/{serviceId}` (`udfAttachmentGet`).
pub const UDF_ATTACHMENT_GET: OperationMetadata = OperationMetadata {
    operation_id: "udfAttachmentGet",
    rust_method: "udf_attachment_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/attachments/{serviceId}",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/udfs/{functionName}/attachments` (`udfAttachmentList`).
pub const UDF_ATTACHMENT_LIST: OperationMetadata = OperationMetadata {
    operation_id: "udfAttachmentList",
    rust_method: "udf_attachment_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/attachments",
    required_permissions: &["control-plane:organization:view"],
};

/// `POST /v1/organizations/{organizationId}/udfs` (`udfCreate`).
pub const UDF_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "udfCreate",
    rust_method: "udf_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/udfs",
    required_permissions: &["control-plane:organization:manage"],
};

/// `DELETE /v1/organizations/{organizationId}/udfs/{functionName}` (`udfDelete`).
pub const UDF_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "udfDelete",
    rust_method: "udf_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `DELETE /v1/organizations/{organizationId}/udfs/{functionName}/attachments/{serviceId}` (`udfDetach`).
pub const UDF_DETACH: OperationMetadata = OperationMetadata {
    operation_id: "udfDetach",
    rust_method: "udf_detach",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/attachments/{serviceId}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/udfs/{functionName}` (`udfGet`).
pub const UDF_GET: OperationMetadata = OperationMetadata {
    operation_id: "udfGet",
    rust_method: "udf_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}",
    required_permissions: &["control-plane:organization:view"],
};

/// `GET /v1/organizations/{organizationId}/udfs` (`udfList`).
pub const UDF_LIST: OperationMetadata = OperationMetadata {
    operation_id: "udfList",
    rust_method: "udf_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/udfs",
    required_permissions: &["control-plane:organization:view"],
};

/// `POST /v1/organizations/{organizationId}/udfUploads/url` (`udfUploadSessionCreate`).
pub const UDF_UPLOAD_SESSION_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "udfUploadSessionCreate",
    rust_method: "udf_upload_session_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/udfUploads/url",
    required_permissions: &["control-plane:organization:manage"],
};

/// `POST /v1/organizations/{organizationId}/udfs/{functionName}/versions` (`udfVersionCreate`).
pub const UDF_VERSION_CREATE: OperationMetadata = OperationMetadata {
    operation_id: "udfVersionCreate",
    rust_method: "udf_version_create",
    method: "POST",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/versions",
    required_permissions: &["control-plane:organization:manage"],
};

/// `DELETE /v1/organizations/{organizationId}/udfs/{functionName}/versions/{version}` (`udfVersionDelete`).
pub const UDF_VERSION_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "udfVersionDelete",
    rust_method: "udf_version_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/versions/{version}",
    required_permissions: &["control-plane:organization:manage"],
};

/// `GET /v1/organizations/{organizationId}/udfs/{functionName}/versions` (`udfVersionList`).
pub const UDF_VERSION_LIST: OperationMetadata = OperationMetadata {
    operation_id: "udfVersionList",
    rust_method: "udf_version_list",
    method: "GET",
    path: "/v1/organizations/{organizationId}/udfs/{functionName}/versions",
    required_permissions: &["control-plane:organization:view"],
};

/// `DELETE /v1/organizations/{organizationId}/services/{serviceId}/upgradeWindow` (`upgradeWindowDelete`).
pub const UPGRADE_WINDOW_DELETE: OperationMetadata = OperationMetadata {
    operation_id: "upgradeWindowDelete",
    rust_method: "upgrade_window_delete",
    method: "DELETE",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/upgradeWindow",
    required_permissions: &["control-plane:service:manage"],
};

/// `GET /v1/organizations/{organizationId}/services/{serviceId}/upgradeWindow` (`upgradeWindowGet`).
pub const UPGRADE_WINDOW_GET: OperationMetadata = OperationMetadata {
    operation_id: "upgradeWindowGet",
    rust_method: "upgrade_window_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/upgradeWindow",
    required_permissions: &["control-plane:service:view"],
};

/// `PUT /v1/organizations/{organizationId}/services/{serviceId}/upgradeWindow` (`upgradeWindowUpdate`).
pub const UPGRADE_WINDOW_UPDATE: OperationMetadata = OperationMetadata {
    operation_id: "upgradeWindowUpdate",
    rust_method: "upgrade_window_update",
    method: "PUT",
    path: "/v1/organizations/{organizationId}/services/{serviceId}/upgradeWindow",
    required_permissions: &["control-plane:service:manage"],
};

/// `GET /v1/organizations/{organizationId}/usageCost` (`usageCostGet`).
pub const USAGE_COST_GET: OperationMetadata = OperationMetadata {
    operation_id: "usageCostGet",
    rust_method: "usage_cost_get",
    method: "GET",
    path: "/v1/organizations/{organizationId}/usageCost",
    required_permissions: &["control-plane:organization:view"],
};

/// Every supported OpenAPI operation, sorted by exact operation ID.
pub const ALL: &[OperationMetadata] = &[
    ACTIVE_BALANCES_GET,
    ACTIVITY_GET,
    ACTIVITY_GET_LIST,
    BACKUP_BUCKET_CREATE,
    BACKUP_BUCKET_DELETE,
    BACKUP_BUCKET_GET,
    BACKUP_BUCKET_UPDATE,
    BACKUP_CONFIGURATION_GET,
    BACKUP_CONFIGURATION_UPDATE,
    BACKUP_GET,
    BACKUP_GET_LIST,
    CLICK_PIPE_CDC_SCALING_GET,
    CLICK_PIPE_CDC_SCALING_UPDATE,
    CLICK_PIPE_CREATE,
    CLICK_PIPE_DELETE,
    CLICK_PIPE_GET,
    CLICK_PIPE_GET_LIST,
    CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_CREATE,
    CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_DELETE,
    CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_GET,
    CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_GET_LIST,
    CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_UPDATE,
    CLICK_PIPE_SCALING_UPDATE,
    CLICK_PIPE_SCHEMA_DISCOVERY,
    CLICK_PIPE_SETTINGS_GET,
    CLICK_PIPE_SETTINGS_UPDATE,
    CLICK_PIPE_STATE_UPDATE,
    CLICK_PIPE_UPDATE,
    CLICK_PIPES_SERVICE_CONTEXT_GET,
    CLICK_STACK_CREATE_ALERT,
    CLICK_STACK_CREATE_DASHBOARD,
    CLICK_STACK_CREATE_ROLE,
    CLICK_STACK_CREATE_SAVED_SEARCH,
    CLICK_STACK_CREATE_SOURCE,
    CLICK_STACK_CREATE_WEBHOOK,
    CLICK_STACK_DELETE_ALERT,
    CLICK_STACK_DELETE_DASHBOARD,
    CLICK_STACK_DELETE_ROLE,
    CLICK_STACK_DELETE_SAVED_SEARCH,
    CLICK_STACK_DELETE_SOURCE,
    CLICK_STACK_DELETE_WEBHOOK,
    CLICK_STACK_GET_ALERT,
    CLICK_STACK_GET_DASHBOARD,
    CLICK_STACK_GET_ROLE,
    CLICK_STACK_GET_SAVED_SEARCH,
    CLICK_STACK_GET_SOURCE,
    CLICK_STACK_LIST_ALERTS,
    CLICK_STACK_LIST_DASHBOARDS,
    CLICK_STACK_LIST_ROLES,
    CLICK_STACK_LIST_SAVED_SEARCHES,
    CLICK_STACK_LIST_SOURCES,
    CLICK_STACK_LIST_WEBHOOKS,
    CLICK_STACK_UPDATE_ALERT,
    CLICK_STACK_UPDATE_DASHBOARD,
    CLICK_STACK_UPDATE_ROLE,
    CLICK_STACK_UPDATE_SAVED_SEARCH,
    CLICK_STACK_UPDATE_SOURCE,
    CLICK_STACK_UPDATE_WEBHOOK,
    CLICK_STACK_VALIDATE_DASHBOARD,
    CREDIT_BALANCES_GET,
    INSTANCE_CREATE,
    INSTANCE_DELETE,
    INSTANCE_GET,
    INSTANCE_GET_LIST,
    INSTANCE_PASSWORD_UPDATE,
    INSTANCE_PRIVATE_ENDPOINT_CONFIG_GET,
    INSTANCE_PRIVATE_ENDPOINT_CREATE,
    INSTANCE_PROMETHEUS_GET,
    INSTANCE_QUERY_ENDPOINT_DELETE,
    INSTANCE_QUERY_ENDPOINT_GET,
    INSTANCE_QUERY_ENDPOINT_UPSERT,
    INSTANCE_REPLICA_SCALING_UPDATE,
    INSTANCE_SCALING_UPDATE,
    INSTANCE_STATE_UPDATE,
    INSTANCE_UPDATE,
    INVITATION_CREATE,
    INVITATION_DELETE,
    INVITATION_GET,
    INVITATION_GET_LIST,
    MEMBER_DELETE,
    MEMBER_GET,
    MEMBER_GET_LIST,
    MEMBER_UPDATE,
    OPENAPI_KEY_CREATE,
    OPENAPI_KEY_DELETE,
    OPENAPI_KEY_GET,
    OPENAPI_KEY_GET_LIST,
    OPENAPI_KEY_UPDATE,
    ORGANIZATION_BYOC_INFRASTRUCTURE_CREATE,
    ORGANIZATION_BYOC_INFRASTRUCTURE_DELETE,
    ORGANIZATION_BYOC_INFRASTRUCTURE_UPDATE,
    ORGANIZATION_GET,
    ORGANIZATION_GET_LIST,
    ORGANIZATION_PRIVATE_ENDPOINT_CONFIG_GET_LIST,
    ORGANIZATION_PROMETHEUS_DISCOVERY_GET,
    ORGANIZATION_PROMETHEUS_GET,
    ORGANIZATION_QUOTA_GET,
    ORGANIZATION_QUOTAS_GET_LIST,
    ORGANIZATION_ROLE_DELETE,
    ORGANIZATION_ROLE_GET,
    ORGANIZATION_ROLE_PATCH,
    ORGANIZATION_ROLE_POST,
    ORGANIZATION_ROLES_GET_LIST,
    ORGANIZATION_UPDATE,
    POSTGRES_INSTANCE_CONFIG_GET,
    POSTGRES_INSTANCE_CONFIG_PATCH,
    POSTGRES_INSTANCE_CONFIG_POST,
    POSTGRES_INSTANCE_CREATE_READ_REPLICA,
    POSTGRES_INSTANCE_METRICS_GET,
    POSTGRES_INSTANCE_PROMETHEUS_GET,
    POSTGRES_INSTANCE_RESTORE,
    POSTGRES_LOGS_GET_LIST,
    POSTGRES_ORG_PROMETHEUS_GET,
    POSTGRES_SERVICE_CERTS_GET,
    POSTGRES_SERVICE_CREATE,
    POSTGRES_SERVICE_DELETE,
    POSTGRES_SERVICE_GET,
    POSTGRES_SERVICE_GET_LIST,
    POSTGRES_SERVICE_PATCH,
    POSTGRES_SERVICE_PATCH_STATE,
    POSTGRES_SERVICE_SET_PASSWORD,
    QUERY_API_ENDPOINT_CREATE,
    QUERY_API_ENDPOINT_DELETE,
    QUERY_API_ENDPOINT_GET,
    QUERY_API_ENDPOINT_LIST,
    QUERY_API_ENDPOINT_UPDATE,
    SCALING_SCHEDULE_DELETE,
    SCALING_SCHEDULE_GET,
    SCALING_SCHEDULE_UPSERT,
    SERVICE_CLICKHOUSE_SETTING_DELETE,
    SERVICE_CLICKHOUSE_SETTING_GET,
    SERVICE_CLICKHOUSE_SETTINGS_LIST_GET,
    SERVICE_CLICKHOUSE_SETTINGS_SCHEMA_GET,
    SERVICE_CLICKHOUSE_SETTINGS_UPDATE,
    SERVICE_PROFILES_LIST,
    SLOW_QUERY_PATTERN_GET,
    SLOW_QUERY_PATTERNS_GET_LIST,
    SNAPSHOT_CONFIGURATION_GET,
    SNAPSHOT_CONFIGURATION_UPDATE,
    SNAPSHOT_GET,
    SNAPSHOT_GET_LIST,
    UDF_ATTACH,
    UDF_ATTACHMENT_GET,
    UDF_ATTACHMENT_LIST,
    UDF_CREATE,
    UDF_DELETE,
    UDF_DETACH,
    UDF_GET,
    UDF_LIST,
    UDF_UPLOAD_SESSION_CREATE,
    UDF_VERSION_CREATE,
    UDF_VERSION_DELETE,
    UDF_VERSION_LIST,
    UPGRADE_WINDOW_DELETE,
    UPGRADE_WINDOW_GET,
    UPGRADE_WINDOW_UPDATE,
    USAGE_COST_GET,
];

/// Look up an exact OpenAPI operation ID. Unknown IDs return `None`.
pub fn by_operation_id(operation_id: &str) -> Option<&'static OperationMetadata> {
    ALL.binary_search_by_key(&operation_id, |operation| operation.operation_id)
        .ok()
        .map(|index| &ALL[index])
}
