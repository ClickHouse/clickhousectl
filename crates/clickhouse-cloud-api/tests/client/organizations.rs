use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, bearer_token, body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_json, setup};

// ===========================================================================
// Organizations
// ===========================================================================

#[tokio::test]
async fn list_organizations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(basic_auth("test-key", "test-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "req-123",
            "result": [
                {
                    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "name": "Test Org"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "test-key", "test-secret");
    let resp = client.organization_get_list(None, None).await.unwrap();
    assert_eq!(resp.status, Some(200));
    let orgs = resp.result.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name.as_deref(), Some("Test Org"));
}

#[tokio::test]
async fn get_organization() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-123"))
        .and(basic_auth("my-key", "my-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "name": "My Org",
                "createdAt": "2024-01-01T00:00:00Z"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "my-key", "my-secret");
    let resp = client.organization_get("org-123").await.unwrap();
    let org = resp.result.unwrap();
    assert_eq!(org.name.as_deref(), Some("My Org"));
}

#[tokio::test]
async fn get_organization_preserves_snapshot_capability() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1"))
        .respond_with(ok_json(serde_json::json!({
            "capabilities": {"snapshots": false}
        })))
        .mount(&server)
        .await;
    let org = client
        .organization_get("org-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(org.capabilities.unwrap().snapshots, Some(false));
}

#[tokio::test]
async fn get_active_balances_with_pagination() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activeBalances"))
        .and(query_param("limit", "25"))
        .and(query_param("offset", "50"))
        .respond_with(ok_json(serde_json::json!({
            "totalRemainingPrepaidCredits": 12.5,
            "prepaidBalances": [{
                "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "remainingPrepaidCredits": 12.5,
                "expirationDate": "2027-01-01T00:00:00Z"
            }]
        })))
        .mount(&s)
        .await;

    let balances = c
        .active_balances_get("org-1", Some(25), Some(50))
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(balances.total_remaining_prepaid_credits, Some(12.5));
    assert_eq!(balances.prepaid_balances.unwrap().len(), 1);
}

#[tokio::test]
async fn update_organization() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1"))
        .and(body_partial_json(
            serde_json::json!({"name": "Renamed Org"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            "name": "Renamed Org"
        })))
        .mount(&s)
        .await;

    let body = OrganizationPatchRequest {
        name: Some("Renamed Org".to_string()),
        ..Default::default()
    };
    let resp = c.organization_update("org-1", &body).await.unwrap();
    let org = resp.result.unwrap();
    assert_eq!(org.name.as_deref(), Some("Renamed Org"));
}

#[tokio::test]
async fn get_usage_cost_with_query_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/usageCost"))
        .and(query_param("from_date", "2024-01-01"))
        .and(query_param("to_date", "2024-01-31"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "costs": [],
                "grandTotalCHC": 50.25
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client
        .usage_cost_get("org-1", "2024-01-01", "2024-01-31", &[])
        .await
        .unwrap();
    let cost = resp.result.unwrap();
    assert_eq!(cost.grand_total_chc, Some(50.25));
}

#[tokio::test]
async fn get_prometheus_metrics() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# HELP ch_metric A metric\nch_metric{service=\"svc-1\"} 42\n"),
        )
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client
        .organization_prometheus_get("org-1", None)
        .await
        .unwrap();
    assert!(resp.contains("ch_metric"));
}

#[tokio::test]
async fn discover_organization_prometheus_targets() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus/discovery"))
        .and(query_param("filtered_metrics", "false"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "targets": ["api.clickhouse.cloud"],
                "labels": {
                    "__scheme__": "https",
                    "__metrics_path__": "/v1/organizations/org-1/services/svc-1/prometheus",
                    "__param_filtered_metrics": "false",
                    "clickhouse_org_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "clickhouse_service_id": "b1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "clickhouse_discovery_service_name": "analytics",
                    "__meta_clickhouse_region": "us-east-1",
                    "__meta_clickhouse_provider": "aws"
                }
            }])),
        )
        .mount(&s)
        .await;

    let groups = c
        .organization_prometheus_discovery_get("org-1", Some("false"))
        .await
        .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0]
            .labels
            .as_ref()
            .and_then(|labels| labels.scheme.as_deref()),
        Some("https")
    );
    let labels = groups[0].labels.as_ref().unwrap();
    assert_eq!(labels.meta_clickhouse_region.as_deref(), Some("us-east-1"));
    assert_eq!(labels.meta_clickhouse_provider.as_deref(), Some("aws"));
    assert_eq!(labels.additional_labels, None);
}

#[tokio::test]
#[allow(deprecated)]
async fn get_private_endpoint_config() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/privateEndpointConfig"))
        .and(query_param("cloud_provider", "aws"))
        .and(query_param("region_id", "us-east-1"))
        .respond_with(ok_json(serde_json::json!({
            "endpointServiceId": "com.amazonaws.vpce.us-east-1.vpce-svc-abc"
        })))
        .mount(&s)
        .await;

    let resp = c
        .organization_private_endpoint_config_get_list("org-1", "aws", "us-east-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(
        config.endpoint_service_id.as_deref(),
        Some("com.amazonaws.vpce.us-east-1.vpce-svc-abc")
    );
}

