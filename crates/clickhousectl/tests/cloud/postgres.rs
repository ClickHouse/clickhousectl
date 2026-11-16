//! `cloud postgres` (src/cloud/postgres/).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Postgres metrics (issue #583) ─────────────────────────────────────────

#[tokio::test]
async fn postgres_metrics_sends_exact_query_supports_oauth_and_preserves_json() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    let result = serde_json::json!({
        "metrics": [{
            "key": "cpu",
            "name": "CPU usage",
            "description": "Average CPU usage",
            "unit": "percent",
            "series": [{
                "label": "primary",
                "dataPoints": [
                    { "timestamp": 1776337200, "value": 12.5 },
                    { "timestamp": 1776337260, "value": 13.25 }
                ]
            }]
        }, {
            "key": "replication-lag",
            "series": []
        }]
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}/metrics"
        )))
        .and(query_param("from_date", "2026-04-16T11:00:00.000Z"))
        .and(query_param("to_date", "2026-04-16T12:00:00.000Z"))
        .and(query_param("bucket_size_seconds", "60"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-postgres-metrics"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "metrics",
            postgres_id,
            "--from-date",
            "2026-04-16T12:00:00+01:00",
            "--to-date",
            "2026-04-16T13:00:00+01:00",
            "--bucket-size-seconds",
            "60",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        [
            (
                "from_date".to_string(),
                "2026-04-16T11:00:00.000Z".to_string()
            ),
            (
                "to_date".to_string(),
                "2026-04-16T12:00:00.000Z".to_string()
            ),
            ("bucket_size_seconds".to_string(), "60".to_string()),
        ]
    );
}

/// The metrics handler, not clap, checks the range order
/// (`validate_datetime_range`), so a reverse range must stop before any request.
#[tokio::test]
async fn postgres_metrics_handler_rejects_a_reverse_range_before_any_request() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T13:00:00Z",
            "--to-date",
            "2026-04-16T12:00:00Z",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_metrics_omits_bucket_and_renders_every_sparse_series_and_point() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/metrics"))
        .and(query_param("from_date", "2026-04-16T12:00:00.000Z"))
        .and(query_param("to_date", "2026-04-16T13:00:00.000Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "metrics": [{
                    "key": "connections",
                    "name": "Open connections",
                    "description": "Connections by role",
                    "unit": "connections",
                    "series": [{
                        "label": "{role=\"primary\",zone=\"eu-west-1\"}",
                        "dataPoints": [
                            { "timestamp": 1776337200, "value": 4 },
                            { "timestamp": 1776337200 }
                        ]
                    }, {
                        "label": "custom label / tenant=a,b",
                        "dataPoints": []
                    }, {}]
                }, {
                    "key": "empty-series",
                    "series": []
                }, {
                    "key": "absent-series"
                }]
            },
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
            "--to-date",
            "2026-04-16T13:00:00Z",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "metrics:\n\
         \x20 - description: Connections by role\n\
         \x20   key: connections\n\
         \x20   name: Open connections\n\
         \x20   series:\n\
         \x20     - dataPoints:\n\
         \x20         1: 2026-04-16T11:00:00Z  4 connections\n\
         \x20         2: 2026-04-16T11:00:00Z  -\n\
         \x20       label: {role=\"primary\",zone=\"eu-west-1\"}\n\
         \x20     - dataPoints: []\n\
         \x20       label: custom label / tenant=a,b\n\
         \x20     - dataPoints: -\n\
         \x20   unit: connections\n\
         \x20 - key: empty-series\n\
         \x20   series: []\n\
         \x20 - key: absent-series\n\
         \x20   series: -\n"
    );
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2, "unexpected query parameters: {query:?}");
    assert!(
        query.iter().all(|(key, _)| key != "bucket_size_seconds"),
        "bucket size must be omitted when the flag is absent: {query:?}"
    );
}

// ── Postgres logs (issue #582) ────────────────────────────────────────────

fn invoke_postgres_logs(
    mock: &MockServer,
    home: &Path,
    json: bool,
    args: &[&str],
) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(home)
        .args(["cloud", "--url", &mock.uri()]);
    if json {
        command.arg("--json");
    }
    command.args(["postgres", "logs", "pg-1"]).args(args);
    command.output().expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn postgres_logs_routes_all_query_parameters_and_preserves_json_with_oauth() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([
        {
            "timestamp": "2026-08-01T12:00:00Z",
            "severity": "LOG",
            "body": "{\"message\":\"checkpoint complete\",\"detail\":\"all buffers written\"}"
        },
        { "severity": "WARNING" }
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-postgres-logs"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = invoke_postgres_logs(
        &mock,
        &home,
        true,
        &[
            "--from-date",
            "2026-08-01T00:00:00+01:00",
            "--to-date",
            "2026-08-02T00:00:00+01:00",
            "--body-contains",
            "checkpoint complete",
            "--severity",
            "LOG",
            "--sort-order",
            "asc",
            "--limit",
            "2000",
            "--offset",
            "0",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].body.is_empty(), "GET must not carry a body");
    let query: std::collections::BTreeMap<_, _> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        std::collections::BTreeMap::from([
            (
                "body_contains".to_string(),
                "checkpoint complete".to_string()
            ),
            (
                "from_date".to_string(),
                "2026-08-01T00:00:00+01:00".to_string()
            ),
            ("limit".to_string(), "2000".to_string()),
            ("offset".to_string(), "0".to_string()),
            ("severity".to_string(), "LOG".to_string()),
            ("sort_order".to_string(), "asc".to_string()),
            (
                "to_date".to_string(),
                "2026-08-02T00:00:00+01:00".to_string()
            ),
        ])
    );
}

#[tokio::test]
async fn postgres_logs_minimal_query_and_sparse_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {
                    "timestamp": "2026-08-01T12:00:00Z",
                    "severity": "LOG",
                    "body": "{\"message\":\"checkpoint complete\",\"detail\":\"all buffers written\"}"
                },
                { "severity": "WARNING", "body": "recovery complete" },
                { "body": "{\"message\":42,\"detail\":\"kept\"}" },
                {}
            ],
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let home = tempfile::tempdir().unwrap();
    let output = invoke_postgres_logs(
        &mock,
        home.path(),
        false,
        &[
            "--from-date",
            "2026-08-01T00:00:00Z",
            "--to-date",
            "2026-08-02T00:00:00Z",
            "--org-id",
            "org-1",
            "--api-key",
            "logs-key",
            "--api-secret",
            "logs-secret",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for heading in ["Timestamp", "Severity", "Message"] {
        assert!(stdout.contains(heading), "{heading} missing from: {stdout}");
    }
    assert!(stdout.contains("2026-08-01T12:00:00Z"), "{stdout}");
    assert!(stdout.contains("checkpoint complete"), "{stdout}");
    assert!(!stdout.contains("all buffers written"), "{stdout}");
    assert!(stdout.contains("recovery complete"), "{stdout}");
    assert!(
        stdout.contains(r#"{"message":42,"detail":"kept"}"#),
        "{stdout}"
    );
    let absent_row = stdout
        .lines()
        .rfind(|line| line.contains('|'))
        .expect("missing final table row");
    assert_eq!(absent_row.matches('-').count(), 3, "{absent_row}");
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.query(),
        Some("from_date=2026-08-01T00%3A00%3A00Z&to_date=2026-08-02T00%3A00%3A00Z")
    );
}

// ── Postgres slow query patterns (issue #585) ─────────────────────────────

#[tokio::test]
async fn postgres_slow_query_list_sends_exact_query_supports_oauth_and_preserves_json() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([{
        "queryId": "query-1",
        "queryText": "SELECT * FROM events WHERE id = $1",
        "dbName": "app db",
        "dbUser": "reader+worker",
        "dbOperation": "SELECT",
        "app": "reporting/api",
        "callCount": 42,
        "errorCount": 1,
        "totalDurationUs": 950000.25,
        "avgDurationUs": 1190.8419405320817,
        "maxDurationUs": 81000.75,
        "p50DurationUs": 18000.25,
        "p95DurationUs": 70000.5,
        "p99DurationUs": 80000.125,
        "totalRows": 420,
        "totalSharedBlksRead": 12,
        "totalSharedBlksHit": 900,
        "totalCpuTimeUs": 700000,
        "totalWalBytes": 128
    }]);
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-slow-query-list"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00+01:00",
            "--to-date",
            "2026-04-16T13:00:00+01:00",
            "--db-name",
            "app db",
            "--db-user",
            "reader+worker",
            "--db-operation",
            "SELECT & EXPLAIN",
            "--app",
            "reporting/api",
            "--sort-by",
            "total_cpu_time",
            "--sort-order",
            "asc",
            "--limit",
            "500",
            "--offset",
            "0",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        [
            (
                "from_date".to_string(),
                "2026-04-16T11:00:00.000Z".to_string()
            ),
            (
                "to_date".to_string(),
                "2026-04-16T12:00:00.000Z".to_string()
            ),
            ("db_name".to_string(), "app db".to_string()),
            ("db_user".to_string(), "reader+worker".to_string()),
            ("db_operation".to_string(), "SELECT & EXPLAIN".to_string()),
            ("app".to_string(), "reporting/api".to_string()),
            ("sort_by".to_string(), "total_cpu_time".to_string()),
            ("sort_order".to_string(), "asc".to_string()),
            ("limit".to_string(), "500".to_string()),
            ("offset".to_string(), "0".to_string()),
        ]
    );
}

