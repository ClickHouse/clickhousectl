//! Request-shape tests for the ClickHouse and Postgres Query API methods
//! against a local wiremock server.
//!
//! These assert the auth header, request body shape, and headers each
//! variant puts on the wire, without touching any cloud infrastructure —
//! the real Query API is exercised by the cloud integration tests. The
//! query host is pinned with `with_query_host` so the tests are independent
//! of the `CLICKHOUSE_CLOUD_QUERY_HOST` env var and host derivation.

use base64::Engine as _;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clickhouse_cloud_api::{Client, Error, RunPostgresQueryRequest};

async fn start_mock_query_host(status: u16, body: &str) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/service/svc-1/run"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&mock)
        .await;
    mock
}

#[tokio::test]
async fn run_query_sends_basic_auth_with_query_key() {
    let mock = start_mock_query_host(200, "1\n").await;
    let client =
        Client::with_base_url(mock.uri(), "org-key", "org-secret").with_query_host(mock.uri());

    let response = client
        .run_query(
            "svc-1",
            "query-key",
            "query-secret",
            "SELECT 1",
            None,
            "TabSeparated",
            false,
        )
        .await
        .expect("run_query failed");
    assert_eq!(response.status(), 200);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];

    // Basic auth must use the per-service Query API key, not the client's
    // primary (org-level) credentials.
    let auth = request
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    let expected = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("query-key:query-secret")
    );
    assert_eq!(auth, expected);

    assert_eq!(request.headers.get("auth-provider").unwrap(), "custom");
    assert_eq!(request.headers.get("x-service-type").unwrap(), "clickhouse");
    assert!(
        request.headers.get("wake-service").is_none(),
        "wake-service header must not be sent unless wake_service is set",
    );

    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["sql"], "SELECT 1");
    assert!(body["runId"].as_str().is_some(), "runId missing: {body}");
    assert!(
        body.get("database").is_none(),
        "database leaked into body when not set: {body}"
    );

    let format = request
        .url
        .query_pairs()
        .find(|(k, _)| k == "format")
        .map(|(_, v)| v.to_string());
    assert_eq!(format.as_deref(), Some("TabSeparated"));
}

#[tokio::test]
async fn run_query_bearer_sends_bearer_token() {
    let mock = start_mock_query_host(200, "1\n").await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let response = client
        .run_query_bearer("svc-1", "SELECT 1", Some("mydb"), "JSONEachRow", false)
        .await
        .expect("run_query_bearer failed");
    assert_eq!(response.status(), 200);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];

    let auth = request
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(auth, "Bearer oauth-token");

    // `auth-provider: custom` marks a custom Query API key; it must not be
    // sent alongside a bearer token.
    assert!(request.headers.get("auth-provider").is_none());
    assert_eq!(request.headers.get("x-service-type").unwrap(), "clickhouse");

    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["sql"], "SELECT 1");
    assert_eq!(body["database"], "mydb");
}

#[tokio::test]
async fn run_query_bearer_on_basic_auth_client_is_auth_mismatch() {
    let client = Client::with_base_url("https://api.clickhouse.cloud", "k", "s");
    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected AuthMismatch");
    assert!(
        matches!(err, Error::AuthMismatch(_)),
        "expected AuthMismatch, got: {err:?}"
    );
}

#[tokio::test]
async fn run_query_wake_service_sends_wake_header() {
    let mock = start_mock_query_host(200, "1\n").await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    client
        .run_query_bearer("svc-1", "SELECT 1", None, "TabSeparated", true)
        .await
        .expect("run_query_bearer failed");

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].headers.get("wake-service").unwrap(), "true");
}

// ── 206 responses: the query host's service-state protocol ─────────────────
//
// An idled or stopped service answers 206 with `{"data": "<state>"}` instead
// of running the query. `Confirm wake service` invites a resend with the
// `wake-service: true` header (which wakes the service); `Service is
// stopped` is terminal until the service is started.

