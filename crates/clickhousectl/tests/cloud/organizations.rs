//! `cloud org`, `cloud member` and `cloud invitation` (src/cloud/organizations/).

use std::process::Command;

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Organization auto-detection (issue #337) ───────────────────────────────

const AUTO_DETECTED_ORG_ID: &str = "11111111-2222-3333-4444-555555555555";

async fn start_mock_org_auto_detection_api() -> MockServer {
    let mock = MockServer::start().await;
    let orgs = serde_json::json!({
        "result": [{ "id": AUTO_DETECTED_ORG_ID, "name": "Only org" }],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(orgs))
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{AUTO_DETECTED_ORG_ID}/prometheus"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_string("metric 1\n"))
        .mount(&mock)
        .await;

    let usage = serde_json::json!({
        "result": { "grandTotalCHC": 0.0, "costs": [] },
        "status": 200,
        "requestId": "stub-org-usage",
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{AUTO_DETECTED_ORG_ID}/usageCost"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(usage))
        .mount(&mock)
        .await;

    mock
}

#[tokio::test]
async fn org_balance_preserves_json_and_supports_oauth() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{ "id": AUTO_DETECTED_ORG_ID, "name": "Only org" }],
            "status": 200,
            "requestId": "stub-org-list"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let result = serde_json::json!({
        "totalRemainingCredits": 12.5,
        "balances": [
            {
                "id": "11111111-1111-1111-1111-111111111111",
                "type": "trial",
                "remainingCredits": 2.5,
                "totalAmount": 5.0,
                "amountSpent": 2.5,
                "startDate": "2026-01-01T00:00:00Z",
                "expirationDate": "2026-02-01T00:00:00Z"
            },
            {
                "id": "22222222-2222-2222-2222-222222222222",
                "type": "prepaid",
                "remainingCredits": 10.0,
                "totalAmount": 20.0,
                "amountSpent": 10.0,
                "startDate": "2026-02-01T00:00:00Z",
                "expirationDate": "2027-02-01T00:00:00Z"
            }
        ]
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{AUTO_DETECTED_ORG_ID}/creditBalances"
        )))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-credit-balances"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args(["cloud", "--url", &mock.uri(), "--json", "org", "balance"])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    let balance_request = requests
        .iter()
        .find(|request| request.url.path().ends_with("/creditBalances"))
        .unwrap();
    assert!(balance_request.url.query().is_none());
}

#[tokio::test]
async fn org_balance_human_output_handles_sparse_and_future_balances() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/creditBalances"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "balances": [{ "type": "promotional", "remainingCredits": 3.25 }]
            },
            "status": 200,
            "requestId": "stub-sparse-credit-balances"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output =
        invoke_cli_with_cloud_credentials_human(&mock, &["org", "balance", "--org-id", "org-1"]);

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The sparse total renders as the `-` placeholder, outside the table.
    assert!(
        stdout
            .lines()
            .any(|line| !line.starts_with('|') && line.ends_with(": - CHC")),
        "{stdout}"
    );
    assert!(stdout.contains(
        "| ID | Type        | Remaining (CHC) | Total (CHC) | Spent (CHC) | Start | Expires |"
    ));
    assert!(stdout.contains("| -  | promotional | 3.25"));
}

const ORG_ROLES_PATH: &str = "/v1/organizations/org-1/roles";

fn organization_role_envelope(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-org-role"
    }))
}

#[tokio::test]
async fn organization_role_writes_use_expected_routes_auth_and_bodies() {
    // The `--config-file` alias is `organizations::cli::primary_json_file_argument_contract`.
    organization_role_writes_use_expected_routes_auth_and_bodies_with_file_flag("--file").await;
}

