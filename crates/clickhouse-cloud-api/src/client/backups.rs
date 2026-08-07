use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Get service backup bucket
    pub async fn backup_bucket_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
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
    /// Create service backup bucket
    pub async fn backup_bucket_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupBucketPostRequest,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
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

    /// Update service backup bucket
    pub async fn backup_bucket_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupBucketPatchRequest,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
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

    /// Delete service backup bucket
    pub async fn backup_bucket_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
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

    /// Get service backup configuration
    pub async fn backup_configuration_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<BackupConfiguration>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/backupConfiguration"
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

    /// Update service backup configuration
    pub async fn backup_configuration_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupConfigurationPatchRequest,
    ) -> Result<ApiResponse<BackupConfiguration>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/backupConfiguration"
        );
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

    /// List of service backups
    pub async fn backup_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<Backup>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backups");
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

    /// Get backup details
    pub async fn backup_get(
        &self,
        organization_id: &str,
        service_id: &str,
        backup_id: &str,
    ) -> Result<ApiResponse<Backup>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/backups/{backup_id}"
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
}
