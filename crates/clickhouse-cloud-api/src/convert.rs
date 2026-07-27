//! Explicit conversions from response models back into request models.
//!
//! Response models are tolerant by construction: every field is `Option<T>`, so
//! a field the API drops or sends as `null` deserializes to `None` instead of
//! failing. Request models are strict: a field the API requires is `T`.
//!
//! A caller that fetches a resource, edits it, and writes it back therefore has
//! to resolve absence explicitly, and that is the point of this module — the old
//! `#[serde(default)]` policy silently fabricated `""`/`0`/`false` for a dropped
//! field and persisted it on the next write. A conversion that can lose that
//! information is a [`TryFrom`] reporting the missing wire field names; a
//! conversion that cannot is a [`From`].

use std::fmt;

use crate::models::{
    PgBouncerConfig, PgBouncerConfigResponse, PgConfig, PgConfigResponse, PostgresInstanceConfig,
    PostgresInstanceConfigResponse,
};

/// The response omitted fields that the matching request model requires.
///
/// Field names are the wire (spec) names, so an error message points at the
/// JSON the API returned rather than at Rust identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRequiredFields {
    fields: Vec<&'static str>,
}

impl MissingRequiredFields {
    pub(crate) fn new(fields: Vec<&'static str>) -> Self {
        Self { fields }
    }

    /// The missing wire field names, in declaration order.
    pub fn fields(&self) -> &[&'static str] {
        &self.fields
    }
}

impl fmt::Display for MissingRequiredFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the API response is missing required field(s): {}",
            self.fields.join(", ")
        )
    }
}

impl std::error::Error for MissingRequiredFields {}

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