async fn organization_role_writes_use_expected_routes_auth_and_bodies_with_file_flag(
    file_flag: &str,
) {
    let mock = MockServer::start().await;
    let create_body = serde_json::json!({
        "name": "auditor",
        "actors": ["user/user-1", "apiKey/key-1"],
        "policies": [{
            "allowDeny": "ALLOW",
            "permissions": ["control-plane:organization:view"],
            "resources": ["organization/org-1"],
            "tags": {
                "grants": ["SELECT"],
                "roleV2": "sql-console-readonly"
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path(ORG_ROLES_PATH))
        .and(header("authorization", BASIC_TEST_AUTH))
        .and(body_json(create_body.clone()))
        .respond_with(organization_role_envelope(serde_json::json!({
            "id": "role-1",
            "name": "auditor",
            "type": "custom"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let directory = tempfile::tempdir().unwrap();
    let config_file = directory.path().join("role.json");
    std::fs::write(&config_file, create_body.to_string()).unwrap();
    let create = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "role",
            "create",
            file_flag,
            config_file.to_str().unwrap(),
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&create);

    // Update selects the role by name: `org role get` is the selector table's
    // case, so this is the update dispatcher's own lookup.
    Mock::given(method("GET"))
        .and(path(ORG_ROLES_PATH))
        .and(header("authorization", BASIC_TEST_AUTH))
        .respond_with(organization_role_envelope(serde_json::json!([
            {"id": "role-1", "name": "auditor", "type": "custom"}
        ])))
        .expect(1)
        .mount(&mock)
        .await;
    let update_body = serde_json::json!({"name": "renamed"});
    Mock::given(method("PATCH"))
        .and(path(format!("{ORG_ROLES_PATH}/role-1")))
        .and(header("authorization", BASIC_TEST_AUTH))
        .and(body_json(update_body.clone()))
        .respond_with(organization_role_envelope(serde_json::json!({
            "id": "role-1",
            "name": "renamed",
            "type": "custom"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let update = invoke_cli_with_cloud_credentials_and_stdin(
        &mock,
        &[
            "org", "role", "update", "--name", "auditor", file_flag, "-", "--org-id", "org-1",
        ],
        &update_body.to_string(),
    );
    assert_success(&update);

    Mock::given(method("DELETE"))
        .and(path(format!("{ORG_ROLES_PATH}/role-1")))
        .and(header("authorization", BASIC_TEST_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-role-delete"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let delete = invoke_cli_with_cloud_credentials(
        &mock,
        &["org", "role", "delete", "role-1", "--org-id", "org-1"],
    );
    assert_success(&delete);
    assert_eq!(
        serde_json::from_slice::<Value>(&delete.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "stub-role-delete"})
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|request| request.url.query().is_none()));
}

#[tokio::test]
async fn organization_role_human_output_handles_sparse_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(ORG_ROLES_PATH))
        .respond_with(organization_role_envelope(serde_json::json!([
            {},
            {"id": "system-1", "name": "admin", "type": "system"},
            {"id": "role-1", "name": "reader", "type": "custom",
             "actors": ["user/user-1", "apiKey/key-1"], "policies": []}
        ])))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{ORG_ROLES_PATH}/role-sparse")))
        .respond_with(organization_role_envelope(serde_json::json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    let list = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "role", "list", "--org-id", "org-1"],
    );
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .filter(|line| line.starts_with('|') && !line.starts_with("|-"))
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect();
    assert_eq!(
        rows,
        [
            ["Name", "ID", "Type", "Actors", "Policies"],
            ["-", "-", "-", "-", "-"],
            ["admin", "system-1", "system", "-", "-"],
            ["reader", "role-1", "custom", "2", "0"],
        ],
        "{stdout}"
    );

    let get = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "role", "get", "role-sparse", "--org-id", "org-1"],
    );
    assert_success(&get);
}

#[tokio::test]
async fn org_quota_list_preserves_sparse_json_and_supports_oauth() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{ "id": AUTO_DETECTED_ORG_ID, "name": "Only org" }],
            "status": 200,
            "requestId": "stub-org-list"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let result = serde_json::json!([
        {
            "quotaCode": "services-per-organization",
            "name": "Services per organization",
            "description": "Limits services.",
            "scope": "organization",
            "value": 20,
            "usage": 3,
            "adjustable": true
        },
        { "quotaCode": "future-quota" }
    ]);
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{AUTO_DETECTED_ORG_ID}/quotas"
        )))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-quota-list"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "org",
            "quota",
            "list",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
}

#[tokio::test]
async fn org_quota_get_uses_lookup_key_and_human_detail_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/quotas/replicas-per-warehouse",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "quotaCode": "replicas-per-warehouse",
                "name": "Replicas per warehouse",
                "scope": "warehouse",
                "value": 20,
                "adjustable": true
            },
            "status": 200,
            "requestId": "stub-quota-get"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "org",
            "quota",
            "get",
            "replicas-per-warehouse",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "adjustable: true\nname: Replicas per warehouse\nquotaCode: replicas-per-warehouse\nscope: warehouse\nvalue: 20\n"
    );
}

#[tokio::test]
async fn org_quota_list_human_output_handles_missing_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/quotas"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{ "quotaCode": "future-quota" }],
            "status": 200,
            "requestId": "stub-sparse-quota-list"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "org",
            "quota",
            "list",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| Name | Code         | Scope | Usage | Limit | Adjustable |"));
    assert!(stdout.contains("| -    | future-quota | -     | -     | -     | -          |"));
}

#[tokio::test]
async fn org_prometheus_auto_detects_the_only_organization() {
    let mock = start_mock_org_auto_detection_api().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["org", "prometheus", "--filtered-metrics", "true"],
    );
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "metric 1\n\n");

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url.path(), "/v1/organizations");
    assert_eq!(
        requests[1].url.path(),
        format!("/v1/organizations/{AUTO_DETECTED_ORG_ID}/prometheus")
    );
    assert_eq!(requests[1].url.query(), Some("filtered_metrics=true"));
}

