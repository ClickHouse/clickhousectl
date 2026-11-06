use clickhouse_cloud_api::models::*;

#[test]
fn deserialize_postgres_service() {
    let json = r#"{
        "id": "44444444-5555-6666-7777-888888888888",
        "name": "my-postgres",
        "provider": "aws",
        "region": "us-east-1",
        "state": "running"
    }"#;
    let pg: PostgresService = serde_json::from_str(json).unwrap();
    assert_eq!(pg.name.as_deref(), Some("my-postgres"));
}

#[test]
fn postgres_gcp_provider_roundtrips() {
    let provider: PgProvider = serde_json::from_str(r#""gcp""#).unwrap();
    assert_eq!(provider, PgProvider::Gcp);
    assert_eq!(provider.to_string(), "gcp");
    assert_eq!(serde_json::to_string(&provider).unwrap(), r#""gcp""#);
    assert_eq!(PgProvider::VALUES, &["aws", "gcp"]);
}

#[test]
fn postgres_config_enum_values_roundtrip_and_match_values() {
    for value in PgConfigDefaultTransactionIsolation::VALUES {
        let parsed: PgConfigDefaultTransactionIsolation =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigDefaultTransactionIsolation::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in PgConfigSslMinProtocolVersion::VALUES {
        let parsed: PgConfigSslMinProtocolVersion =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigSslMinProtocolVersion::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in PgConfigWalCompression::VALUES {
        let parsed: PgConfigWalCompression =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigWalCompression::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in PgConfigDefaultToastCompression::VALUES {
        let parsed: PgConfigDefaultToastCompression =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigDefaultToastCompression::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in PgConfigEnableParallelHash::VALUES {
        let parsed: PgConfigEnableParallelHash =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigEnableParallelHash::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in PgConfigIoMethod::VALUES {
        let parsed: PgConfigIoMethod = serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigIoMethod::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in PgConfigLogLockWaits::VALUES {
        let parsed: PgConfigLogLockWaits = serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, PgConfigLogLockWaits::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    let future: PgConfigIoMethod = serde_json::from_str(r#""posix_aio""#).unwrap();
    assert_eq!(future, PgConfigIoMethod::Unknown("posix_aio".to_string()));
    assert_eq!(future.to_string(), "posix_aio");
}

#[test]
fn postgres_slow_query_sort_values_roundtrip_as_known_variants() {
    for value in SlowQueryPatternsGetListSortby::VALUES {
        let parsed: SlowQueryPatternsGetListSortby =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, SlowQueryPatternsGetListSortby::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
    for value in SlowQueryPatternsGetListSortorder::VALUES {
        let parsed: SlowQueryPatternsGetListSortorder =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, SlowQueryPatternsGetListSortorder::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
    }
}

#[test]
fn postgres_gcp_sizes_roundtrip_as_known_variants() {
    let sizes = [
        "c4a-highmem-lssd-4",
        "c4a-highmem-lssd-8",
        "c4a-highmem-lssd-16",
        "c4a-highmem-lssd-32",
        "c4a-highmem-lssd-48",
        "c4a-highmem-lssd-64",
        "c4a-highmem-lssd-72",
        "c4a-standard-lssd-4",
        "c4a-standard-lssd-8",
        "c4a-standard-lssd-16",
        "c4a-standard-lssd-32",
        "c4a-standard-lssd-48",
        "c4a-standard-lssd-64",
        "c4a-standard-lssd-72",
        "c4-highmem-lssd-4",
        "c4-highmem-lssd-8",
        "c4-highmem-lssd-16",
        "c4-highmem-lssd-24",
        "c4-highmem-lssd-32",
        "c4-highmem-lssd-48",
        "c4-highmem-lssd-96",
        "c4-highmem-lssd-144",
        "c4-highmem-lssd-192",
        "c4-highmem-lssd-288",
        "c4-standard-lssd-4",
        "c4-standard-lssd-8",
        "c4-standard-lssd-16",
        "c4-standard-lssd-24",
        "c4-standard-lssd-32",
        "c4-standard-lssd-48",
        "c4-standard-lssd-96",
        "c4-standard-lssd-144",
        "c4-standard-lssd-192",
        "c4-standard-lssd-288",
        "c4d-highmem-lssd-8",
        "c4d-highmem-lssd-16",
        "c4d-highmem-lssd-32",
        "c4d-highmem-lssd-48",
        "c4d-highmem-lssd-64",
        "c4d-highmem-lssd-96",
        "c4d-highmem-lssd-192",
        "c4d-highmem-lssd-384",
        "c4d-standard-lssd-8",
        "c4d-standard-lssd-16",
        "c4d-standard-lssd-32",
        "c4d-standard-lssd-48",
        "c4d-standard-lssd-64",
        "c4d-standard-lssd-96",
        "c4d-standard-lssd-192",
        "c4d-standard-lssd-384",
        "z3-highmem-highlssd-8",
        "z3-highmem-highlssd-16",
        "z3-highmem-highlssd-22",
        "z3-highmem-highlssd-32",
        "z3-highmem-highlssd-44",
        "z3-highmem-highlssd-88",
        "z3-highmem-standardlssd-14",
        "z3-highmem-standardlssd-22",
        "z3-highmem-standardlssd-44",
        "z3-highmem-standardlssd-88",
        "z3-highmem-standardlssd-176",
    ];

    for wire_value in sizes {
        let size: PgSize = serde_json::from_value(wire_value.into()).unwrap();
        assert!(
            !matches!(&size, PgSize::Unknown(_)),
            "{wire_value} deserialized through the catch-all"
        );
        assert_eq!(size.to_string(), wire_value);
        assert_eq!(serde_json::to_value(&size).unwrap(), wire_value);
    }
}

#[test]
fn serialize_postgres_service_post_request() {
    let req = PostgresServicePostRequest {
        name: "pg-new".to_string(),
        provider: PgProvider::Aws,
        region: "us-east-1".to_string(),
        size: PgSize::C6gd_large,
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "pg-new");
    assert_eq!(json["provider"], "aws");
    assert_eq!(json["region"], "us-east-1");
    assert_eq!(json["size"], "c6gd.large");
    assert!(json.get("storageSize").is_none());
    // Optional fields omitted
    assert!(json.get("haType").is_none());
    assert!(json.get("pgConfig").is_none());
    assert!(json.get("pgBouncerConfig").is_none());
}

#[test]
fn serialize_postgres_service_set_state() {
    let req = PostgresServiceSetState {
        command: PostgresServiceSetStateCommand::Restart,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "restart");
}

#[test]
fn serialize_postgres_service_set_password() {
    let req = PostgresServiceSetPassword {
        password: "s3cur3".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["password"], "s3cur3");
}

#[test]
fn serialize_postgres_read_replica_request() {
    let req = PostgresServiceReadReplicaRequest {
        name: "pg-replica".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "pg-replica");
    assert!(json.get("pgConfig").is_none());
    assert!(json.get("pgBouncerConfig").is_none());
}

#[test]
fn slow_query_sort_enums_preserve_unknown_values() {
    let sort_by: SlowQueryPatternsGetListSortby =
        serde_json::from_str(r#""future_metric""#).unwrap();
    let sort_order: SlowQueryPatternsGetListSortorder =
        serde_json::from_str(r#""future_order""#).unwrap();

    assert_eq!(sort_by.to_string(), "future_metric");
    assert_eq!(sort_order.to_string(), "future_order");
}

#[test]
fn serialize_postgres_instance_config() {
    let config = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(200)),
            autovacuum_max_workers: Some(serde_json::json!(5)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::default(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["pgConfig"]["max_connections"], 200);
    assert_eq!(json["pgConfig"]["autovacuum_max_workers"], 5);
    assert!(json.get("pgBouncerConfig").is_some());
}

#[test]
fn serialize_postgres_instance_config_always_includes_both_nested() {
    // The live API rejects PATCH/POST bodies that omit either `pgConfig` or
    // `pgBouncerConfig` with `BAD_REQUEST: ... 'undefined'`, so the envelope
    // always serializes both — defaulting to `{}` — while inner pgConfig
    // fields stay opt-in. See #163 for the matrix evidence.
    let config = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(200)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::default(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert!(
        json.get("pgBouncerConfig").is_some(),
        "pgBouncerConfig must always be present"
    );
    assert_eq!(json["pgBouncerConfig"], serde_json::json!({}));
    assert_eq!(json["pgConfig"]["max_connections"], 200);
    let pg = json["pgConfig"].as_object().unwrap();
    assert_eq!(
        pg.len(),
        1,
        "PgConfig should only serialize the one set field, got {pg:?}"
    );
}

#[test]
fn serialize_postgres_instance_config_default_envelope() {
    // Default envelope serializes to the minimal accepted body shape.
    let json = serde_json::to_value(PostgresInstanceConfig::default()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": {} })
    );
}

#[test]
fn postgres_service_ignores_extra_fields() {
    let json = r#"{"name":"pg","state":"running","maintenanceWindow":"sun-02:00"}"#;
    let pg: PostgresService = serde_json::from_str(json).unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg"));
}

#[test]
fn postgres_service_minimal_response() {
    let pg: PostgresService =
        serde_json::from_str(r#"{"id":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}"#).unwrap();
    // Every response field is `Option<T>`: an omitted key is `None`, not a
    // fabricated zero value.
    assert_eq!(pg.name, None);
    assert_eq!(pg.state, None);
}

#[test]
fn postgres_service_response_tolerates_dropped_and_null_fields() {
    // A response field the API stops sending, and one it sends as an explicit
    // `null`, must both land as `None` rather than failing the response. `null`
    // is the case `#[serde(default)]` never covered: it only fills a missing
    // key.
    let dropped: PostgresService = serde_json::from_str("{}").unwrap();
    let nulled: PostgresService = serde_json::from_str(
        r#"{"id":null,"name":null,"state":null,"tags":null,"storageSize":null,"createdAt":null}"#,
    )
    .unwrap();
    assert_eq!(dropped, PostgresService::default());
    assert_eq!(nulled, PostgresService::default());
    assert_eq!(nulled.tags, None);
    assert_eq!(nulled.storage_size, None);
}

#[test]
fn postgres_service_response_omits_absent_fields_when_serialized() {
    // Absent means absent: a response field that was not returned is omitted
    // from `--json` output rather than emitted as `null`.
    let pg = PostgresService {
        name: Some("pg-1".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&pg).unwrap();
    assert_eq!(json, serde_json::json!({ "name": "pg-1" }));
}

#[test]
fn postgres_instance_config_response_converts_back_into_a_request_body() {
    let response = PostgresInstanceConfigResponse {
        pg_config: Some(PgConfigResponse {
            max_connections: Some(serde_json::json!(200)),
            ..Default::default()
        }),
        pg_bouncer_config: Some(PgBouncerConfigResponse::from([
            ("default_pool_size".to_string(), "16".to_string()),
            ("future_parameter".to_string(), "on".to_string()),
        ])),
    };
    let request = PostgresInstanceConfig::try_from(response).unwrap();
    assert_eq!(
        request.pg_config.max_connections,
        Some(serde_json::json!(200))
    );
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({ "pgConfig": { "max_connections": 200 }, "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"} })
    );
}

#[test]
fn postgres_instance_config_response_conversion_reports_missing_required_fields() {
    // Both nested objects are required in a write body, so the write-back
    // conversion must fail loudly instead of inventing empty objects.
    let missing_bouncer = PostgresInstanceConfig::try_from(PostgresInstanceConfigResponse {
        pg_config: Some(PgConfigResponse::default()),
        pg_bouncer_config: None,
    })
    .unwrap_err();
    assert_eq!(missing_bouncer.fields(), ["pgBouncerConfig"]);

    let missing_both =
        PostgresInstanceConfig::try_from(PostgresInstanceConfigResponse::default()).unwrap_err();
    assert_eq!(missing_both.fields(), ["pgBouncerConfig", "pgConfig"]);
    assert_eq!(
        missing_both.to_string(),
        "the API response is missing required field(s): pgBouncerConfig, pgConfig"
    );
}

#[test]
fn deserialize_postgres_instance_config() {
    let json = r#"{
        "pgConfig": {
            "max_connections": 200,
            "shared_buffers": "256MB"
        },
        "pgBouncerConfig": {}
    }"#;
    let config: PostgresInstanceConfigResponse = serde_json::from_str(json).unwrap();
    assert_eq!(
        config.pg_config.unwrap().max_connections,
        Some(serde_json::json!(200))
    );
}

#[test]
fn deserialize_postgres_instance_config_string_wrapped_numbers() {
    // The live GET endpoint returns numeric pgConfig values wrapped in JSON
    // strings (e.g. "max_connections": "100"). The spec types these fields
    // as string-or-number, so they are modelled as serde_json::Value and
    // both representations must deserialize.
    let json = r#"{
        "pgConfig": {
            "max_connections": "100",
            "random_page_cost": "1.1",
            "max_worker_processes": 8,
            "autovacuum_naptime": "5s",
            "autovacuum_vacuum_scale_factor": "0.2",
            "autovacuum_max_workers": 3
        },
        "pgBouncerConfig": {}
    }"#;
    let config: PostgresInstanceConfigResponse = serde_json::from_str(json).unwrap();
    let pg_config = config.pg_config.expect("pgConfig present in the payload");
    assert_eq!(pg_config.max_connections, Some(serde_json::json!("100")));
    assert_eq!(pg_config.random_page_cost, Some(serde_json::json!("1.1")));
    assert_eq!(pg_config.max_worker_processes, Some(serde_json::json!(8)));
    assert_eq!(pg_config.autovacuum_naptime, Some(serde_json::json!("5s")));
    assert_eq!(
        pg_config.autovacuum_vacuum_scale_factor,
        Some(serde_json::json!("0.2"))
    );
    assert_eq!(pg_config.autovacuum_max_workers, Some(serde_json::json!(3)));
}

#[test]
fn pg_state_property_stopped() {
    let parsed: PgStateProperty = serde_json::from_str("\"stopped\"").unwrap();
    assert_eq!(parsed, PgStateProperty::Stopped);
    assert_eq!(parsed.to_string(), "stopped");
}

#[test]
fn pgbouncer_map_round_trips_arbitrary_string_parameters() {
    let document = serde_json::json!({
        "default_pool_size": "16", "future_parameter": "on", "empty": ""
    });
    let request: PgBouncerConfig = serde_json::from_value(document.clone()).unwrap();
    let response: PgBouncerConfigResponse = serde_json::from_value(document.clone()).unwrap();
    assert_eq!(request, response);
    assert_eq!(serde_json::to_value(request).unwrap(), document);
    assert_eq!(serde_json::to_value(response).unwrap(), document);
}

#[test]
fn pgbouncer_requests_reject_non_string_values() {
    for value in [
        serde_json::json!(16),
        serde_json::json!(true),
        serde_json::json!(null),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        assert!(
            serde_json::from_value::<PgBouncerConfig>(
                serde_json::json!({"default_pool_size": value})
            )
            .is_err()
        );
    }
}

#[test]
fn pgbouncer_parent_responses_preserve_missing_null_and_empty_sections() {
    for document in [
        serde_json::json!({}),
        serde_json::json!({"pgBouncerConfig": null}),
    ] {
        let config: PostgresInstanceConfigResponse =
            serde_json::from_value(document.clone()).unwrap();
        assert!(config.pg_bouncer_config.is_none());
        assert_eq!(serde_json::to_value(config).unwrap(), serde_json::json!({}));
        let update: PostgresInstanceUpdateConfigResponse =
            serde_json::from_value(document).unwrap();
        assert!(update.pg_bouncer_config.is_none());
        assert_eq!(serde_json::to_value(update).unwrap(), serde_json::json!({}));
    }
    let config: PostgresInstanceConfigResponse = serde_json::from_value(
        serde_json::json!({"pgBouncerConfig": {}, "futureField": "ignored"}),
    )
    .unwrap();
    assert_eq!(config.pg_bouncer_config, Some(PgBouncerConfig::new()));
    assert_eq!(
        serde_json::to_value(config).unwrap(),
        serde_json::json!({"pgBouncerConfig": {}})
    );
}

#[test]
fn postgres_slow_query_durations_preserve_fractions_and_integral_counts() {
    let aggregate = serde_json::json!({
        "avgDurationUs": 1190.8419405320817,
        "maxDurationUs": 2000.75,
        "p50DurationUs": 1100.25,
        "p95DurationUs": 1800.5,
        "p99DurationUs": 1950.125,
        "totalDurationUs": 3572.525821596245,
        "callCount": 3, "errorCount": 0, "totalRows": 9007199254740993_i64,
        "totalSharedBlksHit": 10, "totalSharedBlksRead": 2,
        "totalCpuTimeUs": 2000, "totalWalBytes": 128
    });
    let list: Vec<PostgresSlowQueryPattern> =
        serde_json::from_value(serde_json::json!([aggregate])).unwrap();
    assert_eq!(list[0].avg_duration_us, Some(1190.8419405320817));
    assert_eq!(list[0].total_rows, Some(9007199254740993));
    assert_eq!(
        serde_json::to_value(&list).unwrap(),
        serde_json::json!([aggregate])
    );

    let detail = serde_json::json!({
        "aggregate": aggregate,
        "recentExecutions": [{"durationUs": 1234.56789, "rows": 9007199254740993_i64,
            "cpuSysTimeUs": 12, "jitFunctions": 2, "sharedBlksHit": 3, "walBytes": 128}]
    });
    let parsed: PostgresSlowQueryPatternDetail = serde_json::from_value(detail.clone()).unwrap();
    assert_eq!(
        parsed.recent_executions.as_ref().unwrap()[0].duration_us,
        Some(1234.56789)
    );
    assert_eq!(serde_json::to_value(parsed).unwrap(), detail);

    // Existing integer-valued measurements remain accepted as durations.
    let integer: PostgresQueryExecution =
        serde_json::from_value(serde_json::json!({"durationUs": 1234})).unwrap();
    assert_eq!(integer.duration_us, Some(1234.0));
    let integer: PostgresSlowQueryPattern = serde_json::from_value(serde_json::json!({
        "avgDurationUs": 1, "maxDurationUs": 2, "p50DurationUs": 1,
        "p95DurationUs": 2, "p99DurationUs": 2, "totalDurationUs": 3
    }))
    .unwrap();
    assert_eq!(integer.avg_duration_us, Some(1.0));
    for field in [
        "callCount",
        "errorCount",
        "totalRows",
        "totalSharedBlksHit",
        "totalSharedBlksRead",
        "totalWalBytes",
    ] {
        assert!(
            serde_json::from_value::<PostgresSlowQueryPattern>(serde_json::json!({field: 1.5}))
                .is_err(),
            "{field} must remain integral"
        );
    }
    for field in ["rows", "jitFunctions", "sharedBlksHit", "walBytes"] {
        assert!(
            serde_json::from_value::<PostgresQueryExecution>(serde_json::json!({field: 1.5}))
                .is_err(),
            "{field} must remain integral"
        );
    }
}

#[test]
fn postgres_slow_query_durations_allow_missing_and_null() {
    for aggregate in [
        serde_json::json!({}),
        serde_json::json!({
            "avgDurationUs": null, "maxDurationUs": null, "p50DurationUs": null,
            "p95DurationUs": null, "p99DurationUs": null, "totalDurationUs": null
        }),
    ] {
        let parsed: PostgresSlowQueryPattern = serde_json::from_value(aggregate.clone()).unwrap();
        assert_eq!(parsed, PostgresSlowQueryPattern::default());
        assert_eq!(serde_json::to_value(parsed).unwrap(), serde_json::json!({}));
        let detail: PostgresSlowQueryPatternDetail = serde_json::from_value(serde_json::json!({
            "aggregate": aggregate, "recentExecutions": [{}, {"durationUs": null}]
        }))
        .unwrap();
        assert_eq!(detail.aggregate, Some(PostgresSlowQueryPattern::default()));
        assert_eq!(
            detail.recent_executions,
            Some(vec![PostgresQueryExecution::default(); 2])
        );
        assert_eq!(
            serde_json::to_value(detail).unwrap(),
            serde_json::json!({"aggregate": {}, "recentExecutions": [{}, {}]})
        );
    }
    for value in [
        serde_json::json!({}),
        serde_json::json!({"aggregate": null, "recentExecutions": null}),
    ] {
        let parsed: PostgresSlowQueryPatternDetail = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, PostgresSlowQueryPatternDetail::default());
        assert_eq!(serde_json::to_value(parsed).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn postgres_backup_tolerates_missing_and_null_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "key": null, "lastModified": null, "status": null, "futureField": 1
        }),
    ] {
        let backup: PostgresBackup = serde_json::from_value(wire).unwrap();
        assert_eq!(backup, PostgresBackup::default());
        assert_eq!(serde_json::to_value(backup).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn postgres_backup_round_trips_every_field() {
    let wire = serde_json::json!({
        "key": "basebackups_005/000000010000000000000002_backup_stop_sentinel.json",
        "lastModified": "2026-03-31T18:17:37Z",
        "status": "ready"
    });
    let backup: PostgresBackup = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        backup.key.as_deref(),
        Some("basebackups_005/000000010000000000000002_backup_stop_sentinel.json")
    );
    assert!(backup.last_modified.is_some());
    assert_eq!(backup.status.as_deref(), Some("ready"));
    assert_eq!(serde_json::to_value(backup).unwrap(), wire);
}

#[test]
fn postgres_service_access_list_and_restore_time_tolerate_missing_and_null() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"ipAccessList": null, "earliestRestoreTime": null}),
    ] {
        let pg: PostgresService = serde_json::from_value(wire).unwrap();
        assert_eq!(pg.ip_access_list, None);
        assert_eq!(pg.earliest_restore_time, None);
        assert_eq!(serde_json::to_value(pg).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn postgres_service_round_trips_access_list_and_restore_time() {
    let wire = serde_json::json!({
        "earliestRestoreTime": "2026-03-31T18:17:37Z",
        "ipAccessList": [
            {
                "id": "rule-1",
                "source": "203.0.113.0/24",
                "portRange": "5432",
                "description": "office"
            },
            {"source": "198.51.100.7/32", "portRange": "5432..6432"}
        ]
    });
    let pg: PostgresService = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        pg.earliest_restore_time,
        Some("2026-03-31T18:17:37Z".parse().unwrap())
    );
    let rules = pg.ip_access_list.as_ref().unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].id.as_deref(), Some("rule-1"));
    assert_eq!(rules[0].port_range.as_deref(), Some("5432"));
    assert_eq!(rules[1].description, None);
    assert_eq!(serde_json::to_value(pg).unwrap(), wire);
}

#[test]
fn postgres_ip_access_list_entry_response_tolerates_missing_and_null() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "id": null, "source": null, "portRange": null, "description": null,
            "futureField": 1
        }),
    ] {
        let entry: PostgresIpAccessListEntryResponse = serde_json::from_value(wire).unwrap();
        assert_eq!(entry, PostgresIpAccessListEntryResponse::default());
        assert_eq!(serde_json::to_value(entry).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn postgres_ip_access_list_request_types_are_strict() {
    // `source` and `portRange` are required on a rule, `id` on a rule
    // reference, and both lists on a patch: a body missing one is rejected
    // rather than silently defaulted.
    assert!(serde_json::from_str::<PostgresIpAccessListEntry>("{}").is_err());
    assert!(
        serde_json::from_str::<PostgresIpAccessListEntry>(r#"{"source":"203.0.113.0/24"}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<PostgresIpAccessListEntry>(r#"{"portRange":"5432"}"#).is_err());
    assert!(serde_json::from_str::<PostgresIpAccessListRuleReference>("{}").is_err());
    assert!(serde_json::from_str::<PostgresIpAccessListPatch>(r#"{"add":[]}"#).is_err());
    assert!(serde_json::from_str::<PostgresIpAccessListPatch>(r#"{"remove":[]}"#).is_err());

    // A new rule omits `id` and `description`.
    let entry = PostgresIpAccessListEntry {
        source: "203.0.113.0/24".to_string(),
        port_range: "5432".to_string(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&entry).unwrap(),
        serde_json::json!({"source": "203.0.113.0/24", "portRange": "5432"})
    );
}

#[test]
fn postgres_ip_access_list_entry_response_converts_back_into_a_request_entry() {
    let entry = PostgresIpAccessListEntry::try_from(PostgresIpAccessListEntryResponse {
        id: Some("rule-1".to_string()),
        source: Some("203.0.113.0/24".to_string()),
        port_range: Some("6432".to_string()),
        description: Some("office".to_string()),
    })
    .unwrap();
    // The returned `id` survives, so writing the rule back edits it.
    assert_eq!(
        serde_json::to_value(&entry).unwrap(),
        serde_json::json!({
            "id": "rule-1", "source": "203.0.113.0/24", "portRange": "6432",
            "description": "office"
        })
    );

    let missing_port = PostgresIpAccessListEntry::try_from(PostgresIpAccessListEntryResponse {
        source: Some("203.0.113.0/24".to_string()),
        ..Default::default()
    })
    .unwrap_err();
    assert_eq!(missing_port.fields(), ["portRange"]);

    let missing_both =
        PostgresIpAccessListEntry::try_from(PostgresIpAccessListEntryResponse::default())
            .unwrap_err();
    assert_eq!(missing_both.fields(), ["portRange", "source"]);
}

#[test]
fn postgres_service_post_request_distinguishes_omitted_and_empty_access_list() {
    let base = PostgresServicePostRequest {
        name: "pg-new".to_string(),
        provider: PgProvider::Aws,
        region: "us-east-1".to_string(),
        size: PgSize::C6gd_large,
        ..Default::default()
    };
    // Omitted allows all IPv4 and IPv6 addresses.
    let omitted = serde_json::to_value(&base).unwrap();
    assert!(omitted.get("ipAccessList").is_none(), "{omitted}");

    // An empty list blocks public access, so it must reach the wire as `[]`.
    let blocked = PostgresServicePostRequest {
        ip_access_list: Some(Vec::new()),
        ..base.clone()
    };
    let blocked = serde_json::to_value(&blocked).unwrap();
    assert_eq!(blocked["ipAccessList"], serde_json::json!([]));

    let allowed = PostgresServicePostRequest {
        ip_access_list: Some(vec![PostgresIpAccessListEntry {
            source: "203.0.113.0/24".to_string(),
            port_range: "5432".to_string(),
            ..Default::default()
        }]),
        ..base
    };
    assert_eq!(
        serde_json::to_value(&allowed).unwrap()["ipAccessList"],
        serde_json::json!([{"source": "203.0.113.0/24", "portRange": "5432"}])
    );
}

#[test]
fn postgres_service_patch_request_serializes_the_access_list_patch() {
    let untouched = PostgresServicePatchRequest::default();
    assert_eq!(
        serde_json::to_value(&untouched).unwrap(),
        serde_json::json!({})
    );

    let patch = PostgresServicePatchRequest {
        ip_access_list: Some(PostgresIpAccessListPatch {
            add: vec![PostgresIpAccessListEntry {
                id: Some("rule-1".to_string()),
                source: "203.0.113.0/24".to_string(),
                port_range: "6432".to_string(),
                description: None,
            }],
            remove: vec![PostgresIpAccessListRuleReference {
                id: "rule-2".to_string(),
            }],
        }),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&patch).unwrap(),
        serde_json::json!({
            "ipAccessList": {
                "add": [{"id": "rule-1", "source": "203.0.113.0/24", "portRange": "6432"}],
                "remove": [{"id": "rule-2"}]
            }
        })
    );
}

#[test]
fn pg_config_carries_new_gucs_through_write_back() {
    let response: PgConfigResponse = serde_json::from_value(serde_json::json!({
        "autovacuum_analyze_threshold": 50,
        "autovacuum_freeze_max_age": 200000000,
        "autovacuum_vacuum_insert_threshold": -1,
        "autovacuum_vacuum_threshold": 50,
        "checkpoint_timeout": "5min",
        "shared_memory_percent": 25,
        "wal_buffers": "16MB"
    }))
    .unwrap();
    let missing: PgConfigResponse = serde_json::from_str(
        r#"{"autovacuum_analyze_threshold":null,"checkpoint_timeout":null,"wal_buffers":null}"#,
    )
    .unwrap();
    assert_eq!(missing, PgConfigResponse::default());

    let request = PgConfig::from(response);
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({
            "autovacuum_analyze_threshold": 50,
            "autovacuum_freeze_max_age": 200000000,
            "autovacuum_vacuum_insert_threshold": -1,
            "autovacuum_vacuum_threshold": 50,
            "checkpoint_timeout": "5min",
            "shared_memory_percent": 25,
            "wal_buffers": "16MB"
        })
    );
}

#[test]
fn pg_config_carries_io_logging_and_replication_gucs_through_write_back() {
    let wire = serde_json::json!({
        "bgwriter_delay": "200ms",
        "bgwriter_flush_after": "512kB",
        "bgwriter_lru_maxpages": 100,
        "bgwriter_lru_multiplier": 2,
        "default_toast_compression": "lz4",
        "enable_parallel_hash": "off",
        "hash_mem_multiplier": 2,
        "io_method": "io_uring",
        "io_workers": 3,
        "log_lock_waits": "on",
        "log_min_duration_statement": 1000,
        "log_parameter_max_length": "-1",
        "log_temp_files": -1,
        "logical_decoding_work_mem": "64MB",
        "maintenance_io_concurrency": 10,
        "max_logical_replication_workers": 4,
        "max_replication_slots": 10,
        "max_sync_workers_per_subscription": 2,
        "max_wal_senders": 10,
        "temp_buffers": "8MB",
        "track_activity_query_size": 4096,
        "vacuum_cost_delay": "0"
    });
    let response: PgConfigResponse = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        response.default_toast_compression,
        Some(PgConfigDefaultToastCompression::Lz4)
    );
    assert_eq!(
        response.enable_parallel_hash,
        Some(PgConfigEnableParallelHash::Off)
    );
    assert_eq!(response.io_method, Some(PgConfigIoMethod::Io_uring));
    assert_eq!(response.log_lock_waits, Some(PgConfigLogLockWaits::On));
    assert_eq!(response.log_temp_files, Some(serde_json::json!(-1)));

    let missing: PgConfigResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, PgConfigResponse::default());
    let null_fields = wire
        .as_object()
        .unwrap()
        .keys()
        .map(|key| (key.clone(), serde_json::Value::Null))
        .collect::<serde_json::Map<_, _>>();
    let nulls: PgConfigResponse =
        serde_json::from_value(serde_json::Value::Object(null_fields)).unwrap();
    assert_eq!(nulls, PgConfigResponse::default());

    // Write-back is lossless: every new GUC survives the response -> request
    // conversion with its wire name and value unchanged.
    let request = PgConfig::from(response);
    assert_eq!(serde_json::to_value(&request).unwrap(), wire);
}

#[test]
fn pg_config_serializes_only_the_new_gucs_that_are_set() {
    let config = PgConfig {
        io_method: Some(PgConfigIoMethod::Worker),
        io_workers: Some(serde_json::json!(8)),
        log_min_duration_statement: Some(serde_json::json!("250ms")),
        max_wal_senders: Some(serde_json::json!(20)),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&config).unwrap(),
        serde_json::json!({
            "io_method": "worker",
            "io_workers": 8,
            "log_min_duration_statement": "250ms",
            "max_wal_senders": 20
        })
    );
}
