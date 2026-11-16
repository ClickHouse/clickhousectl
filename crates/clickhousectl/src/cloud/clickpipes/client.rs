use crate::cloud::client::{CloudClient, ResourceKind, ResourceLookup};

impl CloudClient {
    pub async fn get_clickpipe_service_context(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipesServiceContext> {
        let response = self
            .api()
            .click_pipes_service_context_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Every ClickPipe of the service, read across all pages.
    pub async fn list_clickpipes(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::ClickPipe>> {
        Self::collect_cursor_pages("ClickPipe", |cursor| async move {
            self.api()
                .click_pipe_get_list(org_id, service_id, None, cursor.as_deref())
                .await
                .map_err(|error| {
                    self.convert_error_for_resource(
                        error,
                        ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
                    )
                })
        })
        .await
    }

    pub async fn get_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api().click_pipe_get(org_id, service_id, clickpipe_id),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn create_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api().click_pipe_create(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipePatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api()
                    .click_pipe_update(org_id, service_id, clickpipe_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn delete_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> crate::cloud::client::Result<Option<String>> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api()
                    .click_pipe_delete(org_id, service_id, clickpipe_id),
            )
            .await?;
        Ok(response.result.and_then(|result| {
            result
                .get("name")
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
        }))
    }

    pub async fn change_clickpipe_state(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        command: clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        use clickhouse_cloud_api::models::ClickPipeStatePatchRequest;
        let request = ClickPipeStatePatchRequest {
            command: Some(command),
        };
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api()
                    .click_pipe_state_update(org_id, service_id, clickpipe_id, &request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeScalingPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api()
                    .click_pipe_scaling_update(org_id, service_id, clickpipe_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe_cdc_scaling(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipesCdcScaling> {
        let response = self
            .api()
            .click_pipe_cdc_scaling_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_cdc_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipesCdcScalingPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipesCdcScaling> {
        let response = self
            .api()
            .click_pipe_cdc_scaling_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe_settings(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipeSettingsResponse> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api()
                    .click_pipe_settings_get(org_id, service_id, clickpipe_id),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_settings(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeSettingsPutRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipeSettingsResponse> {
        let response = self
            .request_in_service(
                ResourceLookup::in_service(
                    ResourceKind::ClickPipe,
                    clickpipe_id,
                    org_id,
                    service_id,
                ),
                self.api()
                    .click_pipe_settings_update(org_id, service_id, clickpipe_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }

    pub async fn click_pipe_schema_discovery(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryResponse>
    {
        let response = self
            .request_on_service(
                org_id,
                service_id,
                self.api()
                    .click_pipe_schema_discovery(org_id, service_id, request),
            )
            .await?;
        Self::unwrap_response(response)
    }
}