#[tokio::test]
async fn org_prometheus_discovery_preserves_groups_and_supports_oauth() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([{
        "targets": ["api.clickhouse.cloud:443"],
        "labels": {
            "__scheme__": "https",
            "__metrics_path__": "/v1/organizations/org-1/services/svc-1/prometheus",
            "__param_filtered_metrics": "false",
            "clickhouse_org_id": "11111111-2222-3333-4444-555555555555",
            "clickhouse_service_id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "clickhouse_discovery_service_name": "analytics"
        }
    }]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus/discovery"))
        .and(query_param("filtered_metrics", "false"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&result))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "org",
            "prometheus",
            "discovery",
            "--org-id",
            "org-1",
            "--filtered-metrics",
            "false",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
}

#[tokio::test]
async fn org_prometheus_discovery_renders_sparse_human_output() {
    let mock = MockServer::start().await;
    // Without --filtered-metrics the query string stays empty.
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus/discovery"))
        .and(wiremock::matchers::query_param_is_missing(
            "filtered_metrics",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "targets": ["api.clickhouse.cloud:443"] },
            { "labels": { "__scheme__": "https" } }
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "org",
            "prometheus",
            "discovery",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "- targets: [api.clickhouse.cloud:443]\n- labels:\n    __scheme__: https\n"
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn org_usage_auto_detects_the_only_organization() {
    let mock = start_mock_org_auto_detection_api().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "usage",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ],
    );
    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url.path(), "/v1/organizations");
    assert_eq!(
        requests[1].url.path(),
        format!("/v1/organizations/{AUTO_DETECTED_ORG_ID}/usageCost")
    );
    let query = requests[1]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    assert!(query.contains(&("from_date".into(), "2025-01-01".into())));
    assert!(query.contains(&("to_date".into(), "2025-01-31".into())));
}

async fn start_mock_usage_entities_api() -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/usageCost"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "grandTotalCHC": 3.0,
                "costs": [
                    {
                        "entityId": "11111111-2222-3333-4444-555555555555",
                        "entityName": "production",
                        "date": "2025-01-01",
                        "totalCHC": 1.0,
                    },
                    {
                        "entityId": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                        "date": "2025-01-01",
                        "totalCHC": 2.0,
                    },
                ],
            },
            "status": 200,
            "requestId": "stub-org-usage",
        })))
        .mount(&mock)
        .await;
    mock
}

