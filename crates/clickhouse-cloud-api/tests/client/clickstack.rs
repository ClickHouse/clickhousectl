use clickhouse_cloud_api::models::*;
use wiremock::matchers::{basic_auth, body_json, body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// ClickStack: Alerts
// ===========================================================================

#[tokio::test]
async fn list_alerts() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts",
        ))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "alert-1",
                "name": "High CPU"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_alerts("org-1", "svc-1", None, None)
        .await
        .unwrap();
    let alerts = resp.result.unwrap();
    assert_eq!(alerts.len(), 1);
}

#[tokio::test]
async fn create_alert() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts",
        ))
        .and(body_partial_json(serde_json::json!({"name": "New Alert"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1",
            "name": "New Alert"
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateAlertRequest {
        name: Some("New Alert".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_alert("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn create_inline_alert_sends_chart_config_and_reads_it_back() {
    let (s, c) = setup().await;
    let chart_config = serde_json::json!({
        "displayType": "number", "configType": "sql", "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM otel_logs", "name": "Error count"
    });

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts",
        ))
        .and(body_json(serde_json::json!({
            "source": "inline", "chartConfig": chart_config, "threshold": 5.0,
            "interval": "5m", "thresholdType": "above", "displayName": "Errors",
            "tags": ["p1"]
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1", "source": "inline", "chartConfig": chart_config,
            "displayName": "Errors", "tags": ["p1"]
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateAlertRequest {
        source: Some(ClickStackCreateAlertRequestSource::Inline),
        chart_config: Some(
            ClickStackAlertChartConfig::ClickStackAlertNumberChartConfig(
                ClickStackAlertNumberChartConfig::ClickStackAlertNumberRawSqlChartConfig(
                    ClickStackAlertNumberRawSqlChartConfig {
                        connection_id: "conn-1".into(),
                        sql_template: "SELECT count() FROM otel_logs".into(),
                        name: Some("Error count".into()),
                        ..Default::default()
                    },
                ),
            ),
        ),
        threshold: 5.0,
        interval: ClickStackCreateAlertRequestInterval::_5m,
        display_name: Some("Errors".into()),
        tags: Some(vec!["p1".into()]),
        ..Default::default()
    };
    let alert = c
        .click_stack_create_alert("org-1", "svc-1", &body)
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(alert.source, Some(ClickStackAlertResponseSource::Inline));
    assert_eq!(alert.display_name.as_deref(), Some("Errors"));
    assert!(matches!(
        alert.chart_config,
        Some(
            ClickStackAlertChartConfigResponse::ClickStackAlertNumberChartConfig(
                ClickStackAlertNumberChartConfigResponse::ClickStackAlertNumberRawSqlChartConfig(_)
            )
        )
    ));
}

#[tokio::test]
async fn get_alert() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1",
            "name": "My Alert"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_alert("org-1", "svc-1", "alert-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_alert() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "Updated Alert"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1",
            "name": "Updated Alert"
        })))
        .mount(&s)
        .await;

    let body = ClickStackUpdateAlertRequest {
        name: Some("Updated Alert".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_alert("org-1", "svc-1", "alert-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_alert() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_alert("org-1", "svc-1", "alert-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Saved Searches
// ===========================================================================

#[tokio::test]
async fn list_saved_searches() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches",
        ))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "search-1",
                "name": "Production Errors",
                "sourceId": "source-1"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_saved_searches("org-1", "svc-1", None, None)
        .await
        .unwrap();
    let searches = resp.result.unwrap();
    assert_eq!(searches.len(), 1);
    assert_eq!(searches[0].source_id.as_deref(), Some("source-1"));
}

#[tokio::test]
async fn create_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Production Errors",
            "sourceId": "source-1"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "search-1",
            "name": "Production Errors",
            "sourceId": "source-1"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSavedSearchInput {
        name: "Production Errors".to_string(),
        source_id: "source-1".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_saved_search("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().id.as_deref(), Some("search-1"));
}

#[tokio::test]
async fn get_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "search-1",
            "name": "Production Errors",
            "sourceId": "source-1"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_saved_search("org-1", "svc-1", "search-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Updated Search",
            "sourceId": "source-1"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "search-1",
            "name": "Updated Search",
            "sourceId": "source-1"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSavedSearchInput {
        name: "Updated Search".to_string(),
        source_id: "source-1".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_saved_search("org-1", "svc-1", "search-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_saved_search("org-1", "svc-1", "search-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Dashboards
// ===========================================================================

#[tokio::test]
async fn list_dashboards() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards",
        ))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "dash-1",
                "name": "Overview"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_dashboards("org-1", "svc-1")
        .await
        .unwrap();
    let dashboards = resp.result.unwrap();
    assert_eq!(dashboards.len(), 1);
}

#[tokio::test]
async fn create_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "New Dashboard", "tiles": []}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "dash-new",
            "name": "New Dashboard"
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateDashboardRequest {
        name: "New Dashboard".to_string(),
        tiles: vec![],
        ..Default::default()
    };
    let resp = c
        .click_stack_create_dashboard("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn get_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "dash-1",
            "name": "My Dashboard"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_dashboard("org-1", "svc-1", "dash-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "Updated Dashboard", "tiles": []}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "dash-1",
            "name": "Updated Dashboard"
        })))
        .mount(&s)
        .await;

    let body = ClickStackUpdateDashboardRequest {
        name: "Updated Dashboard".to_string(),
        tiles: vec![],
        ..Default::default()
    };
    let resp = c
        .click_stack_update_dashboard("org-1", "svc-1", "dash-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_dashboard("org-1", "svc-1", "dash-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Sources & Webhooks
// ===========================================================================

#[tokio::test]
async fn list_sources() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.click_stack_list_sources("org-1", "svc-1").await.unwrap();
    let sources = resp.result.unwrap();
    assert_eq!(sources.len(), 0);
}

#[tokio::test]
async fn create_source() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources",
        ))
        .and(body_partial_json(
            serde_json::json!({"kind": "promql", "name": "Prometheus Metrics"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "source-1",
            "kind": "promql",
            "name": "Prometheus Metrics",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSource::ClickStackPromqlSource(ClickStackPromqlSource {
        name: "Prometheus Metrics".to_string(),
        kind: ClickStackPromqlSourceKind::Promql,
        connection: "conn-1".to_string(),
        from: ClickStackSourceFrom {
            database_name: "default".to_string(),
            table_name: "metrics".to_string(),
        },
        timestamp_value_expression: "timestamp".to_string(),
        ..Default::default()
    });
    let resp = c
        .click_stack_create_source("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn get_source() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources/source-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "source-1",
            "kind": "promql",
            "name": "My Source",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_source("org-1", "svc-1", "source-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_source() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources/source-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "Updated Source"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "source-1",
            "kind": "promql",
            "name": "Updated Source",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSource::ClickStackPromqlSource(ClickStackPromqlSource {
        name: "Updated Source".to_string(),
        kind: ClickStackPromqlSourceKind::Promql,
        connection: "conn-1".to_string(),
        from: ClickStackSourceFrom {
            database_name: "default".to_string(),
            table_name: "metrics".to_string(),
        },
        timestamp_value_expression: "timestamp".to_string(),
        ..Default::default()
    });
    let resp = c
        .click_stack_update_source("org-1", "svc-1", "source-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_source() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources/source-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_source("org-1", "svc-1", "source-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn list_webhooks() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_webhooks("org-1", "svc-1", None, None)
        .await
        .unwrap();
    let webhooks = resp.result.unwrap();
    assert_eq!(webhooks.len(), 0);
}

// ===========================================================================
// ClickStack: Roles
// ===========================================================================

#[tokio::test]
async fn list_roles() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.click_stack_list_roles("org-1", "svc-1").await.unwrap();
    let roles = resp.result.unwrap();
    assert_eq!(roles.len(), 0);
}

#[tokio::test]
async fn create_role() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Deploy Bot",
            "permissions": [{
                "action": "read",
                "subject": "Dashboard",
                "integration": "mongodb",
                "conditions": { "team": "team-1" }
            }]
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "role-1",
            "name": "Deploy Bot",
            "isPredefined": false,
            "permissions": [{
                "action": "read",
                "subject": "Dashboard",
                "integration": "mongodb",
                "conditions": { "team": "team-1" }
            }]
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateRoleRequest {
        name: "Deploy Bot".to_string(),
        permissions: vec![ClickStackCASLPermission::ClickStackMongoDBCASLPermission(
            ClickStackMongoDBCASLPermission {
                action: ClickStackCASLPermissionAction::Read,
                subject: ClickStackCASLPermissionSubject::Dashboard,
                conditions: Some(
                    [(
                        "team".to_string(),
                        ClickStackMongoDBCASLPermissionConditionValue::String("team-1".to_string()),
                    )]
                    .into(),
                ),
                ..Default::default()
            },
        )],
        ..Default::default()
    };
    let resp = c
        .click_stack_create_role("org-1", "svc-1", &body)
        .await
        .unwrap();
    let role = resp.result.unwrap();
    assert_eq!(role.id.as_deref(), Some("role-1"));
    assert_eq!(role.name.as_deref(), Some("Deploy Bot"));
    assert_eq!(role.is_predefined, Some(false));
    assert!(matches!(
        role.permissions.as_deref(),
        Some([ClickStackCASLPermissionResponse::ClickStackMongoDBCASLPermission(_)])
    ));
}

#[tokio::test]
async fn get_role() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles/role-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "role-1",
            "name": "Read Only",
            "isPredefined": true,
            "permissions": [{ "action": "read", "subject": "Dashboard", "integration": "mongodb" }]
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_role("org-1", "svc-1", "role-1")
        .await
        .unwrap();
    let role = resp.result.unwrap();
    assert_eq!(role.id.as_deref(), Some("role-1"));
    assert_eq!(role.is_predefined, Some(true));
}

#[tokio::test]
async fn update_role() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles/role-1",
        ))
        .and(body_partial_json(serde_json::json!({
            "permissions": [{ "action": "manage", "subject": "all", "integration": "clickhouse" }]
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "role-1",
            "name": "Deploy Bot",
            "isPredefined": false,
            "permissions": [{ "action": "manage", "subject": "all", "integration": "clickhouse" }]
        })))
        .mount(&s)
        .await;

    let body = ClickStackUpdateRoleRequest {
        permissions: vec![
            ClickStackCASLPermission::ClickStackClickHouseCASLPermission(
                ClickStackClickHouseCASLPermission {
                    action: ClickStackCASLPermissionAction::Manage,
                    subject: ClickStackCASLPermissionSubject::All,
                    ..Default::default()
                },
            ),
        ],
        ..Default::default()
    };
    let resp = c
        .click_stack_update_role("org-1", "svc-1", "role-1", &body)
        .await
        .unwrap();
    let role = resp.result.unwrap();
    let permissions = role.permissions.unwrap_or_default();
    assert_eq!(permissions.len(), 1);
    let ClickStackCASLPermissionResponse::ClickStackClickHouseCASLPermission(permission) =
        &permissions[0]
    else {
        panic!("expected the ClickHouse variant, got {:?}", permissions[0]);
    };
    assert_eq!(
        permission.action,
        Some(ClickStackCASLPermissionAction::Manage)
    );
}

#[tokio::test]
async fn delete_role() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles/role-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_role("org-1", "svc-1", "role-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Webhooks & Dashboard validation
// ===========================================================================

#[tokio::test]
async fn create_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Production Alerts",
            "service": "slack",
            "url": "https://hooks.slack.com/services/T/B/X"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-1",
            "name": "Production Alerts",
            "service": "slack",
            "url": "https://hooks.slack.com/services/T/B/X",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Production Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_webhook("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/webhook-1",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Updated Alerts"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-1",
            "name": "Updated Alerts",
            "service": "slack",
            "url": "https://hooks.slack.com/services/T/B/X",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Updated Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_webhook("org-1", "svc-1", "webhook-1", &body)
        .await
        .unwrap();
    // The response union resolves to a concrete Slack webhook variant.
    match resp.result.unwrap() {
        ClickStackWebhook::ClickStackSlackWebhook(w) => {
            assert_eq!(w.id.as_deref(), Some("webhook-1"));
            assert_eq!(w.name.as_deref(), Some("Updated Alerts"));
        }
        other => panic!("expected Slack webhook variant, got {other}"),
    }
}

#[tokio::test]
async fn create_webhook_incidentio_response() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-2",
            "name": "Incident Alerts",
            "service": "incidentio",
            "url": "https://api.incident.io/v2/alert_events/http/abc",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Incident Alerts".to_string(),
        service: ClickStackWebhookInputService::Incidentio,
        url: "https://api.incident.io/v2/alert_events/http/abc".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_webhook("org-1", "svc-1", &body)
        .await
        .unwrap();
    // The response union resolves to a concrete IncidentIO webhook variant
    // rather than greedily matching the structurally-identical Slack variant.
    match resp.result.unwrap() {
        ClickStackWebhook::ClickStackIncidentIOWebhook(w) => {
            assert_eq!(w.id.as_deref(), Some("webhook-2"));
            assert_eq!(w.name.as_deref(), Some("Incident Alerts"));
        }
        other => panic!("expected IncidentIO webhook variant, got {other}"),
    }
}

#[tokio::test]
async fn update_webhook_generic_response() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/webhook-3",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-3",
            "name": "Generic Alerts",
            "service": "generic",
            "url": "https://example.com/hook",
            "body": "{\"text\": \"{{ message }}\"}",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Generic Alerts".to_string(),
        service: ClickStackWebhookInputService::Generic,
        url: "https://example.com/hook".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_webhook("org-1", "svc-1", "webhook-3", &body)
        .await
        .unwrap();
    // The response union resolves to the Generic variant and preserves its
    // optional `body` field, which the greedy Slack match would have discarded.
    match resp.result.unwrap() {
        ClickStackWebhook::ClickStackGenericWebhook(w) => {
            assert_eq!(w.id.as_deref(), Some("webhook-3"));
            assert_eq!(w.body.as_deref(), Some("{\"text\": \"{{ message }}\"}"));
        }
        other => panic!("expected Generic webhook variant, got {other}"),
    }
}

