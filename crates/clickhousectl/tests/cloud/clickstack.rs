//! `cloud clickstack` (src/cloud/clickstack.rs).

use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── ClickStack source and role commands (issue #692) ────────────────────────

fn invoke_clickstack_cli(
    mock: &MockServer,
    cli_args: &[&str],
    stdin: Option<&str>,
    json: bool,
) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", url.as_str()];
    if json {
        args.push("--json");
    }
    args.extend_from_slice(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(directory.path())
        .args(args);
    if let Some(input) = stdin {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn clickhousectl");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.output().expect("failed to spawn clickhousectl")
    }
}

async fn mount_clickstack_success_routes(mock: &MockServer) {
    let sources = "/v1/organizations/org-1/services/svc-1/clickstack/sources";
    let roles = "/v1/organizations/org-1/services/svc-1/clickstack/roles";
    Mock::given(method("GET"))
        .and(path(sources))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{}]
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{sources}/source-get")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"kind": "log", "name": null}
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("POST"))
        .and(path(sources))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"kind": "promql", "id": "source-created"}
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("PUT"))
        .and(path(format!("{sources}/source-update")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"kind": "promql", "id": "source-update"}
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{sources}/source-delete")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})))
        .expect(1)
        .mount(mock)
        .await;

    Mock::given(method("GET"))
        .and(path(roles))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{}]
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{roles}/role-get")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": null}
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("POST"))
        .and(path(roles))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "role-created"}
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("PUT"))
        .and(path(format!("{roles}/role-update")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "role-update"}
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{roles}/role-delete")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})))
        .expect(1)
        .mount(mock)
        .await;
}

// The `--config-file` alias is `primary_json_file_argument_contract`.
#[tokio::test]
async fn clickstack_all_ten_routes_use_service_org_auth_and_full_bodies() {
    let file_flag = "--file";
    let mock = MockServer::start().await;
    mount_clickstack_success_routes(&mock).await;
    let directory = tempfile::tempdir().unwrap();
    let source_file = directory.path().join("source.json");
    let role_file = directory.path().join("role.json");
    let source = serde_json::json!({
        "kind": "promql",
        "name": "Prometheus",
        "connection": "connection-1",
        "from": {"databaseName": "default", "tableName": "metrics"},
        "timestampValueExpression": "timestamp",
        "section": "production",
        "disabled": false,
        "querySettings": [{"setting": "max_threads", "value": "2"}]
    });
    let role = serde_json::json!({
        "name": "Operators",
        "description": "Production operators",
        "permissions": [{
            "action": "manage",
            "subject": "Dashboard",
            "inverted": false,
            "integration": "mongodb",
            "conditions": {"team": "team-1"}
        }]
    });
    std::fs::write(&source_file, source.to_string()).unwrap();
    std::fs::write(&role_file, role.to_string()).unwrap();
    let source_path = source_file.to_str().unwrap();
    let role_path = role_file.to_str().unwrap();
    let role_stdin = role.to_string();
    let commands: Vec<(Vec<&str>, Option<&str>)> = vec![
        (
            vec!["clickstack", "source", "list", "svc-1", "--org-id", "org-1"],
            None,
        ),
        (
            vec![
                "clickstack",
                "source",
                "get",
                "svc-1",
                "source-get",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "source",
                "create",
                "svc-1",
                file_flag,
                source_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "source",
                "update",
                "svc-1",
                "source-update",
                file_flag,
                source_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "source",
                "delete",
                "svc-1",
                "source-delete",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec!["clickstack", "role", "list", "svc-1", "--org-id", "org-1"],
            None,
        ),
        (
            vec![
                "clickstack",
                "role",
                "get",
                "svc-1",
                "role-get",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "role",
                "create",
                "svc-1",
                file_flag,
                role_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "role",
                "update",
                "svc-1",
                "role-update",
                file_flag,
                "-",
                "--org-id",
                "org-1",
            ],
            Some(role_stdin.as_str()),
        ),
        (
            vec![
                "clickstack",
                "role",
                "delete",
                "svc-1",
                "role-delete",
                "--org-id",
                "org-1",
            ],
            None,
        ),
    ];
    for (args, stdin) in commands {
        let output = invoke_clickstack_cli(&mock, &args, stdin, true);
        assert_success(&output);
        let printed = serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid JSON output for {args:?}: {error}\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
        // A delete confirms the id it was given, not the service's.
        if args[2] == "delete" {
            assert_eq!(printed, serde_json::json!({"deleted": args[4]}), "{args:?}");
        }
    }

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 10);
    for request in &requests {
        assert!(
            request
                .headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("Basic "),
            "{} {}",
            request.method,
            request.url.path()
        );
    }
    let write_bodies = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST
                || request.method == wiremock::http::Method::PUT
        })
        .map(|request| request.body_json::<Value>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        write_bodies,
        vec![source.clone(), source, role.clone(), role]
    );
}