fn invoke_org_usage(mock: &MockServer, json: bool) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url];
    if json {
        args.push("--json");
    }
    args.extend([
        "org",
        "usage",
        "--org-id",
        "org-1",
        "--from-date",
        "2025-01-01",
        "--to-date",
        "2025-01-31",
    ]);
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn org_usage_marks_uuid_only_entities_as_unknown_only_in_human_output() {
    // The label itself is `usage_entity_label_distinguishes_named_unknown_and_absent_entities`.
    let mock = start_mock_usage_entities_api().await;
    let output = invoke_org_usage(&mock, false);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("production"));
    // Human output annotates the UUID-only entity; the label itself is unit-tested.
    let cell = stdout
        .split('|')
        .map(str::trim)
        .find(|cell| cell.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"))
        .unwrap_or_else(|| panic!("{stdout}"));
    assert_ne!(cell, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "{stdout}");

    let output = invoke_org_usage(&mock, true);
    assert_success(&output);
    let usage: Value = serde_json::from_slice(&output.stdout).unwrap();
    let unknown = &usage["costs"][1];
    assert_eq!(unknown["entityId"], "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
    assert!(unknown.get("entityName").is_none());
}

// Explicit list clearing (#597).

#[tokio::test]
async fn member_update_sends_set_and_cleared_role_lists() {
    // The omitted body is `members::build_member_update_request_supports_minimal_fields`.
    let cases: &[(&[&str], Value)] = &[
        (
            &["--role-id", "role-1", "--role-id", "role-2"],
            serde_json::json!({"assignedRoleIds": ["role-1", "role-2"]}),
        ),
        (
            &["--clear-roles"],
            serde_json::json!({"assignedRoleIds": []}),
        ),
    ];

    for (flags, expected_body) in cases {
        let mock = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/members/user-1"))
            .and(body_json(expected_body.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"email": "member@example.com"},
                "status": 200,
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let mut args = vec!["member", "update", "user-1", "--org-id", "org-1"];
        args.extend_from_slice(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_success(&output);
        assert_eq!(mock.received_requests().await.unwrap().len(), 1);
    }
}

// BYOC infrastructure and service placement (#578).

#[tokio::test]
async fn byoc_infrastructure_commands_send_exact_paths_and_bodies() {
    let mock = MockServer::start().await;
    let collection = "/v1/organizations/org-1/byocInfrastructure";
    let item = "/v1/organizations/org-1/byocInfrastructure/byoc-1";

    Mock::given(method("POST"))
        .and(path(collection))
        .and(body_json(serde_json::json!({
            "regionId": "us-east-1",
            "accountId": "123456789012",
            "availabilityZoneSuffixes": ["a", "b"],
            "vpcCidrRange": "10.0.0.0/16",
            "displayName": "production"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "byoc-1", "displayName": "production"},
            "status": 200,
            "requestId": "stub-byoc-create"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(item))
        .and(body_json(serde_json::json!({"displayName": "renamed"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "byoc-1", "displayName": "renamed"},
            "status": 200,
            "requestId": "stub-byoc-update"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(item))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-byoc-delete"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let create = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
            "--availability-zone-suffix",
            "a",
            "--availability-zone-suffix",
            "b",
            "--vpc-cidr-range",
            "10.0.0.0/16",
            "--display-name",
            "production",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&create);
    let printed: Value = serde_json::from_slice(&create.stdout).unwrap();
    assert_eq!(
        printed,
        serde_json::json!({"id": "byoc-1", "displayName": "production"})
    );

    // Human mode: the handler renders the returned infrastructure.
    let update = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "org",
            "byoc",
            "update",
            "byoc-1",
            "--display-name",
            "renamed",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&update);
    let stdout = String::from_utf8(update.stdout).unwrap();
    assert!(serde_json::from_str::<Value>(&stdout).is_err(), "{stdout}");
    assert!(
        stdout.lines().any(|line| line == "displayName: renamed"),
        "{stdout}"
    );

    let delete = invoke_cli_with_cloud_credentials(
        &mock,
        &["org", "byoc", "delete", "byoc-1", "--org-id", "org-1"],
    );
    assert_success(&delete);
    assert_eq!(
        serde_json::from_slice::<Value>(&delete.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "stub-byoc-delete"})
    );
}

fn byoc_envelope(result: Value, request_id: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": request_id
    }))
}

#[tokio::test]
async fn byoc_get_and_progress_send_exact_paths_and_print_results() {
    let mock = MockServer::start().await;
    let item = "/v1/organizations/org-1/byocInfrastructure/byoc-1";
    let details = serde_json::json!({
        "id": "byoc-1",
        "accountId": "123456789012",
        "state": "infra-ready",
        "isByoVpc": true,
        "byoVpcId": "vpc-1",
        "byoVpcPrivateSubnetIds": ["subnet-a"]
    });
    let progress = serde_json::json!({
        "id": "byoc-1",
        "status": "in_progress",
        "updatedAt": "2026-09-30T10:00:00Z",
        "stages": [{
            "name": "network",
            "status": "in_progress",
            "subStages": [
                {"name": "vpc", "status": "ready"},
                {"name": "nat-gateway", "status": "pending", "message": "waiting for quota"}
            ]
        }]
    });
    Mock::given(method("GET"))
        .and(path(item))
        .respond_with(byoc_envelope(details.clone(), "stub-byoc-get"))
        .expect(2)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{item}/progress")))
        .respond_with(byoc_envelope(progress.clone(), "stub-byoc-progress"))
        .expect(2)
        .mount(&mock)
        .await;

    let get = invoke_cli_with_cloud_credentials(
        &mock,
        &["org", "byoc", "get", "byoc-1", "--org-id", "org-1"],
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        details
    );

    let progress_json = invoke_cli_with_cloud_credentials(
        &mock,
        &["org", "byoc", "progress", "byoc-1", "--org-id", "org-1"],
    );
    assert_success(&progress_json);
    assert_eq!(
        serde_json::from_slice::<Value>(&progress_json.stdout).unwrap(),
        progress
    );

    let get_human = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "byoc", "get", "byoc-1", "--org-id", "org-1"],
    );
    assert_success(&get_human);
    let stdout = String::from_utf8_lossy(&get_human.stdout);
    assert!(stdout.contains("infra-ready"), "{stdout}");
    assert!(stdout.contains("vpc-1"), "{stdout}");

    let progress_human = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "byoc", "progress", "byoc-1", "--org-id", "org-1"],
    );
    assert_success(&progress_human);
    let stdout = String::from_utf8_lossy(&progress_human.stdout);
    for nested in ["network", "nat-gateway", "waiting for quota"] {
        assert!(stdout.contains(nested), "{nested}: {stdout}");
    }

    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.method.as_str(), "GET");
        assert!(request.body.is_empty());
    }
}

fn byoc_validate_args<'a>() -> Vec<&'a str> {
    vec![
        "org",
        "byoc",
        "validate",
        "--region",
        "us-east-1",
        "--account-id",
        "123456789012",
        "--availability-zone-suffix",
        "a",
        "--vpc-id",
        "vpc-1",
        "--private-subnet-id",
        "subnet-a",
        "--private-subnet-id",
        "subnet-b",
        "--public-subnet-id",
        "subnet-pub",
        "--external-id",
        "external-1",
        "--tag",
        "team=data",
        "--org-id",
        "org-1",
    ]
}

fn byoc_validate_body() -> Value {
    serde_json::json!({
        "regionId": "us-east-1",
        "accountId": "123456789012",
        "availabilityZoneSuffixes": ["a"],
        "vpcId": "vpc-1",
        "privateSubnetIds": ["subnet-a", "subnet-b"],
        "publicSubnetIds": ["subnet-pub"],
        "externalId": "external-1",
        "tags": {"team": "data"}
    })
}