#[tokio::test]
async fn run_query_206_confirm_wake_maps_to_service_idle() {
    let mock = start_mock_query_host(206, r#"{"data":"Confirm wake service"}"#).await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected ServiceIdle");
    assert!(
        matches!(err, Error::ServiceIdle),
        "expected ServiceIdle, got: {err:?}"
    );
}

#[tokio::test]
async fn run_query_206_service_stopped_maps_to_service_stopped() {
    let mock = start_mock_query_host(206, r#"{"data":"Service is stopped"}"#).await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected ServiceStopped");
    assert!(
        matches!(err, Error::ServiceStopped),
        "expected ServiceStopped, got: {err:?}"
    );
}

#[tokio::test]
async fn run_query_404_unavailable_service_maps_to_service_stopped() {
    let mock = start_mock_query_host(
        404,
        r#"{"error":"ClickHouse service is currently unavailable. Please try again later."}"#,
    )
    .await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected ServiceStopped");
    assert!(
        matches!(err, Error::ServiceStopped),
        "expected ServiceStopped, got: {err:?}"
    );
}

#[tokio::test]
async fn run_query_206_unrecognized_body_maps_to_api_error() {
    let mock = start_mock_query_host(206, r#"{"data":"Something new"}"#).await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected Api error");
    match err {
        Error::Api { status, message } => {
            assert_eq!(status, 206);
            assert_eq!(
                message,
                r#"Query API returned HTTP 206 Partial Content: {"data":"Something new"}"#
            );
        }
        other => panic!("expected Error::Api, got: {other:?}"),
    }
}

#[tokio::test]
async fn run_query_non_success_status_maps_to_api_error() {
    let mock = start_mock_query_host(404, "query endpoint not found").await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected Api error");
    match err {
        Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(
                message,
                "Query API returned HTTP 404 Not Found: query endpoint not found"
            );
        }
        other => panic!("expected Error::Api, got: {other:?}"),
    }
}

#[tokio::test]
async fn run_query_formats_documented_sql_error_envelope() {
    let mock = start_mock_query_host(
        400,
        r#"{"error":{"code":"62","details":"Syntax error near FROM"}}"#,
    )
    .await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT broken FROM", None, "CSV", false)
        .await
        .expect_err("expected Sql error");
    // The documented envelope maps to the structural Error::Sql variant so
    // callers match on it instead of sniffing the "SQL error " prefix; the
    // Display text is the stable user-facing string.
    assert_eq!(
        err.to_string(),
        "SQL error 62: Syntax error near FROM",
        "Display must stay stable: {err:?}"
    );
    match err {
        Error::Sql {
            status,
            code,
            details,
        } => {
            assert_eq!(status, 400);
            assert_eq!(code, "62");
            assert_eq!(details, "Syntax error near FROM");
        }
        other => panic!("expected Error::Sql, got: {other:?}"),
    }
}

#[tokio::test]
async fn run_query_preserves_malformed_json_error_with_status() {
    let body = r#"{"error":{"code":"62","details":"truncated"#;
    let mock = start_mock_query_host(400, body).await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT broken FROM", None, "CSV", false)
        .await
        .expect_err("expected Api error");
    match err {
        Error::Api { status, message } => {
            assert_eq!(status, 400);
            assert_eq!(
                message,
                format!("Query API returned HTTP 400 Bad Request: {body}")
            );
        }
        other => panic!("expected Error::Api, got: {other:?}"),
    }
}

// ── the gateway's own timeout (issue #644) ─────────────────────────────────
//
// The Query API gateway stops waiting after roughly 30 seconds and answers
// HTTP 500 with `{"error": "Timeout error."}`. The statement keeps running on
// the service, so the failure gets its own variant: a caller that read it as
// a transient 500 and resent the request would run the statement twice.

#[tokio::test]
async fn run_query_500_gateway_timeout_maps_to_query_timeout() {
    let mock = start_mock_query_host(500, r#"{"error":"Timeout error."}"#).await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT sleep(3)", None, "CSV", false)
        .await
        .expect_err("expected QueryTimeout");
    assert!(
        matches!(err, Error::QueryTimeout),
        "expected QueryTimeout, got: {err:?}"
    );
    // Exactly one request: the library never resends a statement that may
    // still be running.
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn run_query_other_500_body_stays_an_api_error() {
    let body = r#"{"error":"Internal error."}"#;
    let mock = start_mock_query_host(500, body).await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV", false)
        .await
        .expect_err("expected Api error");
    match err {
        Error::Api { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(
                message,
                format!("Query API returned HTTP 500 Internal Server Error: {body}")
            );
        }
        other => panic!("expected Error::Api, got: {other:?}"),
    }
}

// ── Postgres Query API (outside the published OpenAPI) ─────────────────────

async fn start_mock_postgres_query_host(status: u16, body: &str) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/service/pg-1/runPostgres"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&mock)
        .await;
    mock
}

#[test]
fn postgres_query_request_requires_sql_and_omits_absent_database() {
    assert!(serde_json::from_str::<RunPostgresQueryRequest>("{}").is_err());
    assert!(serde_json::from_str::<RunPostgresQueryRequest>(r#"{"sql":null}"#).is_err());
    let request: clickhouse_cloud_api::models::RunPostgresQueryRequest =
        serde_json::from_str(r#"{"sql":"SELECT 1"}"#).unwrap();
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({"sql": "SELECT 1"})
    );
    let request = RunPostgresQueryRequest {
        sql: "SELECT 1".into(),
        database: Some("app".into()),
    };
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({"sql": "SELECT 1", "database": "app"})
    );
}

#[tokio::test]
async fn run_postgres_query_bearer_sends_postgres_contract() {
    let body = "[\"a\"]\n[\"Int32\"]\n[1]\n";
    let mock = start_mock_postgres_query_host(200, body).await;
    // The explicit query host wins over the management host's derived host.
    let client = Client::with_bearer_token("https://api.clickhouse.cloud", "oauth-token")
        .with_query_host(format!("{}/", mock.uri()));
    for database in [None, Some("app & analytics".to_string())] {
        let request = RunPostgresQueryRequest {
            sql: "SELECT 1 AS a".into(),
            database: database.clone(),
        };
        let response = client
            .run_postgres_query_bearer("org & /?=1", "pg-1", &request)
            .await
            .expect("Postgres query failed");
        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await.unwrap(), body);
    }
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for (index, request) in requests.iter().enumerate() {
        assert_eq!(
            request.headers.get("authorization").unwrap(),
            "Bearer oauth-token"
        );
        assert_eq!(request.headers.get("x-service-type").unwrap(), "postgres");
        assert_eq!(
            request.headers.get("content-type").unwrap(),
            "application/json"
        );
        assert!(request.headers.get("auth-provider").is_none());
        assert!(request.headers.get("wake-service").is_none());
        let query: std::collections::BTreeMap<_, _> = request.url.query_pairs().collect();
        assert_eq!(query.len(), 2);
        assert_eq!(query.get("orgId").unwrap(), "org & /?=1");
        assert_eq!(
            query.get("format").unwrap(),
            "JSONCompactEachRowWithNamesAndTypes"
        );
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let expected = if index == 0 {
            serde_json::json!({"sql": "SELECT 1 AS a"})
        } else {
            serde_json::json!({"sql": "SELECT 1 AS a", "database": "app & analytics"})
        };
        assert_eq!(body, expected);
    }
}

#[tokio::test]
async fn run_postgres_query_bearer_rejects_basic_auth_without_network() {
    let mock = MockServer::start().await;
    let client =
        Client::with_base_url(mock.uri(), "api-key", "api-secret").with_query_host(mock.uri());
    let request = RunPostgresQueryRequest {
        sql: "SELECT 1".into(),
        database: None,
    };
    let error = client
        .run_postgres_query_bearer("org-1", "pg-1", &request)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::AuthMismatch(_)), "{error:?}");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn run_postgres_query_bearer_decompresses_streamed_gzip() {
    let mock = MockServer::start().await;
    // A gzip member containing ["a"], ["Int32"], and [1], each on a new line.
    let compressed = base64::engine::general_purpose::STANDARD
        .decode("H4sIAAAAAAAC/4tWSlSK5YpW8swrMTYCsQxjuQDeBduhFAAAAA==")
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/service/pg-1/runPostgres"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_bytes(compressed),
        )
        .mount(&mock)
        .await;
    let clients = [
        Client::with_bearer_token(mock.uri(), "oauth-token"),
        Client::with_http_client_bearer(
            reqwest::Client::builder().build().unwrap(),
            mock.uri(),
            "oauth-token",
        ),
    ];
    let request = RunPostgresQueryRequest {
        sql: "SELECT 1 AS a".into(),
        database: None,
    };
    for client in clients {
        let mut response = client
            .with_query_host(mock.uri())
            .run_postgres_query_bearer("org-1", "pg-1", &request)
            .await
            .unwrap();
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.unwrap() {
            body.extend_from_slice(&chunk);
        }
        assert_eq!(body, b"[\"a\"]\n[\"Int32\"]\n[1]\n");
    }
}

#[tokio::test]
async fn run_postgres_query_bearer_preserves_empty_success() {
    let mock = start_mock_postgres_query_host(200, "").await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());
    let request = RunPostgresQueryRequest {
        sql: "SELECT 1 WHERE false".into(),
        database: None,
    };
    let response = client
        .run_postgres_query_bearer("org-1", "pg-1", &request)
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(response.bytes().await.unwrap().is_empty());
}

