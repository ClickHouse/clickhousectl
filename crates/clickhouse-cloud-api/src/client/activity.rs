use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// List of organization activities
    pub async fn activity_get_list(
        &self,
        organization_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<ApiResponse<Vec<Activity>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/activities");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = from_date {
            req = req.query(&[("from_date", v)]);
        }
        if let Some(v) = to_date {
            req = req.query(&[("to_date", v)]);
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
    /// Organization activity
    pub async fn activity_get(
        &self,
        organization_id: &str,
        activity_id: &str,
    ) -> Result<ApiResponse<Activity>, Error> {
        let path = format!("/v1/organizations/{organization_id}/activities/{activity_id}");
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
}
