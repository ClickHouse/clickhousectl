use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult, read_flag_file};
use crate::cloud::output::print_human;
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    PgConfig, PgConfigDefaultToastCompression, PgConfigDefaultTransactionIsolation,
    PgConfigEnableParallelHash, PgConfigIoMethod, PgConfigLogLockWaits,
    PgConfigSslMinProtocolVersion, PgConfigWalCompression, PostgresInstanceConfig,
};
use serde::de::DeserializeOwned;
use std::path::Path;

/// Read and parse the JSON file `flag` names. A file that cannot be read is an
/// `Io` failure naming the flag; one that does not parse is a usage error.
pub(super) fn load_json_file<T: DeserializeOwned>(flag: &str, path: &Path) -> CloudResult<T> {
    let contents = read_flag_file(flag, path)?;
    serde_json::from_str(&contents).map_err(|e| {
        CloudError::usage(format!("failed to parse {} as JSON: {}", path.display(), e))
    })
}

/// The closed set of Postgres GUCs accepted by the Cloud API's `pgConfig`
/// schema. The library stays tolerant when reading responses, so write inputs
/// are checked here before serde can discard a misspelled field.
const PG_CONFIG_KEYS: [&str; 60] = [
    "autovacuum_analyze_scale_factor",
    "autovacuum_analyze_threshold",
    "autovacuum_freeze_max_age",
    "autovacuum_max_workers",
    "autovacuum_naptime",
    "autovacuum_vacuum_cost_delay",
    "autovacuum_vacuum_cost_limit",
    "autovacuum_vacuum_insert_scale_factor",
    "autovacuum_vacuum_insert_threshold",
    "autovacuum_vacuum_scale_factor",
    "autovacuum_vacuum_threshold",
    "autovacuum_work_mem",
    "bgwriter_delay",
    "bgwriter_flush_after",
    "bgwriter_lru_maxpages",
    "bgwriter_lru_multiplier",
    "checkpoint_timeout",
    "default_toast_compression",
    "default_transaction_isolation",
    "effective_cache_size",
    "effective_io_concurrency",
    "enable_parallel_hash",
    "hash_mem_multiplier",
    "idle_in_transaction_session_timeout",
    "idle_session_timeout",
    "io_method",
    "io_workers",
    "lock_timeout",
    "log_lock_waits",
    "log_min_duration_statement",
    "log_parameter_max_length",
    "log_temp_files",
    "logical_decoding_work_mem",
    "maintenance_io_concurrency",
    "maintenance_work_mem",
    "max_connections",
    "max_logical_replication_workers",
    "max_parallel_maintenance_workers",
    "max_parallel_workers",
    "max_parallel_workers_per_gather",
    "max_replication_slots",
    "max_slot_wal_keep_size",
    "max_sync_workers_per_subscription",
    "max_wal_senders",
    "max_wal_size",
    "max_worker_processes",
    "min_wal_size",
    "random_page_cost",
    "shared_memory_percent",
    "ssl_min_protocol_version",
    "statement_timeout",
    "temp_buffers",
    "track_activity_query_size",
    "transaction_timeout",
    "vacuum_cost_delay",
    "wal_buffers",
    "wal_compression",
    "wal_keep_size",
    "wal_sender_timeout",
    "work_mem",
];

