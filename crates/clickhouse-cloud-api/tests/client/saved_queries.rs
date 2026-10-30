use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, bearer_token, body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{created_json, ok_empty, ok_json, setup};

#[tokio::test]
async fn saved_query_create_and_update_send_complete_requests() {
    let (server, client) = setup().await;
    let collection = "/v1/organizations/org-1/services/svc-1/saved-queries";
    let minimal = serde_json::json!({
        "name": "daily total", "sql": "SELECT count() FROM events", "database": "default"
    });
    let maximal = serde_json::json!({
        "name": "filtered total", "sql": "SELECT count() FROM events WHERE kind = {kind:String}",
        "database": "analytics", "parameters": {"kind": "page view"}
    });
    for body in [minimal, maximal] {
        let request: PublicSavedQueryRequest = serde_json::from_value(body.clone()).unwrap();
        let mut returned = body.clone();
        returned["id"] = serde_json::json!("00000000-0000-4000-8000-000000000003");
        if returned.get("parameters").is_none() {
            returned["parameters"] = serde_json::json!({});
        }
        Mock::given(method("POST"))
            .and(path(collection))
            .and(basic_auth("key", "secret"))
            .and(body_json(body.clone()))
            .respond_with(created_json(returned.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .saved_query_create("org-1", "svc-1", &request)
            .await
            .unwrap();
        assert_eq!(response.status, Some(201));
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            returned
        );

        Mock::given(method("PUT"))
            .and(path(format!("{collection}/query-1")))
            .and(basic_auth("key", "secret"))
            .and(body_json(body))
            .respond_with(ok_json(returned.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .saved_query_update("org-1", "svc-1", "query-1", &request)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            returned
        );
    }
    for request in server.received_requests().await.unwrap() {
        assert!(request.url.query().is_none());
    }
}

#[tokio::test]
async fn saved_query_get_and_delete_use_query_path() {
    let (server, client) = setup().await;
    let query_path = "/v1/organizations/org-1/services/svc-1/saved-queries/query-1";
    Mock::given(method("GET"))
        .and(path(query_path))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "id": "00000000-0000-4000-8000-000000000003",
            "name": "example",
            "sql": "SELECT {n:UInt8}",
            "database": "default",
            "parameters": {"n": "1"}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let query = client
        .saved_query_get("org-1", "svc-1", "query-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(
        query.id,
        Some(uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000003").unwrap())
    );
    assert_eq!(query.name.as_deref(), Some("example"));
    assert_eq!(query.sql.as_deref(), Some("SELECT {n:UInt8}"));
    assert_eq!(query.database.as_deref(), Some("default"));
    assert_eq!(
        query.parameters,
        Some(std::collections::BTreeMap::from([(
            "n".to_string(),
            "1".to_string()
        )]))
    );
    Mock::given(method("DELETE"))
        .and(path(query_path))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_empty())
        .expect(1)
        .mount(&server)
        .await;
    let response = client
        .saved_query_delete("org-1", "svc-1", "query-1")
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
async fn saved_query_list_encodes_cursor_and_reads_envelope_pagination() {
    for (cursor, limit) in [
        (None, None),
        (Some("next+/=&? page"), None),
        (None, Some(25)),
        (Some("next+/=&? page"), Some(25)),
    ] {
        let (server, client) = setup().await;
        let items = serde_json::json!([
            {"id": "00000000-0000-4000-8000-000000000003", "name": "example", "database": "default"}
        ]);
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/services/svc-1/saved-queries"))
            .and(basic_auth("key", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": 200,
                "requestId": "req-test",
                "result": items,
                "limit": 25,
                "totalCount": 2,
                "nextCursor": "another-page"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .saved_query_list("org-1", "svc-1", cursor, limit)
            .await
            .unwrap();
        assert_eq!(response.limit, Some(25));
        assert_eq!(response.total_count, Some(2));
        assert_eq!(response.next_cursor.as_deref(), Some("another-page"));
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            items
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
async fn saved_query_list_last_page_has_null_next_cursor() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/services/svc/saved-queries"))
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
        .saved_query_list("org", "svc", None, None)
        .await
        .unwrap();
    assert_eq!(response.result, Some(vec![]));
    assert_eq!(response.next_cursor, None);
    assert_eq!(response.total_count, Some(0));
}

#[tokio::test]
async fn saved_query_reads_use_bearer_auth() {
    let server = MockServer::start().await;
    let client = Client::with_bearer_token(server.uri(), "token");
    for (query_path, result) in [
        (
            "/v1/organizations/org/services/svc/saved-queries",
            serde_json::json!([]),
        ),
        (
            "/v1/organizations/org/services/svc/saved-queries/query",
            serde_json::json!({}),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(query_path))
            .and(bearer_token("token"))
            .respond_with(ok_json(result))
            .expect(1)
            .mount(&server)
            .await;
    }
    client
        .saved_query_list("org", "svc", None, None)
        .await
        .unwrap();
    client.saved_query_get("org", "svc", "query").await.unwrap();
}

#[tokio::test]
async fn saved_query_methods_propagate_api_errors() {
    for (status, body, expected_message) in [
        (
            404,
            serde_json::json!({"status": 404, "error": "Saved query not found"}).to_string(),
            "Saved query not found",
        ),
        (
            409,
            serde_json::json!({"status": 409, "error": "name already exists"}).to_string(),
            "name already exists",
        ),
        (500, "upstream failed".to_owned(), "upstream failed"),
    ] {
        let (server, client) = setup().await;
        Mock::given(basic_auth("key", "secret"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(5)
            .mount(&server)
            .await;
        let request = PublicSavedQueryRequest {
            name: "example".into(),
            sql: "SELECT 1".into(),
            database: "default".into(),
            parameters: None,
        };
        let errors = [
            client
                .saved_query_create("org", "svc", &request)
                .await
                .unwrap_err(),
            client
                .saved_query_get("org", "svc", "query")
                .await
                .unwrap_err(),
            client
                .saved_query_list("org", "svc", None, None)
                .await
                .unwrap_err(),
            client
                .saved_query_update("org", "svc", "query", &request)
                .await
                .unwrap_err(),
            client
                .saved_query_delete("org", "svc", "query")
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
