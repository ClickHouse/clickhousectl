use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// Create a Query API endpoint.
    pub async fn query_api_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &PublicQueryApiEndpointRequest,
    ) -> Result<ApiResponse<PublicQueryApiEndpoint>, Error> {
        let req = self
            .request(
                &operations::QUERY_API_ENDPOINT_CREATE,
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

    /// Get a Query API endpoint.
    pub async fn query_api_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
        endpoint_id: &str,
    ) -> Result<ApiResponse<PublicQueryApiEndpoint>, Error> {
        let req = self.request(
            &operations::QUERY_API_ENDPOINT_GET,
            &[organization_id, service_id, endpoint_id],
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

    /// List Query API endpoints.
    ///
    /// Pass the previous response's `next_cursor` to retrieve the next page.
    /// The API defaults to 100 records per page and accepts limits from 1 to 100.
    pub async fn query_api_endpoint_list(
        &self,
        organization_id: &str,
        service_id: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<Vec<PublicQueryApiEndpointListItem>>, Error> {
        let mut req = self.request(
            &operations::QUERY_API_ENDPOINT_LIST,
            &[organization_id, service_id],
        )?;
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

    /// Update a Query API endpoint.
    ///
    /// User-owned endpoints cannot be managed through this operation.
    pub async fn query_api_endpoint_update(
        &self,
        organization_id: &str,
        service_id: &str,
        endpoint_id: &str,
        body: &PublicQueryApiEndpointRequest,
    ) -> Result<ApiResponse<PublicQueryApiEndpoint>, Error> {
        let req = self
            .request(
                &operations::QUERY_API_ENDPOINT_UPDATE,
                &[organization_id, service_id, endpoint_id],
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

    /// Delete a Query API endpoint.
    ///
    /// User-owned endpoints cannot be managed through this operation.
    pub async fn query_api_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        endpoint_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::QUERY_API_ENDPOINT_DELETE,
            &[organization_id, service_id, endpoint_id],
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
