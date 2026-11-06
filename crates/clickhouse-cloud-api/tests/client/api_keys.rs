use chrono::Utc;
use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, body_json, body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// API Keys
// ===========================================================================

#[tokio::test]
async fn list_api_keys() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                    "name": "Production Key",
                    "state": "enabled"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client
        .openapi_key_get_list("org-1", None, None)
        .await
        .unwrap();
    let keys = resp.result.unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name.as_deref(), Some("Production Key"));
}

#[tokio::test]
async fn list_api_keys_pagination_encodes_parameters_and_preserves_envelope() {
    let (server, client) = setup().await;
    let cursor = "opaque +/=&?雪";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/keys"))
        .and(basic_auth("key", "secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200.0, "requestId": "req-page", "result": [],
            "limit": 25, "totalCount": 100, "nextCursor": cursor
        })))
        .expect(4)
        .mount(&server)
        .await;
    for (limit, cursor_arg) in [
        (None, None),
        (Some(25), None),
        (None, Some(cursor)),
        (Some(25), Some(cursor)),
    ] {
        let response = client
            .openapi_key_get_list("org", limit, cursor_arg)
            .await
            .unwrap();
        assert_eq!(response.status, Some(200));
        assert_eq!(response.request_id.as_deref(), Some("req-page"));
        assert_eq!(response.result, Some(vec![]));
        assert_eq!(response.error, None);
        assert_eq!(response.limit, Some(25));
        assert_eq!(response.total_count, Some(100));
        assert_eq!(response.next_cursor.as_deref(), Some(cursor));
    }
    let requests = server.received_requests().await.unwrap();
    let queries: Vec<Vec<(String, String)>> = requests
        .iter()
        .map(|request| request.url.query_pairs().into_owned().collect())
        .collect();
    assert_eq!(
        queries,
        vec![
            vec![],
            vec![("limit".into(), "25".into())],
            vec![("cursor".into(), cursor.into())],
            vec![
                ("limit".into(), "25".into()),
                ("cursor".into(), cursor.into())
            ],
        ]
    );
}

#[tokio::test]
async fn list_api_keys_preserves_api_errors() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/keys"))
        .and(basic_auth("key", "secret"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403.0, "error": "forbidden", "requestId": "req-denied"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let error = client
        .openapi_key_get_list("org", Some(25), Some("cursor"))
        .await
        .unwrap_err();
    assert!(
        matches!(error, clickhouse_cloud_api::Error::Api { status: 403, message } if message == "forbidden")
    );
}

#[tokio::test]
async fn create_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .and(body_partial_json(serde_json::json!({"name": "New Key"})))
        .respond_with(ok_json(serde_json::json!({
            "key": {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "New Key",
                "state": "enabled"
            },
            "keyId": "key-id-abc",
            "keySecret": "key-secret-xyz"
        })))
        .mount(&s)
        .await;

    let body = ApiKeyPostRequest {
        name: "New Key".to_string(),
        ..Default::default()
    };
    let resp = c.openapi_key_create("org-1", &body).await.unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.key_id.as_deref(), Some("key-id-abc"));
    assert_eq!(result.key_secret.as_deref(), Some("key-secret-xyz"));
    assert_eq!(
        result.key.as_ref().and_then(|key| key.name.as_deref()),
        Some("New Key")
    );
}

#[tokio::test]
async fn get_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "My Key",
            "state": "enabled",
            "keySuffix": "abc"
        })))
        .mount(&s)
        .await;

    let resp = c.openapi_key_get("org-1", "key-1").await.unwrap();
    let key = resp.result.unwrap();
    assert_eq!(key.name.as_deref(), Some("My Key"));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
    assert_eq!(key.key_suffix.as_deref(), Some("abc"));
}

#[tokio::test]
async fn update_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .and(body_partial_json(
            serde_json::json!({"name": "Renamed Key"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "Renamed Key",
            "state": "enabled"
        })))
        .mount(&s)
        .await;

    let body = ApiKeyPatchRequest {
        name: Some("Renamed Key".to_string()),
        ..Default::default()
    };
    let resp = c.openapi_key_update("org-1", "key-1", &body).await.unwrap();
    let key = resp.result.unwrap();
    assert_eq!(key.name.as_deref(), Some("Renamed Key"));
}

#[tokio::test]
async fn delete_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.openapi_key_delete("org-1", "key-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn update_api_key_sends_omitted_timestamp_and_null_expiry() {
    let timestamp = chrono::DateTime::parse_from_rfc3339("2030-01-02T03:04:05Z")
        .unwrap()
        .with_timezone(&Utc);
    for (expire_at, wire) in [
        (None, serde_json::json!({"name": "retained"})),
        (
            Some(Some(timestamp)),
            serde_json::json!({"name": "retained", "expireAt": "2030-01-02T03:04:05Z"}),
        ),
        (
            Some(None),
            serde_json::json!({"name": "retained", "expireAt": null}),
        ),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/keys/key-1"))
            .and(body_json(wire))
            .respond_with(ok_json(serde_json::json!({"name": "retained"})))
            .expect(1)
            .mount(&server)
            .await;
        let request = ApiKeyPatchRequest {
            name: Some("retained".into()),
            expire_at,
            ..Default::default()
        };
        let response = client
            .openapi_key_update("org-1", "key-1", &request)
            .await
            .unwrap();
        assert_eq!(response.result.unwrap().name.as_deref(), Some("retained"));
    }
}
