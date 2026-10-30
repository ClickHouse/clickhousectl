use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Create BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_create(
        &self,
        organization_id: &str,
        body: &ByocInfrastructurePostRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure");
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

    /// Validate BYOC infrastructure prerequisites
    ///
    /// Preflight-checks the cloud account's permissions without creating infrastructure.
    pub async fn organization_byoc_infrastructure_validate(
        &self,
        organization_id: &str,
        body: &ByocInfrastructureValidatePostRequest,
    ) -> Result<ApiResponse<ByocInfrastructureValidation>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure/validate");
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

    /// Get BYOC infrastructure details
    pub async fn organization_byoc_infrastructure_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureDetails>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}"
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

    /// Get BYOC infrastructure provisioning progress
    pub async fn organization_byoc_infrastructure_progress_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureProgress>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}/progress"
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

    /// Get BYOC infrastructure private endpoint configuration
    ///
    /// For BYOC organizations this replaces the deprecated organization-level
    /// `organization_private_endpoint_config_get_list`.
    pub async fn organization_byoc_infrastructure_private_endpoint_config_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructurePrivateEndpointConfig>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}/privateEndpointConfig"
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

    /// Get BYOC infrastructure tags
    ///
    /// Tags are replaced as a whole through `organization_byoc_infrastructure_update`.
    pub async fn organization_byoc_infrastructure_tags_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureTagsResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}/tags"
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

    /// Remove a BYOC infrastructure
    pub async fn organization_byoc_infrastructure_delete(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}"
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

    /// Update BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_update(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
        body: &ByocInfrastructurePatchRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}"
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
}
