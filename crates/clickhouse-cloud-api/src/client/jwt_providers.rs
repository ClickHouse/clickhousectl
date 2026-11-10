use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// List a service's JWT authentication providers (Beta).
    ///
    /// A service has at most 5 providers. Requires the Enterprise tier.
    pub async fn jwt_provider_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<JwtProvider>>, Error> {
        let req = self.request(
            &operations::JWT_PROVIDER_GET_LIST,
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

    /// Add a JWT authentication provider to a service (Beta).
    ///
    /// The server fetches and validates the JWKS URL before saving. Repeating
    /// a create with the same name and configuration succeeds unchanged; the
    /// same name with a different configuration is rejected with 409.
    pub async fn jwt_provider_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &JwtProviderPostRequest,
    ) -> Result<ApiResponse<JwtProvider>, Error> {
        let req = self
            .request(
                &operations::JWT_PROVIDER_CREATE,
                &[organization_id, service_id],
            )?
            .json(body);
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

    /// Get one JWT authentication provider of a service (Beta).
    pub async fn jwt_provider_get(
        &self,
        organization_id: &str,
        service_id: &str,
        jwt_provider_name: &str,
    ) -> Result<ApiResponse<JwtProvider>, Error> {
        let req = self.request(
            &operations::JWT_PROVIDER_GET,
            &[organization_id, service_id, jwt_provider_name],
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

    /// Replace a JWT authentication provider's configuration (Beta).
    ///
    /// The request replaces the whole configuration, so omitting
    /// `roles_claim` removes an existing roles claim.
    pub async fn jwt_provider_update(
        &self,
        organization_id: &str,
        service_id: &str,
        jwt_provider_name: &str,
        body: &JwtProviderPutRequest,
    ) -> Result<ApiResponse<JwtProvider>, Error> {
        let req = self
            .request(
                &operations::JWT_PROVIDER_UPDATE,
                &[organization_id, service_id, jwt_provider_name],
            )?
            .json(body);
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

    /// Remove a JWT authentication provider from a service (Beta).
    ///
    /// Tokens issued by the provider are no longer accepted. The response
    /// carries no `result`.
    pub async fn jwt_provider_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        jwt_provider_name: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::JWT_PROVIDER_DELETE,
            &[organization_id, service_id, jwt_provider_name],
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
}