#[tokio::test]
async fn postgres_slow_query_list_omits_filters_and_renders_sparse_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {
                    "queryId": "query-1",
                    "queryText": "SELECT * FROM events WHERE id = $1",
                    "callCount": 2,
                    "avgDurationUs": 1190.8419405320817,
                    "totalDurationUs": 2381.6838810641634,
                    "p95DurationUs": 2000.25
                },
                {
                    "queryId": "query-2",
                    "totalDurationUs": 0.125
                }
            ],
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
            "--to-date",
            "2026-04-16T13:00:00Z",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for heading in [
        "Query ID",
        "Query",
        "Calls",
        "Avg duration (µs)",
        "Total duration (µs)",
    ] {
        assert!(
            stdout.contains(heading),
            "missing {heading:?} from:\n{stdout}"
        );
    }
    for value in [
        "query-1",
        "SELECT * FROM events WHERE id = $1",
        "1190.8419405320817",
        "2381.6838810641634",
        "query-2",
        "0.125",
    ] {
        assert!(stdout.contains(value), "missing {value:?} from:\n{stdout}");
    }
    let cells = |id: &str| -> Vec<String> {
        stdout
            .lines()
            .find(|line| line.contains(id))
            .unwrap_or_else(|| panic!("missing {id} row:\n{stdout}"))
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        cells("query-1"),
        [
            "query-1",
            "SELECT * FROM events WHERE id = $1",
            "2",
            "1190.8419405320817",
            "2381.6838810641634"
        ]
    );
    let second_row = stdout
        .lines()
        .find(|line| line.contains("query-2"))
        .expect("second slow-query row");
    assert_eq!(second_row.matches(" - ").count(), 3, "{stdout}");
    assert!(!stdout.contains("p95DurationUs"), "{stdout}");
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2, "unexpected query parameters: {query:?}");
}

#[tokio::test]
async fn postgres_slow_query_get_has_distinct_query_and_preserves_recent_executions() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "aggregate": {
            "queryId": "query-1",
            "queryText": "SELECT $1",
            "dbName": "app db",
            "dbUser": "reader+worker",
            "dbOperation": "SELECT & EXPLAIN",
            "app": "reporting/api",
            "callCount": 2,
            "avgDurationUs": 1190.8419405320817,
            "p50DurationUs": 1100.25,
            "p95DurationUs": 1800.5,
            "p99DurationUs": 1950.125
        },
        "recentExecutions": [{
            "queryId": "query-1",
            "queryText": "SELECT 42",
            "timestamp": "2026-04-16T12:30:00Z",
            "durationUs": 1234.56789,
            "rows": 1,
            "errMessage": "optional detail"
        }]
    });
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns/query-1",
        ))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-slow-query-detail"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app db",
            "--db-user",
            "reader+worker",
            "--db-operation",
            "SELECT & EXPLAIN",
            "--app",
            "reporting/api",
            "--timestamp",
            "2026-04-16T12:30:00+01:00",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        [
            ("db_name".to_string(), "app db".to_string()),
            ("db_user".to_string(), "reader+worker".to_string()),
            ("db_operation".to_string(), "SELECT & EXPLAIN".to_string()),
            ("app".to_string(), "reporting/api".to_string()),
            (
                "timestamp".to_string(),
                "2026-04-16T12:30:00+01:00".to_string()
            ),
        ]
    );
}

#[tokio::test]
async fn postgres_slow_query_get_omits_optional_query_and_renders_sparse_detail() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns/query-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "aggregate": { "queryId": "query-1", "avgDurationUs": 1190.8419405320817, "p99DurationUs": 2000.25 },
                "recentExecutions": [{ "timestamp": "2026-04-16T12:30:00Z", "durationUs": 1234.56789, "rows": 1 }]
            },
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app",
            "--db-user",
            "reader",
            "--db-operation",
            "SELECT",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("avgDurationUs: 1190.8419405320817"),
        "{stdout}"
    );
    assert!(stdout.contains("p99DurationUs: 2000.25"), "{stdout}");
    assert!(stdout.contains("durationUs: 1234.56789"), "{stdout}");
    assert!(stdout.contains("rows: 1"), "{stdout}");
    for text in [
        "aggregate:",
        "queryId: query-1",
        "recentExecutions:",
        "timestamp: 2026-04-16T12:30:00Z",
    ] {
        assert!(stdout.contains(text), "missing {text:?} from:\n{stdout}");
    }
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 3, "unexpected query parameters: {query:?}");
    assert!(
        query
            .iter()
            .all(|(key, _)| key != "app" && key != "timestamp"),
        "optional detail filters must be omitted: {query:?}"
    );
}

// ── Postgres Prometheus metrics (issue #584) ──────────────────────────────

#[tokio::test]
async fn postgres_prometheus_service_uses_oauth_and_returns_a_json_string() {
    let mock = MockServer::start().await;
    let metrics = "# HELP pg_up Whether Postgres is available.\n# TYPE pg_up gauge\npg_up 1\n";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/prometheus"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain; charset=UTF-8")
                .set_body_string(metrics),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "prometheus",
            "service",
            "pg-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<String>(&output.stdout).unwrap(),
        metrics
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn postgres_prometheus_org_preserves_raw_text_and_agent_mode_is_json() {
    let mock = MockServer::start().await;
    let metrics = "# TYPE pg_connections gauge\npg_connections{service=\"pg-1\"} 4\n";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain; charset=UTF-8")
                .set_body_string(metrics),
        )
        .expect(2)
        .mount(&mock)
        .await;

    let args = ["postgres", "prometheus", "org", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    assert_eq!(human.stdout, metrics.as_bytes());

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let agent = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("AI_AGENT", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args(["cloud", "--url", &mock.uri()])
        .args(args)
        .output()
        .unwrap();
    assert_success(&agent);
    assert_eq!(
        serde_json::from_slice::<String>(&agent.stdout).unwrap(),
        metrics
    );

    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), None);
    }
}

// ── Postgres deletion JSON output (issue #614) ────────────────────────────

/// `postgres delete --json` must emit the Postgres resource object, not the
/// raw `{"status":...,"requestId":...}` API envelope the delete endpoint
/// itself returns: the handler fetches the resource before issuing the
/// delete and renders that instead, consistent with every other
/// `cloud postgres` subcommand.
#[tokio::test]
async fn postgres_delete_json_emits_the_resource_object_not_the_envelope() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": postgres_id,
                "name": "my-postgres",
                "state": "running",
            },
            "status": 200,
            "requestId": "stub-postgres-get",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}"
        )))
        .respond_with(successful_delete_response("stub-postgres-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["postgres", "delete", postgres_id, "--org-id", "org-1"],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource object as JSON");
    assert_eq!(stdout["id"], postgres_id);
    assert_eq!(stdout["name"], "my-postgres");
    assert_eq!(stdout["state"], "running");
    assert!(
        stdout.get("status").is_none() && stdout.get("requestId").is_none(),
        "must not emit the raw API envelope, got: {stdout}"
    );
}

// ── Postgres update --name (issue #663) ────────────────────────────────────

/// Non-tag updates must echo the current tag snapshot alongside the changed
/// fields. Key-only and empty-value tags remain distinct under the API model's
/// optional-value contract; an empty snapshot is the merge unit test.
#[tokio::test]
async fn postgres_update_preserves_tags_when_changing_other_fields() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    let postgres_path = format!("/v1/organizations/org-1/postgres/{postgres_id}");
    let tags = serde_json::json!([
        {"key": "env", "value": "prod"},
        {"key": "owner", "value": "me"},
        {"key": "key-only"},
        {"key": "empty", "value": ""}
    ]);
    Mock::given(method("GET"))
        .and(path(&postgres_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": postgres_id, "tags": tags},
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(&postgres_path))
        .and(body_json(serde_json::json!({
            "name": "renamed-pg", "size": "c6gd.large", "haType": "async",
            "deletionProtection": false, "tags": tags
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": postgres_id, "tags": tags},
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            postgres_id,
            "--org-id",
            "org-1",
            "--new-name",
            "renamed-pg",
            "--size",
            "c6gd.large",
            "--ha-type",
            "async",
            "--deletion-protection",
            "false",
        ],
    );
    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("GET".into(), postgres_path.clone()),
            ("PATCH".into(), postgres_path)
        ]
    );
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["tags"], tags);
    // `haType` flips on acceptance, so an HA change gets no hint.
    assert_eq!(output.get("next"), None);
}