#[tokio::test]
async fn list_organizations_pagination_encodes_parameters_and_preserves_envelope() {
    let (server, client) = setup().await;
    let cursor = "opaque +/=&?雪";
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(basic_auth("key", "secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200.0, "requestId": "req-page", "result": [],
            "limit": 25, "nextCursor": cursor
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
            .organization_get_list(limit, cursor_arg)
            .await
            .unwrap();
        assert_eq!(response.status, Some(200));
        assert_eq!(response.request_id.as_deref(), Some("req-page"));
        assert_eq!(response.result, Some(vec![]));
        assert_eq!(response.error, None);
        assert_eq!(response.limit, Some(25));
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

// ===========================================================================
// Organization quotas
// ===========================================================================

#[tokio::test]
async fn list_quotas() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/quotas"))
        .respond_with(ok_json(serde_json::json!([
            {
                "quotaCode": "services-per-organization",
                "name": "Services per organization",
                "description": "Limits services.",
                "scope": "organization",
                "value": 20,
                "usage": 3,
                "adjustable": true
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.organization_quotas_get_list("org-1").await.unwrap();
    let quotas = resp.result.unwrap();
    assert_eq!(quotas.len(), 1);
    assert_eq!(
        quotas[0].quota_code,
        Some(OrganizationQuotaQuotacode::Services_per_organization)
    );
    assert_eq!(quotas[0].scope, Some(OrganizationQuotaScope::Organization));
    assert_eq!(quotas[0].usage, Some(3));
}

#[tokio::test]
async fn get_quota() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/quotas/replicas-per-warehouse",
        ))
        .respond_with(ok_json(serde_json::json!({
            "quotaCode": "replicas-per-warehouse",
            "name": "Replicas per warehouse",
            "description": "Limits each warehouse individually.",
            "scope": "warehouse",
            "value": 20,
            "adjustable": true
        })))
        .mount(&s)
        .await;

    let resp = c
        .organization_quota_get("org-1", "replicas-per-warehouse")
        .await
        .unwrap();
    let quota = resp.result.unwrap();
    assert_eq!(
        quota.quota_code,
        Some(OrganizationQuotaQuotacode::Replicas_per_warehouse)
    );
    assert_eq!(quota.scope, Some(OrganizationQuotaScope::Warehouse));
    assert_eq!(quota.usage, None);
}

#[tokio::test]
async fn list_organizations_returns_empty_vec() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.organization_get_list(None, None).await.unwrap();
    let orgs = resp.result.unwrap();
    assert!(orgs.is_empty());
}

#[tokio::test]
async fn usage_cost_with_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/usageCost"))
        .and(query_param("from_date", "2024-01-01"))
        .and(query_param("to_date", "2024-01-31"))
        .and(query_param("filter", "service_id=svc-1"))
        .respond_with(ok_json(serde_json::json!({
            "costs": [],
            "grandTotalCHC": 10.0
        })))
        .mount(&s)
        .await;

    let resp = c
        .usage_cost_get("org-1", "2024-01-01", "2024-01-31", &["service_id=svc-1"])
        .await
        .unwrap();
    let cost = resp.result.unwrap();
    assert_eq!(cost.grand_total_chc, Some(10.0));
}

#[tokio::test]
async fn organization_prometheus_with_filtered_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .and(query_param("filtered_metrics", "cpu,memory"))
        .respond_with(ResponseTemplate::new(200).set_body_string("cpu 42\nmemory 1024\n"))
        .mount(&s)
        .await;

    let resp = c
        .organization_prometheus_get("org-1", Some("cpu,memory"))
        .await
        .unwrap();
    assert!(resp.contains("cpu 42"));
}

#[tokio::test]
async fn organization_prometheus_without_filter() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# HELP metric\nmetric 1\n"))
        .mount(&s)
        .await;

    let resp = c.organization_prometheus_get("org-1", None).await.unwrap();
    assert!(resp.contains("metric"));
}

