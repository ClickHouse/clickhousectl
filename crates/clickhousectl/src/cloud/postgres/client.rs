use super::observability::{
    PostgresBackupPage, PostgresLogsQuery, SlowQueryDetailQuery, SlowQueryListQuery,
};
use crate::cloud::client::{CloudClient, ResourceKind, ResourceLookup, Result as CloudResult};
use clickhouse_cloud_api::RunPostgresQueryRequest;
use clickhouse_cloud_api::models::{
    PostgresInstanceConfig, PostgresInstanceConfigResponse, PostgresInstanceUpdateConfigResponse,
    PostgresLogEntry, PostgresMetrics, PostgresService, PostgresServiceListItem,
    PostgresServicePasswordResource, PostgresServicePatchRequest, PostgresServicePostRequest,
    PostgresServiceReadReplicaRequest, PostgresServiceRestoreRequest, PostgresServiceSetPassword,
    PostgresServiceSetState, PostgresServiceSetStateCommand, PostgresSlowQueryPattern,
    PostgresSlowQueryPatternDetail,
};

impl CloudClient {
    pub(super) async fn run_postgres_query(
        &self,
        org_id: &str,
        service_id: &str,
        request: &RunPostgresQueryRequest,
    ) -> CloudResult<reqwest::Response> {
        self.api()
            .run_postgres_query_bearer(org_id, service_id, request)
            .await
            .map_err(|error| {
                // The generic OAuth 403 hint recommends API keys for writes.
                // This read-only route accepts OAuth only, so keep its typed
                // classification while providing route-specific guidance.
                let forbidden_message = match &error {
                    clickhouse_cloud_api::Error::Api {
                        status: 403,
                        message,
                    } => Some(message.clone()),
                    _ => None,
                };
                let mut converted = self.convert_error(error);
                if let Some(message) = forbidden_message {
                    converted.message = format!(
                        "{message}\n\nConfirm your OAuth user has Postgres query access and that \
                         OAuth support for Postgres queries is deployed. Queries must be read-only."
                    );
                }
                converted
            })
    }

    pub(in crate::cloud) async fn list_postgres_services(
        &self,
        org_id: &str,
    ) -> CloudResult<Vec<PostgresServiceListItem>> {
        let response = self
            .api()
            .postgres_service_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Fetch one page of a Postgres service's retained base backups.
    pub(super) async fn list_postgres_backups_page(
        &self,
        org_id: &str,
        postgres_id: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> CloudResult<PostgresBackupPage> {
        let mut response = self
            .api()
            .postgres_service_backup_get_list(org_id, postgres_id, cursor, limit)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        let limit = response.limit;
        let total_count = response.total_count;
        let next_cursor = response.next_cursor.take();
        Ok(PostgresBackupPage {
            result: Self::unwrap_response(response)?,
            limit,
            total_count,
            next_cursor,
        })
    }

    /// Fetch raw Prometheus exposition text for a Postgres service.
    pub async fn get_postgres_prometheus(
        &self,
        org_id: &str,
        postgres_id: &str,
    ) -> CloudResult<String> {
        self.api()
            .postgres_instance_prometheus_get(org_id, postgres_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))
    }

    /// Fetch raw Prometheus exposition text for all Postgres services in an organization.
    pub async fn get_postgres_org_prometheus(&self, org_id: &str) -> CloudResult<String> {
        self.api()
            .postgres_org_prometheus_get(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))
    }

