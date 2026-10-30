use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// Members
// ===========================================================================

#[tokio::test]
async fn list_members() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "userId": "user-1",
                    "name": "Alice",
                    "email": "alice@example.com",
                    "role": "admin"
                },
                {
                    "userId": "user-2",
                    "name": "Bob",
                    "email": "bob@example.com",
                    "role": "developer"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.member_get_list("org-1").await.unwrap();
    let members = resp.result.unwrap();
    assert_eq!(members.len(), 2);
    #[cfg(feature = "deprecated-fields")]
    {
        assert_eq!(members[0].role, Some(MemberRole::Admin));
        assert_eq!(members[1].role, Some(MemberRole::Developer));
    }
}

#[tokio::test]
async fn get_member() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .respond_with(ok_json(serde_json::json!({
            "userId": "user-1",
            "name": "Alice",
            "email": "alice@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let resp = c.member_get("org-1", "user-1").await.unwrap();
    let member = resp.result.unwrap();
    assert_eq!(member.name.as_deref(), Some("Alice"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(member.role, Some(MemberRole::Admin));
}

#[tokio::test]
async fn update_member() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .respond_with(ok_json(serde_json::json!({
            "userId": "user-1",
            "name": "Alice",
            "email": "alice@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let body = MemberPatchRequest {
        #[cfg(feature = "deprecated-fields")]
        role: Some(MemberPatchRequestRole::Admin),
        ..Default::default()
    };
    let resp = c.member_update("org-1", "user-1", &body).await.unwrap();
    let member = resp.result.unwrap();
    assert_eq!(member.email.as_deref(), Some("alice@example.com"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(member.role, Some(MemberRole::Admin));
}

#[tokio::test]
async fn delete_member() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.member_delete("org-1", "user-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}