fn validate_pg_config(doc: &serde_json::Value) -> CloudResult<()> {
    let config = doc
        .as_object()
        .ok_or_else(|| CloudError::usage("pgConfig must be a JSON object"))?;

    for (key, value) in config {
        if !PG_CONFIG_KEYS.contains(&key.as_str()) {
            return Err(CloudError::usage(format!(
                "unknown pgConfig key '{key}'; expected one of: {}",
                PG_CONFIG_KEYS.join(", ")
            )));
        }

        if value.is_null() {
            return Err(CloudError::usage(format!(
                "invalid pgConfig value for '{key}': null is not supported"
            )));
        }

        let allowed = match key.as_str() {
            "default_toast_compression" => Some(PgConfigDefaultToastCompression::VALUES),
            "default_transaction_isolation" => Some(PgConfigDefaultTransactionIsolation::VALUES),
            "enable_parallel_hash" => Some(PgConfigEnableParallelHash::VALUES),
            "io_method" => Some(PgConfigIoMethod::VALUES),
            "log_lock_waits" => Some(PgConfigLogLockWaits::VALUES),
            "ssl_min_protocol_version" => Some(PgConfigSslMinProtocolVersion::VALUES),
            "wal_compression" => Some(PgConfigWalCompression::VALUES),
            _ => None,
        };
        if let Some(allowed) = allowed {
            let Some(value) = value.as_str() else {
                return Err(CloudError::usage(format!(
                    "invalid pgConfig value for '{key}': expected one of: {}",
                    allowed.join(", ")
                )));
            };
            if !allowed.contains(&value) {
                return Err(CloudError::usage(format!(
                    "invalid pgConfig value '{value}' for '{key}'; expected one of: {}",
                    allowed.join(", ")
                )));
            }
        }
    }
    Ok(())
}

fn pg_config_from_json(doc: serde_json::Value) -> CloudResult<PgConfig> {
    validate_pg_config(&doc).map_err(|e| CloudError::usage(format!("invalid pgConfig: {e}")))?;
    serde_json::from_value(doc).map_err(|e| CloudError::usage(format!("invalid pgConfig: {e}")))
}

pub(super) fn load_pg_config_file(path: &Path) -> CloudResult<PgConfig> {
    let doc = load_json_file::<serde_json::Value>("--pg-config-file", path)?;
    pg_config_from_json(doc)
        .map_err(|e| CloudError::usage(format!("invalid PgConfig file {}: {e}", path.display())))
}

/// Builds the `postgres config` write body from a user-supplied JSON document.
///
/// The document root must be a JSON object; anything else (a scalar, an array,
/// `null`) is rejected rather than read as "no sections", which would send an
/// empty body and reset the configuration.
///
/// The API schema requires both `pgConfig` and `pgBouncerConfig`. Require each
/// section explicitly so a missing section cannot be mistaken for an intentional
/// empty object.
fn instance_config_from_json(doc: &serde_json::Value) -> CloudResult<PostgresInstanceConfig> {
    let root = doc
        .as_object()
        .ok_or_else(|| CloudError::usage("configuration document must be a JSON object"))?;
    for key in root.keys() {
        if key != "pgConfig" && key != "pgBouncerConfig" {
            return Err(CloudError::usage(format!(
                "unknown configuration section '{key}'; expected pgConfig and pgBouncerConfig"
            )));
        }
    }
    let pg_config = root.get("pgConfig").ok_or_else(|| {
        CloudError::usage(
            "configuration document is missing required 'pgConfig' section; use {} to make it explicitly empty",
        )
    })?;
    let pg_bouncer_config = root.get("pgBouncerConfig").ok_or_else(|| {
        CloudError::usage(
            "configuration document is missing required 'pgBouncerConfig' section; use {} to make it explicitly empty",
        )
    })?;
    Ok(PostgresInstanceConfig {
        pg_config: pg_config_from_json(pg_config.clone())?,
        pg_bouncer_config: serde_json::from_value(pg_bouncer_config.clone())
            .map_err(|e| CloudError::usage(format!("invalid pgBouncerConfig: {e}")))?,
    })
}

/// Parse `--set key=value` overrides into a JSON object.
///
/// Each value is parsed as JSON first (so `max_connections=500` becomes a number),
/// falling back to a string if JSON parsing fails (`statement_timeout=5s`).
pub(super) fn parse_pg_config_overrides(
    sets: &[String],
) -> CloudResult<serde_json::Map<String, serde_json::Value>> {
    let mut out = serde_json::Map::new();
    for entry in sets {
        let (key, val) = entry.split_once('=').ok_or_else(|| {
            CloudError::usage(format!("invalid --set '{}': expected key=value", entry))
        })?;
        let key = key.trim();
        if key.is_empty() {
            return Err(CloudError::usage(format!(
                "invalid --set '{}': key cannot be empty",
                entry
            )));
        }
        let parsed = serde_json::from_str::<serde_json::Value>(val)
            .unwrap_or_else(|_| serde_json::Value::String(val.to_string()));
        out.insert(key.to_string(), parsed);
    }
    validate_pg_config(&serde_json::Value::Object(out.clone()))?;
    Ok(out)
}