/// Each ClickStack list renders its own table in human mode. A sparse `{}`
/// row shows every column as `-`, and an unknown webhook variant still shows
/// the fields it carries.
#[tokio::test]
async fn clickstack_human_lists_render_every_resource_table() {
    for (resource, route, result, header, rows) in [
        (
            "source",
            "sources",
            serde_json::json!([{}]),
            vec!["Name", "ID", "Kind"],
            vec![vec!["-", "-", "-"]],
        ),
        (
            "role",
            "roles",
            serde_json::json!([
                {"name": "ops", "id": "role-1", "isPredefined": false, "description": "Operators"},
                {}
            ]),
            vec!["Name", "ID", "Predefined", "Description"],
            vec![
                vec!["ops", "role-1", "false", "Operators"],
                vec!["-", "-", "-", "-"],
            ],
        ),
        (
            "dashboard",
            "dashboards",
            serde_json::json!([{}]),
            vec!["Name", "ID", "Tiles", "Tags"],
            vec![vec!["-", "-", "-", "-"]],
        ),
        (
            "alert",
            "alerts",
            serde_json::json!([{}]),
            vec!["Name", "ID", "Source", "State", "Interval"],
            vec![vec!["-", "-", "-", "-", "-"]],
        ),
        (
            "webhook",
            "webhooks",
            serde_json::json!([
                {"service": "future", "name": "hook", "futureField": true},
                {}
            ]),
            vec!["Name", "ID", "Service", "URL"],
            vec![vec!["hook", "-", "future", "-"], vec!["-", "-", "-", "-"]],
        ),
        (
            "saved-search",
            "saved-searches",
            serde_json::json!([
                {"name": "errors", "id": "search-1", "sourceId": "source-1", "whereLanguage": "sql"},
                {}
            ]),
            vec!["Name", "ID", "Source ID", "Language"],
            vec![
                vec!["errors", "search-1", "source-1", "sql"],
                vec!["-", "-", "-", "-"],
            ],
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/org-1/services/svc-1/clickstack/{route}"
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_clickstack_cli(
            &mock,
            &["clickstack", resource, "list", "svc-1", "--org-id", "org-1"],
            None,
            false,
        );
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let table: Vec<Vec<&str>> = stdout
            .lines()
            .filter(|line| line.starts_with('\u{2502}'))
            .map(|line| {
                line.trim_matches('\u{2502}')
                    .split('\u{2502}')
                    .map(str::trim)
                    .collect()
            })
            .collect();
        let mut expected = vec![header];
        expected.extend(rows);
        assert_eq!(table, expected, "{resource}:\n{stdout}");
    }
}

// ── ClickStack saved-search commands (issue #694) ──────────────────────────

// The `--config-file` alias is `primary_json_file_argument_contract`.
#[tokio::test]
async fn clickstack_saved_search_all_five_routes_preserve_bodies_and_json_output() {
    let file_flag = "--file";
    let mock = MockServer::start().await;
    let searches = "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches";
    Mock::given(method("GET"))
        .and(path(searches))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{}]
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{searches}/search-get")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "search-get", "name": null, "futureField": "kept-compatible"}
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(searches))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "search-created"}
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PUT"))
        .and(path(format!("{searches}/search-update")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "search-update", "sourceId": null}
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{searches}/search-delete")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})))
        .expect(1)
        .mount(&mock)
        .await;

    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("saved-search.json");
    let body = serde_json::json!({
        "name": "production errors",
        "sourceId": "source-1",
        "select": "Timestamp, Body",
        "where": "SeverityText = 'ERROR'",
        "whereLanguage": "sql",
        "orderBy": "Timestamp DESC",
        "tags": ["production", "errors"],
        "filters": [{"type": "sql", "condition": "ServiceName = 'api'"}]
    });
    std::fs::write(&file, body.to_string()).unwrap();
    let body_stdin = body.to_string();
    let commands: Vec<(Vec<&str>, Option<&str>)> = vec![
        (
            vec![
                "clickstack",
                "saved-search",
                "list",
                "svc-1",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "saved-search",
                "get",
                "svc-1",
                "search-get",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "saved-search",
                "create",
                "svc-1",
                file_flag,
                file.to_str().unwrap(),
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "saved-search",
                "update",
                "svc-1",
                "search-update",
                file_flag,
                "-",
                "--org-id",
                "org-1",
            ],
            Some(body_stdin.as_str()),
        ),
        (
            vec![
                "clickstack",
                "saved-search",
                "delete",
                "svc-1",
                "search-delete",
                "--org-id",
                "org-1",
            ],
            None,
        ),
    ];
    for (args, stdin) in commands {
        let output = invoke_clickstack_cli(&mock, &args, stdin, true);
        assert_success(&output);
        let printed = serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid JSON output for {args:?}: {error}\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
        // A delete confirms the id it was given, not the service's.
        if args[2] == "delete" {
            assert_eq!(printed, serde_json::json!({"deleted": args[4]}), "{args:?}");
        }
    }

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    for request in &requests {
        assert!(
            request
                .headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("Basic "),
            "{} {}",
            request.method,
            request.url.path()
        );
    }
    let write_bodies = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST
                || request.method == wiremock::http::Method::PUT
        })
        .map(|request| request.body_json::<Value>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(write_bodies, vec![body.clone(), body]);
}

