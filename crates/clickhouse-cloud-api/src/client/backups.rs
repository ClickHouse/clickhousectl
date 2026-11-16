use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// Get service snapshot configuration (beta).
    pub async fn snapshot_configuration_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<SnapshotConfiguration>, Error> {
        let req = self.request(
            &operations::SNAPSHOT_CONFIGURATION_GET,
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

    /// Update service snapshot configuration (beta).
    ///
    /// Requires an ADMIN auth key role. Provide at least one field; omitted fields
    /// stay unchanged, and the API rejects explicit nulls. When snapshots are
    /// enabled, the resulting `(gap, timeFrame)` pair in minutes must be
    /// `(30, 1440)` or `(60, 2880)`. The server validates the resulting configuration.
    pub async fn snapshot_configuration_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &SnapshotConfigurationPatchRequest,
    ) -> Result<ApiResponse<SnapshotConfiguration>, Error> {
        let req = self
            .request(
                &operations::SNAPSHOT_CONFIGURATION_UPDATE,
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

    /// Get one page of service snapshots, most recent first (beta).
    ///
    /// `limit` must be at least 1; omission uses the server default of 250. Pass
    /// the response's `next_cursor` to fetch another page, until it is `None`.
    pub async fn snapshot_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ApiResponse<Vec<Snapshot>>, Error> {
        let mut req = self.request(
            &operations::SNAPSHOT_GET_LIST,
            &[organization_id, service_id],
        )?;
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(cursor) = cursor {
            req = req.query(&[("cursor", cursor)]);
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

    /// Get service snapshot details (beta).
    pub async fn snapshot_get(
        &self,
        organization_id: &str,
        service_id: &str,
        snapshot_id: &str,
    ) -> Result<ApiResponse<Snapshot>, Error> {
        let req = self.request(
            &operations::SNAPSHOT_GET,
            &[organization_id, service_id, snapshot_id],
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

    /// Get service backup bucket
    pub async fn backup_bucket_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let req = self.request(
            &operations::BACKUP_BUCKET_GET,
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
    /// Create service backup bucket
    pub async fn backup_bucket_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupBucketPostRequest,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let mut req = self.request(
            &operations::BACKUP_BUCKET_CREATE,
            &[organization_id, service_id],
        )?;
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
        let mut req = self.request(
            &operations::BACKUP_BUCKET_UPDATE,
            &[organization_id, service_id],
        )?;
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
        let req = self.request(
            &operations::BACKUP_BUCKET_DELETE,
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

    /// Get service backup configuration
    pub async fn backup_configuration_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<BackupConfiguration>, Error> {
        let req = self.request(
            &operations::BACKUP_CONFIGURATION_GET,
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

    /// Update service backup configuration
    pub async fn backup_configuration_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupConfigurationPatchRequest,
    ) -> Result<ApiResponse<BackupConfiguration>, Error> {
        let mut req = self.request(
            &operations::BACKUP_CONFIGURATION_UPDATE,
            &[organization_id, service_id],
        )?;
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
        let req = self.request(&operations::BACKUP_GET_LIST, &[organization_id, service_id])?;
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
        let req = self.request(
            &operations::BACKUP_GET,
            &[organization_id, service_id, backup_id],
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
