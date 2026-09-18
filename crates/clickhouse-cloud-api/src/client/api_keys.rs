use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Get list of all keys
    pub async fn openapi_key_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<ApiKey>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys");
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
    /// Create key
    pub async fn openapi_key_create(
        &self,
        organization_id: &str,
        body: &ApiKeyPostRequest,
    ) -> Result<ApiResponse<ApiKeyPostResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys");
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

    /// Get key details
    pub async fn openapi_key_get(
        &self,
        organization_id: &str,
        key_id: &str,
    ) -> Result<ApiResponse<ApiKey>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys/{key_id}");
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

    /// Update key
    pub async fn openapi_key_update(
        &self,
        organization_id: &str,
        key_id: &str,
        body: &ApiKeyPatchRequest,
    ) -> Result<ApiResponse<ApiKey>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys/{key_id}");
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

    /// Delete key
    pub async fn openapi_key_delete(
        &self,
        organization_id: &str,
        key_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys/{key_id}");
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
}
