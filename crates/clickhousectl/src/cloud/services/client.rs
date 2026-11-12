use super::lifecycle::{
    build_service_state_patch_request, service_delete_error, service_delete_rejected,
};
use super::query_key::{KeyPropagation, upsert_until_key_propagates};
use crate::cloud::client::{CloudClient, CloudError, ResourceKind, ResourceLookup};
use crate::cloud::output::or_absent;
use crate::cloud::types::DeleteResponse;
use clickhouse_cloud_api::models::{
    InstanceServiceQueryApiEndpointsPostRequest, JwtProvider, JwtProviderPostRequest,
    JwtProviderPutRequest, ReplicaAwareRouting, ReplicaAwareRoutingPatchRequest, ScalingSchedule,
    ScalingSchedulePostRequest, ServicPrivateEndpointePostRequest, Service,
    ServiceClickhouseSetting, ServiceClickhouseSettingsList, ServiceClickhouseSettingsPatchRequest,
    ServiceClickhouseSettingsPatchResponse, ServiceClickhouseSettingsSchema,
    ServicePasswordPatchRequest, ServicePatchRequest, ServicePostRequest, ServiceProfile,
    ServiceReplicaScalingPatchRequest, ServiceStatePatchRequestCommand, UpgradeWindow,
    UpgradeWindowPutRequest,
};

