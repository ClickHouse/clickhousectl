use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// Get ClickPipes service capabilities and workload identity.
    pub async fn click_pipes_service_context_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ClickPipesServiceContext>, Error> {
        let req = self.request(
            &operations::CLICK_PIPES_SERVICE_CONTEXT_GET,
            &[organization_id, service_id],
        )?;

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

    /// List ClickPipes.
    ///
    /// With neither `limit` nor `cursor`, the server returns every ClickPipe and
    /// `next_cursor` is `None`. Either one requests a page, most recent first:
    /// `limit` must be at least 1 (server default 50, capped at its maximum page
    /// size). Pass the response's `next_cursor` as `cursor` until it is `None`.
    pub async fn click_pipe_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ApiResponse<Vec<ClickPipe>>, Error> {
        let mut req = self.request(
            &operations::CLICK_PIPE_GET_LIST,
            &[organization_id, service_id],
        )?;
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(cursor) = cursor {
            req = req.query(&[("cursor", cursor)]);
        }
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
        let mut req = self.request(
            &operations::CLICK_PIPE_CREATE,
            &[organization_id, service_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_GET,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_UPDATE,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_DELETE,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_SCALING_UPDATE,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_SETTINGS_GET,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_SETTINGS_UPDATE,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_STATE_UPDATE,
            &[organization_id, service_id, click_pipe_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_CDC_SCALING_GET,
            &[organization_id, service_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_CDC_SCALING_UPDATE,
            &[organization_id, service_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_GET_LIST,
            &[organization_id, service_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_CREATE,
            &[organization_id, service_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_GET,
            &[organization_id, service_id, reverse_private_endpoint_id],
        )?;
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
        let req = self.request(
            &operations::CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_DELETE,
            &[organization_id, service_id, reverse_private_endpoint_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_REVERSE_PRIVATE_ENDPOINT_UPDATE,
            &[organization_id, service_id, reverse_private_endpoint_id],
        )?;
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
        let mut req = self.request(
            &operations::CLICK_PIPE_SCHEMA_DISCOVERY,
            &[organization_id, service_id],
        )?;
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