#[tokio::test]
async fn postgres_update_without_tag_flags_refuses_incomplete_or_failed_reads() {
    // Which snapshots are incomplete is `merge_response_tags_refuses_*`; here
    // one incomplete and one failed read each stop before the PATCH.
    for response in [
        ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {}})),
        ResponseTemplate::new(500).set_body_json(serde_json::json!({"error": "read failed"})),
    ] {
        let mock = MockServer::start().await;
        let postgres_id = "11111111-2222-3333-4444-555555555555";
        let postgres_path = format!("/v1/organizations/org-1/postgres/{postgres_id}");
        Mock::given(method("GET"))
            .and(path(&postgres_path))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "postgres",
                "update",
                postgres_id,
                "--org-id",
                "org-1",
                "--new-name",
                "renamed",
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            received_request_shape(&mock).await,
            vec![("GET".into(), postgres_path)]
        );
    }
}

// ── Postgres IP access rules (issue #1125) ────────────────────────────────

const POSTGRES_ID: &str = "11111111-2222-3333-4444-555555555555";

fn postgres_access_rules_response() -> Value {
    serde_json::json!([
        {"id": "rule-1", "source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
        {"id": "rule-2", "source": "198.51.100.7/32", "portRange": "5432..6432"}
    ])
}

/// `--block-public-access` sends `[]`, and each `--ip-allow` becomes one rule
/// without an `id`. A trailing `=` sends an empty description, as ClickHouse
/// `--ip-allow` does. Omitting both is in `postgres_config_files_reach_every_write_path`.
#[tokio::test]
async fn postgres_create_sends_access_rules_only_when_asked() {
    let base = serde_json::json!({
        "name": "pg1", "provider": "aws", "region": "us-east-1", "size": "c6gd.large"
    });
    let with_list = |list: Value| {
        let mut body = base.clone();
        body["ipAccessList"] = list;
        body
    };
    for (flags, expected) in [
        (
            vec!["--block-public-access"],
            with_list(serde_json::json!([])),
        ),
        (
            vec![
                "--ip-allow",
                "203.0.113.0/24:5432=office",
                "--ip-allow",
                "198.51.100.7:5432..6432",
                "--ip-allow",
                "192.0.2.1:6432=",
            ],
            with_list(serde_json::json!([
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "198.51.100.7", "portRange": "5432..6432"},
                {"source": "192.0.2.1", "portRange": "6432", "description": ""}
            ])),
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/organizations/org-1/postgres"))
            .and(body_json(expected.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": POSTGRES_ID,
                    "ipAccessList": postgres_access_rules_response(),
                },
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec![
            "postgres",
            "create",
            "--org-id",
            "org-1",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "c6gd.large",
        ];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output["ipAccessList"], postgres_access_rules_response());
        assert_eq!(
            output["next"]["command"],
            format!("clickhousectl cloud postgres get {POSTGRES_ID} --org-id org-1 --json")
        );
    }
}

/// Rule changes ride the same PATCH as the preserved tag snapshot. A
/// remove-only update sending `add: []` is the builder's unit test.
#[tokio::test]
async fn postgres_update_sends_access_rule_patch() {
    let postgres_path = format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}");
    let tags = serde_json::json!([{"key": "env", "value": "prod"}]);
    for (flags, patch) in [(
        vec![
            "--add-ip-allow",
            "203.0.113.0/24:5432=office",
            "--remove-ip-allow",
            "rule-9",
            "--add-ip-allow",
            "203.0.113.0/24:6432",
            "--remove-ip-allow",
            "rule-8",
        ],
        serde_json::json!({
            "add": [
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "203.0.113.0/24", "portRange": "6432"}
            ],
            "remove": [{"id": "rule-9"}, {"id": "rule-8"}]
        }),
    )] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&postgres_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"id": POSTGRES_ID, "tags": tags},
            })))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PATCH"))
            .and(path(&postgres_path))
            .and(body_json(
                serde_json::json!({"tags": tags, "ipAccessList": patch}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": POSTGRES_ID,
                    "tags": tags,
                    "ipAccessList": postgres_access_rules_response(),
                },
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec!["postgres", "update", POSTGRES_ID, "--org-id", "org-1"];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output["ipAccessList"], postgres_access_rules_response());
    }
}

#[tokio::test]
async fn postgres_get_human_output_shows_access_rules_and_restore_point() {
    let mock = MockServer::start().await;
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}"),
        serde_json::json!({
            "id": POSTGRES_ID,
            "earliestRestoreTime": "2026-10-01T12:00:00Z",
            "ipAccessList": postgres_access_rules_response(),
        }),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["postgres", "get", POSTGRES_ID, "--org-id", "org-1"],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2026-10-01T12:00:00"), "{stdout}");
    for rule in postgres_access_rules_response().as_array().unwrap() {
        let line = stdout
            .lines()
            .find(|line| line.contains(rule["id"].as_str().unwrap()))
            .unwrap_or_else(|| panic!("missing {rule}: {stdout}"));
        for field in ["source", "portRange", "description"] {
            if let Some(value) = rule[field].as_str() {
                assert!(line.contains(value), "{field}: {line}");
            }
        }
    }
}

// ── Postgres restore access rules ─────────────────────────────────────────

const RESTORE_SOURCE_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

/// The allow-all rules the restore endpoint creates on every restored service.
fn postgres_restore_default_rules() -> Value {
    serde_json::json!([
        {"id": "default-5432", "source": "0.0.0.0/0", "portRange": "5432"},
        {"id": "default-6432", "source": "0.0.0.0/0", "portRange": "6432"}
    ])
}

fn postgres_restore_args(extra: &[&'static str]) -> Vec<&'static str> {
    let mut args = vec![
        "postgres",
        "restore",
        RESTORE_SOURCE_ID,
        "--name",
        "restored",
        "--restore-target",
        "2026-09-01T12:00:00Z",
        "--org-id",
        "org-1",
    ];
    args.extend(extra);
    args
}

async fn mount_postgres_restore(mock: &MockServer, rules: Value) {
    Mock::given(method("POST"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}/restoredService"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": POSTGRES_ID,
                "state": "creating",
                "tags": [{"key": "env", "value": "prod"}],
                "ipAccessList": rules,
            },
        })))
        .expect(1)
        .mount(mock)
        .await;
}

async fn mount_postgres_source(mock: &MockServer, service: Value, expected: u64) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
        })))
        .expect(expected)
        .mount(mock)
        .await;
}

/// Mount the PATCH that replaces the restored service's rules. It must carry
/// the restored tags, because the API clears tags a PATCH omits.
async fn mount_postgres_restored_patch(mock: &MockServer, patch: Value, applied: Value) {
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{POSTGRES_ID}"
        )))
        .and(body_json(serde_json::json!({
            "tags": [{"key": "env", "value": "prod"}],
            "ipAccessList": patch,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": POSTGRES_ID, "ipAccessList": applied},
        })))
        .expect(1)
        .mount(mock)
        .await;
}

/// UAT F1: a restored copy of a restricted source must not keep the restore
/// endpoint's allow-all rules. The source's rules are read before restoring,
/// then replace the defaults.
#[tokio::test]
async fn postgres_restore_copies_the_source_access_rules() {
    let mock = MockServer::start().await;
    mount_postgres_source(
        &mock,
        serde_json::json!({"id": RESTORE_SOURCE_ID, "ipAccessList": postgres_access_rules_response()}),
        1,
    )
    .await;
    mount_postgres_restore(&mock, postgres_restore_default_rules()).await;
    let applied = serde_json::json!([
        {"id": "rule-11", "source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
        {"id": "rule-12", "source": "198.51.100.7/32", "portRange": "5432..6432"}
    ]);
    mount_postgres_restored_patch(
        &mock,
        serde_json::json!({
            "add": [
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "198.51.100.7/32", "portRange": "5432..6432"}
            ],
            "remove": [{"id": "default-5432"}, {"id": "default-6432"}]
        }),
        applied.clone(),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&[]));
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["ipAccessList"], applied);
    // The restored service, not the source, is the one to poll.
    assert_eq!(
        output["next"]["command"],
        format!("clickhousectl cloud postgres get {POSTGRES_ID} --org-id org-1 --json")
    );
    let source = format!("/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}");
    let restored = format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}");
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("GET".into(), source.clone()),
            ("POST".into(), format!("{source}/restoredService")),
            ("PATCH".into(), restored),
        ]
    );
}