#[tokio::test]
async fn byoc_validate_posts_the_shared_body_and_succeeds_when_checks_pass() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "cloudProvider": "aws",
        "supported": true,
        "allPassed": true,
        "anyPassed": true,
        "checks": [{"name": "Create VPC endpoint", "action": "ec2:CreateVpcEndpoint", "allowed": true, "group": "base"}]
    });
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/validate"))
        .and(body_json(byoc_validate_body()))
        .respond_with(byoc_envelope(result.clone(), "stub-byoc-validate"))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(&mock, &byoc_validate_args());
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
}

#[tokio::test]
async fn byoc_validate_prints_denied_checks_and_exits_one() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "cloudProvider": "aws",
        "supported": true,
        "allPassed": false,
        "anyPassed": true,
        "checks": [
            {"name": "Create VPC endpoint", "action": "ec2:CreateVpcEndpoint", "allowed": true},
            {"name": "Tag resources", "action": "ec2:CreateTags", "allowed": false, "reason": "explicitDeny"}
        ]
    });
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/validate"))
        .and(body_json(byoc_validate_body()))
        .respond_with(byoc_envelope(result.clone(), "stub-byoc-validate-denied"))
        .expect(2)
        .mount(&mock)
        .await;

    let json = invoke_cli_with_cloud_credentials(&mock, &byoc_validate_args());
    assert_eq!(json.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        result
    );
    // The summary's only numbers: one denied of two checks.
    assert_eq!(
        digits(cloud_runtime_error(&json)["message"].as_str().unwrap()),
        "12"
    );

    let human = invoke_cli_with_cloud_credentials_human(&mock, &byoc_validate_args());
    assert_eq!(human.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&human.stdout);
    for shown in ["Tag resources", "ec2:CreateTags", "explicitDeny"] {
        assert!(stdout.contains(shown), "{shown}: {stdout}");
    }
    // The summary's only numbers: one denied of two checks.
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert_eq!(digits(&stderr), "12", "{stderr}");
}

#[tokio::test]
async fn byoc_validate_unsupported_configuration_is_reported_not_failed() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/validate"))
        .respond_with(byoc_envelope(
            serde_json::json!({"cloudProvider": "azure", "supported": false, "checks": []}),
            "stub-byoc-validate-unsupported",
        ))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "org",
            "byoc",
            "validate",
            "--region",
            "eastus",
            "--account-id",
            "subscription-1",
            "--tenant-id",
            "tenant-1",
            "--service-principal-client-id",
            "client-1",
            "--org-id",
            "org-1",
        ],
    );
    // The response's three fields, then one notice line; no failure.
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("supported: false"), "{stdout}");
    assert_eq!(
        stdout.lines().filter(|line| !line.is_empty()).count(),
        4,
        "{stdout}"
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
        serde_json::json!({
            "regionId": "eastus",
            "accountId": "subscription-1",
            "tenantId": "tenant-1",
            "servicePrincipalClientId": "client-1"
        })
    );
}

fn invoke_org_with_oauth(mock: &MockServer, cli_args: &[&str]) -> std::process::Output {
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url, "--json", "org"];
    args.extend(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args(args)
        .output()
        .unwrap()
}