impl CloudClient {
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
            .request_on_service(
                org_id,
                service_id,
                self.api().scaling_schedule_get(org_id, service_id),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn set_scaling_schedule(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ScalingSchedulePostRequest,
    ) -> crate::cloud::client::Result<ScalingSchedule> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .scaling_schedule_upsert(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn delete_scaling_schedule(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().scaling_schedule_delete(org_id, service_id),
            )
            .await?;
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
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
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
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickhouse_setting(
        &self,
        org_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> crate::cloud::client::Result<ServiceClickhouseSetting> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickHouseSetting,
                    setting_name,
                    org_id,
                    service_id,
                ),
                self.api()
                    .service_clickhouse_setting_get(org_id, service_id, setting_name),
            )
            .await?;
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
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickhouse_settings(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServiceClickhouseSettingsPatchRequest,
    ) -> crate::cloud::client::Result<ServiceClickhouseSettingsPatchResponse> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .service_clickhouse_settings_update(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn reset_clickhouse_setting(
        &self,
        org_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickHouseSetting,
                    setting_name,
                    org_id,
                    service_id,
                ),
                self.api()
                    .service_clickhouse_setting_delete(org_id, service_id, setting_name),
            )
            .await?;
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
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
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
            .request_on_service(
                org_id,
                service_id,
                self.api().upgrade_window_get(org_id, service_id),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn set_upgrade_window(
        &self,
        org_id: &str,
        service_id: &str,
        request: &UpgradeWindowPutRequest,
    ) -> crate::cloud::client::Result<UpgradeWindow> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .upgrade_window_update(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn get_replica_aware_routing(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<ReplicaAwareRouting> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().replica_aware_routing_get(org_id, service_id),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn update_replica_aware_routing(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ReplicaAwareRoutingPatchRequest,
    ) -> crate::cloud::client::Result<ReplicaAwareRouting> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .replica_aware_routing_update(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn delete_upgrade_window(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().upgrade_window_delete(org_id, service_id),
            )
            .await?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_jwt_providers(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Vec<JwtProvider>> {
        let response = self
            .api()
            .jwt_provider_get_list(org_id, service_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_jwt_provider(
        &self,
        org_id: &str,
        service_id: &str,
        provider_name: &str,
    ) -> crate::cloud::client::Result<JwtProvider> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::JwtProvider,
                    provider_name,
                    org_id,
                    service_id,
                ),
                self.api()
                    .jwt_provider_get(org_id, service_id, provider_name),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn create_jwt_provider(
        &self,
        org_id: &str,
        service_id: &str,
        request: &JwtProviderPostRequest,
    ) -> crate::cloud::client::Result<JwtProvider> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().jwt_provider_create(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn update_jwt_provider(
        &self,
        org_id: &str,
        service_id: &str,
        provider_name: &str,
        request: &JwtProviderPutRequest,
    ) -> crate::cloud::client::Result<JwtProvider> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::JwtProvider,
                    provider_name,
                    org_id,
                    service_id,
                ),
                self.api()
                    .jwt_provider_update(org_id, service_id, provider_name, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn delete_jwt_provider(
        &self,
        org_id: &str,
        service_id: &str,
        provider_name: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::JwtProvider,
                    provider_name,
                    org_id,
                    service_id,
                ),
                self.api()
                    .jwt_provider_delete(org_id, service_id, provider_name),
            )
            .await?;
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
        let response = match self.api().instance_delete(org_id, service_id).await {
            Ok(response) => response,
            Err(error @ clickhouse_cloud_api::Error::Api { status: 400, .. }) => {
                return Err(self
                    .service_delete_bad_request(error, org_id, service_id)
                    .await);
            }
            Err(error) => return Err(service_delete_error(self, error, force, service_id, org_id)),
        };
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    /// A 400 from a service DELETE. It names a missing service only once a
    /// read of the service agrees (#666); for a service that exists, such as
    /// one still provisioning, it names the state instead (#1226). When the
    /// read fails too, the original error is reported unchanged.
    async fn service_delete_bad_request(
        &self,
        error: clickhouse_cloud_api::Error,
        org_id: &str,
        service_id: &str,
    ) -> CloudError {
        match self.get_service_if_exists(org_id, service_id).await {
            Ok(None) => self.convert_error_for_lookup(
                error,
                ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
            ),
            Ok(Some(service)) => service_delete_rejected(
                self.convert_error_for_organization(error, org_id),
                service_id,
                &or_absent(service.state.as_ref()),
            ),
            Err(_) => self.convert_error_for_organization(error, org_id),
        }
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
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn update_service(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicePatchRequest,
    ) -> crate::cloud::client::Result<Service> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().instance_update(org_id, service_id, request),
            )
            .await?;
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
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .instance_replica_scaling_update(org_id, service_id, request),
            )
            .await?;
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
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .instance_password_update(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn get_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        // A 404 can mean the endpoint is absent on an existing service, so
        // only a read of the service can say the service is missing.
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().instance_query_endpoint_get(org_id, service_id),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn create_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &InstanceServiceQueryApiEndpointsPostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .instance_query_endpoint_upsert(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// The service's query endpoint, or `None` when the service has none yet.
    /// Only a 404 means "none": any other failure is reported.
    pub(super) async fn get_query_endpoint_for_binding(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Option<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint>>
    {
        match self
            .api()
            .instance_query_endpoint_get(org_id, service_id)
            .await
        {
            Ok(response) => Self::unwrap_response(response).map(Some),
            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(None),
            Err(error) => Err(self.convert_error_for_organization(error, org_id)),
        }
    }

    /// Upsert the endpoint configuration that binds the caller's own key on
    /// first use (#1043), waiting out key propagation (#658). Everything else
    /// about the request is the caller's business.
    pub(super) async fn upsert_query_endpoint_binding(
        &self,
        org_id: &str,
        service_id: &str,
        request: &InstanceServiceQueryApiEndpointsPostRequest,
        propagation: KeyPropagation,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = upsert_until_key_propagates(propagation, || {
            self.api()
                .instance_query_endpoint_upsert(org_id, service_id, request)
        })
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
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .instance_query_endpoint_delete(org_id, service_id),
            )
            .await?;
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
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .instance_private_endpoint_create(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_private_endpoint_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::PrivateEndpointConfig> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .instance_private_endpoint_config_get(org_id, service_id),
            )
            .await?;
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
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                )
            })
    }
}
