use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_json, setup};

// ===========================================================================
// Whoami
// ===========================================================================

#[tokio::test]
async fn whoami_get_parses_user_identity_with_bearer_auth() {
    let server = MockServer::start().await;
    let client = Client::with_bearer_token(server.uri(), "token");
    let wire = serde_json::json!({
        "actorType": "user",
        "userId": "00000000-0000-4000-8000-000000000001",
        "email": "user@example.com",
        "name": "Example User",
        "organizations": [
            {
                "organizationId": "00000000-0000-4000-8000-000000000002",
                "organizationName": "Example Org"
            }
        ]
    });
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .and(bearer_token("token"))
        .respond_with(ok_json(wire.clone()))
        .expect(1)
        .mount(&server)
        .await;
    let response = client.whoami_get().await.unwrap();
    assert_eq!(response.status, Some(200));
    let whoami = response.result.unwrap();
    let Whoami::WhoamiUser(user) = &whoami else {
        panic!("expected a user identity, got {whoami:?}");
    };
    assert_eq!(user.actor_type, Some(WhoamiUserActortype::User));
    assert_eq!(
        user.user_id,
        Some(uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap())
    );
    assert_eq!(user.email.as_deref(), Some("user@example.com"));
    assert_eq!(user.name.as_deref(), Some("Example User"));
    let organizations = user.organizations.as_ref().unwrap();
    assert_eq!(
        organizations[0].organization_id,
        Some(uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000002").unwrap())
    );
    assert_eq!(
        organizations[0].organization_name.as_deref(),
        Some("Example Org")
    );
    assert_eq!(serde_json::to_value(&whoami).unwrap(), wire);

    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].url.query().is_none());
    assert!(requests[0].body.is_empty());
}

#[tokio::test]
async fn whoami_get_parses_api_key_identity_with_basic_auth() {
    let (server, client) = setup().await;
    let wire = serde_json::json!({
        "actorType": "apiKey",
        "keyId": "key-1",
        "name": "ci key",
        "organizationId": "00000000-0000-4000-8000-000000000002"
    });
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(wire.clone()))
        .expect(1)
        .mount(&server)
        .await;
    let whoami = client.whoami_get().await.unwrap().result.unwrap();
    let Whoami::WhoamiApiKey(key) = &whoami else {
        panic!("expected an API key identity, got {whoami:?}");
    };
    assert_eq!(key.actor_type, Some(WhoamiApiKeyActortype::ApiKey));
    assert_eq!(key.key_id.as_deref(), Some("key-1"));
    assert_eq!(key.name.as_deref(), Some("ci key"));
    assert_eq!(
        key.organization_id,
        Some(uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000002").unwrap())
    );
    assert_eq!(serde_json::to_value(&whoami).unwrap(), wire);
}

#[tokio::test]
async fn whoami_get_keeps_unknown_actor_type_verbatim() {
    let (server, client) = setup().await;
    let wire = serde_json::json!({"actorType": "serviceAccount", "accountId": "sa-1"});
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(ok_json(wire.clone()))
        .expect(1)
        .mount(&server)
        .await;
    let whoami = client.whoami_get().await.unwrap().result.unwrap();
    assert_eq!(whoami, Whoami::Unknown(wire));
}

#[tokio::test]
async fn whoami_get_propagates_api_errors() {
    for (status, body, expected_message) in [
        (
            401,
            serde_json::json!({"status": 401, "error": "Unauthorized"}).to_string(),
            "Unauthorized",
        ),
        (500, "upstream failed".to_owned(), "upstream failed"),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = client.whoami_get().await.unwrap_err();
        assert!(
            matches!(error, clickhouse_cloud_api::Error::Api { status: actual_status, ref message } if actual_status == status && message == expected_message),
            "unexpected error: {error:?}"
        );
    }
}