// ── ClickStack dashboard commands (issue #693) ─────────────────────────────

// The `--config-file` alias is `primary_json_file_argument_contract`.
#[tokio::test]
async fn clickstack_dashboard_all_six_routes_preserve_typed_bodies_and_auth() {
    let file_flag = "--file";
    let mock = MockServer::start().await;
    let dashboards = "/v1/organizations/org-1/services/svc-1/clickstack/dashboards";
    Mock::given(method("GET"))
        .and(path(dashboards))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":[{}]})))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{dashboards}/dash-get")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{}})))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(dashboards))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result":{"id":"dash-created"}})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PUT"))
        .and(path(format!("{dashboards}/dash-update")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result":{"id":"dash-update"}})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{dashboards}/dash-delete")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status":200})))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{dashboards}/validate")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result":{"valid":true,"errors":[],"normalized":{"name":"minimal"}}
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let create = serde_json::json!({
        "name":"minimal", "tiles":[], "savedQuery":null, "savedQueryLanguage":null
    });
    let update = serde_json::json!({
        "name":"replacement", "tags":[], "filters":[], "savedFilterValues":[], "containers":[],
        "tiles":[{"name":"rps","x":0,"y":0,"w":6,"h":3,"config":{
            "configType":"promql", "displayType":"line", "sourceId":"prom-1",
            "expressions":[{"expression":"sum(rate(http_requests_total[5m]))","alias":"rps"}],
            "legendTemplate":"{{service}}", "lineInterpolation":"step"
        }}]
    });
    let directory = tempfile::tempdir().unwrap();
    let create_file = directory.path().join("create.json");
    std::fs::write(&create_file, create.to_string()).unwrap();
    let create_path = create_file.to_str().unwrap();
    let update_stdin = update.to_string();
    let commands: Vec<(Vec<&str>, Option<&str>)> = vec![
        (
            vec![
                "clickstack",
                "dashboard",
                "list",
                "svc-1",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "dashboard",
                "get",
                "svc-1",
                "dash-get",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "dashboard",
                "create",
                "svc-1",
                file_flag,
                create_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "dashboard",
                "update",
                "svc-1",
                "dash-update",
                file_flag,
                "-",
                "--org-id",
                "org-1",
            ],
            Some(update_stdin.as_str()),
        ),
        (
            vec![
                "clickstack",
                "dashboard",
                "delete",
                "svc-1",
                "dash-delete",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "dashboard",
                "validate",
                "svc-1",
                file_flag,
                create_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
    ];
    for (args, stdin) in commands {
        let output = invoke_clickstack_cli(&mock, &args, stdin, true);
        assert_success(&output);
        let printed = serde_json::from_slice::<Value>(&output.stdout).unwrap();
        // A delete confirms the id it was given, not the service's.
        if args[2] == "delete" {
            assert_eq!(printed, serde_json::json!({"deleted": args[4]}), "{args:?}");
        }
    }

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 6);
    for request in &requests {
        assert!(
            request
                .headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("Basic ")
        );
    }
    let writes = requests
        .iter()
        .filter(|request| {
            matches!(
                request.method,
                wiremock::http::Method::POST | wiremock::http::Method::PUT
            )
        })
        .map(|request| {
            (
                request.url.path().to_owned(),
                request.body_json::<Value>().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 3);
    assert_eq!(
        writes[0].1,
        serde_json::json!({"name":"minimal","tiles":[]})
    );
    assert_eq!(writes[1].1, update);
    assert_eq!(
        writes[2].1,
        serde_json::json!({"name":"minimal","tiles":[]})
    );
}

// ── ClickStack alert and webhook commands (issue #695) ─────────────────────

// The `--config-file` alias is `primary_json_file_argument_contract`.
#[tokio::test]
async fn clickstack_alert_and_webhook_all_nine_routes_preserve_bodies_and_auth() {
    let file_flag = "--file";
    let mock = MockServer::start().await;
    let alerts = "/v1/organizations/org-1/services/svc-1/clickstack/alerts";
    let webhooks = "/v1/organizations/org-1/services/svc-1/clickstack/webhooks";
    for (method_name, route, result) in [
        ("GET", alerts.to_owned(), serde_json::json!([{}])),
        ("GET", format!("{alerts}/alert-get"), serde_json::json!({})),
        (
            "POST",
            alerts.to_owned(),
            serde_json::json!({"id":"alert-created"}),
        ),
        (
            "PUT",
            format!("{alerts}/alert-update"),
            serde_json::json!({"id":"alert-update"}),
        ),
        (
            "GET",
            webhooks.to_owned(),
            serde_json::json!([{"service":"generic"}]),
        ),
        (
            "POST",
            webhooks.to_owned(),
            serde_json::json!({"service":"generic","id":"hook-created"}),
        ),
        (
            "PUT",
            format!("{webhooks}/hook-update"),
            serde_json::json!({"service":"generic","id":"hook-update"}),
        ),
    ] {
        Mock::given(method(method_name))
            .and(path(route))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":result})),
            )
            .expect(1)
            .mount(&mock)
            .await;
    }
    for route in [
        format!("{alerts}/alert-delete"),
        format!("{webhooks}/hook-delete"),
    ] {
        Mock::given(method("DELETE"))
            .and(path(route))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status":200})),
            )
            .expect(1)
            .mount(&mock)
            .await;
    }

    let alert = serde_json::json!({
        "source":"tile", "dashboardId":"dash-1", "tileId":"tile-1", "groupBy":"service",
        "threshold":10.0, "thresholdMax":20.0, "interval":"30s", "thresholdType":"between",
        "scheduleOffsetMinutes":0, "scheduleStartAt":"2026-09-05T10:00:00Z",
        "channel":{"type":"webhook","webhookId":"hook-1","webhookService":"pagerduty_api",
            "slackChannelId":"C123","severity":"critical"},
        "channels":[
            {"type":"webhook","webhookId":"hook-1","webhookService":"pagerduty_api","severity":"warning"},
            {"type":"email","emailRecipients":["ops@example.com"]}
        ],
        "name":"Latency", "message":"Slow", "note":"Runbook", "numConsecutiveWindows":3
    });
    let webhook = serde_json::json!({
        "name":"Receiver", "service":"generic", "url":"https://example.com/hook",
        "description":"Production", "body":"{\"title\":\"{{title}}\"}",
        "headers":{"Authorization":"Bearer secret"}, "queryParams":{"team":"ops"}
    });
    let directory = tempfile::tempdir().unwrap();
    let alert_file = directory.path().join("alert.json");
    let webhook_file = directory.path().join("webhook.json");
    std::fs::write(&alert_file, alert.to_string()).unwrap();
    std::fs::write(&webhook_file, webhook.to_string()).unwrap();
    let alert_path = alert_file.to_str().unwrap();
    let webhook_path = webhook_file.to_str().unwrap();
    let alert_stdin = alert.to_string();
    let webhook_stdin = webhook.to_string();
    let commands: Vec<(Vec<&str>, Option<&str>)> = vec![
        (
            vec!["clickstack", "alert", "list", "svc-1", "--org-id", "org-1"],
            None,
        ),
        (
            vec![
                "clickstack",
                "alert",
                "get",
                "svc-1",
                "alert-get",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "alert",
                "create",
                "svc-1",
                file_flag,
                alert_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "alert",
                "update",
                "svc-1",
                "alert-update",
                file_flag,
                "-",
                "--org-id",
                "org-1",
            ],
            Some(alert_stdin.as_str()),
        ),
        (
            vec![
                "clickstack",
                "alert",
                "delete",
                "svc-1",
                "alert-delete",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "webhook",
                "list",
                "svc-1",
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "webhook",
                "create",
                "svc-1",
                file_flag,
                webhook_path,
                "--org-id",
                "org-1",
            ],
            None,
        ),
        (
            vec![
                "clickstack",
                "webhook",
                "update",
                "svc-1",
                "hook-update",
                file_flag,
                "-",
                "--org-id",
                "org-1",
            ],
            Some(webhook_stdin.as_str()),
        ),
        (
            vec![
                "clickstack",
                "webhook",
                "delete",
                "svc-1",
                "hook-delete",
                "--org-id",
                "org-1",
            ],
            None,
        ),
    ];
    for (args, stdin) in commands {
        let output = invoke_clickstack_cli(&mock, &args, stdin, true);
        assert_success(&output);
        let printed = serde_json::from_slice::<Value>(&output.stdout).unwrap();
        // A delete confirms the id it was given, not the service's.
        if args[2] == "delete" {
            assert_eq!(printed, serde_json::json!({"deleted": args[4]}), "{args:?}");
        }
    }
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 9);
    assert!(requests.iter().all(|request| {
        request
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("Basic ")
    }));
    let bodies = requests
        .iter()
        .filter(|request| {
            matches!(
                request.method,
                wiremock::http::Method::POST | wiremock::http::Method::PUT
            )
        })
        .map(|request| request.body_json::<Value>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(bodies, vec![alert.clone(), alert, webhook.clone(), webhook]);
}

