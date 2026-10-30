use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, bearer_token, body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{created_json, ok_empty, ok_json, setup};

#[tokio::test]
async fn query_api_endpoint_create_and_update_send_complete_requests() {
    let (server, client) = setup().await;
    let collection = "/v1/organizations/org-1/services/svc-1/query-api-endpoints";
    let minimal = serde_json::json!({
        "name": "daily total", "sql": "SELECT count() FROM events", "database": "default",
        "apiKeyIds": ["00000000-0000-4000-8000-000000000001"], "roles": ["reader"]
    });
    let maximal = serde_json::json!({
        "name": "filtered total", "sql": "SELECT count() FROM events WHERE kind = {kind:String}",
        "database": "analytics", "apiKeyIds": ["00000000-0000-4000-8000-000000000001", "00000000-0000-4000-8000-000000000002"], "roles": ["reader"],
        "parameters": {"kind": "page view"}, "allowedOrigins": ["https://example.com"]
    });
    for body in [minimal, maximal] {
        let request: PublicQueryApiEndpointRequest = serde_json::from_value(body.clone()).unwrap();
        let mut returned = body.clone();
        returned["id"] = serde_json::json!("00000000-0000-4000-8000-000000000003");
        returned["url"] = serde_json::json!("https://queries.clickhouse.cloud/run/endpoint-1");
        returned["ownerType"] = serde_json::json!("queryApiEndpoint");
        Mock::given(method("POST"))
            .and(path(collection))
            .and(basic_auth("key", "secret"))
            .and(body_json(body.clone()))
            .respond_with(created_json(returned.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .query_api_endpoint_create("org-1", "svc-1", &request)
            .await
            .unwrap();
        assert_eq!(response.status, Some(201));
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            returned
        );

        Mock::given(method("PUT"))
            .and(path(format!("{collection}/endpoint-1")))
            .and(basic_auth("key", "secret"))
            .and(body_json(body))
            .respond_with(ok_json(returned.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .query_api_endpoint_update("org-1", "svc-1", "endpoint-1", &request)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            returned
        );
    }
}

#[tokio::test]
async fn query_api_endpoint_get_and_delete_use_endpoint_path() {
    let (server, client) = setup().await;
    let endpoint_path = "/v1/organizations/org-1/services/svc-1/query-api-endpoints/endpoint-1";
    Mock::given(method("GET"))
        .and(path(endpoint_path))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(
            serde_json::json!({"id": "00000000-0000-4000-8000-000000000003", "name": "example"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let response = client
        .query_api_endpoint_get("org-1", "svc-1", "endpoint-1")
        .await
        .unwrap();
    assert_eq!(
        response.result.unwrap().id,
        Some(uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000003").unwrap())
    );
    Mock::given(method("DELETE"))
        .and(path(endpoint_path))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_empty())
        .expect(1)
        .mount(&server)
        .await;
    let response = client
        .query_api_endpoint_delete("org-1", "svc-1", "endpoint-1")
        .await
        .unwrap();
    assert_eq!(response.status, Some(200));
    assert_eq!(response.request_id.as_deref(), Some("req-test"));
    assert!(response.result.is_none());
    for request in server.received_requests().await.unwrap() {
        assert!(request.body.is_empty());
        assert!(request.url.query().is_none());
    }
}

#[tokio::test]
async fn query_api_endpoint_list_encodes_cursor_and_omits_absent_parameters() {
    for (cursor, limit) in [
        (None, None),
        (Some("next+/=&? page"), None),
        (None, Some(25)),
        (Some("next+/=&? page"), Some(25)),
    ] {
        let (server, client) = setup().await;
        let result = serde_json::json!([
            {"id": "00000000-0000-4000-8000-000000000003", "name": "example", "ownerType": "user"}
        ]);
        Mock::given(method("GET"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/query-api-endpoints",
            ))
            .and(basic_auth("key", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result, "limit": 25, "totalCount": 42,
                "nextCursor": "another-page", "status": 200
            })))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .query_api_endpoint_list("org-1", "svc-1", cursor, limit)
            .await
            .unwrap();
        assert_eq!(response.limit, Some(25));
        assert_eq!(response.total_count, Some(42));
        assert_eq!(response.next_cursor.as_deref(), Some("another-page"));
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            result
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
async fn query_api_endpoint_reads_use_bearer_auth() {
    let server = MockServer::start().await;
    let client = Client::with_bearer_token(server.uri(), "token");
    for (endpoint_path, result) in [
        (
            "/v1/organizations/org/services/svc/query-api-endpoints",
            serde_json::json!([]),
        ),
        (
            "/v1/organizations/org/services/svc/query-api-endpoints/endpoint",
            serde_json::json!({}),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(endpoint_path))
            .and(bearer_token("token"))
            .respond_with(ok_json(result))
            .expect(1)
            .mount(&server)
            .await;
    }
    client
        .query_api_endpoint_list("org", "svc", None, None)
        .await
        .unwrap();
    client
        .query_api_endpoint_get("org", "svc", "endpoint")
        .await
        .unwrap();
}

#[tokio::test]
async fn query_api_endpoint_methods_propagate_api_errors() {
    for (status, body, expected_message) in [
        (
            403,
            serde_json::json!({"status": 403, "error": "forbidden"}).to_string(),
            "forbidden",
        ),
        (
            409,
            serde_json::json!({"status": 409, "error": "user-owned endpoint"}).to_string(),
            "user-owned endpoint",
        ),
        (500, "upstream failed".to_owned(), "upstream failed"),
    ] {
        let (server, client) = setup().await;
        Mock::given(basic_auth("key", "secret"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(5)
            .mount(&server)
            .await;
        let request: PublicQueryApiEndpointRequest = serde_json::from_value(serde_json::json!({
            "name": "example", "sql": "SELECT 1", "database": "default",
            "apiKeyIds": ["00000000-0000-4000-8000-000000000001"], "roles": ["reader"]
        }))
        .unwrap();
        let errors = [
            client
                .query_api_endpoint_create("org", "svc", &request)
                .await
                .unwrap_err(),
            client
                .query_api_endpoint_get("org", "svc", "endpoint")
                .await
                .unwrap_err(),
            client
                .query_api_endpoint_list("org", "svc", None, None)
                .await
                .unwrap_err(),
            client
                .query_api_endpoint_update("org", "svc", "endpoint", &request)
                .await
                .unwrap_err(),
            client
                .query_api_endpoint_delete("org", "svc", "endpoint")
                .await
                .unwrap_err(),
        ];
        for error in errors {
            assert!(
                matches!(error, clickhouse_cloud_api::Error::Api { status: actual_status, message } if actual_status == status && message == expected_message)
            );
        }
    }
}
