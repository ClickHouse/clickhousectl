use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Delete UDF (Beta).
    pub async fn udf_delete(
        &self,
        organization_id: &str,
        function_name: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs/{function_name}");
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
    /// Detach UDF from service (Beta).
    pub async fn udf_detach(
        &self,
        organization_id: &str,
        function_name: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/udfs/{function_name}/attachments/{service_id}"
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
    /// Delete UDF version (Beta).
    pub async fn udf_version_delete(
        &self,
        organization_id: &str,
        function_name: &str,
        version: i64,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/udfs/{function_name}/versions/{version}");
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

    /// List UDFs (Beta).
    pub async fn udf_list(
        &self,
        organization_id: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<UdfListResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = cursor {
            req = req.query(&[("cursor", v)]);
        }
        if let Some(v) = limit {
            req = req.query(&[("limit", v)]);
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

    /// Get UDF (Beta).
    pub async fn udf_get(
        &self,
        organization_id: &str,
        function_name: &str,
    ) -> Result<ApiResponse<Udf>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs/{function_name}");
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

    /// List UDF attachments (Beta).
    pub async fn udf_attachment_list(
        &self,
        organization_id: &str,
        function_name: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<UdfAttachmentListResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs/{function_name}/attachments");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = cursor {
            req = req.query(&[("cursor", v)]);
        }
        if let Some(v) = limit {
            req = req.query(&[("limit", v)]);
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

    /// Get UDF attachment (Beta).
    pub async fn udf_attachment_get(
        &self,
        organization_id: &str,
        function_name: &str,
        service_id: &str,
    ) -> Result<ApiResponse<UdfAttachment>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/udfs/{function_name}/attachments/{service_id}"
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

    /// List UDF versions (Beta).
    pub async fn udf_version_list(
        &self,
        organization_id: &str,
        function_name: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<UdfVersionListResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs/{function_name}/versions");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = cursor {
            req = req.query(&[("cursor", v)]);
        }
        if let Some(v) = limit {
            req = req.query(&[("limit", v)]);
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

    /// Create a UDF upload session (Beta).
    pub async fn udf_upload_session_create(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<UdfUploadSession>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfUploads/url");
        let req = self.request(reqwest::Method::POST, &path);
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

    /// Create UDF (Beta).
    pub async fn udf_create(
        &self,
        organization_id: &str,
        body: &UdfCreateRequest,
    ) -> Result<ApiResponse<Udf>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs");
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

    /// Create UDF version (Beta).
    pub async fn udf_version_create(
        &self,
        organization_id: &str,
        function_name: &str,
        body: &UdfVersionCreateRequest,
    ) -> Result<ApiResponse<Udf>, Error> {
        let path = format!("/v1/organizations/{organization_id}/udfs/{function_name}/versions");
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

    /// Attach UDF to service (Beta).
    pub async fn udf_attach(
        &self,
        organization_id: &str,
        function_name: &str,
        service_id: &str,
        version: Option<i64>,
    ) -> Result<ApiResponse<UdfAttachment>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/udfs/{function_name}/attachments/{service_id}"
        );
        let body = version
            .map(|version| serde_json::json!({ "version": version }))
            .unwrap_or_else(|| serde_json::json!({}));
        let mut req = self.request(reqwest::Method::PUT, &path);
        req = req.json(&body);
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