// ── ClickStack webhook validation and inline alerts (issue #1127) ──────────

#[tokio::test]
async fn clickstack_webhook_validate_posts_full_body_and_reports_results() {
    let route = "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/validate";
    let body = serde_json::json!({
        "name":"Receiver", "service":"generic", "url":"https://example.com/hook",
        "description":"Production", "body":"{\"title\":\"{{title}}\"}",
        "headers":{"Authorization":"Bearer secret"}, "queryParams":{"team":"ops"}
    });
    let stdin = body.to_string();
    for (result, json) in [
        (
            serde_json::json!({"valid":true,"errors":[],"normalized":{
                "name":"Receiver","service":"generic","url":"https://example.com/hook"
            }}),
            true,
        ),
        (
            serde_json::json!({"valid":false,"errors":[{"path":"url","message":"Invalid URL"}],"normalized":null}),
            false,
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"result":result.clone()})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "webhook",
                "validate",
                "svc-1",
                "--file",
                "-",
                "--org-id",
                "org-1",
            ],
            Some(stdin.as_str()),
            json,
        );
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, wiremock::http::Method::POST);
        assert_eq!(requests[0].url.path(), route);
        assert!(
            requests[0]
                .headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("Basic ")
        );
        assert_eq!(requests[0].body_json::<Value>().unwrap(), body);
        // A null `normalized` (an invalid body) is omitted from the output.
        let normalized = result.get("normalized").filter(|value| !value.is_null());
        if json {
            let output: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output.get("valid"), result.get("valid"));
            assert_eq!(output.get("errors"), result.get("errors"));
            assert_eq!(output.get("normalized"), normalized);
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let valid = result["valid"].as_bool().unwrap();
            assert!(stdout.contains(&valid.to_string()), "{stdout}");
            assert_eq!(stdout.contains("Invalid URL"), !valid, "{stdout}");
            assert_eq!(
                stdout.contains("https://example.com/hook"),
                normalized.is_some(),
                "{stdout}"
            );
        }
    }
}

#[tokio::test]
async fn clickstack_alert_list_and_name_selector_use_display_name() {
    let mock = MockServer::start().await;
    let alerts = "/v1/organizations/org-1/services/svc-1/clickstack/alerts";
    Mock::given(method("GET"))
        .and(path(alerts))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result":[
                {"id":"alert-1","displayName":"Checkout errors","name":"Errors hit {{value}}","tags":[]},
                {"id":"alert-2","displayName":"Latency","tags":[]}
            ]
        })))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{alerts}/alert-1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result":{"id":"alert-1","displayName":"Checkout errors","tags":[]}
        })))
        .expect(2)
        .mount(&mock)
        .await;

    let output = invoke_clickstack_cli(
        &mock,
        &["clickstack", "alert", "list", "svc-1", "--org-id", "org-1"],
        None,
        false,
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Checkout errors"), "{stdout}");
    assert!(!stdout.contains("Errors hit"), "{stdout}");

    for name in ["Checkout errors", "Errors hit {{value}}"] {
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "alert",
                "get",
                "svc-1",
                "--name",
                name,
                "--org-id",
                "org-1",
            ],
            None,
            true,
        );
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output["id"], "alert-1");
    }
}
