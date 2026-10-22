use super::MissingRequiredFields;
use crate::models::*;

impl From<PgConfigResponse> for PgConfig {
    fn from(value: PgConfigResponse) -> Self {
        // Every `pgConfig` GUC is optional in both directions (omitting one
        // selects the server default), so the conversion is total.
        Self {
            autovacuum_analyze_scale_factor: value.autovacuum_analyze_scale_factor,
            autovacuum_analyze_threshold: value.autovacuum_analyze_threshold,
            autovacuum_freeze_max_age: value.autovacuum_freeze_max_age,
            autovacuum_max_workers: value.autovacuum_max_workers,
            autovacuum_naptime: value.autovacuum_naptime,
            autovacuum_vacuum_cost_delay: value.autovacuum_vacuum_cost_delay,
            autovacuum_vacuum_cost_limit: value.autovacuum_vacuum_cost_limit,
            autovacuum_vacuum_insert_scale_factor: value.autovacuum_vacuum_insert_scale_factor,
            autovacuum_vacuum_insert_threshold: value.autovacuum_vacuum_insert_threshold,
            autovacuum_vacuum_scale_factor: value.autovacuum_vacuum_scale_factor,
            autovacuum_vacuum_threshold: value.autovacuum_vacuum_threshold,
            autovacuum_work_mem: value.autovacuum_work_mem,
            bgwriter_delay: value.bgwriter_delay,
            bgwriter_flush_after: value.bgwriter_flush_after,
            bgwriter_lru_maxpages: value.bgwriter_lru_maxpages,
            bgwriter_lru_multiplier: value.bgwriter_lru_multiplier,
            checkpoint_timeout: value.checkpoint_timeout,
            default_toast_compression: value.default_toast_compression,
            default_transaction_isolation: value.default_transaction_isolation,
            effective_cache_size: value.effective_cache_size,
            effective_io_concurrency: value.effective_io_concurrency,
            enable_parallel_hash: value.enable_parallel_hash,
            hash_mem_multiplier: value.hash_mem_multiplier,
            idle_in_transaction_session_timeout: value.idle_in_transaction_session_timeout,
            idle_session_timeout: value.idle_session_timeout,
            io_method: value.io_method,
            io_workers: value.io_workers,
            lock_timeout: value.lock_timeout,
            log_lock_waits: value.log_lock_waits,
            log_min_duration_statement: value.log_min_duration_statement,
            log_parameter_max_length: value.log_parameter_max_length,
            log_temp_files: value.log_temp_files,
            logical_decoding_work_mem: value.logical_decoding_work_mem,
            maintenance_io_concurrency: value.maintenance_io_concurrency,
            maintenance_work_mem: value.maintenance_work_mem,
            max_connections: value.max_connections,
            max_logical_replication_workers: value.max_logical_replication_workers,
            max_parallel_maintenance_workers: value.max_parallel_maintenance_workers,
            max_parallel_workers: value.max_parallel_workers,
            max_parallel_workers_per_gather: value.max_parallel_workers_per_gather,
            max_replication_slots: value.max_replication_slots,
            max_slot_wal_keep_size: value.max_slot_wal_keep_size,
            max_sync_workers_per_subscription: value.max_sync_workers_per_subscription,
            max_wal_senders: value.max_wal_senders,
            max_wal_size: value.max_wal_size,
            max_worker_processes: value.max_worker_processes,
            min_wal_size: value.min_wal_size,
            random_page_cost: value.random_page_cost,
            shared_memory_percent: value.shared_memory_percent,
            ssl_min_protocol_version: value.ssl_min_protocol_version,
            statement_timeout: value.statement_timeout,
            temp_buffers: value.temp_buffers,
            track_activity_query_size: value.track_activity_query_size,
            transaction_timeout: value.transaction_timeout,
            vacuum_cost_delay: value.vacuum_cost_delay,
            wal_buffers: value.wal_buffers,
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
                pg_bouncer_config,
                pg_config: pg_config.into(),
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<PostgresIpAccessListEntryResponse> for PostgresIpAccessListEntry {
    type Error = MissingRequiredFields;

    /// Turns a fetched access rule into one that can be sent back.
    ///
    /// The returned `id` is kept, so writing the rule back through
    /// `PostgresIpAccessListPatch::add` edits that rule instead of creating a
    /// new one. A rule without `portRange` or `source` cannot be written back:
    /// inventing either would open or close the wrong port or address.
    fn try_from(value: PostgresIpAccessListEntryResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.port_range.is_none() {
            missing.push("portRange");
        }
        if value.source.is_none() {
            missing.push("source");
        }
        match (value.port_range, value.source) {
            (Some(port_range), Some(source)) => Ok(Self {
                description: value.description,
                id: value.id,
                port_range,
                source,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}