/// `--ip-allow` and `--block-public-access` replace the defaults without
/// reading the source.
#[tokio::test]
async fn postgres_restore_access_flags_replace_the_default_rules() {
    for (flags, add) in [
        (
            vec!["--ip-allow", "203.0.113.0/24:5432=office"],
            serde_json::json!([
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"}
            ]),
        ),
        (vec!["--block-public-access"], serde_json::json!([])),
    ] {
        let mock = MockServer::start().await;
        mount_postgres_source(&mock, serde_json::json!({}), 0).await;
        mount_postgres_restore(&mock, postgres_restore_default_rules()).await;
        mount_postgres_restored_patch(
            &mock,
            serde_json::json!({
                "add": add,
                "remove": [{"id": "default-5432"}, {"id": "default-6432"}]
            }),
            serde_json::json!([]),
        )
        .await;

        let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&flags));
        assert_success(&output);
    }
}

/// A source whose rules already match the restore's defaults needs no PATCH.
#[tokio::test]
async fn postgres_restore_skips_the_patch_when_rules_already_match() {
    let mock = MockServer::start().await;
    mount_postgres_source(
        &mock,
        serde_json::json!({"ipAccessList": [
            {"id": "source-1", "source": "0.0.0.0/0", "portRange": "5432", "description": ""},
            {"id": "source-2", "source": "0.0.0.0/0", "portRange": "6432"}
        ]}),
        1,
    )
    .await;
    mount_postgres_restore(&mock, postgres_restore_default_rules()).await;

    let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&[]));
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["ipAccessList"], postgres_restore_default_rules());
    assert_eq!(received_request_shape(&mock).await.len(), 2);
}

/// Copying an incomplete rule list would silently drop restrictions, so the
/// restore is refused before anything is created.
#[tokio::test]
async fn postgres_restore_refuses_a_source_without_complete_access_rules() {
    for source in [
        serde_json::json!({"id": RESTORE_SOURCE_ID}),
        serde_json::json!({"ipAccessList": [{"id": "rule-1", "portRange": "5432"}]}),
    ] {
        let mock = MockServer::start().await;
        mount_postgres_source(&mock, source, 1).await;

        let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&[]));
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--block-public-access"), "{stderr}");
        assert_eq!(received_request_shape(&mock).await.len(), 1);
    }
}

/// When the rules cannot be applied, the restored service still exists with
/// allow-all rules, so the error names it and how to fix it.
#[tokio::test]
async fn postgres_restore_rule_failure_names_the_restored_service() {
    let mock = MockServer::start().await;
    mount_postgres_restore(&mock, postgres_restore_default_rules()).await;
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{POSTGRES_ID}"
        )))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "boom",
        })))
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &postgres_restore_args(&["--block-public-access"]),
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(POSTGRES_ID), "{stderr}");
    assert!(stderr.contains("--remove-ip-allow"), "{stderr}");
}

/// A sparse restore response omits the rule IDs, so they are read back from
/// the restored service before the PATCH.
#[tokio::test]
async fn postgres_restore_reads_rules_omitted_from_a_sparse_response() {
    let mock = MockServer::start().await;
    mount_postgres_service_response(
        &mock,
        "POST",
        &format!("/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}/restoredService"),
        serde_json::json!({"id": POSTGRES_ID}),
    )
    .await;
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}"),
        serde_json::json!({
            "id": POSTGRES_ID,
            "tags": [{"key": "env", "value": "prod"}],
            "ipAccessList": postgres_restore_default_rules(),
        }),
    )
    .await;
    mount_postgres_restored_patch(
        &mock,
        serde_json::json!({
            "add": [],
            "remove": [{"id": "default-5432"}, {"id": "default-6432"}]
        }),
        serde_json::json!([]),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &postgres_restore_args(&["--block-public-access"]),
    );
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["ipAccessList"], serde_json::json!([]));
}

// ── Postgres list --filter validation (issue #603) ────────────────────────

fn postgres_list_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": [
            {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "primary-pg",
                "state": "running",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": true,
                "deletionProtection": true,
            },
            {
                "id": "66666666-7777-8888-9999-000000000000",
                "name": "replica-pg",
                "state": "restoring_backup",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": false,
                "deletionProtection": false,
            },
            {
                // Absent `isPrimary`/`state`: must match no filter value.
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "unknown-pg",
                "region": "us-east-1",
            },
        ],
        "status": 200,
        "requestId": "stub-postgres-list",
    }))
}

/// `isPrimary` is a supported key (it is the `Primary` column), `state` matches
/// the serde wire value including multi-word states, and an item whose field the
/// API omitted matches nothing.
#[tokio::test]
async fn postgres_list_applies_supported_filters_client_side() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(postgres_list_response())
        .mount(&mock)
        .await;

    let names = |filter: &str| -> Vec<String> {
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["postgres", "list", "--org-id", "org-1", "--filter", filter],
        );
        assert_success(&output);
        let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
            .expect("stdout should be a JSON array");
        stdout
            .as_array()
            .expect("array")
            .iter()
            .map(|item| item["name"].as_str().unwrap_or_default().to_string())
            .collect()
    };

    // Each key's matching rules are the `apply_filter_*` unit tests.
    assert_eq!(names("isPrimary=false"), vec!["replica-pg".to_string()]);
    assert_eq!(
        names("state=restoring_backup"),
        vec!["replica-pg".to_string()]
    );
}

#[tokio::test]
async fn postgres_list_shows_provider_in_human_output_and_preserves_json() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(postgres_list_response())
        .expect(2)
        .mount(&mock)
        .await;

    let human =
        invoke_cli_with_cloud_credentials_human(&mock, &["postgres", "list", "--org-id", "org-1"]);
    assert_success(&human);
    let human = String::from_utf8(human.stdout).expect("human output should be UTF-8");
    let row_cells = |name: &str| -> Vec<&str> {
        human
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("missing row for {name}:\n{human}"))
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect()
    };
    assert_eq!(
        row_cells("Name"),
        [
            "Name",
            "ID",
            "State",
            "Provider",
            "Region",
            "Size",
            "PG",
            "HA",
            "Primary",
            "Deletion protection"
        ]
    );
    assert_eq!(row_cells("primary-pg")[3], "aws");
    assert_eq!(row_cells("unknown-pg")[3], "-");
    assert_eq!(row_cells("primary-pg")[9], "yes");
    assert_eq!(row_cells("replica-pg")[9], "no");
    assert_eq!(row_cells("unknown-pg")[9], "-");

    let json = invoke_cli_with_cloud_credentials(&mock, &["postgres", "list", "--org-id", "org-1"]);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).expect("JSON output should be valid"),
        serde_json::json!([
            {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "primary-pg",
                "state": "running",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": true,
                "deletionProtection": true,
            },
            {
                "id": "66666666-7777-8888-9999-000000000000",
                "name": "replica-pg",
                "state": "restoring_backup",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": false,
                "deletionProtection": false,
            },
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "unknown-pg",
                "region": "us-east-1",
            },
        ])
    );
}

// ── Postgres promote / switchover role changes (issue #604) ───────────────

async fn mount_postgres_service_response(
    mock: &MockServer,
    method_name: &str,
    endpoint: &str,
    service: Value,
) {
    Mock::given(method(method_name))
        .and(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
            "status": 200,
            "requestId": "stub-postgres-connection",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn issue_836_postgres_get_human_guidance_handles_sparse_endpoint_fields() {
    let mock = MockServer::start().await;
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}"),
        serde_json::json!({"id": POSTGRES_ID}),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["postgres", "get", POSTGRES_ID, "--org-id", "org-1"],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("host=<host-from-get>"), "{stdout}");
    assert!(stdout.contains("user=<username-from-get>"), "{stdout}");
    assert!(stdout.contains(POSTGRES_ID), "{stdout}");
    assert!(stdout.contains("certs get"), "{stdout}");
    assert!(!stdout.contains("password="), "{stdout}");
}

#[tokio::test]
async fn issue_836_postgres_get_json_keeps_the_api_shape_without_connection_defaults() {
    let mock = MockServer::start().await;
    let service = serde_json::json!({
        "id": POSTGRES_ID,
        "hostname": "example.pg.clickhouse.cloud",
        "username": "postgres"
    });
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}"),
        service.clone(),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["postgres", "get", POSTGRES_ID, "--org-id", "org-1"],
    );
    assert_success(&output);
    let stdout: Value = serde_json::from_slice(&output.stdout).expect("valid JSON response");
    assert_eq!(stdout, service);
}

