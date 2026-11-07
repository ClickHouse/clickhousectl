use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, body_json, method, path, query_param};
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

#[tokio::test]
async fn list_member_notification_settings_with_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .and(basic_auth("key", "secret"))
        .and(query_param("limit", "50"))
        .and(query_param("cursor", "next-1"))
        .and(query_param("type", "service_backup_failed"))
        .and(query_param("resource_id", "svc-1"))
        .and(query_param("resource_type", "service"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [{
                "type": "service_backup_failed",
                "resourceType": "service",
                "resourceId": "svc-1",
                "channelType": "email",
                "effect": "disabled"
            }],
            "limit": 50,
            "nextCursor": null
        })))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c
        .member_notification_settings_get_list(
            "org-1",
            "user-1",
            Some(50),
            Some("next-1"),
            Some("service_backup_failed"),
            Some("svc-1"),
            Some("service"),
        )
        .await
        .unwrap();
    assert_eq!(resp.limit, Some(50));
    assert_eq!(resp.next_cursor, None);
    let settings = resp.result.unwrap();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0].channel_type.as_deref(), Some("email"));
    assert_eq!(settings[0].effect.as_deref(), Some("disabled"));
}

#[tokio::test]
async fn list_member_notification_settings_sends_no_query_by_default() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .expect(1)
        .mount(&s)
        .await;

    c.member_notification_settings_get_list("org-1", "user-1", None, None, None, None, None)
        .await
        .unwrap();
    let requests = s.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn update_member_notification_settings() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .and(basic_auth("key", "secret"))
        .and(body_json(serde_json::json!({
            "rules": [
                {
                    "effect": "disabled",
                    "notifications": ["service_backup_failed"],
                    "resources": ["svc-1"],
                    "channels": ["email"]
                },
                {
                    "effect": "inherit",
                    "notifications": ["billing_credit_low"],
                    "resources": ["org-1"]
                }
            ]
        })))
        .respond_with(ok_json(serde_json::json!({
            "settings": [{
                "type": "service_backup_failed",
                "resourceType": "service",
                "resourceId": "svc-1",
                "channelType": "email",
                "effect": "disabled"
            }]
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = MemberNotificationSettingsPatchRequest {
        rules: Some(vec![
            NotificationSettingRule {
                effect: "disabled".to_string(),
                notifications: vec!["service_backup_failed".to_string()],
                resources: vec!["svc-1".to_string()],
                channels: Some(vec!["email".to_string()]),
            },
            NotificationSettingRule {
                effect: "inherit".to_string(),
                notifications: vec!["billing_credit_low".to_string()],
                resources: vec!["org-1".to_string()],
                channels: None,
            },
        ]),
    };
    let settings = c
        .member_notification_settings_update("org-1", "user-1", &body)
        .await
        .unwrap()
        .result
        .unwrap()
        .settings
        .unwrap();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0].r#type.as_deref(), Some("service_backup_failed"));
}

#[tokio::test]
async fn update_member_notification_settings_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": "unknown notification type"})),
        )
        .expect(1)
        .mount(&s)
        .await;

    let error = c
        .member_notification_settings_update(
            "org-1",
            "user-1",
            &MemberNotificationSettingsPatchRequest::default(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, clickhouse_cloud_api::Error::Api { status: 400, message } if message == "unknown notification type")
    );
}
