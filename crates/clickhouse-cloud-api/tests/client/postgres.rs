use chrono::Utc;
use clickhouse_cloud_api::models::*;
use wiremock::matchers::{basic_auth, body_partial_json, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// PostgreSQL Services
// ===========================================================================

#[tokio::test]
async fn list_postgres_logs_with_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
        .and(query_param("from_date", "2026-08-01T00:00:00Z"))
        .and(query_param("to_date", "2026-08-02T00:00:00Z"))
        .and(query_param("body_contains", "checkpoint"))
        .and(query_param("severity", "LOG"))
        .and(query_param("sort_order", "asc"))
        .and(query_param("limit", "100"))
        .and(query_param("offset", "20"))
        .respond_with(ok_json(serde_json::json!([{
            "timestamp": "2026-08-01T12:00:00Z",
            "severity": "LOG",
            "body": "checkpoint complete"
        }])))
        .mount(&s)
        .await;

    let logs = c
        .postgres_logs_get_list(
            "org-1",
            "pg-1",
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z",
            Some("checkpoint"),
            Some("LOG"),
            Some(&PostgresLogsGetListSortorder::Asc),
            Some(100),
            Some(20),
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].body.as_deref(), Some("checkpoint complete"));
}

#[tokio::test]
async fn list_slow_query_patterns_with_typed_sorting() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .and(query_param("from_date", "2026-08-01T00:00:00Z"))
        .and(query_param("to_date", "2026-08-02T00:00:00Z"))
        .and(query_param("db_name", "analytics"))
        .and(query_param("db_user", "reporter"))
        .and(query_param("db_operation", "SELECT"))
        .and(query_param("app", "dashboard"))
        .and(query_param("sort_by", "p95_duration"))
        .and(query_param("sort_order", "asc"))
        .and(query_param("limit", "100"))
        .and(query_param("offset", "20"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let patterns = c
        .slow_query_patterns_get_list(
            "org-1",
            "pg-1",
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z",
            Some("analytics"),
            Some("reporter"),
            Some("SELECT"),
            Some("dashboard"),
            Some(&SlowQueryPatternsGetListSortby::P95_duration),
            Some(&SlowQueryPatternsGetListSortorder::Asc),
            Some(100),
            Some(20),
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(patterns.is_empty());
}

#[tokio::test]
async fn create_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres"))
        .and(body_partial_json(serde_json::json!({"name": "pg-svc", "provider": "aws", "region": "us-east-1", "size": "c6gd.large", "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-svc",
            "state": "creating",
            "provider": "aws",
            "region": "us-east-1",
            "password": "generated-pw"
        })))
        .mount(&s)
        .await;

    let body = PostgresServicePostRequest {
        name: "pg-svc".to_string(),
        provider: PgProvider::Aws,
        region: "us-east-1".to_string(),
        size: PgSize::C6gd_large,
        pg_bouncer_config: Some(PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ])),
        ..Default::default()
    };
    let resp = c.postgres_service_create("org-1", &body).await.unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-svc"));
    assert_eq!(pg.password.as_deref(), Some("generated-pw"));
}

#[tokio::test]
async fn list_postgres_services() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "pg-1",
                "state": "running"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.postgres_service_get_list("org-1").await.unwrap();
    let services = resp.result.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name.as_deref(), Some("pg-1"));
}

#[tokio::test]
async fn get_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-1",
            "state": "running",
            "connectionString": "postgres://user@host/db"
        })))
        .mount(&s)
        .await;

    let resp = c.postgres_service_get("org-1", "pg-1").await.unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1"));
    assert_eq!(
        pg.connection_string.as_deref(),
        Some("postgres://user@host/db")
    );
}

#[tokio::test]
async fn update_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .and(body_partial_json(serde_json::json!({"size": "c6gd.large"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-1"
        })))
        .mount(&s)
        .await;

    let body = PostgresServicePatchRequest {
        size: Some(PgSize::C6gd_large),
        ..Default::default()
    };
    let resp = c
        .postgres_service_patch("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1"));
}