#[tokio::test]
async fn issue_836_replica_and_restore_human_outputs_point_sparse_results_to_get() {
    for (endpoint, mut args) in [
        (
            "readReplica",
            vec![
                "postgres",
                "read-replica",
                "create",
                "source-id",
                "--name",
                "replica",
            ],
        ),
        (
            "restoredService",
            vec![
                "postgres",
                "restore",
                "source-id",
                "--name",
                "restored",
                "--restore-target",
                "2026-09-01T12:00:00Z",
                "--block-public-access",
            ],
        ),
    ] {
        let mock = MockServer::start().await;
        mount_postgres_service_response(
            &mock,
            "POST",
            &format!("/v1/organizations/org-1/postgres/source-id/{endpoint}"),
            serde_json::json!({"id": POSTGRES_ID}),
        )
        .await;
        mount_sparse_restored_get(&mock, endpoint).await;
        args.extend(["--org-id", "org-1"]);

        let output = invoke_cli_with_cloud_credentials_human(&mock, &args);
        assert_success(&output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("host=<host-from-get>"), "{stdout}");
        assert!(stdout.contains("user=<username-from-get>"), "{stdout}");
        assert!(
            stdout.contains(&format!("postgres get {POSTGRES_ID}")),
            "{stdout}"
        );
        assert!(!stdout.contains("reset-password"), "{stdout}");

        // Read replica `--json` prints the returned service and polls the new
        // replica (restore's JSON output is asserted by the
        // `postgres_restore_*` tests).
        if endpoint == "readReplica" {
            let service = serde_json::json!({"id": POSTGRES_ID});
            let json_mock = MockServer::start().await;
            mount_postgres_service_response(
                &json_mock,
                "POST",
                &format!("/v1/organizations/org-1/postgres/source-id/{endpoint}"),
                service.clone(),
            )
            .await;
            let output = invoke_cli_with_cloud_credentials(&json_mock, &args);
            assert_success(&output);
            let (response, next) = take_next(&output.stdout);
            assert_eq!(response, service);
            let next = next.expect("a new replica names its poll");
            assert_eq!(
                next["command"],
                format!("clickhousectl cloud postgres get {POSTGRES_ID} --org-id org-1 --json")
            );
            assert_eq!(next["field"], "state");
            assert_eq!(next["until"], serde_json::json!(["running"]));
        }
    }
}

/// A sparse restore response sends the CLI to `get` for the rules it
/// replaces; these already match `--block-public-access`, so nothing is patched.
async fn mount_sparse_restored_get(mock: &MockServer, endpoint: &str) {
    if endpoint == "restoredService" {
        mount_postgres_service_response(
            mock,
            "GET",
            &format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}"),
            serde_json::json!({"id": POSTGRES_ID, "tags": [], "ipAccessList": []}),
        )
        .await;
    }
}

fn postgres_role_service(ha_type: &str) -> serde_json::Value {
    serde_json::json!({
        "id": POSTGRES_ID,
        "name": "my-postgres",
        "state": "running",
        "haType": ha_type,
    })
}

async fn mount_postgres_state(mock: &MockServer, command: &str, service: serde_json::Value) {
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{POSTGRES_ID}/state"
        )))
        .and(header("authorization", BASIC_TEST_AUTH))
        .and(body_json(serde_json::json!({"command": command})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
            "status": 200,
            "requestId": "stub-postgres-state",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

/// Each role command sends one state change and returns its acknowledgment.
/// The exact request list proves the CLI performs no preflight or polling GET.
#[tokio::test]
async fn postgres_role_changes_issue_only_the_expected_state_patch() {
    for (command, ha_type) in [("promote", "async"), ("switchover", "sync")] {
        let mock = MockServer::start().await;
        let service = postgres_role_service(ha_type);
        mount_postgres_state(&mock, command, service.clone()).await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["postgres", command, POSTGRES_ID, "--org-id", "org-1"],
        );

        assert_success(&output);
        assert_eq!(
            received_request_shape(&mock).await,
            vec![(
                "PATCH".to_string(),
                format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}/state")
            )],
            "{command} must issue only the state change"
        );
        let (stdout, next) = take_next(&output.stdout);
        assert_eq!(
            stdout, service,
            "{command} must preserve the response shape"
        );
        // Nothing `postgres get` returns confirms a switchover.
        match command {
            "promote" => {
                let next = next.expect("promote names its poll");
                assert_eq!(next["field"], "isPrimary");
                assert_eq!(next["until"], serde_json::json!([true]));
            }
            _ => assert_eq!(next, None, "{command}"),
        }
    }
}

/// Start a human-mode (no `--json`, no agent env) switchover so the test can
/// close its stdout first (see the #598 tests).
fn spawn_postgres_switchover_human(mock: &MockServer, project_dir: &Path) -> std::process::Child {
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project_dir)
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "postgres",
            "switchover",
            POSTGRES_ID,
            "--org-id",
            "org-1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl")
}

/// A closed stdout must not turn an accepted switchover into a panic and exit
/// 101 (#598).
#[tokio::test]
async fn postgres_switchover_human_output_survives_a_closed_stdout() {
    let mock = MockServer::start().await;
    mount_postgres_state(&mock, "switchover", postgres_role_service("sync")).await;

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("home")).unwrap();
    let mut child = spawn_postgres_switchover_human(&mock, dir.path());
    drop(child.stdout.take().expect("stdout was piped"));
    let status = child.wait().expect("failed to wait for clickhousectl");

    assert_eq!(
        status.code(),
        Some(0),
        "a closed stdout must not turn an accepted switchover into a panic"
    );
    let shape = received_request_shape(&mock).await;
    assert!(
        shape.contains(&(
            "PATCH".to_string(),
            format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}/state")
        )),
        "the switchover must still reach the API: {shape:?}"
    );
}

/// `certs get`, `reset-password` and `restart` each reach their own route:
/// the certificate is written to `--output` with owner-only permissions, the
/// password the caller chose is the PATCH body, and restart is a state PATCH.
#[tokio::test]
async fn postgres_certs_reset_password_and_restart_reach_their_routes() {
    use std::os::unix::fs::PermissionsExt;
    let mock = MockServer::start().await;
    let service = format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}");
    let pem = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";
    Mock::given(method("GET"))
        .and(path(format!("{service}/caCertificates")))
        .respond_with(ResponseTemplate::new(200).set_body_string(pem))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(format!("{service}/password")))
        .and(body_json(
            serde_json::json!({"password": "Chosen-Pass-123"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {}, "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    mount_postgres_state(&mock, "restart", postgres_role_service("sync")).await;

    let directory = tempfile::tempdir().unwrap();
    let ca_path = directory.path().join("ca.pem");
    let ca = ca_path.to_str().unwrap();
    let id = POSTGRES_ID;
    let certs = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres", "certs", "get", id, "--output", ca, "--org-id", "org-1",
        ],
    );
    assert_success(&certs);
    assert_eq!(std::fs::read_to_string(&ca_path).unwrap(), pem);
    assert_eq!(
        std::fs::metadata(&ca_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&certs.stdout).unwrap(),
        serde_json::json!({"path": ca})
    );

    let reset = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "reset-password",
            id,
            "--password",
            "Chosen-Pass-123",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&reset);
    // The response omitted the password, so the one sent is reported.
    assert_eq!(
        serde_json::from_slice::<Value>(&reset.stdout).unwrap(),
        serde_json::json!({"password": "Chosen-Pass-123"})
    );

    let restart =
        invoke_cli_with_cloud_credentials(&mock, &["postgres", "restart", id, "--org-id", "org-1"]);
    assert_success(&restart);
    assert_eq!(
        serde_json::from_slice::<Value>(&restart.stdout).unwrap(),
        postgres_role_service("sync")
    );
}

#[tokio::test]
async fn postgres_unknown_authority_error_preserves_api_detail_and_adds_ca_hint() {
    let mock = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let ca_path = directory.path().join("private-ca.pem");
    let ca_path_display = ca_path.to_string_lossy().into_owned();
    let ca_contents = "certificate-body-must-not-appear";
    std::fs::write(&ca_path, ca_contents).unwrap();
    let password = "postgres-password-must-not-appear";
    let api_error = "BAD_REQUEST: failed to establish connection: tls: failed to verify \
                     certificate: x509: certificate signed by unknown authority";
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": api_error,
        })))
        .mount(&mock)
        .await;

    let mut args = postgres_args_minimal();
    let password_index = args.iter().position(|arg| arg == "--password").unwrap();
    args[password_index + 1] = password.into();
    args.extend(["--ca-certificate".into(), ca_path_display.clone()]);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(api_error), "{stderr}");
    assert!(stderr.contains("--ca-certificate <PATH>"), "{stderr}");
    for secret in [password, ca_contents, ca_path_display.as_str()] {
        assert!(
            !stderr.contains(secret),
            "secret leaked into error: {stderr}"
        );
    }
}

// ── A well-formed but unknown id reads as not found (issue #666) ───────────
//
// The API answers HTTP 400 `Invalid <thing> id string:"<id>"` for a
// syntactically valid UUID that resolves to nothing, so a by-id read reported
// a bad request instead of a missing resource. The refinement is structural
// (the status, plus whether the identifiers the CLI put in the path parse as
// UUIDs), so a malformed id must still get the server's own answer.

