use wiremock::Mock;
use wiremock::matchers::{method, path, query_param};

use crate::{ok_json, setup};

// ===========================================================================
// Activities
// ===========================================================================

#[tokio::test]
async fn list_activities() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "act-1",
                "type": "SERVICE_START",
                "actorType": "user",
                "organizationId": "org-1",
                "serviceId": "svc-1"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.activity_get_list("org-1", None, None).await.unwrap();
    let activities = resp.result.unwrap();
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].id.as_deref(), Some("act-1"));
}

#[tokio::test]
async fn list_activities_with_date_filter() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .and(query_param("from_date", "2024-06-01"))
        .and(query_param("to_date", "2024-06-30"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .activity_get_list("org-1", Some("2024-06-01"), Some("2024-06-30"))
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn get_activity() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities/act-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "act-1",
            "type": "SERVICE_STOP",
            "actorType": "api"
        })))
        .mount(&s)
        .await;

    let resp = c.activity_get("org-1", "act-1").await.unwrap();
    let activity = resp.result.unwrap();
    assert_eq!(activity.id.as_deref(), Some("act-1"));
}

#[tokio::test]
async fn activity_list_with_only_from_date() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .and(query_param("from_date", "2024-06-01"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .activity_get_list("org-1", Some("2024-06-01"), None)
        .await
        .unwrap();
    let activities = resp.result.unwrap();
    assert!(activities.is_empty());
}