    /// Fetch time-bucketed metrics for a Postgres service.
    pub async fn get_postgres_metrics(
        &self,
        org_id: &str,
        postgres_id: &str,
        from_date: &str,
        to_date: &str,
        bucket_size_seconds: Option<i64>,
    ) -> CloudResult<PostgresMetrics> {
        let response = self
            .api()
            .postgres_instance_metrics_get(
                org_id,
                postgres_id,
                from_date,
                to_date,
                bucket_size_seconds,
            )
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Fetch aggregate slow query patterns for a Postgres service.
    pub async fn list_postgres_slow_query_patterns(
        &self,
        org_id: &str,
        postgres_id: &str,
        query: &SlowQueryListQuery<'_>,
    ) -> CloudResult<Vec<PostgresSlowQueryPattern>> {
        let response = self
            .api()
            .slow_query_patterns_get_list(
                org_id,
                postgres_id,
                query.from_date,
                query.to_date,
                query.db_name,
                query.db_user,
                query.db_operation,
                query.app,
                query.sort_by.as_ref(),
                query.sort_order.as_ref(),
                query.limit,
                query.offset,
            )
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Fetch one slow query pattern and its recent executions.
    pub async fn get_postgres_slow_query_pattern(
        &self,
        org_id: &str,
        postgres_id: &str,
        query_id: &str,
        query: &SlowQueryDetailQuery<'_>,
    ) -> CloudResult<PostgresSlowQueryPatternDetail> {
        let response = self
            .api()
            .slow_query_pattern_get(
                org_id,
                postgres_id,
                query_id,
                query.db_name,
                query.db_user,
                query.db_operation,
                query.app,
                query.timestamp,
            )
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Convert a failed by-id Postgres request whose path names a
    /// subresource of the service or whose body carries user input.
    ///
    /// [`CloudClient::convert_error_for_lookup`] reads a 400 or 404 against
    /// well-formed ids as a missing service. That is sound only when the
    /// path names the service itself and nothing else in the request is
    /// user input (`get`, `delete`). Here a 404 can mean the subresource is
    /// absent and a 400 can be the body's validation failure, so the status
    /// alone cannot tell. A GET of the service itself decides: when it also
    /// rejects the well-formed ids, the service is missing and the original
    /// error is reported as such; otherwise the original error stands. The
    /// extra GET only happens after such a rejection.
    async fn convert_postgres_by_id_result<T>(
        &self,
        result: Result<T, clickhouse_cloud_api::Error>,
        org_id: &str,
        postgres_id: &str,
    ) -> CloudResult<T> {
        let error = match result {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        let lookup = ResourceLookup::in_org(ResourceKind::PostgresService, postgres_id, org_id);
        if lookup.rejected_well_formed_ids(&error)
            && let Err(get_error) = self.api().postgres_service_get(org_id, postgres_id).await
            && lookup.rejected_well_formed_ids(&get_error)
        {
            return Err(self.convert_error_for_lookup(error, lookup));
        }
        Err(self.convert_error_for_organization(error, org_id))
    }

    /// Create a Postgres service.
    pub(super) async fn create_postgres_service(
        &self,
        org_id: &str,
        request: &PostgresServicePostRequest,
    ) -> CloudResult<PostgresService> {
        let response = self
            .api()
            .postgres_service_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Delete a Postgres service. The endpoint returns no resource.
    pub(super) async fn delete_postgres_service(
        &self,
        org_id: &str,
        postgres_id: &str,
    ) -> CloudResult<()> {
        self.api()
            .postgres_service_delete(org_id, postgres_id)
            .await
            .map_err(|error| {
                // This path identifies the Postgres service itself.
                self.convert_error_for_lookup(
                    error,
                    ResourceLookup::in_org(ResourceKind::PostgresService, postgres_id, org_id),
                )
            })?;
        Ok(())
    }

    /// Fetch a Postgres service's CA certificate as PEM text.
    pub(super) async fn get_postgres_service_certs(
        &self,
        org_id: &str,
        postgres_id: &str,
    ) -> CloudResult<String> {
        self.convert_postgres_by_id_result(
            self.api()
                .postgres_service_certs_get(org_id, postgres_id)
                .await,
            org_id,
            postgres_id,
        )
        .await
    }

    /// Fetch a Postgres service's runtime configuration.
    pub(super) async fn get_postgres_config(
        &self,
        org_id: &str,
        postgres_id: &str,
    ) -> CloudResult<PostgresInstanceConfigResponse> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_instance_config_get(org_id, postgres_id)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Replace a Postgres service's runtime configuration.
    pub(super) async fn replace_postgres_config(
        &self,
        org_id: &str,
        postgres_id: &str,
        config: &PostgresInstanceConfig,
    ) -> CloudResult<PostgresInstanceUpdateConfigResponse> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_instance_config_post(org_id, postgres_id, config)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Merge settings into a Postgres service's runtime configuration.
    pub(super) async fn patch_postgres_config(
        &self,
        org_id: &str,
        postgres_id: &str,
        config: &PostgresInstanceConfig,
    ) -> CloudResult<PostgresInstanceUpdateConfigResponse> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_instance_config_patch(org_id, postgres_id, config)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Set a Postgres service's password.
    pub(super) async fn set_postgres_service_password(
        &self,
        org_id: &str,
        postgres_id: &str,
        request: &PostgresServiceSetPassword,
    ) -> CloudResult<PostgresServicePasswordResource> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_service_set_password(org_id, postgres_id, request)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Create a read replica of a Postgres service.
    pub(super) async fn create_postgres_read_replica(
        &self,
        org_id: &str,
        postgres_id: &str,
        request: &PostgresServiceReadReplicaRequest,
    ) -> CloudResult<PostgresService> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_instance_create_read_replica(org_id, postgres_id, request)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Restore a Postgres service to a point in time as a new service.
    pub(super) async fn restore_postgres_service(
        &self,
        org_id: &str,
        postgres_id: &str,
        request: &PostgresServiceRestoreRequest,
    ) -> CloudResult<PostgresService> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_instance_restore(org_id, postgres_id, request)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Update a Postgres service.
    pub(super) async fn patch_postgres_service(
        &self,
        org_id: &str,
        postgres_id: &str,
        request: &PostgresServicePatchRequest,
    ) -> CloudResult<PostgresService> {
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_service_patch(org_id, postgres_id, request)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }

    /// Fetch a single Postgres service.
    pub async fn get_postgres_service(
        &self,
        org_id: &str,
        postgres_id: &str,
    ) -> CloudResult<PostgresService> {
        let response = self
            .api()
            .postgres_service_get(org_id, postgres_id)
            .await
            .map_err(|error| {
                // This path identifies the Postgres service itself.
                self.convert_error_for_lookup(
                    error,
                    ResourceLookup::in_org(ResourceKind::PostgresService, postgres_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    /// Fetch Postgres server logs for a validated time window.
    pub(super) async fn list_postgres_logs(
        &self,
        org_id: &str,
        postgres_id: &str,
        query: &PostgresLogsQuery,
    ) -> CloudResult<Vec<PostgresLogEntry>> {
        let response = self
            .api()
            .postgres_logs_get_list(
                org_id,
                postgres_id,
                &query.from_date,
                &query.to_date,
                query.body_contains.as_deref(),
                query.severity.as_deref(),
                query.sort_order.as_ref(),
                query.limit,
                query.offset,
            )
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// Issue a `restart`/`promote`/`switchover` command against a Postgres service.
    pub async fn set_postgres_service_state(
        &self,
        org_id: &str,
        postgres_id: &str,
        command: PostgresServiceSetStateCommand,
    ) -> CloudResult<PostgresService> {
        let request = PostgresServiceSetState { command };
        let response = self
            .convert_postgres_by_id_result(
                self.api()
                    .postgres_service_patch_state(org_id, postgres_id, &request)
                    .await,
                org_id,
                postgres_id,
            )
            .await?;
        Self::unwrap_response(response)
    }
}