pub async fn postgres_config_get(
    client: &CloudClient,
    postgres_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let cfg = client.get_postgres_config(&org_id, postgres_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&cfg)?);
    } else {
        print_human(&cfg)?;
    }
    Ok(())
}

pub async fn postgres_config_replace(
    client: &CloudClient,
    postgres_id: &str,
    file: &Path,
    json: bool,
) -> CloudResult<()> {
    let cfg = instance_config_from_json(&load_json_file::<serde_json::Value>("--file", file)?)?;
    let org_id = resolve_org_id(client).await?;
    let out = client
        .replace_postgres_config(&org_id, postgres_id, &cfg)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Configuration replaced");
        if let Some(msg) = &out.message {
            println!("Note: {}", msg);
        }
    }
    Ok(())
}

pub async fn postgres_config_patch(
    client: &CloudClient,
    postgres_id: &str,
    sets: &[String],
    file: Option<&Path>,
    json: bool,
) -> CloudResult<()> {
    debug_assert!(
        !sets.is_empty() || file.is_some(),
        "clap ArgGroup(\"patch_source\") requires --set or --file"
    );

    let cfg = if let Some(path) = file {
        instance_config_from_json(&load_json_file::<serde_json::Value>("--file", path)?)?
    } else {
        // Build the request body from --set entries: the overrides become the
        // pgConfig object, and pgBouncerConfig resolves to `{}`.
        let overrides = parse_pg_config_overrides(sets)?;
        instance_config_from_json(&serde_json::json!({
            "pgConfig": serde_json::Value::Object(overrides),
            "pgBouncerConfig": {},
        }))
        .map_err(|e| {
            CloudError::usage(format!("failed to build config from --set entries: {}", e))
        })?
    };

    let org_id = resolve_org_id(client).await?;
    let out = client
        .patch_postgres_config(&org_id, postgres_id, &cfg)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Configuration patched");
        if let Some(msg) = &out.message {
            println!("Note: {}", msg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clickhouse_cloud_api::models::PgBouncerConfig;

    /// Values parse as JSON first and fall back to a string; the last value
    /// wins on a duplicate key.
    #[test]
    fn parse_pg_config_overrides_reads_json_then_strings_and_keeps_the_last_duplicate() {
        let m = parse_pg_config_overrides(&[
            "max_connections=100".into(),
            "random_page_cost=1.1".into(),
            "statement_timeout=5s".into(),
            "max_connections=500".into(),
        ])
        .unwrap();
        assert_eq!(m.get("max_connections"), Some(&serde_json::json!(500)));
        assert_eq!(m.get("random_page_cost"), Some(&serde_json::json!(1.1)));
        assert_eq!(
            m.get("statement_timeout"),
            Some(&serde_json::Value::String("5s".to_string()))
        );
    }

    #[test]
    fn parse_pg_config_overrides_rejects_malformed_unknown_null_and_invalid_enum_values() {
        assert!(parse_pg_config_overrides(&["no_equals".into()]).is_err());
        assert!(parse_pg_config_overrides(&["=value".into()]).is_err());

        // An unknown key is quoted, and the valid keys are listed.
        let err = parse_pg_config_overrides(&["max_conections=500".into()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("'max_conections'"), "{err}");
        assert!(err.contains(&PG_CONFIG_KEYS.join(", ")), "{err}");

        // An invalid enum value names its key and lists the allowed values.
        for (entry, key, allowed) in [
            (
                "default_transaction_isolation=read uncommitted",
                "default_transaction_isolation",
                PgConfigDefaultTransactionIsolation::VALUES,
            ),
            (
                "ssl_min_protocol_version=SSLv3",
                "ssl_min_protocol_version",
                PgConfigSslMinProtocolVersion::VALUES,
            ),
            (
                "wal_compression=gzip",
                "wal_compression",
                PgConfigWalCompression::VALUES,
            ),
            (
                "default_toast_compression=zstd",
                "default_toast_compression",
                PgConfigDefaultToastCompression::VALUES,
            ),
            (
                "enable_parallel_hash=true",
                "enable_parallel_hash",
                PgConfigEnableParallelHash::VALUES,
            ),
            ("io_method=posix_aio", "io_method", PgConfigIoMethod::VALUES),
            (
                "log_lock_waits=1",
                "log_lock_waits",
                PgConfigLogLockWaits::VALUES,
            ),
        ] {
            let err = parse_pg_config_overrides(&[entry.into()])
                .unwrap_err()
                .to_string();
            assert!(err.contains(&format!("'{key}'")), "{entry}: {err}");
            assert!(err.ends_with(&allowed.join(", ")), "{entry}: {err}");
        }

        // A null names its key and lists nothing.
        let err = parse_pg_config_overrides(&["work_mem=null".into()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("'work_mem'") && err.contains("null"), "{err}");
        assert!(!err.contains(&PG_CONFIG_KEYS.join(", ")), "{err}");
    }

    #[test]
    fn instance_config_from_json_requires_explicit_sections() {
        for (document, missing) in [
            (serde_json::json!({}), "pgConfig"),
            (
                serde_json::json!({ "pgConfig": { "max_connections": 500 } }),
                "pgBouncerConfig",
            ),
            (
                serde_json::json!({ "pgBouncerConfig": { "default_pool_size": "16" } }),
                "pgConfig",
            ),
        ] {
            // The missing section, and the explicit empty value to use.
            let err = instance_config_from_json(&document)
                .unwrap_err()
                .to_string();
            assert!(err.contains(&format!("'{missing}'")), "{err}");
            assert!(err.contains("{}"), "{err}");
        }

        let empty = instance_config_from_json(
            &serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": {} }),
        )
        .unwrap();
        assert_eq!(empty.pg_config, PgConfig::default());
        assert_eq!(empty.pg_bouncer_config, PgBouncerConfig::default());
        // Explicitly empty sections reach the wire as `{}`, not as nulls.
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": {} })
        );
    }

    #[test]
    fn pg_config_files_reject_unknown_null_enum_and_non_object_values() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("pg.json");
        let parse_error = serde_json::from_str::<serde_json::Value>("{")
            .unwrap_err()
            .to_string();
        // Every failure names the file, and what in it was rejected.
        for (contents, expected) in [
            (r#"{"max_conections": 500}"#, &["'max_conections'"][..]),
            (r#"{"work_mem": null}"#, &["'work_mem'", "null"][..]),
            (
                r#"{"wal_compression": "gzip"}"#,
                &["'gzip'", "'wal_compression'"][..],
            ),
            (
                r#"{"enable_parallel_hash": true}"#,
                &["'enable_parallel_hash'"][..],
            ),
            ("[]", &[][..]),
            ("{", &[parse_error.as_str()][..]),
        ] {
            std::fs::write(&file, contents).unwrap();
            let error = load_pg_config_file(&file).unwrap_err().to_string();
            assert!(
                error.contains(&file.display().to_string()),
                "{contents}: {error}"
            );
            for identifier in expected {
                assert!(error.contains(identifier), "{contents}: {error}");
            }
        }
        let missing = directory.path().join("missing.json");
        let error = load_pg_config_file(&missing).unwrap_err().to_string();
        let io_error = std::fs::read_to_string(&missing).unwrap_err().to_string();
        assert!(
            error.contains(&missing.display().to_string()) && error.ends_with(&io_error),
            "{error}"
        );
    }

    #[test]
    fn pgbouncer_inputs_reject_non_string_values() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("pgbouncer.json");
        for value in [
            serde_json::json!(16),
            serde_json::json!(false),
            serde_json::json!(null),
        ] {
            let parameters = serde_json::json!({ "default_pool_size": value });
            std::fs::write(&file, parameters.to_string()).unwrap();
            // Both paths relay serde's own diagnostic for the value.
            let serde_error = serde_json::from_value::<PgBouncerConfig>(parameters.clone())
                .unwrap_err()
                .to_string();
            let error = load_json_file::<PgBouncerConfig>("--pg-bouncer-config-file", &file)
                .unwrap_err()
                .to_string();
            assert!(error.contains("pgbouncer.json"), "{value}: {error}");
            assert!(error.contains(&serde_error), "{value}: {error}");

            let error = instance_config_from_json(
                &serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": parameters }),
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("pgBouncerConfig"), "{value}: {error}");
            assert!(error.contains(&serde_error), "{value}: {error}");
        }
    }

    #[test]
    fn instance_config_from_json_accepts_both_sections() {
        let cfg = instance_config_from_json(&serde_json::json!({
            "pgConfig": { "max_connections": 500, "work_mem": "64MB" },
            "pgBouncerConfig": { "default_pool_size": "16", "future_parameter": "on" },
        }))
        .unwrap();
        assert_eq!(cfg.pg_config.max_connections, Some(serde_json::json!(500)));
        assert_eq!(cfg.pg_config.work_mem, Some(serde_json::json!("64MB")));
        assert_eq!(cfg.pg_bouncer_config["default_pool_size"], "16");
        assert_eq!(cfg.pg_bouncer_config["future_parameter"], "on");
    }

    /// A non-object root must not read as "no sections": that would send
    /// `{"pgConfig": {}, "pgBouncerConfig": {}}` and reset the config.
    #[test]
    fn instance_config_from_json_refuses_non_object_roots_and_invalid_or_unknown_sections() {
        // A non-object root has its own error, not a missing-section one.
        let missing_section = instance_config_from_json(&serde_json::json!({}))
            .unwrap_err()
            .to_string();
        for root in [
            serde_json::Value::Null,
            serde_json::json!([{ "pgConfig": {} }]),
            serde_json::json!("pgConfig"),
            serde_json::json!(7),
            serde_json::json!(true),
        ] {
            let err = instance_config_from_json(&root).unwrap_err().to_string();
            assert_ne!(err, missing_section, "{root}");
            assert!(!err.contains("pgConfig"), "{root}: {err}");
        }

        // Each error names the offending section, and only that one.
        for (document, expected, other) in [
            (
                serde_json::json!({ "pgConfig": 7, "pgBouncerConfig": {} }),
                "pgConfig",
                "pgBouncerConfig",
            ),
            (
                serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": [] }),
                "pgBouncerConfig",
                "pgConfig",
            ),
            (
                serde_json::json!({
                    "pgConfig": {},
                    "pgBouncerConfig": {},
                    "pgBouncerConfigs": {}
                }),
                "'pgBouncerConfigs'",
                "'pgConfig'",
            ),
        ] {
            let err = instance_config_from_json(&document)
                .unwrap_err()
                .to_string();
            assert!(err.contains(expected), "unexpected error: {err}");
            assert!(!err.contains(other), "unexpected error: {err}");
        }
    }

    #[test]
    fn pg_config_accepts_the_autovacuum_checkpoint_and_memory_gucs() {
        let input = serde_json::json!({
            "autovacuum_analyze_threshold": 50,
            "autovacuum_freeze_max_age": 200000000,
            "autovacuum_vacuum_insert_threshold": -1,
            "autovacuum_vacuum_threshold": 50,
            "checkpoint_timeout": "5min",
            "shared_memory_percent": 25,
            "wal_buffers": "16MB"
        });
        let parsed = pg_config_from_json(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), input);
    }

    #[test]
    fn pg_config_keys_cover_every_pg_config_field() {
        // An exhaustive literal: a new library field fails to compile here
        // until it is added to `PG_CONFIG_KEYS`.
        let config = PgConfig {
            autovacuum_analyze_scale_factor: Some(Default::default()),
            autovacuum_analyze_threshold: Some(Default::default()),
            autovacuum_freeze_max_age: Some(Default::default()),
            autovacuum_max_workers: Some(Default::default()),
            autovacuum_naptime: Some(Default::default()),
            autovacuum_vacuum_cost_delay: Some(Default::default()),
            autovacuum_vacuum_cost_limit: Some(Default::default()),
            autovacuum_vacuum_insert_scale_factor: Some(Default::default()),
            autovacuum_vacuum_insert_threshold: Some(Default::default()),
            autovacuum_vacuum_scale_factor: Some(Default::default()),
            autovacuum_vacuum_threshold: Some(Default::default()),
            autovacuum_work_mem: Some(Default::default()),
            bgwriter_delay: Some(Default::default()),
            bgwriter_flush_after: Some(Default::default()),
            bgwriter_lru_maxpages: Some(Default::default()),
            bgwriter_lru_multiplier: Some(Default::default()),
            checkpoint_timeout: Some(Default::default()),
            default_toast_compression: Some(Default::default()),
            default_transaction_isolation: Some(Default::default()),
            effective_cache_size: Some(Default::default()),
            effective_io_concurrency: Some(Default::default()),
            enable_parallel_hash: Some(Default::default()),
            hash_mem_multiplier: Some(Default::default()),
            idle_in_transaction_session_timeout: Some(Default::default()),
            idle_session_timeout: Some(Default::default()),
            io_method: Some(Default::default()),
            io_workers: Some(Default::default()),
            lock_timeout: Some(Default::default()),
            log_lock_waits: Some(Default::default()),
            log_min_duration_statement: Some(Default::default()),
            log_parameter_max_length: Some(Default::default()),
            log_temp_files: Some(Default::default()),
            logical_decoding_work_mem: Some(Default::default()),
            maintenance_io_concurrency: Some(Default::default()),
            maintenance_work_mem: Some(Default::default()),
            max_connections: Some(Default::default()),
            max_logical_replication_workers: Some(Default::default()),
            max_parallel_maintenance_workers: Some(Default::default()),
            max_parallel_workers: Some(Default::default()),
            max_parallel_workers_per_gather: Some(Default::default()),
            max_replication_slots: Some(Default::default()),
            max_slot_wal_keep_size: Some(Default::default()),
            max_sync_workers_per_subscription: Some(Default::default()),
            max_wal_senders: Some(Default::default()),
            max_wal_size: Some(Default::default()),
            max_worker_processes: Some(Default::default()),
            min_wal_size: Some(Default::default()),
            random_page_cost: Some(Default::default()),
            shared_memory_percent: Some(Default::default()),
            ssl_min_protocol_version: Some(Default::default()),
            statement_timeout: Some(Default::default()),
            temp_buffers: Some(Default::default()),
            track_activity_query_size: Some(Default::default()),
            transaction_timeout: Some(Default::default()),
            vacuum_cost_delay: Some(Default::default()),
            wal_buffers: Some(Default::default()),
            wal_compression: Some(Default::default()),
            wal_keep_size: Some(Default::default()),
            wal_sender_timeout: Some(Default::default()),
            work_mem: Some(Default::default()),
        };
        let serde_json::Value::Object(fields) = serde_json::to_value(config).unwrap() else {
            panic!("PgConfig must serialize as an object");
        };
        let mut expected = PG_CONFIG_KEYS.to_vec();
        expected.sort_unstable();
        assert_eq!(
            fields.keys().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn pg_config_validation_preserves_every_known_key_and_value() {
        let mut values = serde_json::Map::new();
        for key in PG_CONFIG_KEYS {
            let value = match key {
                "default_transaction_isolation" => serde_json::json!("repeatable read"),
                "ssl_min_protocol_version" => serde_json::json!("TLSv1.3"),
                "wal_compression" => serde_json::json!("zstd"),
                "default_toast_compression" => serde_json::json!("lz4"),
                "enable_parallel_hash" => serde_json::json!("off"),
                "io_method" => serde_json::json!("io_uring"),
                "log_lock_waits" => serde_json::json!("on"),
                "autovacuum_analyze_scale_factor" => serde_json::json!(false),
                _ => serde_json::json!(0),
            };
            values.insert(key.to_string(), value);
        }
        let input = serde_json::Value::Object(values);
        let parsed = pg_config_from_json(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), input);
    }
}