#[tokio::test]
async fn postgres_get_unknown_well_formed_id_reads_as_not_found() {
    let mock = MockServer::start().await;
    mount_invalid_id_400(
        &mock,
        format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{UNKNOWN_UUID}"),
        "Postgres service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_human(
        &mock,
        &["postgres", "get", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID],
    );

    assert_postgres_not_found_human(
        &output,
        &format!("BAD_REQUEST: Invalid Postgres service id string:\"{UNKNOWN_UUID}\""),
    );
}

// ── Postgres by-id commands report a missing service like `get` (#1156) ──

fn assert_postgres_not_found_json(output: &std::process::Output, server_message: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = cloud_runtime_error(output);
    assert_eq!(error["code"], "resource_not_found");
    assert_eq!(
        error["command"],
        format!("clickhousectl cloud postgres list --org-id {LOOKUP_ORG_ID}")
    );
    assert_postgres_not_found_message(error["message"].as_str().unwrap(), server_message);
}

/// `postgres update` reads the service's tags before it patches. A missing
/// service must stop there with the same answer `postgres get` gives, and
/// must not send the PATCH.
#[tokio::test]
async fn postgres_update_of_a_missing_service_reads_as_not_found() {
    let server_message =
        format!("BAD_REQUEST: Invalid Postgres service id string:\"{UNKNOWN_UUID}\"");
    let args = [
        "postgres",
        "update",
        UNKNOWN_UUID,
        "--org-id",
        LOOKUP_ORG_ID,
        "--new-name",
        "renamed",
    ];
    let mock = MockServer::start().await;
    mount_invalid_id_400(
        &mock,
        unknown_postgres_path(""),
        "Postgres service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_postgres_not_found_json(&output, &server_message);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), unknown_postgres_path(""))]
    );
}

/// `--clear-tags` skips the tag read, so the PATCH is what fails. Its body is
/// user input, so the CLI only calls the service missing after a GET of the
/// service also rejects the id.
#[tokio::test]
async fn postgres_update_clear_tags_of_a_missing_service_reads_as_not_found() {
    let mock = MockServer::start().await;
    let server_message =
        format!("BAD_REQUEST: Invalid Postgres service id string:\"{UNKNOWN_UUID}\"");
    Mock::given(method("PATCH"))
        .and(path(unknown_postgres_path("")))
        .respond_with(invalid_id_400_response("Postgres service", UNKNOWN_UUID))
        .expect(1)
        .mount(&mock)
        .await;
    mount_invalid_id_400(
        &mock,
        unknown_postgres_path(""),
        "Postgres service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            UNKNOWN_UUID,
            "--org-id",
            LOOKUP_ORG_ID,
            "--clear-tags",
        ],
    );

    assert_postgres_not_found_json(&output, &server_message);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("PATCH".to_string(), unknown_postgres_path("")),
            ("GET".to_string(), unknown_postgres_path("")),
        ]
    );
}

fn postgres_config_patch_args(setting: &'static str) -> [&'static str; 8] {
    [
        "postgres",
        "config",
        "patch",
        UNKNOWN_UUID,
        "--org-id",
        LOOKUP_ORG_ID,
        "--set",
        setting,
    ]
}

/// A config PATCH for a deleted service answers 404; the confirming GET of
/// the service agrees, so the command reports the service as missing. The
/// JSON form of the answer is in the update and clear-tags tests.
#[tokio::test]
async fn postgres_config_patch_of_a_missing_service_reads_as_not_found() {
    let server_message = "NOT_FOUND: Postgres service not found";
    let args = postgres_config_patch_args("max_connections=500");
    let mock = MockServer::start().await;
    for (http_method, route) in [
        ("PATCH", unknown_postgres_path("/config")),
        ("GET", unknown_postgres_path("")),
    ] {
        Mock::given(method(http_method))
            .and(path(route))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "status": 404,
                "error": server_message,
            })))
            .expect(1)
            .mount(&mock)
            .await;
    }

    let output = invoke_cli_human(&mock, &args);
    assert_postgres_not_found_human(&output, server_message);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("PATCH".to_string(), unknown_postgres_path("/config")),
            ("GET".to_string(), unknown_postgres_path("")),
        ]
    );
}

/// A config PATCH rejected for its body answers 400 too. The confirming GET
/// returns a service that decodes, so the service exists and the server's
/// validation message must reach the user unchanged rather than being
/// misreported as a missing service.
#[tokio::test]
async fn postgres_config_patch_validation_400_keeps_the_servers_message() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(unknown_postgres_path("/config")))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "BAD_REQUEST: max_connections must be at least 10",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    // The success envelope the `postgres get` tests decode.
    mount_postgres_service_response(
        &mock,
        "GET",
        &unknown_postgres_path(""),
        serde_json::json!({
            "id": UNKNOWN_UUID,
            "name": "existing-pg",
            "state": "running",
            "region": "us-east-1",
            "provider": "aws",
            "isPrimary": true,
        }),
    )
    .await;

    let output =
        invoke_cli_with_cloud_credentials(&mock, &postgres_config_patch_args("max_connections=1"));

    assert_eq!(output.status.code(), Some(1));
    let error = cloud_runtime_error(&output);
    assert_eq!(
        error["message"],
        "BAD_REQUEST: max_connections must be at least 10"
    );
    assert_eq!(error["code"], "http_4xx");
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("PATCH".to_string(), unknown_postgres_path("/config")),
            ("GET".to_string(), unknown_postgres_path("")),
        ]
    );
}

/// When the confirming GET itself fails for another reason (a server error,
/// or a permission the caller lacks), it cannot show the service is missing.
/// The original PATCH error stands: its message, JSON code and exit code are
/// unchanged, never not-found and never the GET's own failure.
#[tokio::test]
async fn postgres_config_patch_keeps_the_original_error_when_the_confirming_get_fails() {
    let args = postgres_config_patch_args("max_connections=1");
    for (patch_status, patch_message, get_status, get_message, json) in [
        (
            400,
            "BAD_REQUEST: max_connections must be at least 10",
            500,
            "INTERNAL_ERROR: confirming GET failed",
            true,
        ),
        (
            404,
            "NOT_FOUND: Postgres service not found",
            403,
            "FORBIDDEN: confirming GET not allowed",
            false,
        ),
    ] {
        let mock = MockServer::start().await;
        for (http_method, route, status, message) in [
            (
                "PATCH",
                unknown_postgres_path("/config"),
                patch_status,
                patch_message,
            ),
            ("GET", unknown_postgres_path(""), get_status, get_message),
        ] {
            Mock::given(method(http_method))
                .and(path(route))
                .respond_with(
                    ResponseTemplate::new(status)
                        .set_body_json(serde_json::json!({"status": status, "error": message})),
                )
                .expect(1)
                .mount(&mock)
                .await;
        }

        let case = format!("PATCH {patch_status}, GET {get_status}, json {json}");
        if json {
            let output = invoke_cli_with_cloud_credentials(&mock, &args);
            assert_eq!(output.status.code(), Some(1), "{case}");
            assert!(output.stdout.is_empty(), "{case}");
            let error = cloud_runtime_error(&output);
            assert_eq!(error["message"], patch_message, "{case}");
            assert_eq!(error["code"], "http_4xx", "{case}");
        } else {
            let output = invoke_cli_human(&mock, &args);
            assert_eq!(output.status.code(), Some(1), "{case}");
            assert!(output.stdout.is_empty(), "{case}");
            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                format!("Error: {patch_message}\n"),
                "{case}"
            );
        }
        assert_eq!(
            received_request_shape(&mock).await,
            vec![
                ("PATCH".to_string(), unknown_postgres_path("/config")),
                ("GET".to_string(), unknown_postgres_path("")),
            ],
            "{case}"
        );
    }
}