#[tokio::test]
async fn credit_balances_get_includes_trial_and_prepaid_balances() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/creditBalances"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "totalRemainingCredits": 12.5,
            "balances": [{"type": "trial", "remainingCredits": 2.5}, {"type": "prepaid", "remainingCredits": 10.0}]
        })))
        .expect(1).mount(&server).await;
    let result = client
        .credit_balances_get("org")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(result.total_remaining_credits, Some(12.5));
    let balances = result.balances.unwrap();
    assert_eq!(balances[0].r#type, Some(CreditBalanceType::Trial));
    assert_eq!(balances[1].r#type, Some(CreditBalanceType::Prepaid));
}

// ---------------------------------------------------------------------------
// Notification types
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_notification_types() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notificationTypes"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!([
            {
                "name": "service_backup_failed",
                "title": "Backup failed",
                "description": "A scheduled backup did not complete.",
                "category": "service",
                "severity": "warning",
                "resourceType": "service",
                "isConfigurable": true,
                "isAvailable": true
            },
            {
                "name": "billing_new_tier",
                "title": "New tier",
                "category": "billing",
                "severity": "page_someone",
                "resourceType": "organization",
                "isConfigurable": false,
                "isAvailable": false
            }
        ])))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c.notification_types_get_list("org-1").await.unwrap();
    let types = resp.result.unwrap();
    assert_eq!(types.len(), 2);
    assert_eq!(types[0].name.as_deref(), Some("service_backup_failed"));
    assert_eq!(types[0].title.as_deref(), Some("Backup failed"));
    assert_eq!(
        types[0].description.as_deref(),
        Some("A scheduled backup did not complete.")
    );
    assert_eq!(types[0].category.as_deref(), Some("service"));
    assert_eq!(types[0].severity, Some(NotificationTypeSeverity::Warning));
    assert_eq!(types[0].resource_type.as_deref(), Some("service"));
    assert_eq!(types[0].is_configurable, Some(true));
    assert_eq!(types[0].is_available, Some(true));
    assert_eq!(types[1].description, None);
    assert_eq!(
        types[1].severity,
        Some(NotificationTypeSeverity::Unknown(
            "page_someone".to_string()
        ))
    );
    assert_eq!(types[1].is_available, Some(false));
}

#[tokio::test]
async fn notification_types_get_list_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notificationTypes"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "Invalid organization"
        })))
        .mount(&s)
        .await;

    let err = c.notification_types_get_list("org-1").await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 400);
            assert_eq!(message, "Invalid organization");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn notification_get_list_preserves_filters_cursor_and_envelope() {
    let (server, client) = setup().await;
    let cursor = "opaque +/=&?é";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notifications"))
        .and(basic_auth("key", "secret"))
        .and(query_param("limit", "25"))
        .and(query_param("cursor", cursor))
        .and(query_param("severity", "warning"))
        .and(query_param("type", "service_backup_failed"))
        .and(query_param("resource_id", "service-1"))
        .and(query_param("resource_type", "service"))
        .and(query_param("from_date", "2026-10-01T00:00:00+02:00"))
        .and(query_param("to_date", "2026-10-08T00:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "notification-page", "limit": 25, "nextCursor": "",
            "result": [{
                "id": "1b7e0a94-2d0f-4c3a-8f1e-6c9b2d5a4e7f",
                "type": "service_backup_failed", "severity": "warning", "resourceType": "service",
                "resourceId": "service-1", "resourceName": "analytics", "createdAt": "2026-10-07T09:30:00Z",
                "title": "Backup failed", "description": "Review backup settings.", "future": 42
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let response = client
        .notification_get_list(
            "org-1",
            Some(25),
            Some(cursor),
            Some("warning"),
            Some("service_backup_failed"),
            Some("service-1"),
            Some("service"),
            Some("2026-10-01T00:00:00+02:00"),
            Some("2026-10-08T00:00:00Z"),
        )
        .await
        .unwrap();
    assert_eq!(response.request_id.as_deref(), Some("notification-page"));
    assert_eq!(response.limit, Some(25));
    assert_eq!(response.next_cursor.as_deref(), Some(""));
    let notifications = response.result.unwrap();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].title.as_deref(), Some("Backup failed"));
    assert_eq!(notifications[0].resource_name.as_deref(), Some("analytics"));
    assert!(notifications[0].created_at.is_some());
}

#[tokio::test]
async fn notification_get_list_tolerates_sparse_null_and_empty_pages() {
    for body in [
        serde_json::json!({"result": [{}, {"id": null, "createdAt": null}], "nextCursor": null}),
        serde_json::json!({"result": []}),
        serde_json::json!({"result": null}),
        serde_json::json!({}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/notifications"))
            .and(bearer_token("token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::with_bearer_token(server.uri(), "token");
        let response = client
            .notification_get_list("org-1", None, None, None, None, None, None, None, None)
            .await
            .unwrap();
        assert_eq!(response.next_cursor, None);
        if body["result"].is_array() {
            assert_eq!(
                response.result.as_ref().unwrap().len(),
                body["result"].as_array().unwrap().len()
            );
        } else {
            assert_eq!(response.result, None);
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests[0].url.query(), None);
    }
}

#[tokio::test]
async fn notification_get_list_surfaces_api_errors() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notifications"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": "invalid cursor"})),
        )
        .mount(&server)
        .await;
    let error = client
        .notification_get_list(
            "org-1",
            None,
            Some("bad"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, clickhouse_cloud_api::Error::Api { status: 400, message } if message == "invalid cursor")
    );
}
