//! Request-shape tests for the Query API methods (`run_query` and
//! `run_query_bearer`) against a local wiremock server.
//!
//! These assert the auth header, request body shape, and headers each
//! variant puts on the wire, without touching any cloud infrastructure —
//! the real Query API is exercised by the cloud integration tests. The
//! query host is pinned with `with_query_host` so the tests are independent
//! of the `CLICKHOUSE_CLOUD_QUERY_HOST` env var and host derivation.

use base64::Engine as _;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clickhouse_cloud_api::{Client, Error};

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
        )
        .await
        .expect("run_query failed");
    assert_eq!(response.status(), 200);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];

    // Basic auth must use the per-service Query API key, not the client's
    // primary (org-level) credentials.
    let auth = request.headers.get("authorization").unwrap().to_str().unwrap();
    let expected = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("query-key:query-secret")
    );
    assert_eq!(auth, expected);

    assert_eq!(request.headers.get("auth-provider").unwrap(), "custom");
    assert_eq!(request.headers.get("x-service-type").unwrap(), "clickhouse");

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
        .run_query_bearer("svc-1", "SELECT 1", Some("mydb"), "JSONEachRow")
        .await
        .expect("run_query_bearer failed");
    assert_eq!(response.status(), 200);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];

    let auth = request.headers.get("authorization").unwrap().to_str().unwrap();
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
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV")
        .await
        .expect_err("expected AuthMismatch");
    assert!(
        matches!(err, Error::AuthMismatch(_)),
        "expected AuthMismatch, got: {err:?}"
    );
}

#[tokio::test]
async fn run_query_non_success_status_maps_to_api_error() {
    let mock = start_mock_query_host(404, "query endpoint not found").await;
    let client = Client::with_bearer_token(mock.uri(), "oauth-token").with_query_host(mock.uri());

    let err = client
        .run_query_bearer("svc-1", "SELECT 1", None, "CSV")
        .await
        .expect_err("expected Api error");
    match err {
        Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "query endpoint not found");
        }
        other => panic!("expected Error::Api, got: {other:?}"),
    }
}