#[tokio::test]
async fn delete_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/webhook-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_webhook("org-1", "svc-1", "webhook-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn validate_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/validate",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "My Dashboard"
        })))
        .respond_with(ok_json(serde_json::json!({
            "valid": false,
            "errors": [
                {"path": "tiles.0.config", "message": "Required"}
            ],
            "normalized": null
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateDashboardRequest {
        name: "My Dashboard".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_validate_dashboard("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.valid, Some(false));
    let errors = result.errors.as_ref().expect("errors should populate");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path.as_deref(), Some("tiles.0.config"));
    assert_eq!(result.normalized, None);
}

#[tokio::test]
async fn validate_webhook() {
    let (s, c) = setup().await;

    // The full body is matched, so optional fields left as `None` must be
    // omitted from the request rather than sent as `null`.
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/validate",
        ))
        .and(body_json(serde_json::json!({
            "name": "Generic Alerts",
            "service": "generic",
            "url": "ftp://example.com/hook",
            "body": "{{title}}"
        })))
        .respond_with(ok_json(serde_json::json!({
            "valid": false,
            "errors": [
                {"path": "url", "message": "Invalid url"}
            ],
            "normalized": null
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Generic Alerts".to_string(),
        service: ClickStackWebhookInputService::Generic,
        url: "ftp://example.com/hook".to_string(),
        body: Some("{{title}}".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_stack_validate_webhook("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.valid, Some(false));
    let errors = result.errors.as_ref().expect("errors should populate");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path.as_deref(), Some("url"));
    assert_eq!(errors[0].message.as_deref(), Some("Invalid url"));
    assert_eq!(result.normalized, None);
}

#[tokio::test]
async fn validate_webhook_returns_the_normalized_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/validate",
        ))
        .respond_with(ok_json(serde_json::json!({
            "valid": true,
            "errors": [],
            "normalized": {
                "name": "Production Alerts",
                "service": "slack",
                "url": "https://hooks.slack.com/services/T/B/X"
            }
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Production Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_validate_webhook("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.valid, Some(true));
    assert_eq!(result.errors.as_deref(), Some(&[][..]));
    let normalized = result.normalized.expect("normalized should populate");
    assert_eq!(normalized["service"], "slack");
}

#[tokio::test]
async fn validate_webhook_surfaces_a_bad_request() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/validate",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "Invalid request body"
        })))
        .mount(&s)
        .await;

    let err = c
        .click_stack_validate_webhook("org-1", "svc-1", &ClickStackWebhookInput::default())
        .await
        .expect_err("a 400 must fail the call");
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 400);
            assert_eq!(message, "Invalid request body");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn clickstack_list_pagination_encodes_only_supplied_parameters() {
    for resource in ["alerts", "saved-searches", "webhooks"] {
        for (limit, offset) in [
            (None, None),
            (Some(25), None),
            (None, Some(50)),
            (Some(25), Some(50)),
        ] {
            let (server, client) = setup().await;
            Mock::given(method("GET"))
                .and(path(format!(
                    "/v1/organizations/org-1/services/svc-1/clickstack/{resource}"
                )))
                .and(basic_auth("key", "secret"))
                .respond_with(ok_json(serde_json::json!([{}])))
                .expect(1)
                .mount(&server)
                .await;
            let count = match resource {
                "alerts" => client
                    .click_stack_list_alerts("org-1", "svc-1", limit, offset)
                    .await
                    .unwrap()
                    .result
                    .unwrap()
                    .len(),
                "saved-searches" => client
                    .click_stack_list_saved_searches("org-1", "svc-1", limit, offset)
                    .await
                    .unwrap()
                    .result
                    .unwrap()
                    .len(),
                "webhooks" => client
                    .click_stack_list_webhooks("org-1", "svc-1", limit, offset)
                    .await
                    .unwrap()
                    .result
                    .unwrap()
                    .len(),
                _ => unreachable!(),
            };
            assert_eq!(count, 1);
            let requests = server.received_requests().await.unwrap();
            assert_eq!(requests.len(), 1);
            let actual: std::collections::BTreeMap<_, _> = requests[0]
                .url
                .query_pairs()
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect();
            let expected: std::collections::BTreeMap<_, _> = [("limit", limit), ("offset", offset)]
                .into_iter()
                .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_string())))
                .collect();
            assert_eq!(actual, expected, "{resource}");
        }
    }
}
