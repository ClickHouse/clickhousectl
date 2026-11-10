use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// Create BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_create(
        &self,
        organization_id: &str,
        body: &ByocInfrastructurePostRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let mut req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_CREATE,
            &[organization_id],
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

    /// Validate BYOC infrastructure prerequisites
    ///
    /// Preflight-checks the cloud account's permissions without creating infrastructure.
    pub async fn organization_byoc_infrastructure_validate(
        &self,
        organization_id: &str,
        body: &ByocInfrastructureValidatePostRequest,
    ) -> Result<ApiResponse<ByocInfrastructureValidation>, Error> {
        let mut req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_VALIDATE,
            &[organization_id],
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

    /// Get BYOC infrastructure details
    pub async fn organization_byoc_infrastructure_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureDetails>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_GET,
            &[organization_id, byoc_infrastructure_id],
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

    /// Get BYOC infrastructure provisioning progress
    pub async fn organization_byoc_infrastructure_progress_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureProgress>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_PROGRESS_GET,
            &[organization_id, byoc_infrastructure_id],
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

    /// Get BYOC infrastructure private endpoint configuration
    ///
    /// For BYOC organizations this replaces the deprecated organization-level
    /// `organization_private_endpoint_config_get_list`.
    pub async fn organization_byoc_infrastructure_private_endpoint_config_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructurePrivateEndpointConfig>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_PRIVATE_ENDPOINT_CONFIG_GET,
            &[organization_id, byoc_infrastructure_id],
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

    /// Get BYOC infrastructure tags
    ///
    /// Tags are replaced as a whole through `organization_byoc_infrastructure_update`.
    pub async fn organization_byoc_infrastructure_tags_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureTagsResponse>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_TAGS_GET,
            &[organization_id, byoc_infrastructure_id],
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

    /// Remove a BYOC infrastructure
    pub async fn organization_byoc_infrastructure_delete(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_DELETE,
            &[organization_id, byoc_infrastructure_id],
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

    /// Update BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_update(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
        body: &ByocInfrastructurePatchRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let mut req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_UPDATE,
            &[organization_id, byoc_infrastructure_id],
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

    /// Get BYOC infrastructure customer-authorized access (Beta)
    ///
    /// Reports when ClickHouse personnel may access the infrastructure and
    /// whether access is allowed right now.
    pub async fn organization_byoc_infrastructure_customer_authorized_access_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocCustomerAuthorizedAccess>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_CUSTOMER_AUTHORIZED_ACCESS_GET,
            &[organization_id, byoc_infrastructure_id],
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

    /// Update BYOC infrastructure customer-authorized access (Beta)
    pub async fn organization_byoc_infrastructure_customer_authorized_access_update(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
        body: &ByocCustomerAuthorizedAccessPatchRequest,
    ) -> Result<ApiResponse<ByocCustomerAuthorizedAccess>, Error> {
        let mut req = self.request(
            &operations::ORGANIZATION_BYOC_INFRASTRUCTURE_CUSTOMER_AUTHORIZED_ACCESS_UPDATE,
            &[organization_id, byoc_infrastructure_id],
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