#[tokio::test]
async fn byoc_private_endpoint_config_and_tags_are_oauth_reads_with_exact_paths() {
    let mock = MockServer::start().await;
    let item = "/v1/organizations/org-1/byocInfrastructure/byoc-1";
    let config = serde_json::json!({
        "endpointName": "com.amazonaws.vpce.us-east-1.vpce-svc-1",
        "privateDnsHostname": "byoc-1.private.example.com"
    });
    let tags = serde_json::json!({"tags": {"team": "data", "env": "prod"}});
    Mock::given(method("GET"))
        .and(path(format!("{item}/privateEndpointConfig")))
        .respond_with(byoc_envelope(config.clone(), "stub-byoc-pe-config"))
        .expect(2)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{item}/tags")))
        .respond_with(byoc_envelope(tags.clone(), "stub-byoc-tags"))
        .expect(2)
        .mount(&mock)
        .await;

    for (subcommand, expected) in [("private-endpoint-config", &config), ("tags", &tags)] {
        let output =
            invoke_org_with_oauth(&mock, &["byoc", subcommand, "byoc-1", "--org-id", "org-1"]);
        assert_success(&output);
        assert_eq!(
            &serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            expected
        );
    }

    let config_human = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "org",
            "byoc",
            "private-endpoint-config",
            "byoc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&config_human);
    let stdout = String::from_utf8_lossy(&config_human.stdout);
    assert!(stdout.contains("byoc-1.private.example.com"), "{stdout}");

    let tags_human = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "byoc", "tags", "byoc-1", "--org-id", "org-1"],
    );
    assert_success(&tags_human);
    let stdout = String::from_utf8_lossy(&tags_human.stdout);
    for value in ["team", "data", "env", "prod"] {
        assert!(stdout.contains(value), "{value}: {stdout}");
    }

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 4);
    for request in requests {
        assert_eq!(request.method.as_str(), "GET");
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn org_notification_types_is_an_oauth_read_preserving_json() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([
        {
            "name": "billing.credit_low",
            "title": "Credits running low",
            "description": "Remaining credits fell below the threshold.",
            "category": "billing",
            "severity": "warning",
            "resourceType": "organization",
            "isConfigurable": true,
            "isAvailable": false
        },
        {"name": "future.type", "severity": "emergency"}
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notificationTypes"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(byoc_envelope(result.clone(), "stub-notification-types"))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_org_with_oauth(&mock, &["notification-types", "--org-id", "org-1"]);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
}

#[tokio::test]
async fn org_notification_types_human_output_lists_rows() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notificationTypes"))
        .respond_with(byoc_envelope(
            serde_json::json!([
                {
                    "name": "billing.credit_low",
                    "title": "Credits running low",
                    "category": "billing",
                    "severity": "critical",
                    "resourceType": "organization",
                    "isConfigurable": false,
                    "isAvailable": true
                },
                {}
            ]),
            "stub-notification-types",
        ))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "notification-types", "--org-id", "org-1"],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .filter(|line| line.starts_with('|') && !line.starts_with("|-"))
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect();
    assert_eq!(
        rows,
        [
            vec![
                "Name",
                "Title",
                "Category",
                "Severity",
                "Resource Type",
                "Configurable",
                "Available"
            ],
            vec![
                "billing.credit_low",
                "Credits running low",
                "billing",
                "critical",
                "organization",
                "false",
                "true"
            ],
            vec!["-", "-", "-", "-", "-", "-", "-"],
        ],
        "{stdout}"
    );
}

#[tokio::test]
async fn org_get_and_update_select_explicit_or_detected_organization() {
    for subcommand in ["get", "update"] {
        for selector in [
            vec![],
            vec!["--org-id", "00000000-0000-0000-0000-000000000001"],
        ] {
            let mock = MockServer::start().await;
            let home = tempfile::tempdir().unwrap();
            Mock::given(method("GET"))
                .and(path("/v1/organizations"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": [{"id": "00000000-0000-0000-0000-000000000001", "name": "Only org"}]
                })))
                .expect(if selector.is_empty() { 1 } else { 0 })
                .mount(&mock)
                .await;
            let result = serde_json::json!({"id": "00000000-0000-0000-0000-000000000001", "name": "Selected org"});
            let mut target = Mock::given(method(if subcommand == "get" { "GET" } else { "PATCH" }))
                .and(path(
                    "/v1/organizations/00000000-0000-0000-0000-000000000001",
                ))
                .and(header(
                    "authorization",
                    "Basic dGVzdC1rZXk6dGVzdC1zZWNyZXQ=",
                ));
            if subcommand == "update" {
                target = target.and(body_json(serde_json::json!({
                    "name": "Selected org",
                    "enableCoreDumps": false,
                    "privateEndpoints": {"remove": [
                        {"id": "pe-1", "cloudProvider": "aws", "region": "us-east-1"}
                    ]}
                })));
            }
            target
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
                )
                .expect(1)
                .mount(&mock)
                .await;
            // The update with an explicit selector also covers the human branch.
            let human = subcommand == "update" && !selector.is_empty();
            let mut cmd = Command::new(clickhousectl_binary());
            cmd.isolated_env();
            cmd.current_dir(home.path())
                .env("HOME", home.path())
                .env("DO_NOT_TRACK", "1")
                .args([
                    "cloud",
                    "--url",
                    &mock.uri(),
                    "--api-key",
                    "test-key",
                    "--api-secret",
                    "test-secret",
                    "org",
                    subcommand,
                ])
                .args(&selector);
            if !human {
                cmd.arg("--json");
            }
            if subcommand == "update" {
                cmd.args([
                    "--new-name",
                    "Selected org",
                    "--remove-private-endpoint",
                    "pe-1,cloud-provider=aws,region=us-east-1",
                    "--enable-core-dumps",
                    "false",
                ]);
            }
            let output = cmd.output().unwrap();
            assert_success(&output);
            if human {
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert_eq!(stdout.lines().count(), 1, "{stdout}");
                assert!(
                    stdout.contains("Selected org")
                        && stdout.contains("00000000-0000-0000-0000-000000000001"),
                    "{stdout}"
                );
            } else {
                assert_eq!(
                    serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                    result
                );
            }
            assert_eq!(
                mock.received_requests().await.unwrap().len(),
                if selector.is_empty() { 2 } else { 1 }
            );
        }
    }
}

