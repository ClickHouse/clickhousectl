use super::lifecycle::{build_service_state_patch_request, service_delete_error};
use crate::cloud::client::{CloudClient, ResourceKind, ResourceLookup};
use crate::cloud::types::DeleteResponse;
use clickhouse_cloud_api::models::{
    InstanceServiceQueryApiEndpointsPostRequest, ReplicaAwareRouting,
    ReplicaAwareRoutingPatchRequest, ScalingSchedule, ScalingSchedulePostRequest,
    ServicPrivateEndpointePostRequest, Service, ServiceClickhouseSetting,
    ServiceClickhouseSettingsList, ServiceClickhouseSettingsPatchRequest,
    ServiceClickhouseSettingsPatchResponse, ServiceClickhouseSettingsSchema,
    ServicePasswordPatchRequest, ServicePatchRequest, ServicePostRequest, ServiceProfile,
    ServiceReplicaScalingPatchRequest, ServiceStatePatchRequestCommand, UpgradeWindow,
    UpgradeWindowPutRequest,
};

impl CloudClient {
    /// The Query API host the library would send queries to, when one is
    /// configured or derivable from the control-plane URL.
    pub(super) fn configured_query_host(&self) -> Option<String> {
        self.api().configured_query_host()
    }

    /// Send one statement to a service's Query API with an API key.
    ///
    /// Unlike every other wrapper, this returns the library error as-is and
    /// the response unread: the caller streams the body, and the query paths
    /// classify the raw error themselves (idle and stopped services, wake
    /// retries, readiness polling, key rejections) before converting it
    /// through `convert_query_error`, which records the telemetry failure.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_service_query(
        &self,
        service_id: &str,
        key_id: &str,
        key_secret: &str,
        sql: &str,
        database: Option<&str>,
        format: &str,
        wake_service: bool,
    ) -> Result<reqwest::Response, clickhouse_cloud_api::Error> {
        self.api()
            .run_query(
                service_id,
                key_id,
                key_secret,
                sql,
                database,
                format,
                wake_service,
            )
            .await
    }

    /// [`Self::run_service_query`] with the client's OAuth bearer token,
    /// returning the raw library error for the same reason.
    pub(super) async fn run_service_query_bearer(
        &self,
        service_id: &str,
        sql: &str,
        database: Option<&str>,
        format: &str,
        wake_service: bool,
    ) -> Result<reqwest::Response, clickhouse_cloud_api::Error> {
        self.api()
            .run_query_bearer(service_id, sql, database, format, wake_service)
            .await
    }

    pub async fn get_scaling_schedule(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<ScalingSchedule> {
        let response = self
            .api()
            .scaling_schedule_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn set_scaling_schedule(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ScalingSchedulePostRequest,
    ) -> crate::cloud::client::Result<ScalingSchedule> {
        let response = self
            .api()
            .scaling_schedule_upsert(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_scaling_schedule(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .scaling_schedule_delete(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_service_profiles(
        &self,
        org_id: &str,
        region_id: Option<&str>,
        byoc_id: Option<&str>,
    ) -> crate::cloud::client::Result<Vec<ServiceProfile>> {
        let response = self
            .api()
            .service_profiles_list(org_id, region_id, byoc_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn list_services(&self, org_id: &str) -> crate::cloud::client::Result<Vec<Service>> {
        let response = self
            .api()
            .instance_get_list(org_id, &[])
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn list_clickhouse_settings(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<ServiceClickhouseSettingsList> {
        let response = self
            .api()
            .service_clickhouse_settings_list_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickhouse_setting(
        &self,
        org_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> crate::cloud::client::Result<ServiceClickhouseSetting> {
        let response = self
            .api()
            .service_clickhouse_setting_get(org_id, service_id, setting_name)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickhouse_settings_schema(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<ServiceClickhouseSettingsSchema> {
        let response = self
            .api()
            .service_clickhouse_settings_schema_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickhouse_settings(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServiceClickhouseSettingsPatchRequest,
    ) -> crate::cloud::client::Result<ServiceClickhouseSettingsPatchResponse> {
        let response = self
            .api()
            .service_clickhouse_settings_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn reset_clickhouse_setting(
        &self,
        org_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .service_clickhouse_setting_delete(org_id, service_id, setting_name)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_services_filtered(
        &self,
        org_id: &str,
        filters: &[String],
    ) -> crate::cloud::client::Result<Vec<Service>> {
        let filters: Vec<&str> = filters.iter().map(String::as_str).collect();
        let response = self
            .api()
            .instance_get_list(org_id, &filters)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Service> {
        let response = self
            .api()
            .instance_get(org_id, service_id)
            .await
            .map_err(|error| {
                // This path identifies the service itself, not a subresource.
                self.convert_error_for_lookup(
                    error,
                    ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_upgrade_window(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<UpgradeWindow> {
        let response = self
            .api()
            .upgrade_window_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn set_upgrade_window(
        &self,
        org_id: &str,
        service_id: &str,
        request: &UpgradeWindowPutRequest,
    ) -> crate::cloud::client::Result<UpgradeWindow> {
        let response = self
            .api()
            .upgrade_window_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_replica_aware_routing(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<ReplicaAwareRouting> {
        let response = self
            .api()
            .replica_aware_routing_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_replica_aware_routing(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ReplicaAwareRoutingPatchRequest,
    ) -> crate::cloud::client::Result<ReplicaAwareRouting> {
        let response = self
            .api()
            .replica_aware_routing_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_upgrade_window(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .upgrade_window_delete(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn get_service_if_exists(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Option<Service>> {
        let lookup = ResourceLookup::in_org(ResourceKind::Service, service_id, org_id);
        match self.api().instance_get(org_id, service_id).await {
            Ok(response) => Self::unwrap_response(response).map(Some),
            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(None),
            // The API answers 400 rather than 404 for a well-formed id no
            // service has, and this GET has no other user-controlled input,
            // so that rejection is its way of saying the same thing (#666).
            // Without this, `service delete --force` aborted on an id
            // `service get` reports as missing.
            Err(error) if lookup.rejected_well_formed_ids(&error) => Ok(None),
            Err(error) => Err(self.convert_error_for_lookup(error, lookup)),
        }
    }

    pub async fn create_service(
        &self,
        org_id: &str,
        request: &ServicePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServicePostResponse> {
        let response = self
            .api()
            .instance_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_service(
        &self,
        org_id: &str,
        service_id: &str,
        force: bool,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .instance_delete(org_id, service_id)
            .await
            .map_err(|error| service_delete_error(self, error, force, service_id, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn change_service_state(
        &self,
        org_id: &str,
        service_id: &str,
        command: ServiceStatePatchRequestCommand,
    ) -> crate::cloud::client::Result<Service> {
        let request = build_service_state_patch_request(command);
        let response = self
            .api()
            .instance_state_update(org_id, service_id, &request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_service(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicePatchRequest,
    ) -> crate::cloud::client::Result<Service> {
        let response = self
            .api()
            .instance_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_replica_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServiceReplicaScalingPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceScalingPatchResponse>
    {
        let response = self
            .api()
            .instance_replica_scaling_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn reset_password(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicePasswordPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServicePasswordPatchResponse>
    {
        let response = self
            .api()
            .instance_password_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        match self
            .api()
            .instance_query_endpoint_get(org_id, service_id)
            .await
        {
            Ok(response) => Self::unwrap_response(response),
            Err(error) => {
                // A 404 can mean the endpoint is absent on an existing service.
                // Only the service lookup can establish that its parent is missing.
                let lookup = ResourceLookup::in_org(ResourceKind::Service, service_id, org_id);
                if matches!(&error, clickhouse_cloud_api::Error::Api { status: 404, .. })
                    && lookup.rejected_well_formed_ids(&error)
                {
                    self.get_service(org_id, service_id).await?;
                }
                Err(self.convert_error_for_organization(error, org_id))
            }
        }
    }

    pub async fn create_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &InstanceServiceQueryApiEndpointsPostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = self
            .api()
            .instance_query_endpoint_upsert(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .instance_query_endpoint_delete(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn create_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicPrivateEndpointePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::InstancePrivateEndpoint> {
        let response = self
            .api()
            .instance_private_endpoint_create(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_private_endpoint_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::PrivateEndpointConfig> {
        let response = self
            .api()
            .instance_private_endpoint_config_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_prometheus(
        &self,
        org_id: &str,
        service_id: &str,
        filtered_metrics: Option<bool>,
    ) -> crate::cloud::client::Result<String> {
        let filtered = filtered_metrics.map(|value| value.to_string());
        self.api()
            .instance_prometheus_get(org_id, service_id, filtered.as_deref())
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))
    }
}
