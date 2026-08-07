use super::MissingRequiredFields;
use crate::models::*;

impl From<PgBouncerConfigResponse> for PgBouncerConfig {
    fn from(_value: PgBouncerConfigResponse) -> Self {
        // The schema declares no properties, so the conversion is total.
        Self {}
    }
}

impl From<PgConfigResponse> for PgConfig {
    fn from(value: PgConfigResponse) -> Self {
        // Every `pgConfig` GUC is optional in both directions (omitting one
        // selects the server default), so the conversion is total.
        Self {
            autovacuum_analyze_scale_factor: value.autovacuum_analyze_scale_factor,
            autovacuum_max_workers: value.autovacuum_max_workers,
            autovacuum_naptime: value.autovacuum_naptime,
            autovacuum_vacuum_cost_delay: value.autovacuum_vacuum_cost_delay,
            autovacuum_vacuum_cost_limit: value.autovacuum_vacuum_cost_limit,
            autovacuum_vacuum_insert_scale_factor: value.autovacuum_vacuum_insert_scale_factor,
            autovacuum_vacuum_scale_factor: value.autovacuum_vacuum_scale_factor,
            autovacuum_work_mem: value.autovacuum_work_mem,
            default_transaction_isolation: value.default_transaction_isolation,
            effective_cache_size: value.effective_cache_size,
            effective_io_concurrency: value.effective_io_concurrency,
            idle_in_transaction_session_timeout: value.idle_in_transaction_session_timeout,
            idle_session_timeout: value.idle_session_timeout,
            lock_timeout: value.lock_timeout,
            maintenance_work_mem: value.maintenance_work_mem,
            max_connections: value.max_connections,
            max_parallel_maintenance_workers: value.max_parallel_maintenance_workers,
            max_parallel_workers: value.max_parallel_workers,
            max_parallel_workers_per_gather: value.max_parallel_workers_per_gather,
            max_slot_wal_keep_size: value.max_slot_wal_keep_size,
            max_wal_size: value.max_wal_size,
            max_worker_processes: value.max_worker_processes,
            min_wal_size: value.min_wal_size,
            random_page_cost: value.random_page_cost,
            ssl_min_protocol_version: value.ssl_min_protocol_version,
            statement_timeout: value.statement_timeout,
            transaction_timeout: value.transaction_timeout,
            wal_compression: value.wal_compression,
            wal_keep_size: value.wal_keep_size,
            wal_sender_timeout: value.wal_sender_timeout,
            work_mem: value.work_mem,
        }
    }
}

impl TryFrom<PostgresInstanceConfigResponse> for PostgresInstanceConfig {
    type Error = MissingRequiredFields;

    /// Turns a fetched configuration into a POST/PATCH body.
    ///
    /// The API requires both `pgConfig` and `pgBouncerConfig` in a write body
    /// (it rejects a body omitting either), so a response missing one cannot be
    /// written back verbatim and the caller has to supply it.
    fn try_from(value: PostgresInstanceConfigResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.pg_bouncer_config.is_none() {
            missing.push("pgBouncerConfig");
        }
        if value.pg_config.is_none() {
            missing.push("pgConfig");
        }
        match (value.pg_bouncer_config, value.pg_config) {
            (Some(pg_bouncer_config), Some(pg_config)) => Ok(Self {
                pg_bouncer_config: pg_bouncer_config.into(),
                pg_config: pg_config.into(),
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}