#[tokio::test]
async fn org_get_and_update_require_one_organization_for_autodetection() {
    // No organization is a runtime failure; several is a usage error (#1198).
    for (subcommand, organizations, exit) in [
        ("get", serde_json::json!([]), 1),
        (
            "update",
            serde_json::json!([
                {"id": "00000000-0000-0000-0000-000000000001", "name": "First"}, {"id": "00000000-0000-0000-0000-000000000002", "name": "Second"}
            ]),
            2,
        ),
    ] {
        let mock = MockServer::start().await;
        let home = tempfile::tempdir().unwrap();
        Mock::given(method("GET"))
            .and(path("/v1/organizations"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"result": organizations})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let mut cmd = Command::new(clickhousectl_binary());
        cmd.isolated_env();
        let output = cmd
            .current_dir(home.path())
            .env("HOME", home.path())
            .env("DO_NOT_TRACK", "1")
            .args([
                "cloud",
                "--url",
                &mock.uri(),
                "--api-key",
                "test-key",
                "--api-secret",
                "test-secret",
                "org",
                subcommand,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit), "{output:?}");
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/v1/organizations");
    }
}

async fn mount_two_org_pages(mock: &MockServer, expected: u64) {
    mount_cursor_page(
        mock,
        "/v1/organizations",
        None,
        serde_json::json!({
            "result": [{"id": PAGED_ORG_FIRST, "name": "first-org"}],
            "nextCursor": "org-page-2"
        }),
        expected,
    )
    .await;
    mount_cursor_page(
        mock,
        "/v1/organizations",
        Some("org-page-2"),
        serde_json::json!({
            "result": [{"id": PAGED_ORG_SECOND, "name": "second-org"}],
            "nextCursor": null
        }),
        expected,
    )
    .await;
}

#[tokio::test]
async fn org_list_reads_every_page_in_json_and_human_output() {
    let mock = MockServer::start().await;
    mount_two_org_pages(&mock, 2).await;
    let json = invoke_cli_with_cloud_credentials(&mock, &["org", "list"]);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!([
            {"id": PAGED_ORG_FIRST, "name": "first-org"},
            {"id": PAGED_ORG_SECOND, "name": "second-org"}
        ])
    );
    let human = invoke_cli_with_cloud_credentials_human(&mock, &["org", "list"]);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(stdout.contains("first-org"), "{stdout}");
    assert!(stdout.contains("second-org"), "{stdout}");
}

#[tokio::test]
async fn org_name_resolution_and_auto_detection_read_every_page() {
    let mock = MockServer::start().await;
    mount_two_org_pages(&mock, 2).await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{PAGED_ORG_SECOND}/services"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": []})))
        .expect(1)
        .mount(&mock)
        .await;

    // A name found only on the second page resolves to that organization.
    let named =
        invoke_cli_with_cloud_credentials(&mock, &["service", "list", "--org-name", "second-org"]);
    assert_success(&named);
    assert_eq!(
        serde_json::from_slice::<Value>(&named.stdout).unwrap(),
        serde_json::json!([])
    );

    // One organization per page is still two organizations, not an auto-detected
    // one: a usage error (exit 2, clap text even in JSON mode) that lists both.
    let detected = invoke_cli_with_cloud_credentials(&mock, &["service", "list"]);
    assert_eq!(detected.status.code(), Some(2), "{detected:?}");
    assert!(detected.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&detected.stderr);
    assert!(serde_json::from_str::<Value>(&stderr).is_err(), "{stderr}");
    for token in [
        "--org-id",
        "--org-name",
        PAGED_ORG_FIRST,
        "first-org",
        PAGED_ORG_SECOND,
        "second-org",
    ] {
        assert!(stderr.contains(token), "missing {token}: {stderr}");
    }
}

#[tokio::test]
async fn org_notifications_forwards_filters_as_an_oauth_read_and_keeps_pagination() {
    let mock = MockServer::start().await;
    let cursor = "-opaque +/=&?'雪";
    let result = serde_json::json!([
        {"title": "Backup failed", "type": "future.type", "severity": "future.severity",
         "resourceType": "service", "resourceId": "service-1", "resourceName": "analytics",
         "description": "Review backup settings."},
        {"id": null, "createdAt": null}
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notifications"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .and(query_param("limit", "25"))
        .and(query_param("cursor", cursor))
        .and(query_param("severity", "future.severity"))
        .and(query_param("type", "future.type"))
        .and(query_param("resource_id", "service-1"))
        .and(query_param("resource_type", "service"))
        .and(query_param("from_date", "2026-10-01T00:00:00+02:00"))
        .and(query_param("to_date", "2026-10-08T00:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result, "limit": 25, "nextCursor": ""
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_org_with_oauth(
        &mock,
        &[
            "notifications",
            "--org-id",
            "org-1",
            "--limit",
            "25",
            "--cursor",
            cursor,
            "--severity",
            "future.severity",
            "--type",
            "future.type",
            "--resource-id",
            "service-1",
            "--resource-type",
            "service",
            "--from-date",
            "2026-10-01T00:00:00+02:00",
            "--to-date",
            "2026-10-08T00:00:00Z",
        ],
    );
    assert_success(&output);
    let mut expected = result;
    expected[1] = serde_json::json!({});
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "result": expected, "limit": 25, "nextCursor": ""
        })
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].body.is_empty());
}

