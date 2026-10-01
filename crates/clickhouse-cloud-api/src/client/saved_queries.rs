use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Create a saved query (Beta).
    ///
    /// A saved query with the same name in the service is rejected with 409.
    pub async fn saved_query_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &PublicSavedQueryRequest,
    ) -> Result<ApiResponse<PublicSavedQuery>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/saved-queries");
        let req = self.request(reqwest::Method::POST, &path).json(body);
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

    /// Get a saved query (Beta).
    pub async fn saved_query_get(
        &self,
        organization_id: &str,
        service_id: &str,
        query_id: &str,
    ) -> Result<ApiResponse<PublicSavedQuery>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/saved-queries/{query_id}"
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

    /// List saved queries (Beta).
    ///
    /// Pagination metadata is on the envelope: pass the previous page's
    /// `ApiResponse::next_cursor` as `cursor` to retrieve the next page.
    /// The API defaults to 100 records per page and accepts limits from 1 to 100.
    pub async fn saved_query_list(
        &self,
        organization_id: &str,
        service_id: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<Vec<PublicSavedQueryListItem>>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/saved-queries");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(cursor) = cursor {
            req = req.query(&[("cursor", cursor)]);
        }
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
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

    /// Update a saved query (Beta).
    ///
    /// Replaces the whole saved query. Saved queries that cannot be managed
    /// through this API, or a name already in use, are rejected with 409.
    pub async fn saved_query_update(
        &self,
        organization_id: &str,
        service_id: &str,
        query_id: &str,
        body: &PublicSavedQueryRequest,
    ) -> Result<ApiResponse<PublicSavedQuery>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/saved-queries/{query_id}"
        );
        let req = self.request(reqwest::Method::PUT, &path).json(body);
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

    /// Delete a saved query (Beta).
    ///
    /// Saved queries that cannot be managed through this API are rejected with 409.
    pub async fn saved_query_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        query_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/saved-queries/{query_id}"
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
}
