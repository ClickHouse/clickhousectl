use super::query::query_api_error;
use super::{Auth, Client};
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// Run read-only SQL through the Postgres Query API using the client's OAuth token.
    ///
    /// This endpoint is outside the published OpenAPI specification and requires
    /// server support for the `clickhousectl` OAuth audience. API keys are not
    /// supported; a Basic-auth client returns [`Error::AuthMismatch`] before
    /// making a request. No query endpoint or database credentials are provisioned.
    ///
    /// Uses the same query-host resolution as [`Client::with_query_host`]. Returns
    /// the streaming `JSONCompactEachRowWithNamesAndTypes` response: a column-name
    /// array, a type-name array, and one array per row, each on its own line. An
    /// empty result may have an empty body. Gzip decoding is enabled by default.
    /// The caller consumes the response; no query retries or service wake-ups occur.
    pub async fn run_postgres_query_bearer(
        &self,
        org_id: &str,
        service_id: &str,
        request: &RunPostgresQueryRequest,
    ) -> Result<reqwest::Response, Error> {
        let token = match &self.auth {
            Auth::Bearer { token } => token,
            Auth::Basic { .. } => {
                return Err(Error::AuthMismatch(
                    "run_postgres_query_bearer called on a Basic-auth client".into(),
                ));
            }
        };
        let url = super::endpoint_url(
            self.resolved_query_host().trim_end_matches('/'),
            "/service/{serviceId}/runPostgres",
            &[service_id],
        )?;
        let response = self
            .http
            .post(url)
            .query(&[
                ("orgId", org_id),
                ("format", "JSONCompactEachRowWithNamesAndTypes"),
            ])
            .bearer_auth(token)
            .header("x-service-type", "postgres")
            .json(request)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.map_err(|error| Error::Api {
                status: status.as_u16(),
                message: format!(
                    "Query API returned HTTP {status}, but its response body could not be read: {error}"
                ),
            })?;
            return Err(query_api_error(status, &body_text));
        }
        Ok(response)
    }

    /// Create new Postgres service
    pub async fn postgres_service_create(
        &self,
        organization_id: &str,
        body: &PostgresServicePostRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let mut req = self.request(&operations::POSTGRES_SERVICE_CREATE, &[organization_id])?;
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
        let req = self.request(&operations::POSTGRES_SERVICE_GET_LIST, &[organization_id])?;
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
        let req = self.request(
            &operations::POSTGRES_SERVICE_GET,
            &[organization_id, postgres_id],
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

    /// Delete a PostgreSQL service
    pub async fn postgres_service_delete(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::POSTGRES_SERVICE_DELETE,
            &[organization_id, postgres_id],
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

    /// Update a PostgreSQL service
    pub async fn postgres_service_patch(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServicePatchRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_SERVICE_PATCH,
            &[organization_id, postgres_id],
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

    /// Get Postgres CA certs
    pub async fn postgres_service_certs_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<String, Error> {
        let req = self.request(
            &operations::POSTGRES_SERVICE_CERTS_GET,
            &[organization_id, postgres_id],
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
        Ok(body_text)
    }

    /// Get PostgreSQL service configuration
    pub async fn postgres_instance_config_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<PostgresInstanceConfigResponse>, Error> {
        let req = self.request(
            &operations::POSTGRES_INSTANCE_CONFIG_GET,
            &[organization_id, postgres_id],
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

    /// Replace Postgres service configuration
    pub async fn postgres_instance_config_post(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresInstanceConfig,
    ) -> Result<ApiResponse<PostgresInstanceUpdateConfigResponse>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_INSTANCE_CONFIG_POST,
            &[organization_id, postgres_id],
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

    /// Update Postgres service configuration
    pub async fn postgres_instance_config_patch(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresInstanceConfig,
    ) -> Result<ApiResponse<PostgresInstanceUpdateConfigResponse>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_INSTANCE_CONFIG_PATCH,
            &[organization_id, postgres_id],
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

    /// Update Postgres superuser password
    pub async fn postgres_service_set_password(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceSetPassword,
    ) -> Result<ApiResponse<PostgresServicePasswordResource>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_SERVICE_SET_PASSWORD,
            &[organization_id, postgres_id],
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

    /// Create a read replica for a Postgres service
    pub async fn postgres_instance_create_read_replica(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceReadReplicaRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_INSTANCE_CREATE_READ_REPLICA,
            &[organization_id, postgres_id],
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

    /// Get PostgreSQL service metrics
    pub async fn postgres_instance_prometheus_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<String, Error> {
        let req = self.request(
            &operations::POSTGRES_INSTANCE_PROMETHEUS_GET,
            &[organization_id, postgres_id],
        )?;
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
        let req = self.request(&operations::POSTGRES_ORG_PROMETHEUS_GET, &[organization_id])?;
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
        let mut req = self.request(
            &operations::POSTGRES_INSTANCE_RESTORE,
            &[organization_id, postgres_id],
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

    /// Update Postgres service state
    pub async fn postgres_service_patch_state(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceSetState,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_SERVICE_PATCH_STATE,
            &[organization_id, postgres_id],
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

    /// List Postgres logs
    #[allow(clippy::too_many_arguments)]
    pub async fn postgres_logs_get_list(
        &self,
        organization_id: &str,
        postgres_id: &str,
        from_date: &str,
        to_date: &str,
        body_contains: Option<&str>,
        severity: Option<&PostgresLogsGetListSeverity>,
        sort_order: Option<&PostgresLogsGetListSortorder>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<PostgresLogEntry>>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_LOGS_GET_LIST,
            &[organization_id, postgres_id],
        )?;
        req = req.query(&[("from_date", from_date), ("to_date", to_date)]);
        if let Some(v) = body_contains {
            req = req.query(&[("body_contains", v)]);
        }
        if let Some(v) = severity {
            req = req.query(&[("severity", v)]);
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
        let mut req = self.request(
            &operations::POSTGRES_INSTANCE_METRICS_GET,
            &[organization_id, postgres_id],
        )?;
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
        sort_by: Option<&SlowQueryPatternsGetListSortby>,
        sort_order: Option<&SlowQueryPatternsGetListSortorder>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<PostgresSlowQueryPattern>>, Error> {
        let mut req = self.request(
            &operations::SLOW_QUERY_PATTERNS_GET_LIST,
            &[organization_id, postgres_id],
        )?;
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
        let mut req = self.request(
            &operations::SLOW_QUERY_PATTERN_GET,
            &[organization_id, postgres_id, query_id],
        )?;
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

    /// List Postgres service backups (Beta).
    ///
    /// Returns the retained base backups, newest first. Pagination metadata is
    /// on the envelope: pass the previous page's `ApiResponse::next_cursor` as
    /// `cursor` to retrieve the next page. The API defaults to 100 records per
    /// page and accepts limits from 1 to 100. A Postgres service is restored to
    /// a point in time with `postgres_instance_restore`, not from a backup key.
    pub async fn postgres_service_backup_get_list(
        &self,
        organization_id: &str,
        postgres_id: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> Result<ApiResponse<Vec<PostgresBackup>>, Error> {
        let mut req = self.request(
            &operations::POSTGRES_SERVICE_BACKUP_GET_LIST,
            &[organization_id, postgres_id],
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
}
