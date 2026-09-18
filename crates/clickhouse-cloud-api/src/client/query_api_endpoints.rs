use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Create a Query API endpoint (Beta).
    pub async fn query_api_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &PublicQueryApiEndpointRequest,
    ) -> Result<ApiResponse<PublicQueryApiEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/query-api-endpoints"
        );
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

    /// Get a Query API endpoint (Beta).
    pub async fn query_api_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
        endpoint_id: &str,
    ) -> Result<ApiResponse<PublicQueryApiEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/query-api-endpoints/{endpoint_id}"
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

    /// List Query API endpoints (Beta).
    ///
    /// Pass the previous page's `pagination.nextCursor` to retrieve the next page.
    /// The API defaults to 100 records per page and accepts limits from 1 to 100.
    pub async fn query_api_endpoint_list(
        &self,
        organization_id: &str,
        service_id: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<QueryApiEndpointListResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/query-api-endpoints"
        );
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

    /// Update a Query API endpoint (Beta).
    ///
    /// User-owned endpoints cannot be managed through this operation.
    pub async fn query_api_endpoint_update(
        &self,
        organization_id: &str,
        service_id: &str,
        endpoint_id: &str,
        body: &PublicQueryApiEndpointRequest,
    ) -> Result<ApiResponse<PublicQueryApiEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/query-api-endpoints/{endpoint_id}"
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

    /// Delete a Query API endpoint (Beta).
    ///
    /// User-owned endpoints cannot be managed through this operation.
    pub async fn query_api_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        endpoint_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/query-api-endpoints/{endpoint_id}"
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
