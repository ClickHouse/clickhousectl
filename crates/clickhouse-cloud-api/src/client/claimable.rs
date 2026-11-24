use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::{ApiResponse, ClaimableOrganization};

impl Client {
    /// Create a new claimable organisation. Unauthenticated.
    ///
    /// This method does not add any `Authorization` header itself — it skips
    /// the per-request auth helper used by all other methods. If the
    /// underlying `reqwest::Client` was supplied via `with_http_client` or
    /// `with_http_client_bearer` and was built with a default `Authorization`
    /// header, that header will still be sent by the transport.
    ///
    /// Returns the organisation ID, a one-time API key pair, the expiry, and
    /// resource limits. The key secret must be persisted immediately; it is
    /// never returned again.
    pub async fn claimable_organization_create(
        &self,
        region: Option<&str>,
    ) -> Result<ApiResponse<ClaimableOrganization>, Error> {
        let url = super::endpoint_url(
            &self.base_url,
            operations::CLAIMABLE_ORGANIZATION_CREATE.path,
            &[],
        )?;
        let mut body = serde_json::Map::new();
        if let Some(r) = region {
            body.insert(
                "region".to_string(),
                serde_json::Value::String(r.to_owned()),
            );
        }
        let resp = self.http.post(url).json(&body).send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }
}