#[tokio::test]
async fn delete_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.postgres_service_delete("org-1", "pg-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn update_postgres_service_state() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/state"))
        .and(body_partial_json(serde_json::json!({"command": "restart"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-1",
            "state": "restarting"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceSetState {
        command: PostgresServiceSetStateCommand::Restart,
    };
    let resp = c
        .postgres_service_patch_state("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1"));
}

#[tokio::test]
async fn set_postgres_password() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/password"))
        .and(body_partial_json(
            serde_json::json!({"password": "new-pg-password"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "password": "new-pg-password"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceSetPassword {
        password: "new-pg-password".to_string(),
    };
    let resp = c
        .postgres_service_set_password("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.password.as_deref(), Some("new-pg-password"));
}

#[tokio::test]
async fn get_postgres_certs() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/caCertificates"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----\n",
            ),
        )
        .mount(&s)
        .await;

    let resp = c.postgres_service_certs_get("org-1", "pg-1").await.unwrap();
    assert!(resp.contains("BEGIN CERTIFICATE"));
}

#[tokio::test]
async fn get_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .respond_with(ok_json(serde_json::json!({
            "pgConfig": {
                "max_connections": 100
            },
            "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
        })))
        .mount(&s)
        .await;

    let resp = c
        .postgres_instance_config_get("org-1", "pg-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(
        config.pg_bouncer_config.as_ref().unwrap()["default_pool_size"],
        "16"
    );
    assert_eq!(
        config.pg_bouncer_config.as_ref().unwrap()["future_parameter"],
        "on"
    );
    assert_eq!(
        config
            .pg_config
            .expect("pgConfig in response")
            .max_connections,
        Some(serde_json::json!(100))
    );
}

#[tokio::test]
async fn replace_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(body_partial_json(
            serde_json::json!({"pgConfig": {"max_connections": 200}, "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "message": "Configuration updated",
            "pgConfig": { "max_connections": 200 },
            "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
        })))
        .mount(&s)
        .await;

    let body = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(200)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ]),
    };
    let resp = c
        .postgres_instance_config_post("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["default_pool_size"],
        "16"
    );
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["future_parameter"],
        "on"
    );
    assert_eq!(result.message, Some("Configuration updated".to_string()));
    assert_eq!(
        result
            .pg_config
            .expect("pgConfig in response")
            .max_connections,
        Some(serde_json::json!(200))
    );
}

#[tokio::test]
async fn patch_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(body_partial_json(
            serde_json::json!({"pgConfig": {"max_connections": 150}, "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "message": "OK",
            "pgConfig": { "max_connections": 150 },
            "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
        })))
        .mount(&s)
        .await;

    let body = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(150)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ]),
    };
    let resp = c
        .postgres_instance_config_patch("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["default_pool_size"],
        "16"
    );
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["future_parameter"],
        "on"
    );
    assert_eq!(
        result
            .pg_config
            .expect("pgConfig in response")
            .max_connections,
        Some(serde_json::json!(150))
    );
}

#[tokio::test]
async fn create_postgres_read_replica() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/readReplica"))
        .and(body_partial_json(
            serde_json::json!({"name": "pg-1-replica", "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
            "name": "pg-1-replica",
            "isPrimary": false
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceReadReplicaRequest {
        name: "pg-1-replica".to_string(),
        pg_bouncer_config: Some(PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ])),
        ..Default::default()
    };
    let resp = c
        .postgres_instance_create_read_replica("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.is_primary, Some(false));
}

#[tokio::test]
async fn restore_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/restoredService",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "pg-1-restored", "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "cccccccc-dddd-eeee-ffff-000000000000",
            "name": "pg-1-restored",
            "state": "creating"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceRestoreRequest {
        name: "pg-1-restored".to_string(),
        pg_bouncer_config: Some(PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ])),
        restore_target: Utc::now(),
        ..Default::default()
    };
    let resp = c
        .postgres_instance_restore("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1-restored"));
}

