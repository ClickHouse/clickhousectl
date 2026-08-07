use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// List ClickPipes
    pub async fn click_pipe_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickPipe>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }
    /// Create ClickPipe
    pub async fn click_pipe_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickPipePostRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get ClickPipe
    pub async fn click_pipe_get(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe
    pub async fn click_pipe_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipePatchRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}"
        );
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete ClickPipe
    pub async fn click_pipe_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}"
        );
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe scaling
    pub async fn click_pipe_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipeScalingPatchRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/scaling"
        );
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get ClickPipe settings
    pub async fn click_pipe_settings_get(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
    ) -> Result<ApiResponse<ClickPipeSettingsResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/settings"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe settings
    pub async fn click_pipe_settings_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipeSettingsPutRequest,
    ) -> Result<ApiResponse<ClickPipeSettingsResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/settings"
        );
        let mut req = self.request(reqwest::Method::PUT, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe state
    pub async fn click_pipe_state_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipeStatePatchRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/state"
        );
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get CDC ClickPipes scaling
    pub async fn click_pipe_cdc_scaling_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ClickPipesCdcScaling>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesCdcScaling"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update CDC ClickPipes scaling
    pub async fn click_pipe_cdc_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickPipesCdcScalingPatchRequest,
    ) -> Result<ApiResponse<ClickPipesCdcScaling>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesCdcScaling"
        );
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List reverse private endpoints
    pub async fn click_pipe_reverse_private_endpoint_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ReversePrivateEndpoint>>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &CreateReversePrivateEndpoint,
    ) -> Result<ApiResponse<ReversePrivateEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints"
        );
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
        reverse_private_endpoint_id: &str,
    ) -> Result<ApiResponse<ReversePrivateEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints/{reverse_private_endpoint_id}"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        reverse_private_endpoint_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints/{reverse_private_endpoint_id}"
        );
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_update(
        &self,
        organization_id: &str,
        service_id: &str,
        reverse_private_endpoint_id: &str,
        body: &UpdateReversePrivateEndpoint,
    ) -> Result<ApiResponse<ReversePrivateEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints/{reverse_private_endpoint_id}"
        );
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Discover ClickPipe source schema (Beta).
    pub async fn click_pipe_schema_discovery(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickPipeSchemaDiscoveryRequest,
    ) -> Result<ApiResponse<ClickPipeSchemaDiscoveryResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickpipes/schemaDiscovery"
        );
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }
}