// Postgres config files must survive every advertised file-input path, PgConfig
// (#591) and PgBouncer (#697) alike. Read-replica and restore bodies are
// assembled in their handlers, so their exact bodies are asserted here.
fn postgres_config_write_commands<'a>(
    pg_config_file: &'a str,
    pg_bouncer_file: &'a str,
    config_file: &'a str,
) -> Vec<(Vec<&'a str>, &'static str, &'static str)> {
    let files = [
        "--pg-config-file",
        pg_config_file,
        "--pg-bouncer-config-file",
        pg_bouncer_file,
        "--tag",
        "env=prod",
    ];
    let with_files = |args: &[&'a str]| -> Vec<&'a str> {
        let mut args = args.to_vec();
        args.extend(files);
        args
    };
    vec![
        (
            with_files(&[
                "postgres",
                "create",
                "--name",
                "pg-test",
                "--region",
                "us-east-1",
                "--size",
                "c6gd.large",
            ]),
            "POST",
            "/v1/organizations/org-1/postgres",
        ),
        (
            with_files(&[
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "replica-test",
            ]),
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/readReplica",
        ),
        (
            with_files(&[
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "restored-test",
                "--restore-target",
                "2026-08-01T00:00:00+01:00",
                "--block-public-access",
            ]),
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/restoredService",
        ),
        (
            vec!["postgres", "config", "patch", "pg-1", "--file", config_file],
            "PATCH",
            "/v1/organizations/org-1/postgres/pg-1/config",
        ),
        (
            vec![
                "postgres",
                "config",
                "replace",
                "pg-1",
                "--file",
                config_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/config",
        ),
    ]
}

fn invoke_pgbouncer_cli(mock: &MockServer, args: &[&str]) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("HOME", directory.path())
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(directory.path())
        .args(["cloud", "--url", &mock.uri(), "--json"])
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn postgres_config_files_reach_every_write_path() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let pg_bouncer_file = directory.path().join("pgbouncer.json");
    let config_file = directory.path().join("config.json");
    let pg_config = serde_json::json!({
        "max_connections": 0,
        "autovacuum_analyze_scale_factor": false,
        "default_transaction_isolation": "repeatable read",
        "wal_compression": "zstd",
        "bgwriter_delay": "200ms"
    });
    let parameters =
        serde_json::json!({"default_pool_size": "16", "future_parameter": "on", "empty": ""});
    let configuration = serde_json::json!({"pgConfig": pg_config, "pgBouncerConfig": parameters});
    std::fs::write(&pg_config_file, pg_config.to_string()).unwrap();
    std::fs::write(&pg_bouncer_file, parameters.to_string()).unwrap();
    std::fs::write(&config_file, configuration.to_string()).unwrap();
    let tags = serde_json::json!([{"key": "env", "value": "prod"}]);
    let with_config = |body: Value| {
        let mut body = body;
        body["tags"] = tags.clone();
        body["pgConfig"] = pg_config.clone();
        body["pgBouncerConfig"] = parameters.clone();
        body
    };

    for (mut args, verb, endpoint) in postgres_config_write_commands(
        pg_config_file.to_str().unwrap(),
        pg_bouncer_file.to_str().unwrap(),
        config_file.to_str().unwrap(),
    ) {
        let mock = MockServer::start().await;
        Mock::given(method(verb))
            .and(path(endpoint))
            .and(wiremock::matchers::basic_auth(
                "fake-key-for-tests",
                "fake-secret-for-tests",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": restored_service_response(endpoint, &configuration), "status": 200,
            })))
            .expect(1)
            .mount(&mock)
            .await;
        args.extend(["--org-id", "org-1"]);
        let output = invoke_pgbouncer_cli(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "{args:?}");
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        // Create omits `ipAccessList` (allow all); `[]` would block public access.
        let expected = match args[1] {
            "create" => with_config(serde_json::json!({
                "name": "pg-test", "provider": "aws", "region": "us-east-1", "size": "c6gd.large"
            })),
            "read-replica" => with_config(serde_json::json!({"name": "replica-test"})),
            "restore" => with_config(serde_json::json!({
                "name": "restored-test", "restoreTarget": "2026-07-31T23:00:00Z"
            })),
            _ => configuration.clone(),
        };
        assert_eq!(body, expected, "{args:?}");
        if endpoint.ends_with("/config") {
            let output: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output["pgBouncerConfig"], parameters);
        }
    }
}

// `config patch` and `config replace` read `--file -` from stdin (#1297).
#[tokio::test]
async fn postgres_config_file_can_be_read_from_stdin() {
    let configuration = serde_json::json!({
        "pgConfig": {"max_connections": 500},
        "pgBouncerConfig": {"default_pool_size": "16"}
    });
    for (command, verb) in [("patch", "PATCH"), ("replace", "POST")] {
        let mock = MockServer::start().await;
        Mock::given(method(verb))
            .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": configuration, "status": 200,
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let args = [
            "postgres", "config", command, "pg-1", "--file", "-", "--org-id", "org-1",
        ];
        let output =
            invoke_cli_with_cloud_credentials_and_stdin(&mock, &args, &configuration.to_string());
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "{command}");
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body, configuration, "{command}");
    }
}

#[tokio::test]
async fn postgres_config_get_honors_human_explicit_json_and_agent_output() {
    let mock = MockServer::start().await;
    let config = serde_json::json!({
        "pgConfig": {"work_mem": "64MB", "max_connections": 500},
        "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(header(
            "authorization",
            "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": config
        })))
        .expect(3)
        .mount(&mock)
        .await;

    let args = ["postgres", "config", "get", "pg-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    assert!(serde_json::from_slice::<Value>(&human.stdout).is_err());
    let human = String::from_utf8(human.stdout).unwrap();
    for line in [
        "pgConfig:",
        "  work_mem: 64MB",
        "  max_connections: 500",
        "pgBouncerConfig:",
        "  default_pool_size: 16",
        "  future_parameter: on",
    ] {
        assert!(human.lines().any(|actual| actual == line), "{human}");
    }

    let explicit = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&explicit);
    assert_eq!(
        serde_json::from_slice::<Value>(&explicit.stdout).unwrap(),
        config
    );

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let agent = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("AI_AGENT", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args(["cloud", "--url", &mock.uri()])
        .args(args)
        .output()
        .unwrap();
    assert_success(&agent);
    assert_eq!(
        serde_json::from_slice::<Value>(&agent.stdout).unwrap(),
        config
    );
    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), None);
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn pg_config_set_preserves_false_zero_and_valid_enums() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"pgConfig": {}, "pgBouncerConfig": {}},
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_pgbouncer_cli(
        &mock,
        &[
            "postgres",
            "config",
            "patch",
            "pg-1",
            "--set",
            "max_connections=0",
            "--set",
            "autovacuum_analyze_scale_factor=false",
            "--set",
            "wal_compression=zstd",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "pgConfig": {
                "max_connections": 0,
                "autovacuum_analyze_scale_factor": false,
                "wal_compression": "zstd"
            },
            "pgBouncerConfig": {}
        })
    );
}

#[tokio::test]
async fn postgres_update_clear_tags_sends_an_empty_array_without_a_get() {
    let mock = MockServer::start().await;
    let postgres_path = format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}");
    Mock::given(method("PATCH"))
        .and(path(postgres_path.clone()))
        .and(body_json(serde_json::json!({"tags": []})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": POSTGRES_ID, "name": "pg-clear-tags"},
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            POSTGRES_ID,
            "--org-id",
            "org-1",
            "--clear-tags",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), postgres_path)],
        "clearing a complete list must not fetch the current service"
    );
}

#[tokio::test]
async fn postgres_update_tag_diff_refuses_a_sparse_get_without_writing() {
    let mock = MockServer::start().await;
    let postgres_path = format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}");
    Mock::given(method("GET"))
        .and(path(postgres_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": POSTGRES_ID, "name": "pg-sparse"},
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(postgres_path.clone()))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            POSTGRES_ID,
            "--org-id",
            "org-1",
            "--add-tag",
            "env=prod",
        ],
    );

    // Refused after the read and before any PATCH (the request shape below);
    // the wording is unit-tested in `postgres::lifecycle`.
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), postgres_path)]
    );
}

#[tokio::test]
async fn postgres_update_clear_tags_propagates_api_errors_without_retrying() {
    let mock = MockServer::start().await;
    let postgres_path = format!("/v1/organizations/org-1/postgres/{POSTGRES_ID}");
    Mock::given(method("PATCH"))
        .and(path(postgres_path.clone()))
        .and(body_json(serde_json::json!({"tags": []})))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "update failed",
            "status": 500,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            POSTGRES_ID,
            "--org-id",
            "org-1",
            "--clear-tags",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), postgres_path)]
    );
}

// Postgres uses the OAuth-only non-OpenAPI Query API, without native clients.
const POSTGRES_QUERY_ROWS: &str = "[\"answer\",\"answer\",\"nullable\"]\n[\"Int32\",\"String\",\"String\"]\n[42,\"hello\\tworld\\nnext\",null]\n";

fn postgres_query_command(root: &Path, control: &MockServer, query_host: &str) -> Command {
    let cloud_dir = root.join("home/.clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &control.uri());
    let mut command = Command::new(clickhousectl_binary());
    command
        .isolated_env()
        .env("HOME", root.join("home"))
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host)
        .current_dir(root)
        .args(["cloud", "--url", &control.uri(), "postgres", "query"])
        .stdin(Stdio::null());
    command
}

