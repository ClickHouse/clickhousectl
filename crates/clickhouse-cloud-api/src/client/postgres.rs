use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Create new Postgres service
    pub async fn postgres_service_create(
        &self,
        organization_id: &str,
        body: &PostgresServicePostRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres");
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
    /// List of organization Postgres services
    pub async fn postgres_service_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<PostgresServiceListItem>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres");
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

    /// Get PostgreSQL service details
    pub async fn postgres_service_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}");
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

    /// Delete a PostgreSQL service
    pub async fn postgres_service_delete(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}");
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

    /// Update a PostgreSQL service
    pub async fn postgres_service_patch(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServicePatchRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}");
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

    /// Get Postgres CA certs
    pub async fn postgres_service_certs_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<String, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/caCertificates");
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
        Ok(body_text)
    }

    /// Get PostgreSQL service configuration
    pub async fn postgres_instance_config_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<PostgresInstanceConfigResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/config");
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

    /// Replace Postgres service configuration
    pub async fn postgres_instance_config_post(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresInstanceConfig,
    ) -> Result<ApiResponse<PostgresInstanceUpdateConfigResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/config");
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

    /// Update Postgres service configuration
    pub async fn postgres_instance_config_patch(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresInstanceConfig,
    ) -> Result<ApiResponse<PostgresInstanceUpdateConfigResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/config");
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

    /// Update Postgres superuser password
    pub async fn postgres_service_set_password(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceSetPassword,
    ) -> Result<ApiResponse<PostgresServicePasswordResource>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/password");
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

    /// Create a read replica for a Postgres service
    pub async fn postgres_instance_create_read_replica(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceReadReplicaRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/readReplica");
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

    /// Get PostgreSQL service metrics
    pub async fn postgres_instance_prometheus_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/prometheus");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text),
            });
        }
        Ok(resp.text().await?)
    }

    /// Get organization PostgreSQL metrics
    pub async fn postgres_org_prometheus_get(
        &self,
        organization_id: &str,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/prometheus");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text),
            });
        }
        Ok(resp.text().await?)
    }

    /// Restore a Postgres service
    pub async fn postgres_instance_restore(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceRestoreRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/restoredService");
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

    /// Update Postgres service state
    pub async fn postgres_service_patch_state(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceSetState,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/state");
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

    /// Get Postgres metrics
    #[allow(clippy::too_many_arguments)]
    pub async fn postgres_instance_metrics_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
        from_date: &str,
        to_date: &str,
        bucket_size_seconds: Option<i64>,
    ) -> Result<ApiResponse<PostgresMetrics>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/metrics");
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[("from_date", from_date), ("to_date", to_date)]);
        if let Some(v) = bucket_size_seconds {
            req = req.query(&[("bucket_size_seconds", v)]);
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

    /// List Postgres slow query patterns
    #[allow(clippy::too_many_arguments)]
    pub async fn slow_query_patterns_get_list(
        &self,
        organization_id: &str,
        postgres_id: &str,
        from_date: &str,
        to_date: &str,
        db_name: Option<&str>,
        db_user: Option<&str>,
        db_operation: Option<&str>,
        app: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<PostgresSlowQueryPattern>>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/slowQueryPatterns");
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[("from_date", from_date), ("to_date", to_date)]);
        if let Some(v) = db_name {
            req = req.query(&[("db_name", v)]);
        }
        if let Some(v) = db_user {
            req = req.query(&[("db_user", v)]);
        }
        if let Some(v) = db_operation {
            req = req.query(&[("db_operation", v)]);
        }
        if let Some(v) = app {
            req = req.query(&[("app", v)]);
        }
        if let Some(v) = sort_by {
            req = req.query(&[("sort_by", v)]);
        }
        if let Some(v) = sort_order {
            req = req.query(&[("sort_order", v)]);
        }
        if let Some(v) = limit {
            req = req.query(&[("limit", v)]);
        }
        if let Some(v) = offset {
            req = req.query(&[("offset", v)]);
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

    /// Get a Postgres slow query pattern with recent executions
    #[allow(clippy::too_many_arguments)]
    pub async fn slow_query_pattern_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
        query_id: &str,
        db_name: &str,
        db_user: &str,
        db_operation: &str,
        app: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<ApiResponse<PostgresSlowQueryPatternDetail>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/postgres/{postgres_id}/slowQueryPatterns/{query_id}"
        );
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[
            ("db_name", db_name),
            ("db_user", db_user),
            ("db_operation", db_operation),
        ]);
        if let Some(v) = app {
            req = req.query(&[("app", v)]);
        }
        if let Some(v) = timestamp {
            req = req.query(&[("timestamp", v)]);
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
}
