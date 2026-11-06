use clickhouse_cloud_api::models::*;
use wiremock::matchers::{basic_auth, body_json, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

const COLLECTION: &str = "/v1/organizations/org-1/services/svc-1/jwtProviders";

#[tokio::test]
async fn jwt_provider_create_sends_minimal_and_maximal_bodies() {
    let minimal = serde_json::json!({
        "name": "my-oidc-provider",
        "issuer": "https://auth.example.com",
        "audience": "my-clickhouse-service",
        "jwksUrl": "https://auth.example.com/.well-known/jwks.json"
    });
    let mut maximal = minimal.clone();
    maximal["rolesClaim"] = serde_json::json!("clickhouse:roles");
    for body in [minimal, maximal] {
        let (server, client) = setup().await;
        let request: JwtProviderPostRequest = serde_json::from_value(body.clone()).unwrap();
        Mock::given(method("POST"))
            .and(path(COLLECTION))
            .and(basic_auth("key", "secret"))
            .and(body_json(body.clone()))
            .respond_with(ok_json(body.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .jwt_provider_create("org-1", "svc-1", &request)
            .await
            .unwrap();
        assert_eq!(response.status, Some(200));
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            body
        );
    }
}

#[tokio::test]
async fn jwt_provider_update_sends_minimal_and_maximal_bodies_to_the_named_provider() {
    let minimal = serde_json::json!({
        "issuer": "https://auth.example.com",
        "audience": "my-clickhouse-service",
        "jwksUrl": "https://auth.example.com/.well-known/jwks.json"
    });
    let mut maximal = minimal.clone();
    maximal["rolesClaim"] = serde_json::json!("clickhouse:roles");
    for body in [minimal, maximal] {
        let (server, client) = setup().await;
        let request: JwtProviderPutRequest = serde_json::from_value(body.clone()).unwrap();
        let mut returned = body.clone();
        returned["name"] = serde_json::json!("my-oidc-provider");
        Mock::given(method("PUT"))
            .and(path(format!("{COLLECTION}/my-oidc-provider")))
            .and(basic_auth("key", "secret"))
            .and(body_json(body))
            .respond_with(ok_json(returned.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .jwt_provider_update("org-1", "svc-1", "my-oidc-provider", &request)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            returned
        );
    }
}

#[tokio::test]
async fn jwt_provider_reads_and_delete_send_no_body() {
    let (server, client) = setup().await;
    let provider = serde_json::json!({
        "name": "my-oidc-provider",
        "issuer": "https://auth.example.com",
        "audience": "my-clickhouse-service",
        "jwksUrl": "https://auth.example.com/.well-known/jwks.json"
    });
    let item_path = format!("{COLLECTION}/my-oidc-provider");
    Mock::given(method("GET"))
        .and(path(COLLECTION))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!([provider.clone()])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(item_path.as_str()))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(provider.clone()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(item_path.as_str()))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_empty())
        .expect(1)
        .mount(&server)
        .await;

    let listed = client
        .jwt_provider_get_list("org-1", "svc-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].roles_claim, None);
    assert_eq!(serde_json::to_value(&listed[0]).unwrap(), provider);

    let fetched = client
        .jwt_provider_get("org-1", "svc-1", "my-oidc-provider")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(fetched.name.as_deref(), Some("my-oidc-provider"));

    let deleted = client
        .jwt_provider_delete("org-1", "svc-1", "my-oidc-provider")
        .await
        .unwrap();
    assert_eq!(deleted.status, Some(200));
    assert_eq!(deleted.request_id.as_deref(), Some("req-test"));
    assert!(deleted.result.is_none());

    for request in server.received_requests().await.unwrap() {
        assert!(request.body.is_empty());
        assert!(request.url.query().is_none());
    }
}

#[tokio::test]
async fn jwt_provider_methods_propagate_api_errors() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path(COLLECTION))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "status": 409, "error": "conflicting provider"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{COLLECTION}/missing")))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let request = JwtProviderPostRequest {
        name: "my-oidc-provider".into(),
        issuer: "https://auth.example.com".into(),
        audience: "my-clickhouse-service".into(),
        jwks_url: "https://auth.example.com/.well-known/jwks.json".into(),
        roles_claim: None,
    };
    match client.jwt_provider_create("org-1", "svc-1", &request).await {
        Err(clickhouse_cloud_api::Error::Api { status, message }) => {
            assert_eq!(status, 409);
            assert_eq!(message, "conflicting provider");
        }
        other => panic!("expected API error, got {other:?}"),
    }
    match client.jwt_provider_get("org-1", "svc-1", "missing").await {
        Err(clickhouse_cloud_api::Error::Api { status, message }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "not found");
        }
        other => panic!("expected API error, got {other:?}"),
    }
}
