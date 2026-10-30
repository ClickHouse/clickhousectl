use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// Invitations
// ===========================================================================

#[tokio::test]
async fn list_invitations() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/invitations"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "email": "alice@example.com",
                "role": "developer"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.invitation_get_list("org-1").await.unwrap();
    let invitations = resp.result.unwrap();
    assert_eq!(invitations.len(), 1);
    assert_eq!(invitations[0].email.as_deref(), Some("alice@example.com"));
}

#[tokio::test]
async fn create_invitation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/invitations"))
        .and(body_partial_json(
            serde_json::json!({"email": "newuser@example.com"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
                "email": "newuser@example.com",
                "role": "developer"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = InvitationPostRequest {
        email: "newuser@example.com".to_string(),
        #[cfg(feature = "deprecated-fields")]
        role: Some(InvitationPostRequestRole::Developer),
        ..Default::default()
    };
    let resp = client.invitation_create("org-1", &body).await.unwrap();
    let inv = resp.result.unwrap();
    assert_eq!(inv.email.as_deref(), Some("newuser@example.com"));
}

#[tokio::test]
async fn get_invitation() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/invitations/inv-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
            "email": "bob@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let resp = c.invitation_get("org-1", "inv-1").await.unwrap();
    let inv = resp.result.unwrap();
    assert_eq!(inv.email.as_deref(), Some("bob@example.com"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(inv.role, Some(InvitationRole::Admin));
}

#[tokio::test]
async fn delete_invitation() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/invitations/inv-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.invitation_delete("org-1", "inv-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}
