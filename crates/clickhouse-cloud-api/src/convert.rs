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
    PostgresInstanceConfigResponse, ResourceTagsV1, ResourceTagsV1Response, ScalingScheduleEntry,
    ScalingScheduleEntryRequest, UpgradeWindow, UpgradeWindowPutRequest,
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

impl TryFrom<ResourceTagsV1Response> for ResourceTagsV1 {
    type Error = MissingRequiredFields;

    /// Turns a fetched tag into one that can be sent back.
    ///
    /// A tag is identified by its key, so a response tag without one cannot be
    /// written back — dropping it silently would delete the tag on the next
    /// write, and inventing an empty key would create a bogus one.
    fn try_from(value: ResourceTagsV1Response) -> Result<Self, Self::Error> {
        match value.key {
            Some(key) => Ok(Self {
                key,
                value: value.value,
            }),
            None => Err(MissingRequiredFields::new(vec!["key"])),
        }
    }
}

impl TryFrom<ScalingScheduleEntry> for ScalingScheduleEntryRequest {
    type Error = MissingRequiredFields;

    /// Turns a fetched schedule entry into one that can be re-sent.
    ///
    /// An upsert replaces the whole schedule, so a caller that reads a schedule
    /// and writes it back has to send every entry in full: the window bounds,
    /// weekdays and name the API requires cannot be defaulted away.
    fn try_from(value: ScalingScheduleEntry) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.end_hour_utc.is_none() {
            missing.push("endHourUtc");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.start_hour_utc.is_none() {
            missing.push("startHourUtc");
        }
        if value.weekdays.is_none() {
            missing.push("weekdays");
        }
        match (
            value.end_hour_utc,
            value.name,
            value.start_hour_utc,
            value.weekdays,
        ) {
            (Some(end_hour_utc), Some(name), Some(start_hour_utc), Some(weekdays)) => Ok(Self {
                autoscaling_mode: value.autoscaling_mode,
                end_hour_utc,
                idle_scaling: value.idle_scaling,
                idle_timeout_minutes: value.idle_timeout_minutes,
                max_replica_memory_gb: value.max_replica_memory_gb,
                max_replicas: value.max_replicas,
                min_replica_memory_gb: value.min_replica_memory_gb,
                min_replicas: value.min_replicas,
                name,
                // Not part of the response shape; a horizontal entry carries a
                // min/max band instead of a fixed replica count.
                num_replicas: None,
                start_hour_utc,
                weekdays,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<UpgradeWindow> for UpgradeWindowPutRequest {
    type Error = MissingRequiredFields;

    /// Turns a fetched upgrade window into one that can be re-sent.
    ///
    /// `duration` is response-only (the API derives it), so only the window's
    /// start hour and weekday cross over — and both are required.
    fn try_from(value: UpgradeWindow) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.start_hour_utc.is_none() {
            missing.push("startHourUtc");
        }
        if value.weekday.is_none() {
            missing.push("weekday");
        }
        match (value.start_hour_utc, value.weekday) {
            (Some(start_hour_utc), Some(weekday)) => Ok(Self {
                start_hour_utc,
                weekday,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}
