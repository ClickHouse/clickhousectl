use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Resolve the identity of the authenticated caller (Beta).
    ///
    /// Not scoped to an organization. A user (OAuth or JWT) resolves to
    /// [`Whoami::WhoamiUser`] with the organizations it belongs to; an
    /// organization API key resolves to [`Whoami::WhoamiApiKey`]. An
    /// unrecognized `actorType` is kept verbatim in [`Whoami::Unknown`].
    pub async fn whoami_get(&self) -> Result<ApiResponse<Whoami>, Error> {
        let req = self.request(reqwest::Method::GET, "/v1/whoami");
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
