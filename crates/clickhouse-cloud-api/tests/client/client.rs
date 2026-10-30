use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_json, setup};

// ===========================================================================
// Authentication
// ===========================================================================

#[tokio::test]
async fn bearer_auth_sends_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(bearer_token("my-oauth-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "name": "Bearer Org"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_bearer_token(mock_server.uri(), "my-oauth-token");
    let resp = client.organization_get_list(None, None).await.unwrap();
    let orgs = resp.result.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name.as_deref(), Some("Bearer Org"));
}

#[tokio::test]
async fn set_bearer_token_updates_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(bearer_token("refreshed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": []
        })))
        .mount(&mock_server)
        .await;

    let mut client = Client::with_bearer_token(mock_server.uri(), "old-token");
    client.set_bearer_token("refreshed-token").unwrap();
    let resp = client.organization_get_list(None, None).await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn with_http_client_basic_auth() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(basic_auth("custom-key", "custom-secret"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let http = reqwest::Client::new();
    let client = Client::with_http_client(http, mock_server.uri(), "custom-key", "custom-secret");
    let resp = client.organization_get_list(None, None).await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn with_http_client_bearer_auth() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(bearer_token("custom-bearer"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let http = reqwest::Client::new();
    let client = Client::with_http_client_bearer(http, mock_server.uri(), "custom-bearer");
    let resp = client.organization_get_list(None, None).await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[test]
fn set_bearer_token_errors_on_basic_auth() {
    let mut client = Client::new("key", "secret");
    let err = client.set_bearer_token("token").unwrap_err();
    assert!(
        err.to_string().contains("auth mismatch"),
        "unexpected error: {err}"
    );
}

// ===========================================================================
// Error handling
// ===========================================================================

#[tokio::test]
async fn api_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "status": 401,
            "error": "Invalid credentials",
            "requestId": "req-err-123"
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "bad-key", "bad-secret");
    let err = client.organization_get_list(None, None).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 401);
            assert_eq!(message, "Invalid credentials");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_403_forbidden() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Forbidden: insufficient permissions"
        })))
        .mount(&s)
        .await;

    let body = ServicePostRequest {
        name: "test".to_string(),
        ..Default::default()
    };
    let err = c.instance_create("org-1", &body).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 403);
            assert!(message.contains("Forbidden"));
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_404_not_found() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "Service not found"
        })))
        .mount(&s)
        .await;

    let err = c.instance_get("org-1", "nonexistent").await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Service not found");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_500_server_error() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "Internal server error"
        })))
        .mount(&s)
        .await;

    let err = c.organization_get_list(None, None).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "Internal server error");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_non_json_body() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
        .mount(&s)
        .await;

    let err = c.organization_get_list(None, None).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 502);
            // Falls back to raw body text when JSON parsing fails
            assert_eq!(message, "Bad Gateway");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_on_prometheus_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Metrics access denied"
        })))
        .mount(&s)
        .await;

    let err = c
        .organization_prometheus_get("org-1", None)
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 403);
            assert_eq!(message, "Metrics access denied");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_on_postgres_certs_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/caCertificates"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "Postgres service not found"
        })))
        .mount(&s)
        .await;

    let err = c
        .postgres_service_certs_get("org-1", "pg-1")
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Postgres service not found");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

// ===========================================================================
// Malformed responses (Error::Json coverage)
// ===========================================================================

#[tokio::test]
async fn malformed_json_success_response() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json"))
        .mount(&s)
        .await;

    let err = c.organization_get_list(None, None).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Json(_) => {} // expected
        other => panic!("Expected Json error, got: {:?}", other),
    }
}

#[tokio::test]
async fn truncated_json_success_response() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status": 200, "result":"#))
        .mount(&s)
        .await;

    let err = c.organization_get_list(None, None).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Json(_) => {} // expected
        other => panic!("Expected Json error, got: {:?}", other),
    }
}

// ===========================================================================
// Additional HTTP status codes
// ===========================================================================

#[tokio::test]
async fn api_error_429_rate_limited() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "status": 429,
            "error": "Rate limit exceeded"
        })))
        .mount(&s)
        .await;

    let err = c.organization_get_list(None, None).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 429);
            assert_eq!(message, "Rate limit exceeded");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_422_validation() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "status": 422,
            "error": "Validation failed: name is required"
        })))
        .mount(&s)
        .await;

    let body = ServicePostRequest::default();
    let err = c.instance_create("org-1", &body).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 422);
            assert!(message.contains("Validation failed"));
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

// ===========================================================================
// Base URL handling
// ===========================================================================

#[tokio::test]
async fn base_url_trailing_slash_stripped() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let url_with_slash = format!("{}/", mock_server.uri());
    let client = Client::with_base_url(url_with_slash, "key", "secret");
    let resp = client.organization_get_list(None, None).await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn default_base_url_is_production() {
    // Client::new() uses https://api.clickhouse.cloud -- we can't hit it,
    // but we can verify the client is constructable without panicking.
    let _client = Client::new("key", "secret");
}

#[tokio::test]
async fn new_discovery_operations_preserve_api_errors() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(serde_json::json!({"error": "forbidden"})),
        )
        .expect(3)
        .mount(&server)
        .await;
    let errors = [
        client.credit_balances_get("org").await.unwrap_err(),
        client
            .service_profiles_list("org", Some("region"), None)
            .await
            .unwrap_err(),
        client
            .click_pipes_service_context_get("org", "service")
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert!(
            matches!(error, clickhouse_cloud_api::Error::Api { status: 403, message } if message == "forbidden")
        );
    }
}