#[tokio::test]
async fn postgres_instance_prometheus_get_returns_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/prometheus"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# HELP pg_metric\npg_metric 7\n"))
        .mount(&s)
        .await;

    let resp = c
        .postgres_instance_prometheus_get("org-1", "pg-1")
        .await
        .unwrap();
    assert!(resp.contains("pg_metric"));
}

#[tokio::test]
async fn postgres_org_prometheus_get_returns_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/prometheus"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("# HELP pg_org_metric\npg_org_metric 3\n"),
        )
        .mount(&s)
        .await;

    let resp = c.postgres_org_prometheus_get("org-1").await.unwrap();
    assert!(resp.contains("pg_org_metric"));
}

// ===========================================================================
// Postgres service backups
// ===========================================================================

#[tokio::test]
async fn postgres_service_backup_get_list_encodes_cursor_and_reads_envelope_pagination() {
    for (cursor, limit) in [
        (None, None),
        (Some("next+/=&? page"), None),
        (None, Some(25)),
        (Some("next+/=&? page"), Some(25)),
    ] {
        let (server, client) = setup().await;
        let backups = serde_json::json!([
            {
                "key": "basebackups_005/000000010000000000000002_backup_stop_sentinel.json",
                "lastModified": "2026-03-31T18:17:37Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/postgres/pg-1/backups"))
            .and(basic_auth("key", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": 200,
                "requestId": "req-test",
                "result": backups,
                "limit": 25,
                "totalCount": 2,
                "nextCursor": "another-page"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .postgres_service_backup_get_list("org-1", "pg-1", cursor, limit)
            .await
            .unwrap();
        assert_eq!(response.limit, Some(25));
        assert_eq!(response.total_count, Some(2));
        assert_eq!(response.next_cursor.as_deref(), Some("another-page"));
        let result = response.result.unwrap();
        assert_eq!(
            result[0].key.as_deref(),
            Some("basebackups_005/000000010000000000000002_backup_stop_sentinel.json")
        );
        assert_eq!(
            result[0].last_modified,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-03-31T18:17:37Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
        );
        let requests = server.received_requests().await.unwrap();
        let query: std::collections::HashMap<_, _> =
            requests[0].url.query_pairs().into_owned().collect();
        let mut expected = std::collections::HashMap::new();
        if let Some(cursor) = cursor {
            expected.insert("cursor".to_owned(), cursor.to_owned());
        }
        if let Some(limit) = limit {
            expected.insert("limit".to_owned(), limit.to_string());
        }
        assert_eq!(query, expected);
        if cursor.is_none() && limit.is_none() {
            assert!(requests[0].url.query().is_none());
        }
        assert!(requests[0].body.is_empty());
    }
}

#[tokio::test]
async fn postgres_service_backup_get_list_last_page_has_null_next_cursor() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/postgres/pg/backups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "req-test",
            "result": [],
            "limit": 100,
            "totalCount": 0,
            "nextCursor": null
        })))
        .expect(1)
        .mount(&server)
        .await;
    let response = client
        .postgres_service_backup_get_list("org", "pg", None, None)
        .await
        .unwrap();
    assert_eq!(response.result, Some(vec![]));
    assert_eq!(response.next_cursor, None);
    assert_eq!(response.total_count, Some(0));
}

#[tokio::test]
async fn postgres_service_backup_get_list_propagates_api_errors() {
    for (status, body, expected_message) in [
        (
            404,
            serde_json::json!({"status": 404, "error": "Postgres service not found"}).to_string(),
            "Postgres service not found",
        ),
        (500, "upstream failed".to_owned(), "upstream failed"),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org/postgres/pg/backups"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = client
            .postgres_service_backup_get_list("org", "pg", None, None)
            .await
            .unwrap_err();
        assert!(
            matches!(error, clickhouse_cloud_api::Error::Api { status: actual_status, ref message } if actual_status == status && message == expected_message),
            "unexpected error: {error:?}"
        );
    }
}