#[tokio::test]
async fn org_notifications_human_output_prints_rows_and_the_next_cursor() {
    // The table itself is `notifications_table_tolerates_absent_fields`; the
    // handler adds the cursor line.
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/notifications"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{}, {"title": "Backup failed", "severity": "warning"}], "nextCursor": ""
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "notifications", "--org-id", "org-1"],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Backup failed"), "{stdout}");
    assert!(
        stdout.lines().any(|line| line.ends_with(": \"\"")),
        "{stdout}"
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), None);
}

// BYOC customer-authorized access, organization previews and member
// notification settings (#1279).

#[tokio::test]
async fn byoc_access_get_and_update_send_exact_paths_and_bodies() {
    let mock = MockServer::start().await;
    let access = "/v1/organizations/org-1/byocInfrastructure/byoc-1/customerAuthorizedAccess";
    let stored = serde_json::json!({
        "mode": "window",
        "startTimestamp": "2026-11-10T07:00:00Z",
        "durationSeconds": 14400,
        "allowed": false
    });
    Mock::given(method("GET"))
        .and(path(access))
        .respond_with(byoc_envelope(
            serde_json::json!({"mode": "always", "allowed": true}),
            "stub-access-get",
        ))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(access))
        .and(body_json(serde_json::json!({
            "mode": "window",
            "startTimestamp": "2026-11-10T07:00:00Z",
            "durationSeconds": 14400
        })))
        .respond_with(byoc_envelope(stored.clone(), "stub-access-update"))
        .expect(1)
        .mount(&mock)
        .await;

    let get = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org", "byoc", "access", "get", "byoc-1", "--org-id", "org-1",
        ],
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        serde_json::json!({"mode": "always", "allowed": true})
    );

    let update = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "byoc",
            "access",
            "update",
            "byoc-1",
            "--org-id",
            "org-1",
            "--mode",
            "window",
            "--start",
            "2026-11-10T09:00:00+02:00",
            "--duration",
            "14400",
        ],
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        stored
    );
}

#[tokio::test]
async fn org_preview_list_and_update_send_exact_paths_and_bodies() {
    let mock = MockServer::start().await;
    let previews = serde_json::json!([{
        "id": "preview-1",
        "name": "Feature",
        "description": "What it does",
        "docsUrl": "https://clickhouse.com/docs",
        "enabled": true,
        "canToggle": true
    }]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/previews"))
        .respond_with(byoc_envelope(previews.clone(), "stub-previews"))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/previews/preview-1"))
        .and(body_json(serde_json::json!({"enabled": false})))
        .respond_with(byoc_envelope(
            serde_json::json!({"id": "preview-1", "enabled": false}),
            "stub-preview-update",
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let list =
        invoke_cli_with_cloud_credentials(&mock, &["org", "preview", "list", "--org-id", "org-1"]);
    assert_success(&list);
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        previews
    );

    let update = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "preview",
            "update",
            "preview-1",
            "--enabled",
            "false",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        serde_json::json!({"id": "preview-1", "enabled": false})
    );
}

#[tokio::test]
async fn member_notification_settings_list_and_update_send_exact_requests() {
    let mock = MockServer::start().await;
    let settings = "/v1/organizations/org-1/members/user-1/notificationSettings";
    let setting = serde_json::json!({
        "type": "service_backup_failed",
        "resourceType": "service",
        "resourceId": "service-1",
        "channelType": "email",
        "effect": "disabled"
    });
    Mock::given(method("GET"))
        .and(path(settings))
        .and(query_param("limit", "10"))
        .and(query_param("cursor", "-opaque"))
        .and(query_param("type", "service_backup_failed"))
        .and(query_param("resource_id", "service-1"))
        .and(query_param("resource_type", "service"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [setting.clone()], "limit": 10, "nextCursor": "next"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(settings))
        .and(body_json(serde_json::json!({"rules": [{
            "effect": "disabled",
            "notifications": ["service_backup_failed"],
            "resources": ["service-1"],
            "channels": ["email"]
        }]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"settings": [setting.clone()]}, "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let list = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "member",
            "notification-settings",
            "list",
            "user-1",
            "--org-id",
            "org-1",
            "--limit",
            "10",
            "--cursor",
            "-opaque",
            "--type",
            "service_backup_failed",
            "--resource-id",
            "service-1",
            "--resource-type",
            "service",
        ],
    );
    assert_success(&list);
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        serde_json::json!({"result": [setting.clone()], "limit": 10, "nextCursor": "next"})
    );

    let update = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "member",
            "notification-settings",
            "update",
            "user-1",
            "--org-id",
            "org-1",
            "--effect",
            "disabled",
            "--notification",
            "service_backup_failed",
            "--resource",
            "service-1",
            "--channel",
            "email",
        ],
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        serde_json::json!({"settings": [setting]})
    );
}