async fn mount_postgres_query(query: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(format!("/service/{POSTGRES_ID}/runPostgres")))
        .and(query_param("orgId", "org-1"))
        .and(query_param("format", "JSONCompactEachRowWithNamesAndTypes"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .and(header("x-service-type", "postgres"))
        .and(header("content-type", "application/json"))
        .respond_with(response)
        .expect(1)
        .mount(query)
        .await;
}

#[tokio::test]
async fn postgres_query_streams_compact_json_in_explicit_and_agent_modes() {
    for mode in ["json", "agent"] {
        let control = MockServer::start().await;
        let query = MockServer::start().await;
        mount_postgres_query(
            &query,
            ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
        )
        .await;
        let root = tempfile::tempdir().unwrap();
        let mut command = postgres_query_command(root.path(), &control, &query.uri());
        command.args([POSTGRES_ID, "--org-id", "org-1", "--query", "SELECT 42"]);
        if mode == "json" {
            command.arg("--json");
        } else {
            command.env("CLAUDECODE", "1");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        assert_eq!(output.stdout, POSTGRES_QUERY_ROWS.as_bytes());
        let requests = query.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
            serde_json::json!({"sql": "SELECT 42"})
        );
        assert!(!requests[0].headers.contains_key("auth-provider"));
        assert!(!requests[0].headers.contains_key("wake-service"));
        assert!(control.received_requests().await.unwrap().is_empty());
        assert!(
            !root.path().join(".clickhouse").exists(),
            "no client downloads or service query keys"
        );
    }
}

#[tokio::test]
async fn postgres_query_human_output_preserves_duplicate_columns_and_escapes_cells() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    mount_postgres_query(
        &query,
        ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .args([POSTGRES_ID, "--org-id", "org-1", "--query", "SELECT 42"])
        .output()
        .unwrap();
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert_eq!(lines[0], "answer\tanswer\tnullable");
    assert_eq!(
        lines[1].split('\t').count(),
        3,
        "escaped data cannot introduce columns"
    );
    assert!(lines[1].contains("hello\\tworld\\nnext"));
}

#[tokio::test]
async fn postgres_query_resolves_name_and_sends_database_and_script_unchanged() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"result": [{"id": POSTGRES_ID, "name": "reporting"}]}),
        ))
        .expect(1)
        .mount(&control)
        .await;
    mount_postgres_query(
        &query,
        ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let sql = "SELECT 1;\nSELECT current_database();\n";
    std::fs::write(root.path().join("queries.sql"), sql).unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            "--name",
            "reporting",
            "--org-id",
            "org-1",
            "--database",
            "analytics",
            "--queries-file",
            "queries.sql",
            "--json",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    let requests = query.received_requests().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
        serde_json::json!({"sql": sql, "database": "analytics"})
    );
    assert_eq!(control.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn postgres_query_accepts_default_and_explicit_stdin_and_ignores_it_with_inline_sql() {
    for source in ["implicit", "file-dash", "inline"] {
        let control = MockServer::start().await;
        let query = MockServer::start().await;
        mount_postgres_query(
            &query,
            ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
        )
        .await;
        let root = tempfile::tempdir().unwrap();
        let mut command = postgres_query_command(root.path(), &control, &query.uri());
        command.args([POSTGRES_ID, "--org-id", "org-1", "--json"]);
        match source {
            "file-dash" => {
                command.args(["--queries-file", "-"]);
            }
            "inline" => {
                command.args(["--query", "SELECT 2"]);
            }
            _ => {}
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"SELECT 1\n").unwrap();
        // Keep stdin open with --query: the binary must not wait for EOF.
        if source != "inline" {
            drop(stdin);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                panic!("Postgres query waited for unused stdin or failed to complete");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_success(&output);
        let requests = query.received_requests().await.unwrap();
        let expected = if source == "inline" {
            "SELECT 2"
        } else {
            "SELECT 1\n"
        };
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap()["sql"],
            expected
        );
    }
}

#[tokio::test]
async fn postgres_query_rejects_api_keys_even_with_saved_oauth_before_input_or_http() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret")
        .args([
            "--name",
            "reporting",
            "--org-id",
            "org-1",
            "--queries-file",
            "does-not-exist.sql",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    let error = cloud_runtime_error(&output);
    assert_eq!(error["code"], "auth_required");
    assert!(error["message"].as_str().unwrap().contains("OAuth"));
    assert!(query.received_requests().await.unwrap().is_empty());
    assert!(control.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_query_preserves_structured_failures_without_retries_or_provisioning() {
    for (status, body, exit, code) in [
        (
            400,
            r#"{"error":{"code":"POSTGRES_ERROR","details":"permission denied for relation private_table"}}"#,
            1,
            "sql_error",
        ),
        (401, r#"{"error":"Unauthorized"}"#, 4, "auth_required"),
        (403, r#"{"error":"Forbidden"}"#, 1, "http_4xx"),
        (404, "route missing", 1, "http_4xx"),
        (429, "rate limited", 1, "rate_limited"),
        (500, r#"{"error":"Timeout error."}"#, 1, "timeout"),
        (502, "upstream unavailable", 1, "http_5xx"),
    ] {
        let control = MockServer::start().await;
        let query = MockServer::start().await;
        mount_postgres_query(&query, ResponseTemplate::new(status).set_body_string(body)).await;
        let root = tempfile::tempdir().unwrap();
        let output = postgres_query_command(root.path(), &control, &query.uri())
            .args([
                POSTGRES_ID,
                "--org-id",
                "org-1",
                "--query",
                "SELECT 1",
                "--json",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit),
            "status {status}: {output:?}"
        );
        assert!(output.stdout.is_empty());
        let error = cloud_runtime_error(&output);
        assert_eq!(error["code"], code, "status {status}: {error}");
        assert_eq!(query.received_requests().await.unwrap().len(), 1);
        assert!(control.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn postgres_query_decodes_gzip_with_the_cli_http_client() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    // The upstream Postgres runner always gzips its compact JSON stream.
    let compressed: Vec<u8> = vec![
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 139, 86, 74, 204, 43, 46, 79, 45, 82, 138, 229, 138, 86,
        242, 204, 43, 49, 54, 2, 177, 76, 140, 98, 185, 0, 244, 250, 162, 112, 26, 0, 0, 0,
    ];
    mount_postgres_query(
        &query,
        ResponseTemplate::new(200)
            .insert_header("content-encoding", "gzip")
            .set_body_bytes(compressed),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            POSTGRES_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
            "--json",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(output.stdout, b"[\"answer\"]\n[\"Int32\"]\n[42]\n");
}

#[tokio::test]
async fn postgres_query_reports_truncated_stream_as_failure() {
    use std::io::{BufRead, Read};
    let control = MockServer::start().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 200\r\nConnection: close\r\n\r\n[\"answer\"]\n").unwrap();
    });
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &url)
        .args([
            POSTGRES_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
            "--json",
        ])
        .output()
        .unwrap();
    // Assert before joining: a child that never reached the query host leaves
    // the fake server blocked in accept(), so joining first would hang.
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(cloud_runtime_error(&output)["code"], "transport");
    assert!(control.received_requests().await.unwrap().is_empty());
    // A transport failure that never reached the host (a refused connection,
    // say) passes the asserts above; the child has exited, so wait boundedly.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !server.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "the child never reached the query host"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    server.join().unwrap();
}

#[tokio::test]
async fn postgres_query_closed_stdout_keeps_successful_query_successful() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    let body = format!("[\"answer\"]\n[\"Int32\"]\n{}", "[42]\n".repeat(100_000));
    mount_postgres_query(&query, ResponseTemplate::new(200).set_body_string(body)).await;
    let root = tempfile::tempdir().unwrap();
    let mut child = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            POSTGRES_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
            "--json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert_success(&output);
}

const POSTGRES_BACKUPS_PATH: &str = "/v1/organizations/org-1/postgres/pg-1/backups";

fn postgres_backup_page(result: Value, next_cursor: Option<&str>) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "postgres-backup-list",
        "result": result,
        "limit": 2,
        "totalCount": 3,
        "nextCursor": next_cursor
    }))
}

#[tokio::test]
async fn postgres_backup_list_sends_pagination_and_renders_json_and_table() {
    let server = MockServer::start().await;
    let backups = serde_json::json!([
        {
            "key": "basebackups_005/000000010000000000000002_backup_stop_sentinel.json",
            "lastModified": "2026-03-31T18:17:37Z"
        },
        {}
    ]);
    Mock::given(method("GET"))
        .and(path(POSTGRES_BACKUPS_PATH))
        .and(wiremock::matchers::basic_auth("drift-key", "drift-secret"))
        .and(query_param("cursor", "page /+?"))
        .and(query_param("limit", "2"))
        .respond_with(postgres_backup_page(backups.clone(), Some("next /+?")))
        .with_priority(1)
        .expect(2)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let args = [
        "postgres", "backup", "list", "pg-1", "--cursor", "page /+?", "--limit", "2", "--org-id",
        "org-1",
    ];

    let json = invoke_cloud_as(&server, project.path(), false, true, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!({
            "result": backups,
            "limit": 2,
            "totalCount": 3,
            "nextCursor": "next /+?"
        })
    );

    let human = invoke_cloud_as(&server, project.path(), false, false, &args);
    assert_success(&human);
    let stdout = String::from_utf8_lossy(&human.stdout);
    for expected in [
        "Key",
        "Last modified",
        "basebackups_005/000000010000000000000002_backup_stop_sentinel.json",
        "2026-03-31T18:17:37Z",
        "Next cursor: next /+?",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
}
