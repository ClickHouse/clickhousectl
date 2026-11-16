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

// ===========================================================================
// Member notification settings
// ===========================================================================

#[tokio::test]
async fn member_notification_settings_get_list_preserves_filters_cursor_and_envelope() {
    let (s, c) = setup().await;
    let cursor = "opaque +/=&?é";

    Mock::given(wiremock::matchers::method("GET"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .and(wiremock::matchers::basic_auth("key", "secret"))
        .and(wiremock::matchers::query_param("limit", "50"))
        .and(wiremock::matchers::query_param("cursor", cursor))
        .and(wiremock::matchers::query_param(
            "type",
            "service_backup_failed",
        ))
        .and(wiremock::matchers::query_param("resource_id", "service-1"))
        .and(wiremock::matchers::query_param("resource_type", "service"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "settings-page", "limit": 50, "nextCursor": "next",
            "result": [{
                "type": "service_backup_failed", "resourceType": "service",
                "resourceId": "service-1", "channelType": "email", "effect": "disabled",
                "future": 1
            }]
        })))
        .expect(1)
        .mount(&s)
        .await;

    let response = c
        .member_notification_settings_get_list(
            "org-1",
            "user-1",
            Some(50),
            Some(cursor),
            Some("service_backup_failed"),
            Some("service-1"),
            Some("service"),
        )
        .await
        .unwrap();
    assert_eq!(response.limit, Some(50));
    assert_eq!(response.next_cursor.as_deref(), Some("next"));
    let settings = response.result.unwrap();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0].r#type.as_deref(), Some("service_backup_failed"));
    assert_eq!(settings[0].resource_type.as_deref(), Some("service"));
    assert_eq!(settings[0].resource_id.as_deref(), Some("service-1"));
    assert_eq!(settings[0].channel_type.as_deref(), Some("email"));
    assert_eq!(settings[0].effect.as_deref(), Some("disabled"));
}

#[tokio::test]
async fn member_notification_settings_get_list_sends_no_query_by_default() {
    let (s, c) = setup().await;

    Mock::given(wiremock::matchers::method("GET"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [], "nextCursor": null
        })))
        .expect(1)
        .mount(&s)
        .await;

    let response = c
        .member_notification_settings_get_list("org-1", "user-1", None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(response.next_cursor, None);
    assert_eq!(response.result, Some(vec![]));
    let requests = s.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn member_notification_settings_update_sends_rules_in_order() {
    let (s, c) = setup().await;

    Mock::given(wiremock::matchers::method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .and(wiremock::matchers::basic_auth("key", "secret"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "rules": [
                {
                    "effect": "disabled",
                    "notifications": ["service_backup_failed"],
                    "resources": ["service-1"],
                    "channels": ["email"]
                },
                {
                    "effect": "inherit",
                    "notifications": ["billing_new_tier"],
                    "resources": ["org-1"]
                }
            ]
        })))
        .respond_with(ok_json(serde_json::json!({
            "settings": [{
                "type": "service_backup_failed", "resourceType": "service",
                "resourceId": "service-1", "channelType": "email", "effect": "disabled"
            }]
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = MemberNotificationSettingsPatchRequest {
        rules: Some(vec![
            NotificationSettingRule {
                effect: "disabled".into(),
                notifications: vec!["service_backup_failed".into()],
                resources: vec!["service-1".into()],
                channels: Some(vec!["email".into()]),
            },
            NotificationSettingRule {
                effect: "inherit".into(),
                notifications: vec!["billing_new_tier".into()],
                resources: vec!["org-1".into()],
                channels: None,
            },
        ]),
    };
    let result = c
        .member_notification_settings_update("org-1", "user-1", &body)
        .await
        .unwrap()
        .result
        .unwrap();
    let settings = result.settings.unwrap();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0].effect.as_deref(), Some("disabled"));
}

#[tokio::test]
async fn member_notification_settings_update_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(wiremock::matchers::method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/members/user-1/notificationSettings",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "unknown notification type"
        })))
        .mount(&s)
        .await;

    let err = c
        .member_notification_settings_update(
            "org-1",
            "user-1",
            &MemberNotificationSettingsPatchRequest::default(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, clickhouse_cloud_api::Error::Api { status: 400, message } if message == "unknown notification type")
    );
}