#[tokio::test]
async fn run_postgres_query_bearer_preserves_error_code_and_details() {
    let mock = start_mock_postgres_query_host(
        400,
        r#"{"error":{"code":"POSTGRES_ERROR","details":"column does not exist"}}"#,
    )
    .await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());
    let request = RunPostgresQueryRequest {
        sql: "SELECT missing_column".into(),
        database: None,
    };
    let error = client
        .run_postgres_query_bearer("org-1", "pg-1", &request)
        .await
        .unwrap_err();
    match error {
        Error::Sql {
            status,
            code,
            details,
        } => {
            assert_eq!(status, 400);
            assert_eq!(code, "POSTGRES_ERROR");
            assert_eq!(details, "column does not exist");
        }
        error => panic!("expected a SQL error, got {error:?}"),
    }
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn run_postgres_query_bearer_preserves_http_errors_without_retry() {
    for (status, body) in [
        (401, r#"{"error":"Unauthorized"}"#),
        (403, r#"{"error":"Forbidden"}"#),
        (404, "Postgres service not found"),
        (429, "rate limited"),
        (500, r#"{"error":{"code":"incomplete"}"#),
        (503, ""),
    ] {
        let mock = start_mock_postgres_query_host(status, body).await;
        let client =
            Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());
        let request = RunPostgresQueryRequest {
            sql: "SELECT 1".into(),
            database: None,
        };
        let error = client
            .run_postgres_query_bearer("org-1", "pg-1", &request)
            .await
            .unwrap_err();
        match error {
            Error::Api {
                status: actual,
                message,
            } => {
                assert_eq!(actual, status);
                assert!(message.contains(body));
            }
            error => panic!("expected HTTP {status} API error, got {error:?}"),
        }
        assert_eq!(mock.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
#[ignore = "requires deployed clickhousectl OAuth support and an existing Postgres service"]
async fn live_postgres_query_bearer_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("CLICKHOUSE_CLOUD_TEST_BEARER_TOKEN")?;
    let org_id = std::env::var("CLICKHOUSE_CLOUD_TEST_ORG_ID")?;
    let service_id = std::env::var("CLICKHOUSE_CLOUD_TEST_POSTGRES_SERVICE_ID")?;
    let database = std::env::var("CLICKHOUSE_CLOUD_TEST_POSTGRES_DATABASE").ok();
    let base_url = std::env::var("CLICKHOUSE_CLOUD_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.clickhouse.cloud".into());
    let client = Client::with_bearer_token(base_url, token);
    let response = client
        .run_postgres_query_bearer(
            &org_id,
            &service_id,
            &RunPostgresQueryRequest {
                sql: "SELECT 1 AS a".into(),
                database: database.clone(),
            },
        )
        .await?;
    let body = response.text().await?;
    let lines = body
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], serde_json::json!(["a"]));
    assert_eq!(lines[1], serde_json::json!(["Int32"]));
    assert_eq!(lines[2], serde_json::json!([1]));
    let response = client
        .run_postgres_query_bearer(
            &org_id,
            &service_id,
            &RunPostgresQueryRequest {
                sql: "SELECT 1 AS a WHERE false".into(),
                database,
            },
        )
        .await?;
    let body = response.text().await?;
    let lines = body
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert!(
        lines.is_empty() || lines == vec![serde_json::json!(["a"]), serde_json::json!(["Int32"])],
        "empty query must return no data rows"
    );
    Ok(())
}
