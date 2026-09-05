//! CLI request-shape regression tests.
//!
//! Each test invokes the real `clickhousectl` binary as a subprocess, pointed
//! at a local `wiremock` server impersonating the ClickHouse Cloud API. The
//! mock records the request body the binary sent, and the test asserts on
//! its JSON shape.
//!
//! This is the cheapest way to structurally guard against Al's `4f6c2ba` bug
//! class — handler regressions like `args.foo.clone().unwrap_or_default()`
//! that serialize `""` on the wire when the user didn't pass `--foo`. The
//! API rejects `""` for `undefinedOr(...)` fields; these tests rejected the
//! same shape locally in ~200ms without touching any cloud infrastructure.
//!
//! Tests run as cargo integration tests:
//!     cargo test -p clickhousectl --test cli_request_shape_test

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SERVICE_PROFILES_PATH: &str = "/v1/organizations/org-1/serviceProfiles";

/// Locate the `clickhousectl` binary. cargo populates `CARGO_BIN_EXE_<name>`
/// for integration tests in the same package — so this is just the absolute
/// path to the build output, no `cargo build` shellout needed.
fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// Start a wiremock server that accepts a clickpipe-create POST, returns a
/// stub ClickPipe JSON, and records the request body for later assertion.
async fn start_mock_clickpipes_api() -> MockServer {
    let mock = MockServer::start().await;

    // Stub response: minimum fields the CLI's `--json` output needs to
    // deserialize into a `ClickPipe`. The CLI prints whatever comes back;
    // we only care about the request body, which wiremock records.
    let stub_pipe = serde_json::json!({
        "result": {
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "stub",
            "state": "Stopped",
            "scaling": { "replicas": 1 },
            "source": {},
            "destination": { "database": "default" },
            "metrics": {},
        },
        "status": 200,
        "requestId": "stub-request-id"
    });

    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_pipe))
        .mount(&mock)
        .await;

    mock
}

/// Assert the binary exited zero, panicking with the captured stderr/stdout
/// so the failure cause is visible in the test output.
fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "clickhousectl exited {}\nstderr:\n{}\nstdout:\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

fn clear_inherited_env(command: &mut Command) {
    command.env_clear();
}

#[test]
fn clear_inherited_env_removes_agent_credentials_home_and_path() {
    let mut command = Command::new(clickhousectl_binary());
    command.envs([
        ("CLAUDECODE", "1"),
        ("CLICKHOUSE_CLOUD_API_KEY", "ambient-key"),
        ("CLICKHOUSE_CLOUD_API_SECRET", "ambient-secret"),
        ("HOME", "/ambient/home"),
        ("PATH", "/ambient/bin"),
    ]);

    clear_inherited_env(&mut command);

    assert!(command.get_envs().next().is_none());
}

/// Run the clickhousectl binary against the mock, returning the JSON body
/// the binary POSTed. Panics with the captured stderr if the binary exits
/// non-zero — a failure here is almost always a clap-parsing error, which
/// is itself a bug worth surfacing.
async fn invoke_cli_capture_body(mock: &MockServer, cli_args: &[&str]) -> Value {
    let mut full_args: Vec<&str> = vec!["cloud", "--url"];
    let url = mock.uri();
    full_args.push(&url);
    full_args.push("--json");
    full_args.extend(cli_args);

    // DO_NOT_TRACK (here and on every other spawn in this file) keeps the
    // binary's telemetry fully silent: no `~/.clickhouse/telemetry.json` write
    // in the developer's real home, no POST to the production endpoint.
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(&full_args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .output()
        .expect("failed to spawn clickhousectl");

    assert!(
        output.status.success(),
        "clickhousectl exited {} for args {:?}\nstderr:\n{}\nstdout:\n{}",
        output.status.code().unwrap_or(-1),
        full_args,
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    // The CLI's first call when --org-id is passed is just the
    // clickpipe-create POST. If a future change adds a discovery GET it
    // would appear here too; assert on the POST specifically.
    let post = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::POST)
        .expect("no POST request recorded by mock");
    serde_json::from_slice(&post.body).expect("POST body wasn't valid JSON")
}

async fn invoke_cli_capture_body_with_stdin(
    mock: &MockServer,
    cli_args: &[&str],
    stdin: &[u8],
) -> Value {
    let mut full_args: Vec<&str> = vec!["cloud", "--url"];
    let url = mock.uri();
    full_args.push(&url);
    full_args.push("--json");
    full_args.extend(cli_args);

    let mut child = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(&full_args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(stdin)
        .expect("write clickhousectl stdin");
    let output = child.wait_with_output().expect("wait for clickhousectl");
    assert_success(&output);

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let post = requests
        .iter()
        .find(|request| request.method == wiremock::http::Method::POST)
        .expect("no POST request recorded by mock");
    serde_json::from_slice(&post.body).expect("POST body wasn't valid JSON")
}

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

fn invoke_cli_with_cloud_credentials(mock: &MockServer, cli_args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url, "--json"];
    args.extend(cli_args);
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

fn invoke_cli_with_cloud_credentials_and_stdin(
    mock: &MockServer,
    cli_args: &[&str],
    stdin: &str,
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url, "--json"];
    args.extend(cli_args);
    let mut child = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(stdin.as_bytes())
        .expect("write clickhousectl stdin");
    child.wait_with_output().expect("wait for clickhousectl")
}

fn invoke_cli_with_cloud_credentials_human(
    mock: &MockServer,
    cli_args: &[&str],
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url];
    args.extend(cli_args);
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
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
        .env_clear()
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
    assert!(stdout.contains("Total remaining credits: - CHC"));
    assert!(stdout.contains(
        "| ID | Type        | Remaining (CHC) | Total (CHC) | Spent (CHC) | Start | Expires |"
    ));
    assert!(stdout.contains("| -  | promotional | 3.25"));
}

#[tokio::test]
async fn org_balance_human_output_handles_empty_balances() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/creditBalances"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "totalRemainingCredits": 0.0, "balances": [] },
            "status": 200,
            "requestId": "stub-empty-credit-balances"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output =
        invoke_cli_with_cloud_credentials_human(&mock, &["org", "balance", "--org-id", "org-1"]);

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Total remaining credits: 0 CHC\nNo active credit balances found\n"
    );
}

const ORG_ROLES_PATH: &str = "/v1/organizations/org-1/roles";
const BASIC_TEST_AUTH: &str = "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==";

fn organization_role_envelope(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-org-role"
    }))
}

#[tokio::test]
async fn organization_role_writes_use_expected_routes_auth_and_bodies() {
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
            "--config-file",
            config_file.to_str().unwrap(),
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&create);

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
            "org",
            "role",
            "update",
            "role-1",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
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
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|request| request.url.query().is_none()));
}

#[tokio::test]
async fn organization_role_reads_support_oauth_and_writes_fail_before_http() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(ORG_ROLES_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(organization_role_envelope(serde_json::json!([
            {"id": "role-1", "name": "reader", "type": "custom"}
        ])))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{ORG_ROLES_PATH}/role-1")))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(organization_role_envelope(serde_json::json!({
            "id": "role-1", "name": "reader", "type": "custom"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let invoke = |args: &[&str], stdin: Option<&str>| {
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(project.path())
            .args(["cloud", "--url", &mock.uri(), "--json"])
            .args(args);
        if let Some(input) = stdin {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            child.wait_with_output().unwrap()
        } else {
            command.output().unwrap()
        }
    };
    assert_success(&invoke(&["org", "role", "list", "--org-id", "org-1"], None));
    assert_success(&invoke(
        &["org", "role", "get", "role-1", "--org-id", "org-1"],
        None,
    ));
    let write = invoke(
        &[
            "org",
            "role",
            "create",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
        ],
        Some(r#"{"name":"x","actors":[],"policies":[]}"#),
    );
    assert_eq!(write.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&write.stderr).contains("API key"));
    assert_eq!(mock.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn organization_role_human_output_handles_sparse_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(ORG_ROLES_PATH))
        .respond_with(organization_role_envelope(serde_json::json!([
            {},
            {"id": "system-1", "name": "admin", "type": "system"},
            {"id": "role-1", "name": "reader", "type": "custom"}
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
    for heading in ["Name", "ID", "Type", "Actors", "Policies"] {
        assert!(
            stdout.contains(heading),
            "missing {heading} column:\n{stdout}"
        );
    }
    assert!(stdout.contains("| -"), "{stdout}");
    assert!(stdout.contains("| admin  | system-1 | system"), "{stdout}");
    assert!(stdout.contains("| reader | role-1   | custom"), "{stdout}");

    let get = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "role", "get", "role-sparse", "--org-id", "org-1"],
    );
    assert_success(&get);
}

#[tokio::test]
async fn organization_role_get_preserves_404_detail() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("{ORG_ROLES_PATH}/missing")))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "role not found",
            "requestId": "stub-role-not-found"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &["org", "role", "get", "missing", "--org-id", "org-1"],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("role not found"), "{stderr}");
}

#[tokio::test]
async fn organization_role_create_rejects_invalid_nested_config_before_http() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials_and_stdin(
        &mock,
        &[
            "org",
            "role",
            "create",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
        ],
        r#"{"name":"bad","actors":[],"policies":[{"allowDeny":"AUDIT","permissions":[],"resources":[],"tagz":{}}]}"#,
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tagz"), "{stderr}");
    assert!(mock.received_requests().await.unwrap().is_empty());
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
        .env_clear()
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
        .env_clear()
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
        .env_clear()
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

fn invoke_cli_without_cloud_credentials(
    mock: &MockServer,
    cli_args: &[String],
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let mut args = vec!["cloud".to_string(), "--url".to_string(), mock.uri()];
    args.extend_from_slice(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

fn write_project_api_credentials(root: &Path, key: &str, secret: &str) {
    let credentials_dir = root.join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(
        credentials_dir.join("credentials.json"),
        serde_json::to_vec(&serde_json::json!({
            "api_key": key,
            "api_secret": secret,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn invoke_api_key_login(project_dir: &Path) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .current_dir(project_dir)
        .args([
            "cloud",
            "auth",
            "login",
            "--api-key",
            "new-key",
            "--api-secret",
            "new-secret",
        ])
        .output()
        .expect("failed to spawn clickhousectl")
}

#[test]
fn api_key_login_preserves_malformed_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    let credentials_path = credentials_dir.join("credentials.json");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    let original = b"{malformed credentials\n";
    std::fs::write(&credentials_path, original).unwrap();

    let output = invoke_api_key_login(dir.path());

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Error: failed to parse ")
            && stderr.contains(".clickhouse/credentials.json"),
        "unexpected stderr: {stderr}",
    );
    assert_eq!(std::fs::read(credentials_path).unwrap(), original);
}

#[test]
fn logout_does_not_report_success_when_credentials_removal_fails() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(&credentials_dir).unwrap();
    std::fs::create_dir(credentials_dir.join("credentials.json")).unwrap();

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(dir.path())
        .args(["cloud", "auth", "logout", "--api-keys"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("API keys cleared"));
    assert!(credentials_dir.join("credentials.json").is_dir());
}

// ── Backup configuration validation (issue #425) ────────────────────────────

#[tokio::test]
async fn backup_config_rejects_incompatible_period_before_any_request() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--backup-period-hours",
            "12",
            "--backup-start-time",
            "02:00",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--backup-period-hours must be 24 or 48 when --backup-start-time is set")
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── Bring-your-own backup buckets (issue #576) ─────────────────────────────

const BACKUP_BUCKET_PATH: &str = "/v1/organizations/org-1/services/svc-1/backupBucket";
const TEST_BASIC_AUTH: &str = "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==";

#[tokio::test]
async fn backup_bucket_get_uses_oauth_and_preserves_sparse_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "bucketProvider": "AWS" },
            "status": 200,
            "requestId": "bucket-get",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "backup",
            "bucket",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .expect("run backup bucket get");

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "bucketProvider": "AWS" })
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].url.query().is_none());
}

#[tokio::test]
async fn backup_bucket_create_reads_secret_from_stdin_and_sends_exact_gcp_body() {
    let mock = MockServer::start().await;
    let expected = serde_json::json!({
        "bucketProvider": "GCP",
        "bucketPath": "gs://company-backups/clickhouse",
        "accessKeyId": "gcp-access",
        "secretAccessKey": "gcp-secret",
    });
    Mock::given(method("POST"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", TEST_BASIC_AUTH))
        .and(body_json(expected.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "bucketProvider": "GCP",
                "bucketPath": "gs://company-backups/clickhouse",
                "accessKeyId": "gcp-access"
            },
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let body = invoke_cli_capture_body_with_stdin(
        &mock,
        &[
            "backup",
            "bucket",
            "create",
            "svc-1",
            "--org-id",
            "org-1",
            "--config-file",
            "-",
        ],
        serde_json::to_string(&expected).unwrap().as_bytes(),
    )
    .await;
    assert_eq!(body, expected);
}

#[tokio::test]
async fn backup_bucket_update_reads_file_and_delete_use_the_service_resource_path() {
    let mock = MockServer::start().await;
    let expected = serde_json::json!({
        "bucketProvider": "AZURE",
        "containerName": "backups",
        "connectionString": "DefaultEndpointsProtocol=https;AccountName=company",
    });
    Mock::given(method("PATCH"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", TEST_BASIC_AUTH))
        .and(body_json(expected.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "bucketProvider": "AZURE", "containerName": "backups" },
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", TEST_BASIC_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "bucket-delete",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let config_dir = tempfile::tempdir().unwrap();
    let config = config_dir.path().join("azure-bucket.json");
    std::fs::write(&config, serde_json::to_vec(&expected).unwrap()).unwrap();
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "backup",
            "bucket",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--config-file",
            config.to_str().unwrap(),
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "bucketProvider": "AZURE", "containerName": "backups" })
    );

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["backup", "bucket", "delete", "svc-1", "--org-id", "org-1"],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "status": 200, "requestId": "bucket-delete" })
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| request.url.query().is_none()));
}

#[tokio::test]
async fn backup_bucket_invalid_stdin_unknown_provider_and_field_fail_before_http() {
    let mock = MockServer::start().await;

    for input in [
        b"not JSON".as_slice(),
        br#"{"bucketProvider":"S3"}"#.as_slice(),
        br#"{"bucketProvider":"AZURE","containerName":"backups","connectionString":"secret","bucketPath":"s3://wrong"}"#.as_slice(),
    ] {
        let url = mock.uri();
        let mut child = Command::new(clickhousectl_binary())
            .env("DO_NOT_TRACK", "1")
            .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
            .args([
                "cloud",
                "--url",
                &url,
                "--json",
                "backup",
                "bucket",
                "create",
                "svc-1",
                "--org-id",
                "org-1",
                "--config-file",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn backup_bucket_api_error_is_reported() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BACKUP_BUCKET_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "status": 503,
            "error": "backup bucket temporarily unavailable",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["backup", "bucket", "get", "svc-1", "--org-id", "org-1"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("backup bucket temporarily unavailable")
    );
}

#[tokio::test]
async fn backup_bucket_writes_reject_oauth_before_http() {
    let mock = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let config = project.path().join("bucket.json");
    std::fs::write(
        &config,
        br#"{"bucketProvider":"AWS","bucketPath":"s3://backups","iamRoleArn":"arn:aws:iam::123:role/backups","iamRoleSessionName":"clickhouse"}"#,
    )
    .unwrap();

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "backup",
            "bucket",
            "create",
            "svc-1",
            "--org-id",
            "org-1",
            "--config-file",
            config.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── Backup start time against the stored period (issue #642) ────────────────

/// The API keeps the stored period when a start-time update omits one, and
/// rejects the PATCH if that period is not 24 or 48. The CLI reads the
/// configuration first so the user gets a message naming the flag to pass.
const BACKUP_CONFIG_PATH: &str = "/v1/organizations/org-1/services/svc-1/backupConfiguration";

async fn mount_stored_backup_config(mock: &MockServer, period_hours: f64) {
    Mock::given(method("GET"))
        .and(path(BACKUP_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "backupPeriodInHours": period_hours,
                "backupRetentionPeriodInHours": 48.0,
            },
            "status": 200,
            "requestId": "stub-backup-config-get",
        })))
        .mount(mock)
        .await;
}

async fn mount_backup_config_patch(mock: &MockServer) {
    Mock::given(method("PATCH"))
        .and(path(BACKUP_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "backupPeriodInHours": 24.0,
                "backupRetentionPeriodInHours": 48.0,
                "backupStartTime": "02:00",
            },
            "status": 200,
            "requestId": "stub-backup-config-patch",
        })))
        .mount(mock)
        .await;
}

fn backup_config_update_args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "service",
        "backup-config",
        "update",
        "svc-1",
        "--org-id",
        "org-1",
        "--backup-start-time",
        "02:00",
    ];
    args.extend_from_slice(extra);
    args
}

#[tokio::test]
async fn backup_config_start_time_refuses_an_incompatible_stored_period() {
    let mock = MockServer::start().await;
    mount_stored_backup_config(&mock, 12.0).await;

    let output = invoke_cli_with_cloud_credentials(&mock, &backup_config_update_args(&[]));

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: the stored backup period is 12 hours, but --backup-start-time requires 24 or 48. \
         Pass --backup-period-hours 24 or --backup-period-hours 48 in the same call.\n"
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected only the configuration read");
    assert_eq!(requests[0].method, wiremock::http::Method::GET);
}

#[tokio::test]
async fn backup_config_start_time_patches_when_the_stored_period_is_compatible() {
    let mock = MockServer::start().await;
    mount_stored_backup_config(&mock, 24.0).await;
    mount_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(&mock, &backup_config_update_args(&[]));

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, wiremock::http::Method::GET);
    assert_eq!(requests[1].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[1].body).expect("PATCH body wasn't JSON");
    assert_eq!(body["backupStartTime"], "02:00");
    assert!(
        body.get("backupPeriodInHours").is_none(),
        "the period must stay absent so the API keeps the stored one: {body}"
    );

    let printed: Value = serde_json::from_slice(&output.stdout).expect("stdout wasn't JSON");
    assert_eq!(printed["backupStartTime"], "02:00");
}

#[tokio::test]
async fn backup_config_start_time_with_an_explicit_period_skips_the_read() {
    let mock = MockServer::start().await;
    mount_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &backup_config_update_args(&["--backup-period-hours", "48"]),
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "an explicit period needs no read");
    assert_eq!(requests[0].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[0].body).expect("PATCH body wasn't JSON");
    assert_eq!(body["backupStartTime"], "02:00");
    assert_eq!(body["backupPeriodInHours"], 48.0);
}

// ── Clearing the backup start time (issue #564) ─────────────────────────────

/// The Cloud API clears a stored start time only on an explicit JSON `null`
/// (an empty string is rejected as an invalid time), and clearing is the only
/// way back to a backup period other than 24 or 48 hours. The flag therefore
/// has to put the key in the body with a null value, and must not be talked
/// out of it by the stored-period read.
async fn mount_cleared_backup_config_patch(mock: &MockServer) {
    Mock::given(method("PATCH"))
        .and(path(BACKUP_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "backupPeriodInHours": 12.0,
                "backupRetentionPeriodInHours": 48.0,
            },
            "status": 200,
            "requestId": "stub-backup-config-clear",
        })))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn backup_config_clear_start_time_sends_an_explicit_null() {
    let mock = MockServer::start().await;
    mount_cleared_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--clear-backup-start-time",
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "clearing needs no stored-period read");
    assert_eq!(requests[0].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[0].body).expect("PATCH body wasn't JSON");
    assert_eq!(
        body,
        serde_json::json!({ "backupStartTime": null }),
        "the key must be present and null, not omitted: {body}"
    );
}

#[tokio::test]
async fn backup_config_clear_start_time_travels_with_an_incompatible_period() {
    let mock = MockServer::start().await;
    mount_cleared_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--clear-backup-start-time",
            "--backup-period-hours",
            "12",
            "--json",
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);

    let body: Value = serde_json::from_slice(&requests[0].body).expect("PATCH body wasn't JSON");
    assert_eq!(
        body,
        serde_json::json!({ "backupPeriodInHours": 12.0, "backupStartTime": null })
    );

    let printed: Value = serde_json::from_slice(&output.stdout).expect("stdout wasn't JSON");
    assert_eq!(printed["backupPeriodInHours"], 12.0);
    assert!(
        printed.get("backupStartTime").is_none(),
        "the cleared start time must be absent from the response: {printed}"
    );
}

#[tokio::test]
async fn backup_config_rejects_setting_and_clearing_the_start_time_together() {
    let mock = MockServer::start().await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--backup-start-time",
            "02:00",
            "--clear-backup-start-time",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot be used with"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── Concrete Cloud error routing (issue #233) ──────────────────────────────

async fn invoke_service_list_api_error(status: u16, message: &str) -> std::process::Output {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(
            ResponseTemplate::new(status).set_body_json(serde_json::json!({
                "status": status,
                "error": message,
                "requestId": format!("stub-{status}"),
            })),
        )
        .mount(&mock)
        .await;

    invoke_cli_with_cloud_credentials(&mock, &["service", "list", "--org-id", "org-1"])
}

#[tokio::test]
async fn dispatched_cloud_401_exits_with_auth_required() {
    let output = invoke_service_list_api_error(401, "Unauthorized").await;
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Unauthorized\n"
    );
}

#[tokio::test]
async fn dispatched_cloud_403_exits_with_auth_required() {
    let output = invoke_service_list_api_error(403, "Forbidden").await;
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Forbidden\n"
    );
}

#[tokio::test]
async fn dispatched_cloud_500_remains_a_generic_error() {
    let output = invoke_service_list_api_error(500, "Internal Server Error").await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Internal Server Error\n"
    );
}

// ── Organization-scoped error context (issue #334) ─────────────────────────

const MANUAL_QUERY_ENDPOINT_PATH: &str =
    "/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint";

async fn manual_query_endpoint_mock(get_response: ResponseTemplate) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(MANUAL_QUERY_ENDPOINT_PATH))
        .respond_with(get_response)
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(MANUAL_QUERY_ENDPOINT_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "endpoint-1" },
            "status": 200,
            "requestId": "stub-query-endpoint-create"
        })))
        .mount(&mock)
        .await;
    mock
}

fn manual_query_endpoint_args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "service",
        "query-endpoint",
        "create",
        "svc-1",
        "--org-id",
        "org-1",
        "--role",
        "sql_console_read_only",
    ];
    args.extend_from_slice(extra);
    args
}

fn manual_query_endpoint_response(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result, "status": 200, "requestId": "stub-query-endpoint-get"
    }))
}

#[tokio::test]
async fn query_endpoint_create_sends_typed_roles_and_explicit_first_origins() {
    for origins in ["https://app.example.com", "*"] {
        let mock = manual_query_endpoint_mock(ResponseTemplate::new(404)).await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &manual_query_endpoint_args(&[
                "--role",
                "sql_console_admin",
                "--open-api-key",
                "key-1",
                "--allowed-origins",
                origins,
            ]),
        );
        assert_success(&output);
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result, serde_json::json!({"id": "endpoint-1"}));
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, wiremock::http::Method::GET);
        assert_eq!(requests[1].method, wiremock::http::Method::POST);
        assert_eq!(
            requests[1].body_json::<Value>().unwrap(),
            serde_json::json!({
                "roles": ["sql_console_read_only", "sql_console_admin"],
                "openApiKeys": ["key-1"], "allowedOrigins": origins,
            })
        );
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
    }
}

#[tokio::test]
async fn query_endpoint_create_preserves_existing_keys_and_default_origins() {
    let mock = manual_query_endpoint_mock(manual_query_endpoint_response(serde_json::json!({
        "id": "endpoint-1", "roles": ["sql_console_admin"],
        "openApiKeys": ["existing-1", "existing-2", "existing-1"],
        "allowedOrigins": "https://before.example.com",
    })))
    .await;
    let body = invoke_cli_capture_body(
        &mock,
        &manual_query_endpoint_args(&[
            "--open-api-key",
            "existing-2",
            "--open-api-key",
            "new-key",
            "--open-api-key",
            "new-key",
        ]),
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({
            "roles": ["sql_console_read_only"],
            "openApiKeys": ["existing-1", "existing-2", "new-key"],
            "allowedOrigins": "https://before.example.com",
        })
    );
}

#[tokio::test]
async fn query_endpoint_create_explicit_origins_replace_without_dropping_keys() {
    for origins in ["https://after.example.com", "*"] {
        let mock = manual_query_endpoint_mock(manual_query_endpoint_response(serde_json::json!({
            "openApiKeys": ["existing-1", "existing-2"],
            "allowedOrigins": "https://before.example.com",
        })))
        .await;
        let body = invoke_cli_capture_body(
            &mock,
            &manual_query_endpoint_args(&["--allowed-origins", origins]),
        )
        .await;
        assert_eq!(
            body,
            serde_json::json!({
                "roles": ["sql_console_read_only"],
                "openApiKeys": ["existing-1", "existing-2"], "allowedOrigins": origins,
            })
        );
    }
}

#[tokio::test]
async fn query_endpoint_create_key_replacement_is_explicit() {
    let mock = manual_query_endpoint_mock(manual_query_endpoint_response(serde_json::json!({
        "openApiKeys": ["existing-1", "existing-2"],
        "allowedOrigins": "https://before.example.com",
    })))
    .await;
    let body = invoke_cli_capture_body(
        &mock,
        &manual_query_endpoint_args(&[
            "--open-api-key",
            "new-key",
            "--open-api-key",
            "new-key",
            "--replace-open-api-keys",
        ]),
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({
            "roles": ["sql_console_read_only"], "openApiKeys": ["new-key"],
            "allowedOrigins": "https://before.example.com",
        })
    );
}

#[tokio::test]
async fn query_endpoint_create_refuses_failed_or_incomplete_get_without_writing() {
    let cases = [
        (
            ResponseTemplate::new(403),
            vec!["--allowed-origins", "*"],
            4,
        ),
        (
            ResponseTemplate::new(503),
            vec![
                "--allowed-origins",
                "*",
                "--replace-open-api-keys",
                "--open-api-key",
                "new-key",
            ],
            1,
        ),
        (
            ResponseTemplate::new(200).set_body_string("invalid JSON"),
            vec![],
            1,
        ),
        (ResponseTemplate::new(404), vec![], 1),
        (
            manual_query_endpoint_response(
                serde_json::json!({"allowedOrigins": "https://before.example.com"}),
            ),
            vec!["--open-api-key", "new-key"],
            1,
        ),
        (
            manual_query_endpoint_response(serde_json::json!({"openApiKeys": ["existing-1"]})),
            vec![],
            1,
        ),
        (
            manual_query_endpoint_response(serde_json::json!(null)),
            vec!["--allowed-origins", "*"],
            1,
        ),
    ];
    for (response, flags, exit_code) in cases {
        let mock = manual_query_endpoint_mock(response).await;
        let output = invoke_cli_with_cloud_credentials(&mock, &manual_query_endpoint_args(&flags));
        assert_eq!(
            output.status.code(),
            Some(exit_code),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, wiremock::http::Method::GET);
    }
}

#[tokio::test]
async fn query_endpoint_create_omitted_role_fails_before_contacting_api() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "query-endpoint",
            "create",
            "svc-1",
            "--org-id",
            "org-1",
            "--open-api-key",
            "new-key",
            "--allowed-origins",
            "*",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_list_bare_not_found_includes_the_requested_organization() {
    const WRONG_ORG_ID: &str = "00000000-0000-4000-8000-000000000001";
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/organizations/{WRONG_ORG_ID}/services")))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "NOT_FOUND",
            "requestId": "stub-wrong-org",
        })))
        .mount(&mock)
        .await;

    let output =
        invoke_cli_with_cloud_credentials(&mock, &["service", "list", "--org-id", WRONG_ORG_ID]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("Error: NOT_FOUND: request scoped to organization {WRONG_ORG_ID}\n")
    );
}

#[tokio::test]
async fn service_get_preserves_a_detailed_not_found_error() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/missing-service"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "Service missing-service was not found",
            "requestId": "stub-missing-service",
        })))
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["service", "get", "missing-service", "--org-id", "org-1"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Service missing-service was not found\n"
    );
}

fn invoke_service_wake(mock: &MockServer, json: bool, agent: bool) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec![
        "cloud", "--url", &url, "service", "wake", "svc-1", "--org-id", "org-1",
    ];
    if json {
        args.push("--json");
    }
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    if agent {
        command.env("AGENT", "opencode");
    }
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "wake-test-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "wake-test-secret")
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to run service wake")
}

#[tokio::test]
async fn service_wake_sends_awake_with_basic_auth_and_prints_json() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .and(wiremock::matchers::basic_auth(
            "wake-test-key",
            "wake-test-secret",
        ))
        .and(body_json(serde_json::json!({"command": "awake"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "11111111-2222-3333-4444-555555555555", "name": "demo", "state": "awaking"},
            "status": 200,
            "requestId": "stub-service-wake",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_service_wake(&mock, true, false);
    assert_success(&output);
    assert!(output.stderr.is_empty());
    let service: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(service["name"], "demo");
    assert_eq!(service["state"], "awaking");
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn service_wake_handles_a_sparse_response_in_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .and(body_json(serde_json::json!({"command": "awake"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {}, "status": 200, "requestId": "stub-service-wake",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_service_wake(&mock, false, false);
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Service - waking (state: -)\n"
    );
}

#[tokio::test]
async fn service_wake_uses_json_output_for_a_detected_agent() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .and(body_json(serde_json::json!({"command": "awake"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": "demo", "state": "awaking"},
            "status": 200,
            "requestId": "stub-service-wake",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_service_wake(&mock, false, true);
    assert_success(&output);
    let service: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(service["name"], "demo");
    assert_eq!(service["state"], "awaking");
}

#[tokio::test]
async fn service_wake_preserves_api_errors() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .and(body_json(serde_json::json!({"command": "awake"})))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "status": 503, "error": "wake temporarily unavailable",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_service_wake(&mock, true, false);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: wake temporarily unavailable\n"
    );
}

#[tokio::test]
async fn service_wake_rejects_oauth_before_http() {
    let mock = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "service",
            "wake",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("failed to run OAuth service wake");

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── Credential precedence visibility (issue #336) ─────────────────────────

#[tokio::test]
async fn credentials_file_reports_that_environment_credentials_are_ignored() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [],
            "status": 200,
            "requestId": "stub-org-list",
        })))
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_project_api_credentials(dir.path(), "file-key", "file-secret");
    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
        .current_dir(dir.path())
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "note: CLICKHOUSE_CLOUD_API_KEY and CLICKHOUSE_CLOUD_API_SECRET are set but ignored; \
         using credentials file — see --debug\n"
    );

    let requests = mock.received_requests().await.unwrap();
    let authorization = requests[0]
        .headers
        .get("Authorization")
        .unwrap()
        .to_str()
        .unwrap();
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "file-key:file-secret",
        )
    );
    assert_eq!(authorization, expected);
}

#[test]
fn auth_status_marks_outranked_environment_credentials_inactive() {
    let dir = tempfile::tempdir().unwrap();
    write_project_api_credentials(dir.path(), "file-key", "file-secret");
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
        .current_dir(dir.path())
        .args(["cloud", "--json", "auth", "status"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);
    assert!(output.stderr.is_empty());
    let rows: Value = serde_json::from_slice(&output.stdout).unwrap();
    let env = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["type"] == "Env vars")
        .unwrap();
    assert_eq!(
        env["status"],
        "Configured (inactive, outranked by credentials file)"
    );
    assert_eq!(env["active"], "-");
}

// ── Service deletion errors (issue #335) ──────────────────────────────────

#[tokio::test]
async fn service_delete_running_conflict_suggests_force() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "status": 409,
            "error": "CONFLICT: Only instance in one of the following states: \
                      'provisioning','starting','awaking','idle','stopped','degraded','failed' \
                      can be terminated. Current state: 'running'"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["service", "delete", "svc-1", "--org-id", "org-1"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: service is running and cannot be deleted. Use --force to stop it first, or \
         `clickhousectl cloud service stop svc-1`.\n"
    );
}

// ── Postgres metrics (issue #583) ─────────────────────────────────────────

#[tokio::test]
async fn postgres_metrics_sends_exact_query_supports_oauth_and_preserves_json() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    let result = serde_json::json!({
        "metrics": [{
            "key": "cpu",
            "name": "CPU usage",
            "description": "Average CPU usage",
            "unit": "percent",
            "series": [{
                "label": "primary",
                "dataPoints": [
                    { "timestamp": 1776337200, "value": 12.5 },
                    { "timestamp": 1776337260, "value": 13.25 }
                ]
            }]
        }, {
            "key": "replication-lag",
            "series": []
        }]
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}/metrics"
        )))
        .and(query_param("from_date", "2026-04-16T12:00:00+01:00"))
        .and(query_param("to_date", "2026-04-16T13:00:00+01:00"))
        .and(query_param("bucket_size_seconds", "60"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-postgres-metrics"
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
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "metrics",
            postgres_id,
            "--from-date",
            "2026-04-16T12:00:00+01:00",
            "--to-date",
            "2026-04-16T13:00:00+01:00",
            "--bucket-size-seconds",
            "60",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        [
            (
                "from_date".to_string(),
                "2026-04-16T12:00:00+01:00".to_string()
            ),
            (
                "to_date".to_string(),
                "2026-04-16T13:00:00+01:00".to_string()
            ),
            ("bucket_size_seconds".to_string(), "60".to_string()),
        ]
    );
}

#[tokio::test]
async fn postgres_metrics_omits_bucket_and_renders_sparse_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/metrics"))
        .and(query_param("from_date", "2026-04-16T12:00:00Z"))
        .and(query_param("to_date", "2026-04-16T13:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "metrics": [{
                    "key": "connections",
                    "series": [{ "dataPoints": [{ "timestamp": 1776337200 }] }]
                }]
            },
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
            "--to-date",
            "2026-04-16T13:00:00Z",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for text in [
        "metrics:",
        "key: connections",
        "dataPoints:",
        "timestamp: 1776337200",
    ] {
        assert!(stdout.contains(text), "missing {text:?} from:\n{stdout}");
    }
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2, "unexpected query parameters: {query:?}");
    assert!(
        query.iter().all(|(key, _)| key != "bucket_size_seconds"),
        "bucket size must be omitted when the flag is absent: {query:?}"
    );
}

#[tokio::test]
async fn postgres_metrics_rejects_reverse_range_before_request_and_surfaces_api_errors() {
    let mock = MockServer::start().await;
    let base_args = [
        "postgres",
        "metrics",
        "pg-1",
        "--from-date",
        "2026-04-16T13:00:00Z",
        "--to-date",
        "2026-04-16T12:00:00Z",
        "--org-id",
        "org-1",
    ];
    let output = invoke_cli_with_cloud_credentials(&mock, &base_args);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--from-date must not be after --to-date")
    );
    assert!(mock.received_requests().await.unwrap().is_empty());

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/metrics"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "status": 429,
            "error": "RATE_LIMIT_EXCEEDED: try later",
            "requestId": "stub-postgres-metrics-error"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let valid_args = [
        "postgres",
        "metrics",
        "pg-1",
        "--from-date",
        "2026-04-16T12:00:00Z",
        "--to-date",
        "2026-04-16T13:00:00Z",
        "--org-id",
        "org-1",
    ];
    let output = invoke_cli_with_cloud_credentials(&mock, &valid_args);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("RATE_LIMIT_EXCEEDED: try later"),
        "{stderr}"
    );
}

// ── Postgres logs (issue #582) ────────────────────────────────────────────

fn invoke_postgres_logs(
    mock: &MockServer,
    home: &Path,
    json: bool,
    args: &[&str],
) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(home)
        .args(["cloud", "--url", &mock.uri()]);
    if json {
        command.arg("--json");
    }
    command.args(["postgres", "logs", "pg-1"]).args(args);
    command.output().expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn postgres_logs_routes_all_query_parameters_and_preserves_json_with_oauth() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([
        {
            "timestamp": "2026-08-01T12:00:00Z",
            "severity": "LOG",
            "body": "checkpoint complete"
        },
        { "severity": "WARNING" }
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-postgres-logs"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = invoke_postgres_logs(
        &mock,
        &home,
        true,
        &[
            "--from-date",
            "2026-08-01T00:00:00+01:00",
            "--to-date",
            "2026-08-02T00:00:00+01:00",
            "--body-contains",
            "checkpoint complete",
            "--severity",
            "LOG",
            "--sort-order",
            "asc",
            "--limit",
            "2000",
            "--offset",
            "0",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].body.is_empty(), "GET must not carry a body");
    let query: std::collections::BTreeMap<_, _> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        std::collections::BTreeMap::from([
            (
                "body_contains".to_string(),
                "checkpoint complete".to_string()
            ),
            (
                "from_date".to_string(),
                "2026-08-01T00:00:00+01:00".to_string()
            ),
            ("limit".to_string(), "2000".to_string()),
            ("offset".to_string(), "0".to_string()),
            ("severity".to_string(), "LOG".to_string()),
            ("sort_order".to_string(), "asc".to_string()),
            (
                "to_date".to_string(),
                "2026-08-02T00:00:00+01:00".to_string()
            ),
        ])
    );
}

#[tokio::test]
async fn postgres_logs_minimal_query_and_sparse_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{ "severity": "WARNING" }, { "body": "recovery complete" }],
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let home = tempfile::tempdir().unwrap();
    let output = invoke_postgres_logs(
        &mock,
        home.path(),
        false,
        &[
            "--from-date",
            "2026-08-01T00:00:00Z",
            "--to-date",
            "2026-08-02T00:00:00Z",
            "--org-id",
            "org-1",
            "--api-key",
            "logs-key",
            "--api-secret",
            "logs-secret",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("severity: WARNING"), "{stdout}");
    assert!(stdout.contains("body: recovery complete"), "{stdout}");
    assert!(!stdout.contains("timestamp:"), "{stdout}");
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.query(),
        Some("from_date=2026-08-01T00%3A00%3A00Z&to_date=2026-08-02T00%3A00%3A00Z")
    );
}

#[tokio::test]
async fn postgres_logs_reports_empty_results_and_structured_api_errors() {
    for (response, expected_status, expected_text) in [
        (
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [], "status": 200
            })),
            0,
            "No Postgres logs found",
        ),
        (
            ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "status": 400,
                "error": "BAD_REQUEST: invalid log window",
                "requestId": "stub-invalid-window"
            })),
            1,
            "invalid log window",
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;
        let home = tempfile::tempdir().unwrap();
        let output = invoke_postgres_logs(
            &mock,
            home.path(),
            false,
            &[
                "--from-date",
                "2026-08-01T00:00:00Z",
                "--to-date",
                "2026-08-02T00:00:00Z",
                "--org-id",
                "org-1",
                "--api-key",
                "logs-key",
                "--api-secret",
                "logs-secret",
            ],
        );
        assert_eq!(output.status.code(), Some(expected_status));
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(combined.contains(expected_text), "{combined}");
    }
}

// ── Postgres slow query patterns (issue #585) ─────────────────────────────

#[tokio::test]
async fn postgres_slow_query_list_sends_exact_query_supports_oauth_and_preserves_json() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([{
        "queryId": "query-1",
        "queryText": "SELECT * FROM events WHERE id = $1",
        "dbName": "app db",
        "dbUser": "reader+worker",
        "dbOperation": "SELECT",
        "app": "reporting/api",
        "callCount": 42,
        "errorCount": 1,
        "totalDurationUs": 950000,
        "avgDurationUs": 22619,
        "maxDurationUs": 81000,
        "p50DurationUs": 18000,
        "p95DurationUs": 70000,
        "p99DurationUs": 80000,
        "totalRows": 420,
        "totalSharedBlksRead": 12,
        "totalSharedBlksHit": 900,
        "totalCpuTimeUs": 700000,
        "totalWalBytes": 128
    }]);
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-slow-query-list"
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
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00+01:00",
            "--to-date",
            "2026-04-16T13:00:00+01:00",
            "--db-name",
            "app db",
            "--db-user",
            "reader+worker",
            "--db-operation",
            "SELECT & EXPLAIN",
            "--app",
            "reporting/api",
            "--sort-by",
            "total_cpu_time",
            "--sort-order",
            "asc",
            "--limit",
            "500",
            "--offset",
            "0",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        [
            (
                "from_date".to_string(),
                "2026-04-16T12:00:00+01:00".to_string()
            ),
            (
                "to_date".to_string(),
                "2026-04-16T13:00:00+01:00".to_string()
            ),
            ("db_name".to_string(), "app db".to_string()),
            ("db_user".to_string(), "reader+worker".to_string()),
            ("db_operation".to_string(), "SELECT & EXPLAIN".to_string()),
            ("app".to_string(), "reporting/api".to_string()),
            ("sort_by".to_string(), "total_cpu_time".to_string()),
            ("sort_order".to_string(), "asc".to_string()),
            ("limit".to_string(), "500".to_string()),
            ("offset".to_string(), "0".to_string()),
        ]
    );
}

#[tokio::test]
async fn postgres_slow_query_list_omits_filters_and_renders_sparse_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{ "queryId": "query-1", "callCount": 2 }],
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
            "--to-date",
            "2026-04-16T13:00:00Z",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("queryId: query-1"), "{stdout}");
    assert!(stdout.contains("callCount: 2"), "{stdout}");
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2, "unexpected query parameters: {query:?}");
}

#[tokio::test]
async fn postgres_slow_query_list_rejects_reverse_range_and_surfaces_api_errors() {
    let mock = MockServer::start().await;
    let reverse_args = [
        "postgres",
        "slow-queries",
        "list",
        "pg-1",
        "--from-date",
        "2026-04-16T13:00:00Z",
        "--to-date",
        "2026-04-16T12:00:00Z",
        "--org-id",
        "org-1",
    ];
    let output = invoke_cli_with_cloud_credentials(&mock, &reverse_args);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--from-date must not be after --to-date")
    );
    assert!(mock.received_requests().await.unwrap().is_empty());

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "status": 429,
            "error": "RATE_LIMIT_EXCEEDED: try later",
            "requestId": "stub-slow-query-error"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let valid_args = [
        "postgres",
        "slow-queries",
        "list",
        "pg-1",
        "--from-date",
        "2026-04-16T12:00:00Z",
        "--to-date",
        "2026-04-16T13:00:00Z",
        "--org-id",
        "org-1",
    ];
    let output = invoke_cli_with_cloud_credentials(&mock, &valid_args);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("RATE_LIMIT_EXCEEDED: try later"));
}

#[tokio::test]
async fn postgres_slow_query_get_has_distinct_query_and_preserves_recent_executions() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "aggregate": {
            "queryId": "query-1",
            "queryText": "SELECT $1",
            "dbName": "app db",
            "dbUser": "reader+worker",
            "dbOperation": "SELECT & EXPLAIN",
            "app": "reporting/api",
            "callCount": 2
        },
        "recentExecutions": [{
            "queryId": "query-1",
            "queryText": "SELECT 42",
            "timestamp": "2026-04-16T12:30:00Z",
            "durationUs": 1234,
            "rows": 1,
            "errMessage": "optional detail"
        }]
    });
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns/query-1",
        ))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-slow-query-detail"
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
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app db",
            "--db-user",
            "reader+worker",
            "--db-operation",
            "SELECT & EXPLAIN",
            "--app",
            "reporting/api",
            "--timestamp",
            "2026-04-16T12:30:00+01:00",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    assert_eq!(
        query,
        [
            ("db_name".to_string(), "app db".to_string()),
            ("db_user".to_string(), "reader+worker".to_string()),
            ("db_operation".to_string(), "SELECT & EXPLAIN".to_string()),
            ("app".to_string(), "reporting/api".to_string()),
            (
                "timestamp".to_string(),
                "2026-04-16T12:30:00+01:00".to_string()
            ),
        ]
    );
}

#[tokio::test]
async fn postgres_slow_query_get_omits_optional_query_and_renders_sparse_detail() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns/query-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "aggregate": { "queryId": "query-1" },
                "recentExecutions": [{ "timestamp": "2026-04-16T12:30:00Z" }]
            },
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app",
            "--db-user",
            "reader",
            "--db-operation",
            "SELECT",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for text in [
        "aggregate:",
        "queryId: query-1",
        "recentExecutions:",
        "timestamp: 2026-04-16T12:30:00Z",
    ] {
        assert!(stdout.contains(text), "missing {text:?} from:\n{stdout}");
    }
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 3, "unexpected query parameters: {query:?}");
    assert!(
        query
            .iter()
            .all(|(key, _)| key != "app" && key != "timestamp"),
        "optional detail filters must be omitted: {query:?}"
    );
}

// ── Postgres Prometheus metrics (issue #584) ──────────────────────────────

#[tokio::test]
async fn postgres_prometheus_service_uses_oauth_and_returns_a_json_string() {
    let mock = MockServer::start().await;
    let metrics = "# HELP pg_up Whether Postgres is available.\n# TYPE pg_up gauge\npg_up 1\n";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/prometheus"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain; charset=UTF-8")
                .set_body_string(metrics),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "postgres",
            "prometheus",
            "service",
            "pg-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<String>(&output.stdout).unwrap(),
        metrics
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn postgres_prometheus_org_preserves_raw_text_and_agent_mode_is_json() {
    let mock = MockServer::start().await;
    let metrics = "# TYPE pg_connections gauge\npg_connections{service=\"pg-1\"} 4\n";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain; charset=UTF-8")
                .set_body_string(metrics),
        )
        .expect(2)
        .mount(&mock)
        .await;

    let args = ["postgres", "prometheus", "org", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    assert_eq!(human.stdout, metrics.as_bytes());

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let agent = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("AI_AGENT", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args(["cloud", "--url", &mock.uri()])
        .args(args)
        .output()
        .unwrap();
    assert_success(&agent);
    assert_eq!(
        serde_json::from_slice::<String>(&agent.stdout).unwrap(),
        metrics
    );

    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), None);
    }
}

#[tokio::test]
async fn postgres_prometheus_converts_api_errors() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/prometheus"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Forbidden",
            "requestId": "stub-postgres-prometheus-error"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "prometheus",
            "service",
            "pg-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Forbidden\n"
    );
}

// ── Postgres deletion JSON output (issue #614) ────────────────────────────

/// `postgres delete --json` must emit the Postgres resource object, not the
/// raw `{"status":...,"requestId":...}` API envelope the delete endpoint
/// itself returns: the handler fetches the resource before issuing the
/// delete and renders that instead, consistent with every other
/// `cloud postgres` subcommand.
#[tokio::test]
async fn postgres_delete_json_emits_the_resource_object_not_the_envelope() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": postgres_id,
                "name": "my-postgres",
                "state": "running",
            },
            "status": 200,
            "requestId": "stub-postgres-get",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}"
        )))
        .respond_with(successful_delete_response("stub-postgres-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["postgres", "delete", postgres_id, "--org-id", "org-1"],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource object as JSON");
    assert_eq!(stdout["id"], postgres_id);
    assert_eq!(stdout["name"], "my-postgres");
    assert_eq!(stdout["state"], "running");
    assert!(
        stdout.get("status").is_none() && stdout.get("requestId").is_none(),
        "must not emit the raw API envelope, got: {stdout}"
    );
}

// ── Postgres update --name (issue #663) ────────────────────────────────────

/// `postgres update --name` must PATCH the API with only `name` set: no
/// `size`, `haType`, or `tags` key should appear when only `--name` is
/// passed, and no discovery `GET` is issued (no tag diff was requested).
#[tokio::test]
async fn postgres_update_name_sends_only_the_name_field() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{postgres_id}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": postgres_id,
                "name": "renamed-pg",
                "state": "running",
            },
            "status": 200,
            "requestId": "stub-postgres-update",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            postgres_id,
            "--org-id",
            "org-1",
            "--name",
            "renamed-pg",
        ],
    );

    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected only the PATCH, no discovery GET"
    );
    assert_eq!(requests[0].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[0].body).expect("PATCH body wasn't JSON");
    assert_eq!(body["name"], "renamed-pg");
    assert!(
        body.get("size").is_none() && body.get("haType").is_none() && body.get("tags").is_none(),
        "unexpected keys in PATCH body: {body}"
    );
}

// ── Postgres list --filter validation (issue #603) ────────────────────────

fn postgres_list_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": [
            {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "primary-pg",
                "state": "running",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": true,
            },
            {
                "id": "66666666-7777-8888-9999-000000000000",
                "name": "replica-pg",
                "state": "restoring_backup",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": false,
            },
            {
                // Absent `isPrimary`/`state`: must match no filter value.
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "unknown-pg",
                "region": "us-east-1",
            },
        ],
        "status": 200,
        "requestId": "stub-postgres-list",
    }))
}

/// An unsupported `--filter` key used to return the whole unfiltered list with
/// exit 0. It is now a clap usage error (exit 2) that names the valid keys, and
/// no request reaches the API.
#[tokio::test]
async fn postgres_list_rejects_an_unknown_filter_key_before_calling_the_api() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(postgres_list_response())
        .expect(0)
        .mount(&mock)
        .await;

    for filter in ["bogus=1", "state=", "isPrimary=maybe", "state"] {
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["postgres", "list", "--org-id", "org-1", "--filter", filter],
        );
        assert_eq!(
            output.status.code(),
            Some(2),
            "--filter {filter} must be a usage error, stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "--filter {filter} must print no results, got: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres", "list", "--org-id", "org-1", "--filter", "bogus=1",
        ],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown filter key 'bogus'"), "{stderr}");
    assert!(
        stderr.contains("state, region, name, provider, isPrimary"),
        "the valid keys must be listed, got: {stderr}"
    );
}

/// `isPrimary` is a supported key (it is the `Primary` column), `state` matches
/// the serde wire value including multi-word states, and an item whose field the
/// API omitted matches nothing.
#[tokio::test]
async fn postgres_list_applies_supported_filters_client_side() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(postgres_list_response())
        .mount(&mock)
        .await;

    let names = |filter: &str| -> Vec<String> {
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["postgres", "list", "--org-id", "org-1", "--filter", filter],
        );
        assert_success(&output);
        let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
            .expect("stdout should be a JSON array");
        stdout
            .as_array()
            .expect("array")
            .iter()
            .map(|item| item["name"].as_str().unwrap_or_default().to_string())
            .collect()
    };

    assert_eq!(names("isPrimary=true"), vec!["primary-pg".to_string()]);
    assert_eq!(names("isPrimary=false"), vec!["replica-pg".to_string()]);
    assert_eq!(
        names("state=restoring_backup"),
        vec!["replica-pg".to_string()]
    );
    assert_eq!(names("state=running"), vec!["primary-pg".to_string()]);
    assert_eq!(
        names("region=us-east-1"),
        vec![
            "primary-pg".to_string(),
            "replica-pg".to_string(),
            "unknown-pg".to_string()
        ]
    );
    assert_eq!(names("name=nope"), Vec::<String>::new());
}

// ── Postgres promote / switchover role changes (issue #604) ───────────────

const ROLE_TEST_POSTGRES_ID: &str = "11111111-2222-3333-4444-555555555555";

fn postgres_role_service(ha_type: &str, is_primary: Option<bool>) -> serde_json::Value {
    let mut service = serde_json::json!({
        "id": ROLE_TEST_POSTGRES_ID,
        "name": "my-postgres",
        "state": "running",
        "haType": ha_type,
    });
    if let Some(is_primary) = is_primary {
        service["isPrimary"] = serde_json::json!(is_primary);
    }
    service
}

/// Mounts the GET a role change may issue: only `--wait` reads the service,
/// once before a switchover to learn the prior role and then once per poll.
/// `expected_calls` pins that, so a command run without `--wait` is verified to
/// issue no GET at all.
async fn mount_postgres_get(mock: &MockServer, service: serde_json::Value, expected_calls: u64) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
            "status": 200,
            "requestId": "stub-postgres-get",
        })))
        .expect(expected_calls)
        .mount(mock)
        .await;
}

async fn mount_postgres_state(mock: &MockServer, service: serde_json::Value, expected_calls: u64) {
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}/state"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
            "status": 200,
            "requestId": "stub-postgres-state",
        })))
        .expect(expected_calls)
        .mount(mock)
        .await;
}

/// Without `--wait`, switchover is issued as-is: no read of the service before
/// or after, just the state PATCH, and stdout is the state-change response.
#[tokio::test]
async fn postgres_switchover_without_wait_issues_only_the_state_change() {
    let mock = MockServer::start().await;
    mount_postgres_get(&mock, postgres_role_service("sync", Some(true)), 0).await;
    mount_postgres_state(&mock, postgres_role_service("sync", Some(true)), 1).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "switchover",
            ROLE_TEST_POSTGRES_ID,
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![(
            "PATCH".to_string(),
            format!("/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}/state")
        )],
        "switchover without --wait must issue exactly one request"
    );
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource object as JSON");
    assert_eq!(stdout["id"], ROLE_TEST_POSTGRES_ID);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--wait"),
        "should say --wait is how the swap gets confirmed, got: {stderr}"
    );
}

/// The #604 failure: the state endpoint answers 200 and the roles never swap.
/// With `--wait` the CLI polls the service and exits non-zero when the role it
/// captured before the command is still the role afterwards.
#[tokio::test]
async fn postgres_switchover_wait_fails_when_the_roles_never_swap() {
    let mock = MockServer::start().await;
    // One read before the command to learn the prior role, one poll after it.
    mount_postgres_get(&mock, postgres_role_service("sync", Some(true)), 2).await;
    mount_postgres_state(&mock, postgres_role_service("sync", Some(true)), 1).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "switchover",
            ROLE_TEST_POSTGRES_ID,
            "--wait",
            "--wait-timeout",
            "0",
            "--org-id",
            "org-1",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("switchover did not take effect within 0s")
            && stderr.contains("isPrimary=true")
            && stderr.contains("expected false"),
        "should report non-convergence, got: {stderr}"
    );
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("the last observed state should still be emitted as JSON");
    assert_eq!(stdout["isPrimary"], serde_json::json!(true));
}

/// `--wait` succeeds once the target reports the new role, and still says the
/// old primary's demotion is not something the CLI can confirm.
#[tokio::test]
async fn postgres_promote_wait_confirms_the_new_primary() {
    let mock = MockServer::start().await;
    // Promote always targets isPrimary=true, so nothing is read before the
    // command; the single poll afterwards sees the new primary.
    mount_postgres_get(&mock, postgres_role_service("async", Some(true)), 1).await;
    // The promote response omits `isPrimary` entirely, exactly as the API does.
    mount_postgres_state(&mock, postgres_role_service("async", None), 1).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "promote",
            ROLE_TEST_POSTGRES_ID,
            "--wait",
            "--wait-timeout",
            "0",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the polled resource object as JSON");
    assert_eq!(
        stdout["isPrimary"],
        serde_json::json!(true),
        "the polled state must replace the promote response that omits isPrimary"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("previous primary is demoted asynchronously"),
        "the dual-primary window must be reported, got: {stderr}"
    );
}

/// Without `--wait`, promote keeps its old behaviour (exit 0 on acceptance, no
/// read of the service) but says on stderr that the role change is not confirmed.
#[tokio::test]
async fn postgres_promote_without_wait_reports_eventual_consistency() {
    let mock = MockServer::start().await;
    mount_postgres_get(&mock, postgres_role_service("async", Some(false)), 0).await;
    mount_postgres_state(&mock, postgres_role_service("async", None), 1).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "promote",
            ROLE_TEST_POSTGRES_ID,
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource object as JSON");
    assert!(
        stdout.get("isPrimary").is_none(),
        "an omitted isPrimary must stay omitted, got: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("previous primary is demoted asynchronously") && stderr.contains("--wait"),
        "should point at --wait and the dual-primary window, got: {stderr}"
    );
}

/// Every response field is optional, so the pre-command GET can omit
/// `isPrimary`. A switchover swaps that role, so `--wait` then has nothing to
/// confirm a swap against: the command must refuse before issuing a state
/// change it could only report as an unverified success (#604).
#[tokio::test]
async fn postgres_switchover_wait_refuses_an_unknown_prior_role() {
    let mock = MockServer::start().await;
    mount_postgres_get(&mock, postgres_role_service("sync", None), 1).await;
    mount_postgres_state(&mock, postgres_role_service("sync", None), 0).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "switchover",
            ROLE_TEST_POSTGRES_ID,
            "--wait",
            "--wait-timeout",
            "0",
            "--org-id",
            "org-1",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("omitted isPrimary") && stderr.contains("--wait"),
        "should explain why --wait cannot confirm the swap, got: {stderr}"
    );
    assert!(
        output.stdout.is_empty(),
        "a refused switchover must print no resource, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// Start a human-mode (no `--json`, no agent env) switchover without waiting
/// for it, so the test can close its stdout first (see the #598 tests).
fn spawn_postgres_switchover_human(mock: &MockServer, project_dir: &Path) -> std::process::Child {
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project_dir)
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "postgres",
            "switchover",
            ROLE_TEST_POSTGRES_ID,
            "--org-id",
            "org-1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl")
}

/// The human rendering of the role-change result lands after the command
/// already took effect — with `--wait`, minutes after — so a closed stdout must
/// not turn an accepted switchover into a panic and exit 101 (#598).
#[tokio::test]
async fn postgres_switchover_human_output_survives_a_closed_stdout() {
    let mock = MockServer::start().await;
    mount_postgres_get(&mock, postgres_role_service("sync", Some(true)), 0).await;
    mount_postgres_state(&mock, postgres_role_service("sync", Some(true)), 1).await;

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("home")).unwrap();
    let mut child = spawn_postgres_switchover_human(&mock, dir.path());
    drop(child.stdout.take().expect("stdout was piped"));
    let status = child.wait().expect("failed to wait for clickhousectl");

    assert_eq!(
        status.code(),
        Some(0),
        "a closed stdout must not turn an accepted switchover into a panic"
    );
    let shape = received_request_shape(&mock).await;
    assert!(
        shape.contains(&(
            "PATCH".to_string(),
            format!("/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}/state")
        )),
        "the switchover must still reach the API: {shape:?}"
    );
}

const DELETE_TEST_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";
const DELETE_TEST_API_KEY_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const DELETE_TEST_PENDING_API_KEY_ID: &str = "99999999-8888-7777-6666-555555555555";

fn write_service_query_key(root: &Path, organization_id: Option<&str>, api_key_id: Option<&str>) {
    let credentials_dir = root.join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    let mut key = serde_json::json!({
        "key_id": "query-key-id",
        "key_secret": "query-key-secret",
        "endpoint_id": "endpoint-id",
        "service_name": "demo",
        "created_at": "2026-05-11T12:00:00Z",
    });
    if let Some(api_key_id) = api_key_id {
        key["api_key_id"] = Value::String(api_key_id.to_string());
    }
    if let Some(organization_id) = organization_id {
        key["organization_id"] = Value::String(organization_id.to_string());
    }
    std::fs::write(
        credentials_dir.join("credentials.json"),
        serde_json::to_vec(&serde_json::json!({
            "service_query_keys": { DELETE_TEST_SERVICE_ID: key },
        }))
        .unwrap(),
    )
    .unwrap();
}

/// Start a `service delete` without waiting for it, so a test can close the
/// child's pipes while the command is still running (see the #598 tests).
fn spawn_service_delete(
    mock: &MockServer,
    project_dir: &Path,
    force: bool,
    stdout: Stdio,
    stderr: Stdio,
) -> std::process::Child {
    let url = mock.uri();
    let mut args = vec![
        "cloud",
        "--url",
        &url,
        "--json",
        "service",
        "delete",
        DELETE_TEST_SERVICE_ID,
        "--org-id",
        "org-1",
    ];
    if force {
        args.push("--force");
    }
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project_dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .expect("failed to spawn clickhousectl")
}

fn invoke_service_delete(
    mock: &MockServer,
    project_dir: &Path,
    force: bool,
) -> std::process::Output {
    spawn_service_delete(mock, project_dir, force, Stdio::piped(), Stdio::piped())
        .wait_with_output()
        .expect("failed to run clickhousectl")
}

/// The HTTP methods and paths the mock received, in order.
async fn received_request_shape(mock: &MockServer) -> Vec<(String, String)> {
    mock.received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| {
            (
                request.method.as_str().to_string(),
                request.url.path().to_string(),
            )
        })
        .collect()
}

fn successful_delete_response(request_id: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": request_id,
    }))
}

#[tokio::test]
async fn service_delete_removes_the_exact_stored_query_key_after_the_service() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}"
        )))
        .respond_with(successful_delete_response("stub-key-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    assert_eq!(
        requests[1].url.path(),
        format!("/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}")
    );
    assert!(requests.iter().all(|request| {
        request.method == wiremock::http::Method::DELETE && request.body.is_empty()
    }));

    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert!(stored.get("service_query_keys").is_none());
}

#[tokio::test]
async fn service_query_key_removal_merges_changes_made_under_the_credentials_lock() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}"
        )))
        .respond_with(successful_delete_response("stub-key-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let credentials_dir = dir.path().join(".clickhouse");
    let credentials_path = credentials_dir.join("credentials.json");
    let credentials_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(credentials_dir.join("credentials.lock"))
        .unwrap();
    credentials_lock.lock().unwrap();

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let mut child = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "service",
            "delete",
            DELETE_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");

    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            if mock.received_requests().await.unwrap().len() == 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("service deletion did not reach local credential cleanup");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        child.try_wait().unwrap().is_none(),
        "service-key removal did not wait for the credentials lock",
    );

    let mut latest: Value = serde_json::from_slice(&std::fs::read(&credentials_path).unwrap())
        .expect("credentials were not valid JSON");
    latest["api_key"] = Value::String("concurrent-key".to_string());
    latest["api_secret"] = Value::String("concurrent-secret".to_string());
    std::fs::write(&credentials_path, serde_json::to_vec(&latest).unwrap()).unwrap();
    drop(credentials_lock);

    let output = tokio::task::spawn_blocking(move || child.wait_with_output().unwrap())
        .await
        .unwrap();
    assert_success(&output);
    let stored: Value = serde_json::from_slice(&std::fs::read(&credentials_path).unwrap()).unwrap();
    assert_eq!(stored["api_key"], "concurrent-key");
    assert_eq!(stored["api_secret"], "concurrent-secret");
    assert!(stored.get("service_query_keys").is_none());
}

#[tokio::test]
async fn service_delete_also_removes_exact_pending_repair_cleanup_keys() {
    let mock = MockServer::start().await;
    for key_id in [DELETE_TEST_PENDING_API_KEY_ID, DELETE_TEST_API_KEY_ID] {
        Mock::given(method("DELETE"))
            .and(path(format!("/v1/organizations/org-1/keys/{key_id}")))
            .respond_with(successful_delete_response("stub-key-delete"))
            .expect(1)
            .mount(&mock)
            .await;
    }
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let credentials_path = dir.path().join(".clickhouse/credentials.json");
    let mut stored: Value =
        serde_json::from_slice(&std::fs::read(&credentials_path).unwrap()).unwrap();
    stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["pending_cleanup_api_key_ids"] =
        serde_json::json!([DELETE_TEST_PENDING_API_KEY_ID]);
    std::fs::write(&credentials_path, serde_json::to_vec(&stored).unwrap()).unwrap();

    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    assert_eq!(
        requests[1].url.path(),
        format!("/v1/organizations/org-1/keys/{DELETE_TEST_PENDING_API_KEY_ID}")
    );
    assert_eq!(
        requests[2].url.path(),
        format!("/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}")
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(credentials_path).unwrap()).unwrap();
    assert!(stored.get("service_query_keys").is_none());
}

#[tokio::test]
async fn service_delete_attempts_every_owned_key_and_reports_the_ones_that_failed() {
    // The pending retirement cannot be deleted but the current key can: both
    // are attempted, the failure names exactly the key that is left, and the
    // record is kept so its ID is not lost (#527).
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{DELETE_TEST_PENDING_API_KEY_ID}"
        )))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "pending cleanup failed",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}"
        )))
        .respond_with(successful_delete_response("stub-key-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let credentials_path = dir.path().join(".clickhouse/credentials.json");
    let mut stored: Value =
        serde_json::from_slice(&std::fs::read(&credentials_path).unwrap()).unwrap();
    stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["pending_cleanup_api_key_ids"] =
        serde_json::json!([DELETE_TEST_PENDING_API_KEY_ID]);
    std::fs::write(&credentials_path, serde_json::to_vec(&stored).unwrap()).unwrap();

    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "({DELETE_TEST_PENDING_API_KEY_ID}: pending cleanup failed)"
        )),
        "{stderr}"
    );
    assert!(
        !stderr.contains(&format!("{DELETE_TEST_API_KEY_ID}:")),
        "a key that was deleted is not reported as failed: {stderr}"
    );
    assert!(
        stderr.contains("clickhousectl cloud key delete <key-id> --org-id org-1"),
        "{stderr}"
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
            ),
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/keys/{DELETE_TEST_PENDING_API_KEY_ID}")
            ),
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}")
            ),
        ]
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(credentials_path).unwrap()).unwrap();
    assert_eq!(
        stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["pending_cleanup_api_key_ids"],
        serde_json::json!([DELETE_TEST_PENDING_API_KEY_ID])
    );
}

#[tokio::test]
async fn service_delete_without_a_stored_query_key_only_deletes_the_service() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, wiremock::http::Method::DELETE);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    assert!(requests[0].body.is_empty());
}

#[tokio::test]
async fn forced_service_delete_surfaces_not_found_for_an_absent_service() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "NOT_FOUND",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "NOT_FOUND",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let output = invoke_service_delete(&mock, dir.path(), true);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: NOT_FOUND: request scoped to organization org-1\n"
    );

    // The delete request never succeeded, so local query-key cleanup and the
    // organization-scoped key deletion (which would follow a successful
    // delete) must not have been attempted.
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            (
                "GET".to_string(),
                format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
            ),
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
            ),
        ]
    );

    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["api_key_id"],
        DELETE_TEST_API_KEY_ID
    );
}

#[tokio::test]
async fn forced_service_delete_reports_only_poll_state_transitions_when_redirected() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let mock = MockServer::start().await;
    let states = ["running", "stopping", "stopping", "stopped"];
    let request_index = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(move |_: &wiremock::Request| {
            let index = request_index.fetch_add(1, Ordering::SeqCst);
            let state = states[index.min(states.len() - 1)];
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": DELETE_TEST_SERVICE_ID,
                    "name": "demo",
                    "state": state,
                },
                "status": 200,
                "requestId": format!("stub-service-get-{index}"),
            }))
        })
        .expect(4)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}/state"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": DELETE_TEST_SERVICE_ID,
                "name": "demo",
                "state": "stopping",
            },
            "status": 200,
            "requestId": "stub-service-stop",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), true);
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Stopping service {DELETE_TEST_SERVICE_ID} before deletion...\n  state: stopping\n  state: stopped\n"
        )
    );
}

/// Mock a `--force` delete of a service that is observed mid-`stopping`:
/// `running` on the pre-stop check, then `stopping` and `stopped` from the
/// poll loop.
async fn mount_forced_delete_stop_sequence(mock: &MockServer) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let states = ["running", "stopping", "stopped"];
    let request_index = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(move |_: &wiremock::Request| {
            let index = request_index.fetch_add(1, Ordering::SeqCst);
            let state = states[index.min(states.len() - 1)];
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": DELETE_TEST_SERVICE_ID,
                    "name": "demo",
                    "state": state,
                },
                "status": 200,
                "requestId": format!("stub-service-get-{index}"),
            }))
        })
        .expect(3)
        .mount(mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}/state"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": DELETE_TEST_SERVICE_ID,
                "name": "demo",
                "state": "stopping",
            },
            "status": 200,
            "requestId": "stub-service-stop",
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(mock)
        .await;
}

/// #598: a `--force` delete that observed the service mid-`stopping` panicked
/// with exit 101. The stop poll streams a progress line per state change for as
/// long as the stop takes (minutes on a real service), so it outlives readers
/// that go away — a pager the user quit, a supervising harness that stopped
/// reading — and `eprintln!` panics when the write fails with `BrokenPipe`.
/// The delete must still run to completion and exit 0.
#[tokio::test]
async fn forced_service_delete_survives_a_closed_stderr_while_stopping() {
    let mock = MockServer::start().await;
    mount_forced_delete_stop_sequence(&mock).await;

    let dir = tempfile::tempdir().unwrap();
    let mut child = spawn_service_delete(&mock, dir.path(), true, Stdio::null(), Stdio::piped());
    // Dropping the read end closes the pipe while the command is still
    // polling. Rust ignores `SIGPIPE`, so the child's next write to stderr
    // fails with `EPIPE` rather than killing the process — exactly the state a
    // reader that walked away leaves behind.
    drop(child.stderr.take().expect("stderr was piped"));
    let status = child.wait().expect("failed to wait for clickhousectl");

    assert_eq!(
        status.code(),
        Some(0),
        "a closed stderr must not turn a completed forced delete into a panic"
    );
    // Not just "didn't panic": the stop-then-delete sequence still completed.
    let shape = received_request_shape(&mock).await;
    assert_eq!(
        shape.last(),
        Some(&(
            "DELETE".to_string(),
            format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
        )),
        "unexpected request sequence: {shape:?}"
    );
}

/// The stdout counterpart of #598: the result line is printed after the
/// service is already gone, so a closed stdout must not turn a completed
/// deletion into a panic either.
#[tokio::test]
async fn service_delete_survives_a_closed_stdout() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut child = spawn_service_delete(&mock, dir.path(), false, Stdio::piped(), Stdio::piped());
    drop(child.stdout.take().expect("stdout was piped"));
    let status = child.wait().expect("failed to wait for clickhousectl");

    assert_eq!(
        status.code(),
        Some(0),
        "a closed stdout must not turn a completed deletion into a panic"
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![(
            "DELETE".to_string(),
            format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
        )]
    );
}

#[tokio::test]
async fn service_delete_cleanup_failure_preserves_credentials_for_retry() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}"
        )))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "cleanup failed",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: failed to delete the auto-provisioned query API key for service \
             {DELETE_TEST_SERVICE_ID} ({DELETE_TEST_API_KEY_ID}: cleanup failed). The local \
             record was kept so the exact IDs are not lost; delete each key with \
             `clickhousectl cloud key delete <key-id> --org-id org-1`\n"
        )
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    assert_eq!(
        requests[1].url.path(),
        format!("/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}")
    );
    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["api_key_id"],
        DELETE_TEST_API_KEY_ID
    );
}

#[tokio::test]
async fn service_delete_failure_preserves_the_query_key_without_cleanup() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "service delete failed",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: service delete failed\n"
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["api_key_id"],
        DELETE_TEST_API_KEY_ID
    );
}

#[tokio::test]
async fn service_delete_rejects_query_key_from_another_organization() {
    let mock = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-2"), Some(DELETE_TEST_API_KEY_ID));

    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: the stored query key for service {DELETE_TEST_SERVICE_ID} belongs to \
             organization org-2, not org-1; refusing to delete either resource\n"
        )
    );
    assert!(mock.received_requests().await.unwrap().is_empty());

    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["organization_id"],
        "org-2"
    );
}

#[tokio::test]
async fn service_delete_retains_a_key_id_without_organization_metadata() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), None, Some(DELETE_TEST_API_KEY_ID));
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Warning: the stored query key for service {DELETE_TEST_SERVICE_ID} has a management \
             API key ID but no provisioning organization; cloud key cleanup was skipped and the \
             local record was retained.\n"
        )
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][DELETE_TEST_SERVICE_ID]["api_key_id"],
        DELETE_TEST_API_KEY_ID
    );
}

#[tokio::test]
async fn service_delete_does_not_treat_a_missing_organization_as_an_absent_service() {
    let mock = MockServer::start().await;
    let not_found = ResponseTemplate::new(404).set_body_json(serde_json::json!({
        "status": 404,
        "error": "NOT_FOUND",
    }));
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}"
        )))
        .respond_with(not_found)
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: NOT_FOUND: request scoped to organization org-1\n"
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
}

#[tokio::test]
async fn service_delete_preserves_a_detailed_not_found_error() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/missing-service"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "Service missing-service was not found",
            "requestId": "stub-missing-service",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["service", "delete", "missing-service", "--org-id", "org-1"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Service missing-service was not found\n"
    );
}

#[tokio::test]
async fn service_delete_aborts_when_query_key_credentials_are_malformed() {
    let mock = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join("credentials.json"), "{").unwrap();

    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Error: failed to parse ")
            && stderr.contains(".clickhouse/credentials.json")
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
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
        .env_clear()
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
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus/discovery"))
        .and(query_param("filtered_metrics", "true"))
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
        .env_clear()
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
            "--filtered-metrics",
            "true",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "- targets: [api.clickhouse.cloud:443]\n- labels:\n    __scheme__: https\n"
    );
}

#[tokio::test]
async fn org_prometheus_discovery_omits_filter_query_when_unspecified() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus/discovery"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["org", "prometheus", "discovery", "--org-id", "org-1"],
    );
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "[]\n");

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
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

#[tokio::test]
async fn org_prometheus_accepts_legacy_positional_org_id() {
    let mock = start_mock_org_auto_detection_api().await;
    let output =
        invoke_cli_with_cloud_credentials(&mock, &["org", "prometheus", AUTO_DETECTED_ORG_ID]);
    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/{AUTO_DETECTED_ORG_ID}/prometheus")
    );
}

#[tokio::test]
async fn org_usage_accepts_legacy_positional_org_id() {
    let mock = start_mock_org_auto_detection_api().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "usage",
            AUTO_DETECTED_ORG_ID,
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ],
    );
    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/{AUTO_DETECTED_ORG_ID}/usageCost")
    );
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
    clear_inherited_env(&mut command);
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
async fn org_usage_marks_uuid_only_entities_as_unknown_in_human_output() {
    let mock = start_mock_usage_entities_api().await;
    let output = invoke_org_usage(&mock, false);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("production"));
    assert!(stdout.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee (unknown)"));
}

#[tokio::test]
async fn org_usage_json_keeps_uuid_only_entities_faithful_to_the_api() {
    let mock = start_mock_usage_entities_api().await;
    let output = invoke_org_usage(&mock, true);
    assert_success(&output);

    let usage: Value = serde_json::from_slice(&output.stdout).unwrap();
    let unknown = &usage["costs"][1];
    assert_eq!(unknown["entityId"], "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
    assert!(unknown.get("entityName").is_none());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("(unknown)"));
}

// ── Bug 1: Postgres CDC must NOT send publicationName / replicationSlotName ─
//
// `cdc` replication mode creates the slot + publication server-side; the
// pre-`4f6c2ba` handler sent `""` for both via `unwrap_or_default()` and the
// API rejected with `replicationSlotName: ''`. The fix made the model field
// `Option<String>` so absence at the args level = absence in the wire body.
// This test would fail if either field reappeared.

#[tokio::test]
async fn postgres_cdc_omits_publication_name_and_slot_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "pg.example.com",
            "--port",
            "5432",
            "--pg-database",
            "test",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--replication-mode",
            "cdc",
            // INTENTIONALLY omit --publication-name + --replication-slot-name.
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let settings = &body["source"]["postgres"]["settings"];
    assert!(
        settings.get("publicationName").is_none(),
        "publicationName leaked into wire body: {settings}",
    );
    assert!(
        settings.get("replicationSlotName").is_none(),
        "replicationSlotName leaked into wire body: {settings}",
    );
}

// ── Bug 2: Database-pipe destination must NOT include table/columns/etc. ────
//
// For Postgres / MySQL / MongoDB / BigQuery, the `destination` body must
// carry only `database` — the per-mapping `targetTable` carries the
// destination table name. The pre-`4f6c2ba` handler defaulted `table: ""`,
// `columns: []`, `managedTable: false`, `tableDefinition: {…default…}` and
// the API rejected with `destination.table: ''` and `columns: minLength`.
// Modeling those four fields as `Option<T>` + `skip_serializing_if` made
// absence in the args translate to absence on the wire.

#[tokio::test]
async fn postgres_destination_omits_table_columns_managed_table_definition() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "pg.example.com",
            "--port",
            "5432",
            "--pg-database",
            "test",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest, &serde_json::json!({ "database": "default" }));
    assert_eq!(body["source"]["postgres"]["database"], "test");
}

#[tokio::test]
async fn postgres_destination_database_is_distinct_from_source_database() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "pg.example.com",
            "--pg-database",
            "source_db",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--destination-database",
            "analytics",
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["destination"],
        serde_json::json!({ "database": "analytics" })
    );
    assert_eq!(body["source"]["postgres"]["database"], "source_db");
}

#[tokio::test]
async fn mysql_destination_omits_table_columns_managed_table_definition() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "mysql.example.com",
            "--port",
            "3306",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest, &serde_json::json!({ "database": "default" }));
    assert_eq!(
        body["source"]["mysql"]["tableMappings"][0]["sourceSchemaName"],
        "mydb"
    );
}

#[tokio::test]
async fn mysql_destination_database_keeps_source_schema_mapping_separate() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "mysql.example.com",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "source_db.t:t",
            "--destination-database",
            "analytics",
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["destination"],
        serde_json::json!({ "database": "analytics" })
    );
    assert_eq!(
        body["source"]["mysql"]["tableMappings"][0]["sourceSchemaName"],
        "source_db"
    );
}

#[tokio::test]
async fn mongodb_destination_omits_table_columns_managed_table_definition() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "test-pipe",
            "--uri",
            "mongodb://mongo.example.com:27017",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.coll:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest, &serde_json::json!({ "database": "default" }));
    assert_eq!(
        body["source"]["mongodb"]["tableMappings"][0]["sourceDatabaseName"],
        "mydb"
    );
}

#[tokio::test]
async fn mongodb_destination_database_keeps_source_database_mapping_separate() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "test-pipe",
            "--uri",
            "mongodb://mongo.example.com:27017",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "source_db.t:t",
            "--destination-database",
            "analytics",
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["destination"],
        serde_json::json!({ "database": "analytics" })
    );
    assert_eq!(
        body["source"]["mongodb"]["tableMappings"][0]["sourceDatabaseName"],
        "source_db"
    );
}

// ── Spot-check: optional flags omitted from non-database (S3) pipe too ──────
//
// S3 isn't a database pipe — it DOES include table/columns/etc. in
// destination. But it has its own optionals that should be absent when the
// flag isn't passed (e.g. --iam-role, --queue-url).

#[tokio::test]
async fn s3_pipe_omits_iam_role_and_queue_url_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "test-pipe",
            "--source-url",
            "https://bucket.s3.us-east-1.amazonaws.com/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--access-key-id",
            "AKIA000000000000FAKE",
            "--secret-key",
            "fake/secret/for/tests/0000000000000000",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let s3 = &body["source"]["objectStorage"];
    for field in [
        "iamRole",
        "queueUrl",
        "connectionString",
        "azureContainerName",
        "path",
        "serviceAccountKey",
        "delimiter",
    ] {
        assert!(
            s3.get(field).is_none(),
            "{field} leaked into S3 body when --{field} not passed: {s3}",
        );
    }
}

// `--service-account-file` for GCS object-storage points at a JSON key on disk;
// the handler must read the file and base64-encode its contents into
// `source.objectStorage.serviceAccountKey`, matching the BigQuery flow.
#[tokio::test]
async fn gcs_service_account_file_is_read_and_base64_encoded() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;

    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("service-account.json");
    let sa_contents = br#"{"type":"service_account","project_id":"test"}"#;
    let mut sa_file = std::fs::File::create(&sa_path).unwrap();
    sa_file.write_all(sa_contents).unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "gcs-pipe",
            "--source-url",
            "https://storage.googleapis.com/bucket/data/*.json",
            "--format",
            "JSONEachRow",
            "--storage-type",
            "gcs",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--service-account-file",
            sa_path.to_str().unwrap(),
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let gcs = &body["source"]["objectStorage"];
    assert_eq!(gcs["authentication"], "SERVICE_ACCOUNT");
    let expected = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sa_contents);
    assert_eq!(
        gcs["serviceAccountKey"].as_str(),
        Some(expected.as_str()),
        "serviceAccountKey on the wire should be base64 of the file contents: {gcs}",
    );
}

// Extra postgres coverage: --tls-host, --iam-role, --ca-certificate (file)
// should all be absent from the wire body when their CLI flags aren't set.

#[tokio::test]
async fn postgres_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "t",
            "--host",
            "pg",
            "--port",
            "5432",
            "--pg-database",
            "test",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;

    let pg = &body["source"]["postgres"];
    assert_eq!(pg["disableTls"], false, "secure TLS default changed: {pg}");
    assert_eq!(
        pg["skipCertVerification"], false,
        "certificate verification default changed: {pg}"
    );
    for field in ["iamRole", "tlsHost", "caCertificate"] {
        assert!(
            pg.get(field).is_none(),
            "{field} leaked into postgres source body: {pg}",
        );
    }
}

#[tokio::test]
async fn postgres_tls_opt_outs_change_only_the_selected_wire_field() {
    for (flag, disable_tls, skip_cert_verification) in [
        ("--disable-tls", true, false),
        ("--skip-cert-verification", false, true),
    ] {
        let mock = start_mock_clickpipes_api().await;
        let mut args = postgres_args_minimal();
        args.push(flag.into());
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let body = invoke_cli_capture_body(&mock, &arg_refs).await;
        let pg = &body["source"]["postgres"];

        assert_eq!(pg["disableTls"], disable_tls, "{flag}: {pg}");
        assert_eq!(
            pg["skipCertVerification"], skip_cert_verification,
            "{flag}: {pg}"
        );
    }
}

// MySQL: same set of absent-when-omitted optional fields.

#[tokio::test]
async fn mysql_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "mysql",
            "--port",
            "3306",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;

    let mysql = &body["source"]["mysql"];
    for field in ["iamRole", "tlsHost", "caCertificate"] {
        assert!(
            mysql.get(field).is_none(),
            "{field} leaked into mysql source body: {mysql}",
        );
    }
    assert_eq!(
        mysql["settings"],
        serde_json::json!({
            "replicationMode": "cdc",
            "replicationMechanism": "GTID",
        })
    );
}

#[tokio::test]
async fn issue_593_mysql_create_sends_every_source_tuning_field() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "tuned",
            "--host",
            "mysql",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--sync-interval-seconds",
            "1",
            "--pull-batch-size",
            "2",
            "--initial-load-parallelism",
            "3",
            "--snapshot-rows-per-partition",
            "1000",
            "--snapshot-parallel-tables",
            "4",
            "--allow-nullable-columns",
            "false",
            "--delete-on-merge",
            "true",
            "--use-compression",
            "false",
            "--skip-cert-verification",
            "--org-id",
            "org",
        ],
    )
    .await;

    let mysql = &body["source"]["mysql"];
    assert_eq!(mysql["skipCertVerification"], true);
    assert_eq!(
        mysql["settings"],
        serde_json::json!({
            "replicationMode": "cdc",
            "replicationMechanism": "GTID",
            "syncIntervalSeconds": 1,
            "pullBatchSize": 2,
            "initialLoadParallelism": 3,
            "snapshotNumRowsPerPartition": 1000,
            "snapshotNumberOfParallelTables": 4,
            "allowNullableColumns": false,
            "deleteOnMerge": true,
            "useCompression": false,
        })
    );
}

// Mongo: tlsHost should be absent when --tls-host not passed.

#[tokio::test]
async fn mongodb_tls_host_absent_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "t",
            "--uri",
            "mongodb://m:27017",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.c:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;

    let mongo = &body["source"]["mongodb"];
    assert!(
        mongo.get("tlsHost").is_none(),
        "tlsHost leaked into mongodb source body: {mongo}",
    );
    assert!(
        mongo.get("caCertificate").is_none(),
        "caCertificate leaked into mongodb source body: {mongo}",
    );
    assert!(mongo.get("skipCertVerification").is_none());
    assert_eq!(
        mongo["settings"],
        serde_json::json!({ "replicationMode": "cdc" })
    );
}

#[tokio::test]
async fn issue_593_mongodb_create_sends_every_source_tuning_and_tls_field() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "tuned",
            "--uri",
            "mongodb://m:27017",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.c:t",
            "--sync-interval-seconds",
            "1",
            "--pull-batch-size",
            "2",
            "--snapshot-rows-per-partition",
            "1000",
            "--snapshot-parallel-collections",
            "3",
            "--delete-on-merge",
            "false",
            "--use-json-native-format",
            "true",
            "--skip-cert-verification",
            "--org-id",
            "org",
        ],
    )
    .await;

    let mongo = &body["source"]["mongodb"];
    assert_eq!(mongo["skipCertVerification"], true);
    assert_eq!(
        mongo["settings"],
        serde_json::json!({
            "replicationMode": "cdc",
            "syncIntervalSeconds": 1,
            "pullBatchSize": 2,
            "snapshotNumRowsPerPartition": 1000,
            "snapshotNumberOfParallelTables": 3,
            "deleteOnMerge": false,
            "useJsonNativeFormat": true,
        })
    );
}

// ── Kafka ──────────────────────────────────────────────────────────────────
//
// Kafka has the largest optional-flag surface; cover the high-traffic ones
// plus all 4 SASL credential shapes (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512,
// MUTUAL_TLS, IAM_ROLE).

/// Kafka create args carrying no authentication flag of any kind.
fn kafka_args_without_auth() -> Vec<&'static str> {
    vec![
        "clickpipe",
        "create",
        "kafka",
        "svc-id",
        "--name",
        "t",
        "--brokers",
        "broker:9092",
        "--topics",
        "topic",
        "--format",
        "JSONEachRow",
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "id:Int64",
        "--kafka-type",
        "kafka",
        "--org-id",
        "org",
    ]
}

fn kafka_args_minimal() -> Vec<&'static str> {
    let mut args = kafka_args_without_auth();
    args.extend(["--auth", "PLAIN", "--username", "u", "--password", "p"]);
    args
}

fn append_initial_scaling_and_validation(args: &mut Vec<String>) {
    args.extend([
        "--replicas".into(),
        "2".into(),
        "--cpu-millicores".into(),
        "500".into(),
        "--memory-gb".into(),
        "2".into(),
    ]);
    append_sample_validation(args);
}

fn append_sample_validation(args: &mut Vec<String>) {
    args.extend(["--validate-samples".into(), "true".into()]);
}

fn append_streaming_create_controls(args: &mut Vec<String>) {
    append_initial_scaling_and_validation(args);
    args.extend([
        "--field-mapping".into(),
        r#"{"sourceField":"source:a=b","destinationField":"destination:x=y"}"#.into(),
        "--clickhouse-max-threads".into(),
        "0".into(),
        "--clickhouse-parallel-view-processing".into(),
        "false".into(),
    ]);
}

#[tokio::test]
async fn cross_source_create_controls_reach_all_eight_request_shapes() {
    let directory = tempfile::tempdir().unwrap();
    let service_account = directory.path().join("service-account.json");
    std::fs::write(&service_account, "{}").unwrap();
    let service_account = service_account.to_str().unwrap();

    let mut object_storage = [
        "clickpipe",
        "create",
        "object-storage",
        "svc-id",
        "--name",
        "object-pipe",
        "--source-url",
        "https://example.test/events.json",
        "--format",
        "JSONEachRow",
        "--database",
        "default",
        "--table",
        "events",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_streaming_create_controls(&mut object_storage);
    object_storage.extend([
        "--object-storage-concurrency".into(),
        "1".into(),
        "--object-storage-polling-interval-ms".into(),
        "100".into(),
        "--object-storage-max-insert-bytes".into(),
        "10485760".into(),
        "--object-storage-max-file-count".into(),
        "1".into(),
        "--object-storage-use-cluster-function".into(),
        "false".into(),
    ]);

    let mut kafka = kafka_args_minimal()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    append_streaming_create_controls(&mut kafka);
    kafka.extend([
        "--streaming-max-insert-wait-ms".into(),
        "500".into(),
        "--kafka-read-committed".into(),
        "false".into(),
    ]);

    let mut kinesis = [
        "clickpipe",
        "create",
        "kinesis",
        "svc-id",
        "--name",
        "kinesis-pipe",
        "--stream-name",
        "events",
        "--region",
        "us-east-1",
        "--format",
        "JSONEachRow",
        "--iam-role",
        "arn:aws:iam::123456789012:role/clickpipe",
        "--database",
        "default",
        "--table",
        "events",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_streaming_create_controls(&mut kinesis);
    kinesis.extend(["--streaming-max-insert-wait-ms".into(), "500".into()]);

    let mut postgres = postgres_args_minimal();
    append_sample_validation(&mut postgres);
    let mut mysql = mysql_args_minimal();
    append_sample_validation(&mut mysql);

    let mut mongodb = [
        "clickpipe",
        "create",
        "mongodb",
        "svc-id",
        "--name",
        "mongo-pipe",
        "--uri",
        "mongodb://mongo.example/source",
        "--username",
        "u",
        "--password",
        "p",
        "--table-mapping",
        "source.events:events",
        "--destination-database",
        "default",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_sample_validation(&mut mongodb);

    let mut bigquery = [
        "clickpipe",
        "create",
        "bigquery",
        "svc-id",
        "--name",
        "bigquery-pipe",
        "--service-account-file",
        service_account,
        "--staging-path",
        "gs://bucket/staging",
        "--table-mapping",
        "source.events:events",
        "--destination-database",
        "default",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_sample_validation(&mut bigquery);

    let mut pubsub = pubsub_create_args(service_account);
    append_streaming_create_controls(&mut pubsub);
    pubsub.extend(["--streaming-max-insert-wait-ms".into(), "500".into()]);

    for (source, args, has_streaming_controls) in [
        ("objectStorage", object_storage, true),
        ("kafka", kafka, true),
        ("kinesis", kinesis, true),
        ("postgres", postgres, false),
        ("mysql", mysql, false),
        ("mongodb", mongodb, false),
        ("bigquery", bigquery, false),
        ("pubsub", pubsub, true),
    ] {
        let mock = start_mock_clickpipes_api().await;
        let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;
        assert_eq!(body["source"]["validateSamples"], true, "{source}");
        assert!(body["source"][source].is_object(), "{source}: {body}");
        if has_streaming_controls {
            assert_eq!(
                body["scaling"],
                serde_json::json!({
                    "replicas": 2,
                    "replicaCpuMillicores": 500,
                    "replicaMemoryGb": 2.0,
                }),
                "{source}"
            );
            assert_eq!(
                body["fieldMappings"],
                serde_json::json!([{
                    "sourceField": "source:a=b",
                    "destinationField": "destination:x=y",
                }]),
                "{source}"
            );
            assert_eq!(body["settings"]["clickhouse_max_threads"], 0, "{source}");
            assert_eq!(
                body["settings"]["clickhouse_parallel_view_processing"], false,
                "{source}"
            );
            assert_eq!(body["settings"]["kafka_read_committed"], false, "{source}");
            if source == "objectStorage" {
                assert_eq!(body["settings"]["object_storage_concurrency"], 1);
                assert_eq!(
                    body["settings"]["object_storage_use_cluster_function"],
                    false
                );
                assert!(
                    body["settings"]
                        .get("streaming_max_insert_wait_ms")
                        .is_none(),
                    "{body}"
                );
            } else {
                assert_eq!(body["settings"]["streaming_max_insert_wait_ms"], 500);
            }
        } else {
            assert!(body.get("scaling").is_none(), "{source}: {body}");
            assert!(body.get("fieldMappings").is_none(), "{source}: {body}");
            assert!(body.get("settings").is_none(), "{source}: {body}");
        }
    }
}

#[tokio::test]
async fn invalid_create_controls_fail_before_file_io_or_http() {
    let mock = start_mock_clickpipes_api().await;
    let mut postgres = postgres_args_minimal();
    postgres.extend([
        "--replicas".into(),
        "2".into(),
        "--ca-certificate".into(),
        "/definitely/missing/clickpipe-ca.pem".into(),
    ]);
    let output = invoke_cli_with_cloud_credentials(&mock, &as_str_args(&postgres));
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument '--replicas'"),
        "{stderr}"
    );
    assert!(!stderr.contains("clickpipe-ca.pem"), "{stderr}");
    assert!(mock.received_requests().await.unwrap().is_empty());

    let mut object_storage = [
        "clickpipe",
        "create",
        "object-storage",
        "svc-id",
        "--name",
        "object-pipe",
        "--source-url",
        "https://example.test/events.json",
        "--format",
        "JSONEachRow",
        "--database",
        "default",
        "--table",
        "events",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    object_storage.extend([
        "--field-mapping".into(),
        r#"{"sourceField":"source","destinationField":"target","typo":true}"#.into(),
    ]);
    let output = invoke_cli_with_cloud_credentials(&mock, &as_str_args(&object_storage));
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown field `typo`"), "{stderr}");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn kafka_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(&mock, &kafka_args_minimal()).await;

    let kafka = &body["source"]["kafka"];
    for field in [
        "consumerGroup",
        "iamRole",
        "schemaRegistry",
        "caCertificate",
    ] {
        assert!(
            kafka.get(field).is_none(),
            "{field} leaked into kafka source body: {kafka}",
        );
    }
    assert!(
        kafka["offset"].get("timestamp").is_none(),
        "offset.timestamp leaked when --offset-timestamp not passed: {kafka}",
    );
}

#[tokio::test]
async fn kafka_plain_credentials_shape() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(&mock, &kafka_args_minimal()).await;
    let creds = &body["source"]["kafka"]["credentials"];
    assert_eq!(creds["username"], "u");
    assert_eq!(creds["password"], "p");
}

#[tokio::test]
async fn issue_593_kafka_create_sends_exactly_once_and_base64_protobuf_schema() {
    let mock = start_mock_clickpipes_api().await;
    let directory = tempfile::tempdir().unwrap();
    let schema = directory.path().join("events.proto");
    std::fs::write(&schema, b"syntax = \"proto3\";").unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kafka",
            "svc-id",
            "--name",
            "protobuf",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "Protobuf",
            "--protobuf-schema-file",
            schema.to_str().unwrap(),
            "--exactly-once",
            "true",
            "--database",
            "default",
            "--table",
            "events",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["exactlyOnce"], true);
    assert_eq!(kafka["protobufSchema"], "c3ludGF4ID0gInByb3RvMyI7");
    assert!(kafka.get("schemaRegistry").is_none());
}

#[tokio::test]
async fn issue_593_kafka_event_hubs_connection_string_uses_its_credential_shape() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kafka",
            "svc-id",
            "--name",
            "event-hubs",
            "--brokers",
            "namespace.servicebus.windows.net:9093",
            "--topics",
            "events",
            "--format",
            "JSONEachRow",
            "--kafka-type",
            "azureeventhub",
            "--event-hubs-connection-string",
            "Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKey=secret",
            "--database",
            "default",
            "--table",
            "events",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["type"], "azureeventhub");
    assert_eq!(kafka["authentication"], "PLAIN");
    assert_eq!(
        kafka["credentials"],
        serde_json::json!({
            "connectionString": "Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKey=secret"
        })
    );
}

#[tokio::test]
async fn kafka_without_auth_flags_sends_no_authentication() {
    // A broker that requires no authentication: the CLI must not invent PLAIN
    // (which used to fail client-side with "PLAIN requires --username and
    // --password"), and `authentication` must be absent from the wire body
    // because the spec enum has no value for "none".
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(&mock, &kafka_args_without_auth()).await;

    let kafka = &body["source"]["kafka"];
    assert!(
        kafka.get("authentication").is_none(),
        "authentication must be omitted for a no-auth broker: {kafka}",
    );
    assert!(
        kafka["credentials"].is_null(),
        "credentials must not carry a mechanism body: {kafka}",
    );
    assert_eq!(kafka["brokers"], "broker:9092");
    assert_eq!(kafka["topics"], "topic");
}

#[tokio::test]
async fn schema_discover_kafka_without_auth_flags_sends_no_authentication() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "svc-id",
            "--org-id",
            "org",
            "kafka",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
        ],
    )
    .await;

    let kafka = &body["source"]["kafka"];
    assert!(
        kafka.get("authentication").is_none(),
        "authentication must be omitted for a no-auth broker: {kafka}",
    );
    assert!(
        kafka["credentials"].is_null(),
        "credentials must not carry a mechanism body: {kafka}",
    );
}

#[tokio::test]
async fn kafka_infers_plain_from_username_and_password_without_auth_flag() {
    // Credential flags without --auth still resolve to the matching mechanism,
    // so omitting --auth is not a silent downgrade to no authentication.
    let mock = start_mock_clickpipes_api().await;
    let mut args = kafka_args_without_auth();
    args.extend(["--username", "u", "--password", "p"]);
    let body = invoke_cli_capture_body(&mock, &args).await;

    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["authentication"], "PLAIN");
    assert_eq!(kafka["credentials"]["username"], "u");
    assert_eq!(kafka["credentials"]["password"], "p");
}

#[tokio::test]
async fn kafka_scram_sha_512_credentials_shape() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = kafka_args_minimal();
    // replace `PLAIN` with SCRAM-SHA-512
    let auth_idx = args.iter().position(|a| *a == "PLAIN").unwrap();
    args[auth_idx] = "SCRAM-SHA-512";
    let body = invoke_cli_capture_body(&mock, &args).await;
    let creds = &body["source"]["kafka"]["credentials"];
    assert_eq!(creds["username"], "u");
    assert_eq!(creds["password"], "p");
}

#[tokio::test]
async fn kafka_iam_role_serializes_iam_role_field() {
    let mock = start_mock_clickpipes_api().await;
    // IAM_ROLE doesn't use --username/--password; build a custom arg list.
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kafka",
            "svc-id",
            "--name",
            "t",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--kafka-type",
            "msk",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123:role/x",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["iamRole"], "arn:aws:iam::123:role/x");
    // credentials for IAM_ROLE is sent as JSON null at the field level.
    assert!(
        kafka["credentials"].is_null(),
        "IAM_ROLE credentials should be null, got: {}",
        kafka["credentials"]
    );
}

#[tokio::test]
async fn kafka_iam_user_credentials_shape() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kafka",
            "svc-id",
            "--name",
            "t",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--kafka-type",
            "msk",
            "--auth",
            "IAM_USER",
            "--access-key-id",
            "AKIA000000000000FAKE",
            "--secret-key",
            "fake/secret/0000000000000000",
            "--org-id",
            "org",
        ],
    )
    .await;

    let creds = &body["source"]["kafka"]["credentials"];
    assert_eq!(creds["accessKeyId"], "AKIA000000000000FAKE");
    assert_eq!(creds["secretKey"], "fake/secret/0000000000000000");
}

#[tokio::test]
async fn kafka_mutual_tls_credentials_use_cert_file_contents() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let cert_path = dir.path().join("client.crt");
    let key_path = dir.path().join("client.key");
    let mut cert_file = std::fs::File::create(&cert_path).unwrap();
    let mut key_file = std::fs::File::create(&key_path).unwrap();
    cert_file
        .write_all(b"-----BEGIN CERTIFICATE-----\nCERT_PEM\n-----END CERTIFICATE-----\n")
        .unwrap();
    key_file
        .write_all(b"-----BEGIN PRIVATE KEY-----\nKEY_PEM\n-----END PRIVATE KEY-----\n")
        .unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kafka",
            "svc-id",
            "--name",
            "t",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--kafka-type",
            "kafka",
            "--auth",
            "MUTUAL_TLS",
            "--client-certificate",
            cert_path.to_str().unwrap(),
            "--client-key",
            key_path.to_str().unwrap(),
            "--org-id",
            "org",
        ],
    )
    .await;

    let creds = &body["source"]["kafka"]["credentials"];
    assert!(
        creds["certificate"]
            .as_str()
            .map(|s| s.contains("CERT_PEM"))
            .unwrap_or(false),
        "MUTUAL_TLS certificate should contain file contents: {creds}",
    );
    assert!(
        creds["privateKey"]
            .as_str()
            .map(|s| s.contains("KEY_PEM"))
            .unwrap_or(false),
        "MUTUAL_TLS privateKey should contain file contents: {creds}",
    );
}

// ── Kinesis ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn kinesis_iam_role_omits_access_key() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kinesis",
            "svc-id",
            "--name",
            "t",
            "--stream-name",
            "s",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123:role/x",
            "--iterator-type",
            "TRIM_HORIZON",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kinesis = &body["source"]["kinesis"];
    assert_eq!(kinesis["iamRole"], "arn:aws:iam::123:role/x");
    assert!(
        kinesis.get("accessKey").is_none(),
        "accessKey leaked when --auth IAM_ROLE: {kinesis}",
    );
}

#[tokio::test]
async fn kinesis_iam_user_omits_iam_role() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kinesis",
            "svc-id",
            "--name",
            "t",
            "--stream-name",
            "s",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--auth",
            "IAM_USER",
            "--access-key-id",
            "AKIA000000000000FAKE",
            "--secret-key",
            "fake/secret/0000000000000000",
            "--iterator-type",
            "TRIM_HORIZON",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kinesis = &body["source"]["kinesis"];
    assert_eq!(kinesis["accessKey"]["accessKeyId"], "AKIA000000000000FAKE");
    assert!(
        kinesis.get("iamRole").is_none(),
        "iamRole leaked when --auth IAM_USER: {kinesis}",
    );
}

// ── BigQuery ───────────────────────────────────────────────────────────────
//
// BigQuery falls into the "database pipe" bucket — destination MUST omit
// table/columns/etc. Its optional snapshot tuning fields must also remain
// absent unless the user chooses them.

#[tokio::test]
async fn bigquery_destination_omits_table_columns_managed_table_definition() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;

    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("service-account.json");
    let mut sa_file = std::fs::File::create(&sa_path).unwrap();
    sa_file
        .write_all(
            br#"{
            "type": "service_account",
            "project_id": "test",
            "private_key_id": "fake",
            "private_key": "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n",
            "client_email": "fake@test.iam.gserviceaccount.com",
            "client_id": "0",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#,
        )
        .unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "bigquery",
            "svc-id",
            "--name",
            "t",
            "--service-account-file",
            sa_path.to_str().unwrap(),
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.t:t",
            "--org-id",
            "org",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest, &serde_json::json!({ "database": "default" }));
    assert_eq!(
        body["source"]["bigquery"]["tableMappings"][0]["sourceDatasetName"],
        "dataset"
    );
    let settings = &body["source"]["bigquery"]["settings"];
    assert_eq!(settings["replicationMode"], "snapshot");
    for field in [
        "allowNullableColumns",
        "initialLoadParallelism",
        "snapshotNumRowsPerPartition",
        "snapshotNumberOfParallelTables",
    ] {
        assert!(
            settings.get(field).is_none(),
            "{field} was sent without a corresponding flag: {settings}",
        );
    }
}

#[tokio::test]
async fn bigquery_snapshot_tuning_flags_are_sent_exactly() {
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("service-account.json");
    std::fs::write(&sa_path, "{}").unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "bigquery",
            "svc-id",
            "--name",
            "tuned-pipe",
            "--service-account-file",
            sa_path.to_str().unwrap(),
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.events:events",
            "--replication-mode",
            "snapshot",
            "--allow-nullable-columns",
            "true",
            "--initial-load-parallelism",
            "2.5",
            "--snapshot-rows-per-partition",
            "1000000",
            "--snapshot-parallel-tables",
            "3",
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["source"]["bigquery"]["settings"],
        serde_json::json!({
            "replicationMode": "snapshot",
            "allowNullableColumns": true,
            "initialLoadParallelism": 2.5,
            "snapshotNumRowsPerPartition": 1_000_000.0,
            "snapshotNumberOfParallelTables": 3.0,
        })
    );
}

#[tokio::test]
async fn bigquery_non_finite_tuning_is_rejected_before_key_file_or_http() {
    let mock = MockServer::start().await;
    let missing_key = "/missing/bigquery-service-account.json";

    for (flag, value) in [
        ("--initial-load-parallelism", "NaN"),
        ("--snapshot-rows-per-partition", "inf"),
        ("--snapshot-parallel-tables", "-inf"),
    ] {
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "create",
                "bigquery",
                "svc-id",
                "--name",
                "invalid-pipe",
                "--service-account-file",
                missing_key,
                "--staging-path",
                "gs://bucket/staging",
                flag,
                value,
                "--org-id",
                "org",
            ],
        );

        assert_eq!(
            output.status.code(),
            Some(2),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(flag), "{stderr}");
        assert!(stderr.contains("finite number"), "{stderr}");
        assert!(!stderr.contains(missing_key), "key file was read: {stderr}");
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn bigquery_destination_database_keeps_source_dataset_mapping_separate() {
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("service-account.json");
    std::fs::write(&sa_path, "{}").unwrap();
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "bigquery",
            "svc-id",
            "--name",
            "test-pipe",
            "--service-account-file",
            sa_path.to_str().unwrap(),
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "source_dataset.t:t",
            "--destination-database",
            "analytics",
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["destination"],
        serde_json::json!({ "database": "analytics" })
    );
    assert_eq!(
        body["source"]["bigquery"]["tableMappings"][0]["sourceDatasetName"],
        "source_dataset"
    );
}

// ── Postgres expansion ─────────────────────────────────────────────────────
//
// Beyond the absence cases above, these tests cover:
//   - The inverse: when --publication-name / --tls-host / --iam-role ARE
//     passed, the body must contain them with the exact value.
//   - Each `--postgres-type` enum variant flows through unchanged.
//   - Each `--replication-mode` enum variant.
//   - Multiple --table-mapping flags produce an array of N entries.
//   - --auth IAM_ROLE selects the right auth and serialises iamRole.

fn postgres_args_minimal() -> Vec<String> {
    [
        "clickpipe",
        "create",
        "postgres",
        "svc-id",
        "--name",
        "t",
        "--host",
        "pg",
        "--port",
        "5432",
        "--pg-database",
        "test",
        "--username",
        "u",
        "--password",
        "p",
        "--table-mapping",
        "public.t:t",
        "--replication-mode",
        "cdc",
        "--org-id",
        "org",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// The minimal create argument set for IAM_ROLE authentication: no
/// `--username`/`--password`, because the role ARN is the whole credential.
fn postgres_args_iam_role() -> Vec<String> {
    [
        "clickpipe",
        "create",
        "postgres",
        "svc-id",
        "--name",
        "t",
        "--host",
        "pg",
        "--port",
        "5432",
        "--pg-database",
        "test",
        "--table-mapping",
        "public.t:t",
        "--replication-mode",
        "cdc",
        "--org-id",
        "org",
        "--auth",
        "IAM_ROLE",
        "--iam-role",
        "arn:aws:iam::123:role/x",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[tokio::test]
async fn postgres_invalid_inputs_exit_as_usage_errors_before_auth_file_or_network() {
    let mock = MockServer::start().await;
    let missing_ca = "/missing/postgres-ca.pem";
    let secret_password = "postgres-password-must-not-appear";
    let replace_value = |args: &mut Vec<String>, flag: &str, value: &str| {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .unwrap_or_else(|| panic!("missing test flag {flag}"));
        args[index + 1] = value.into();
    };
    let base = || {
        let mut args = postgres_args_minimal();
        replace_value(&mut args, "--password", secret_password);
        args.extend(["--ca-certificate".into(), missing_ca.into()]);
        args
    };

    let mut cases = Vec::new();
    for port in ["0", "65536"] {
        let mut args = base();
        replace_value(&mut args, "--port", port);
        cases.push((args, "--port"));
    }

    let mut no_mapping = base();
    let mapping = no_mapping
        .iter()
        .position(|arg| arg == "--table-mapping")
        .expect("baseline table mapping");
    no_mapping.drain(mapping..=mapping + 1);
    cases.push((no_mapping, "--table-mapping"));

    for (mapping, diagnostic) in [
        (".events:events", "source schema must not be empty"),
        ("public.:events", "source table must not be empty"),
        ("public.events:", "target table must not be empty"),
    ] {
        let mut args = base();
        replace_value(&mut args, "--table-mapping", mapping);
        cases.push((args, diagnostic));
    }

    let mut missing_iam_role = base();
    missing_iam_role.extend(["--auth".into(), "IAM_ROLE".into()]);
    cases.push((missing_iam_role, "--iam-role"));

    let mut ignored_iam_role = base();
    ignored_iam_role.extend([
        "--iam-role".into(),
        "arn:aws:iam::123456789012:role/clickpipe".into(),
    ]);
    cases.push((
        ignored_iam_role,
        "--iam-role cannot be used with --auth basic",
    ));

    for mode in ["cdc", "snapshot"] {
        let mut args = base();
        replace_value(&mut args, "--replication-mode", mode);
        args.extend(["--replication-slot-name".into(), "existing_slot".into()]);
        cases.push((
            args,
            "--replication-slot-name can only be used with --replication-mode cdc_only",
        ));
    }

    let mut disable_with_ca = base();
    disable_with_ca.push("--disable-tls".into());
    cases.push((disable_with_ca, "--disable-tls"));

    for (tls_args, diagnostic) in [
        (
            vec!["--tls-host", "postgres.internal.example"],
            "--tls-host",
        ),
        (vec!["--skip-cert-verification"], "--skip-cert-verification"),
    ] {
        let mut args = postgres_args_minimal();
        replace_value(&mut args, "--password", secret_password);
        args.push("--disable-tls".into());
        args.extend(tls_args.into_iter().map(String::from));
        cases.push((args, diagnostic));
    }

    for (args, diagnostic) in cases {
        let output = invoke_cli_without_cloud_credentials(&mock, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(diagnostic), "{stderr}");
        assert!(!stderr.contains(missing_ca), "CA file was read: {stderr}");
        assert!(
            !stderr.contains(secret_password),
            "password leaked into diagnostic: {stderr}"
        );
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── Destination roles (issue #568) ─────────────────────────────────────────
//
// `--role` maps to `destination.roles`. Omitting it must leave the key out of
// the body entirely, because ClickPipes reads absence as "grant the default
// role"; an empty array would be a different instruction.

#[tokio::test]
async fn kafka_destination_roles_serialize_in_declaration_order() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = kafka_args_minimal();
    args.extend([
        "--role",
        "analytics_writer",
        "--role",
        "analytics_reader",
        // A repeated value is de-duplicated rather than sent twice.
        "--role",
        "analytics_writer",
    ]);
    let body = invoke_cli_capture_body(&mock, &args).await;

    assert_eq!(
        body["destination"]["roles"],
        serde_json::json!(["analytics_writer", "analytics_reader"]),
        "destination.roles should carry the --role values in order: {}",
        body["destination"],
    );
}

#[tokio::test]
async fn kafka_destination_roles_absent_when_role_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(&mock, &kafka_args_minimal()).await;

    let dest = &body["destination"];
    assert!(
        dest.get("roles").is_none(),
        "roles leaked into the destination body when --role was omitted: {dest}",
    );
}

#[tokio::test]
async fn kafka_destination_table_definition_file_controls_the_complete_wire_shape() {
    let mock = start_mock_clickpipes_api().await;
    let directory = tempfile::tempdir().unwrap();
    let definition_path = directory.path().join("table-definition.json");
    std::fs::write(
        &definition_path,
        serde_json::json!({
            "engine": {
                "columnIds": ["amount", "tax"],
                "type": "SummingMergeTree",
                "versionColumnId": null
            },
            "partitionBy": "toYYYYMM(created_at)",
            "primaryKey": "event_id",
            "sortingKey": ["event_id", "created_at"]
        })
        .to_string(),
    )
    .unwrap();
    let mut args: Vec<String> = kafka_args_minimal().into_iter().map(String::from).collect();
    args.extend([
        "--managed-table".into(),
        "false".into(),
        "--table-definition-file".into(),
        definition_path.to_string_lossy().into_owned(),
    ]);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    assert_eq!(
        body["destination"],
        serde_json::json!({
            "columns": [{"name": "id", "type": "Int64"}],
            "database": "default",
            "managedTable": false,
            "table": "events",
            "tableDefinition": {
                "engine": {
                    "columnIds": ["amount", "tax"],
                    "type": "SummingMergeTree"
                },
                "partitionBy": "toYYYYMM(created_at)",
                "primaryKey": "event_id",
                "sortingKey": ["event_id", "created_at"]
            }
        })
    );
}

#[tokio::test]
async fn object_storage_create_uses_the_shared_destination_table_definition() {
    let mock = start_mock_clickpipes_api().await;
    let directory = tempfile::tempdir().unwrap();
    let definition_path = directory.path().join("table-definition.json");
    std::fs::write(
        &definition_path,
        serde_json::json!({
            "engine": {
                "columnIds": [],
                "type": "MergeTree",
                "versionColumnId": null
            },
            "partitionBy": "toYYYYMM(created_at)",
            "primaryKey": "event_id",
            "sortingKey": ["event_id"]
        })
        .to_string(),
    )
    .unwrap();
    let definition_path = definition_path.to_string_lossy();
    let args = [
        "clickpipe",
        "create",
        "object-storage",
        "svc-id",
        "--name",
        "objects",
        "--source-url",
        "https://bucket.example/events",
        "--format",
        "JSONEachRow",
        "--database",
        "analytics",
        "--table",
        "events",
        "--column",
        "event_id:Int64",
        "--table-definition-file",
        definition_path.as_ref(),
        "--org-id",
        "org",
    ];

    let body = invoke_cli_capture_body(&mock, &args).await;

    assert_eq!(body["destination"]["managedTable"], true);
    assert_eq!(
        body["destination"]["tableDefinition"],
        serde_json::json!({
            "engine": {"type": "MergeTree"},
            "partitionBy": "toYYYYMM(created_at)",
            "primaryKey": "event_id",
            "sortingKey": ["event_id"]
        })
    );
    assert_eq!(
        body["source"]["objectStorage"]["url"],
        "https://bucket.example/events"
    );
}

#[tokio::test]
async fn kinesis_destination_table_definition_can_be_read_from_stdin() {
    let mock = start_mock_clickpipes_api().await;
    let args = [
        "clickpipe",
        "create",
        "kinesis",
        "svc-id",
        "--name",
        "kinesis-pipe",
        "--stream-name",
        "events",
        "--region",
        "eu-west-1",
        "--format",
        "JSONEachRow",
        "--database",
        "analytics",
        "--table",
        "events",
        "--column",
        "event_id:Int64",
        "--table-definition-file",
        "-",
        "--org-id",
        "org",
    ];
    let definition = serde_json::json!({
        "engine": {
            "columnIds": [],
            "type": "ReplacingMergeTree",
            "versionColumnId": "version"
        },
        "partitionBy": "toYYYYMM(created_at)",
        "primaryKey": "event_id",
        "sortingKey": ["event_id"]
    });

    let body =
        invoke_cli_capture_body_with_stdin(&mock, &args, definition.to_string().as_bytes()).await;

    assert_eq!(body["destination"]["managedTable"], true);
    assert_eq!(
        body["destination"]["tableDefinition"],
        serde_json::json!({
            "engine": {
                "type": "ReplacingMergeTree",
                "versionColumnId": "version"
            },
            "partitionBy": "toYYYYMM(created_at)",
            "primaryKey": "event_id",
            "sortingKey": ["event_id"]
        })
    );
}

#[tokio::test]
async fn invalid_destination_table_definition_fails_before_the_api_request() {
    let mock = start_mock_clickpipes_api().await;
    let directory = tempfile::tempdir().unwrap();

    for (definition, diagnostic) in [
        (
            serde_json::json!({
                "engine": {
                    "columnIds": [],
                    "type": "MergeTree",
                    "versionColumnId": null,
                    "versionColumn": "typo"
                },
                "partitionBy": "tuple()",
                "primaryKey": "event_id",
                "sortingKey": ["event_id"]
            }),
            "versionColumn",
        ),
        (
            serde_json::json!({
                "engine": {
                    "columnIds": [],
                    "type": "UnknownTree",
                    "versionColumnId": null
                },
                "partitionBy": "tuple()",
                "primaryKey": "event_id",
                "sortingKey": ["event_id"]
            }),
            "UnknownTree",
        ),
    ] {
        let definition_path = directory.path().join(format!("{diagnostic}.json"));
        std::fs::write(&definition_path, definition.to_string()).unwrap();
        let mut args: Vec<String> = kafka_args_minimal().into_iter().map(String::from).collect();
        args.extend([
            "--table-definition-file".into(),
            definition_path.to_string_lossy().into_owned(),
        ]);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(diagnostic), "{stderr}");
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_destination_roles_serialize_on_database_pipes() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.extend([
        "--role".to_string(),
        "analytics_reader".to_string(),
        "--role".to_string(),
        "analytics_writer".to_string(),
    ]);
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    let dest = &body["destination"];
    assert_eq!(
        dest["roles"],
        serde_json::json!(["analytics_reader", "analytics_writer"]),
        "destination.roles should survive the database-pipe destination: {dest}",
    );
    // The four fields database pipes reject must still be absent.
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into the postgres destination body: {dest}",
        );
    }
}

#[tokio::test]
async fn reserved_destination_role_is_a_usage_error_before_any_request() {
    let mock = start_mock_clickpipes_api().await;

    for reserved in ["clickpipes", "clickpipes_system"] {
        let mut args = postgres_args_minimal();
        args.extend(["--role".to_string(), reserved.to_string()]);
        let output = invoke_cli_without_cloud_credentials(&mock, &args);

        assert_eq!(
            output.status.code(),
            Some(2),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("reserved by ClickPipes"), "{stderr}");
        assert!(stderr.contains(reserved), "{stderr}");
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_publication_name_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--publication-name".into());
    args.push("my_pub".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["publicationName"], "my_pub",
        "publicationName should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_replication_slot_name_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    let mode = args.iter().position(|arg| arg == "cdc").unwrap();
    args[mode] = "cdc_only".into();
    args.push("--replication-slot-name".into());
    args.push("my_slot".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["replicationSlotName"], "my_slot",
        "replicationSlotName should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_tls_host_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--tls-host".into());
    args.push("pg.example.com".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["tlsHost"], "pg.example.com",
        "tlsHost should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_iam_role_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let args = postgres_args_iam_role();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["iamRole"], "arn:aws:iam::123:role/x",
        "iamRole should round-trip the user-provided value"
    );
    assert_eq!(body["source"]["postgres"]["authentication"], "IAM_ROLE");
}

#[tokio::test]
async fn postgres_iam_role_create_omits_the_credentials_object() {
    // IAM_ROLE authentication has no username or password: the role ARN is the
    // whole credential, so `credentials` must be absent from the wire rather
    // than sent as an empty username/password pair.
    let mock = start_mock_clickpipes_api().await;
    let args = postgres_args_iam_role();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert!(
        body["source"]["postgres"].get("credentials").is_none(),
        "credentials must not be sent for IAM_ROLE auth, got {}",
        body["source"]["postgres"]
    );
}

#[tokio::test]
async fn postgres_basic_auth_create_sends_the_credentials_object() {
    let mock = start_mock_clickpipes_api().await;
    let args = postgres_args_minimal();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(body["source"]["postgres"]["credentials"]["username"], "u");
    assert_eq!(body["source"]["postgres"]["credentials"]["password"], "p");
    assert_eq!(body["source"]["postgres"]["authentication"], "basic");
}

#[tokio::test]
async fn postgres_credentials_with_iam_role_auth_are_rejected() {
    let mock = MockServer::start().await;
    let mut args = postgres_args_iam_role();
    args.push("--username".into());
    args.push("u".into());
    args.push("--password".into());
    args.push("p".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
    // Reported as a usage error against the owning command, the same way
    // `--iam-role` with basic auth is: clap cannot express "forbidden for this
    // value of another argument", so the check runs after parsing.
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "--username and --password cannot be used with --auth IAM_ROLE; use --auth basic"
        ),
        "{stderr}"
    );
}

/// The minimal `clickpipe create mysql` argument set for basic auth.
fn mysql_args_minimal() -> Vec<String> {
    [
        "clickpipe",
        "create",
        "mysql",
        "svc-id",
        "--name",
        "t",
        "--host",
        "mysql",
        "--port",
        "3306",
        "--table-mapping",
        "mydb.t:t",
        "--replication-mode",
        "cdc",
        "--org-id",
        "org",
        "--username",
        "u",
        "--password",
        "p",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// The same set for IAM_ROLE authentication: no `--username`/`--password`,
/// because the role ARN is the whole credential.
fn mysql_args_iam_role() -> Vec<String> {
    [
        "clickpipe",
        "create",
        "mysql",
        "svc-id",
        "--name",
        "t",
        "--host",
        "mysql",
        "--port",
        "3306",
        "--table-mapping",
        "mydb.t:t",
        "--replication-mode",
        "cdc",
        "--org-id",
        "org",
        "--auth",
        "IAM_ROLE",
        "--iam-role",
        "arn:aws:iam::123:role/x",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[tokio::test]
async fn mysql_iam_role_create_omits_the_credentials_object() {
    // IAM_ROLE authentication has no username or password: the role ARN is the
    // whole credential, so `credentials` must be absent from the wire rather
    // than sent as an empty username/password pair.
    let mock = start_mock_clickpipes_api().await;
    let args = mysql_args_iam_role();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert!(
        body["source"]["mysql"].get("credentials").is_none(),
        "credentials must not be sent for IAM_ROLE auth, got {}",
        body["source"]["mysql"]
    );
    assert_eq!(
        body["source"]["mysql"]["iamRole"], "arn:aws:iam::123:role/x",
        "iamRole should round-trip the user-provided value"
    );
    assert_eq!(body["source"]["mysql"]["authentication"], "IAM_ROLE");
}

#[tokio::test]
async fn mysql_basic_auth_create_sends_the_credentials_object() {
    let mock = start_mock_clickpipes_api().await;
    let args = mysql_args_minimal();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(body["source"]["mysql"]["credentials"]["username"], "u");
    assert_eq!(body["source"]["mysql"]["credentials"]["password"], "p");
    assert_eq!(body["source"]["mysql"]["authentication"], "basic");
}

#[tokio::test]
async fn mysql_credentials_with_iam_role_auth_are_rejected() {
    let mock = MockServer::start().await;
    let mut args = mysql_args_iam_role();
    args.push("--username".into());
    args.push("u".into());
    args.push("--password".into());
    args.push("p".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
    // Reported as a usage error against the owning command, the same way
    // `--iam-role` with basic auth is: clap cannot express "forbidden for this
    // value of another argument", so the check runs after parsing.
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "--username and --password cannot be used with --auth IAM_ROLE; use --auth basic"
        ),
        "{stderr}"
    );
    // The usage line names the source subcommand the flags belong to.
    assert!(stderr.contains("clickpipe create mysql"), "{stderr}");
}

#[tokio::test]
async fn mysql_iam_role_with_basic_auth_is_rejected() {
    let mock = MockServer::start().await;
    let mut args = mysql_args_minimal();
    args.push("--iam-role".into());
    args.push("arn:aws:iam::123:role/x".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--iam-role cannot be used with --auth basic; use --auth IAM_ROLE"),
        "{stderr}"
    );
}

#[tokio::test]
async fn postgres_ca_certificate_file_contents_flow_to_body() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let ca_path = dir.path().join("ca.pem");
    let pem = "-----BEGIN CERTIFICATE-----\nCA_PEM_CONTENT\n-----END CERTIFICATE-----\n";
    std::fs::File::create(&ca_path)
        .unwrap()
        .write_all(pem.as_bytes())
        .unwrap();

    let mut args = postgres_args_minimal();
    args.push("--ca-certificate".into());
    args.push(ca_path.to_str().unwrap().to_string());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert!(
        body["source"]["postgres"]["caCertificate"]
            .as_str()
            .map(|s| s.contains("CA_PEM_CONTENT"))
            .unwrap_or(false),
        "caCertificate body should contain the file's PEM content, got {}",
        body["source"]["postgres"]["caCertificate"]
    );
    assert_eq!(body["source"]["postgres"]["disableTls"], false);
    assert_eq!(body["source"]["postgres"]["skipCertVerification"], false);
}

#[tokio::test]
async fn postgres_unknown_authority_error_preserves_api_detail_and_adds_ca_hint() {
    let mock = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let ca_path = directory.path().join("private-ca.pem");
    let ca_path_display = ca_path.to_string_lossy().into_owned();
    let ca_contents = "certificate-body-must-not-appear";
    std::fs::write(&ca_path, ca_contents).unwrap();
    let password = "postgres-password-must-not-appear";
    let api_error = "BAD_REQUEST: failed to establish connection: tls: failed to verify \
                     certificate: x509: certificate signed by unknown authority";
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": api_error,
        })))
        .mount(&mock)
        .await;

    let mut args = postgres_args_minimal();
    let password_index = args.iter().position(|arg| arg == "--password").unwrap();
    args[password_index + 1] = password.into();
    args.extend(["--ca-certificate".into(), ca_path_display.clone()]);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(api_error), "{stderr}");
    assert!(stderr.contains("--ca-certificate <PATH>"), "{stderr}");
    assert!(
        stderr.contains("private or self-signed source CA"),
        "{stderr}"
    );
    for secret in [password, ca_contents, ca_path_display.as_str()] {
        assert!(
            !stderr.contains(secret),
            "secret leaked into error: {stderr}"
        );
    }
}

#[tokio::test]
async fn postgres_hostname_mismatch_error_preserves_api_detail_and_adds_tls_host_hint() {
    let mock = MockServer::start().await;
    let api_error = "BAD_REQUEST: failed to establish connection: tls: failed to verify \
                     certificate: x509: certificate is valid for postgres.internal.example.com, \
                     not 10.0.0.8";
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": api_error,
        })))
        .mount(&mock)
        .await;

    let args = postgres_args_minimal();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = invoke_cli_with_cloud_credentials(&mock, &arg_refs);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(api_error), "{stderr}");
    assert!(stderr.contains("--tls-host <HOSTNAME>"), "{stderr}");
    assert!(stderr.contains("does not match `--host`"), "{stderr}");
}

#[tokio::test]
async fn postgres_replication_mode_snapshot_serializes() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    let idx = args.iter().position(|a| a == "cdc").unwrap();
    args[idx] = "snapshot".into();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["replicationMode"],
        "snapshot",
    );
}

#[tokio::test]
async fn postgres_replication_mode_cdc_only_serializes() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    let idx = args.iter().position(|a| a == "cdc").unwrap();
    args[idx] = "cdc_only".into();
    // cdc_only typically requires explicit publication + slot to be useful;
    // assert the wire shape regardless.
    args.push("--publication-name".into());
    args.push("p".into());
    args.push("--replication-slot-name".into());
    args.push("s".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["replicationMode"],
        "cdc_only",
    );
}

#[tokio::test]
async fn postgres_multiple_table_mappings_serialize_as_array() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    // Append a second --table-mapping. The first one was set in
    // `postgres_args_minimal` as public.t:t.
    args.push("--table-mapping".into());
    args.push("public.t2:t2_dst".into());
    args.push("--table-mapping".into());
    args.push("other_schema.t3:t3_dst".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    let mappings = body["source"]["postgres"]["tableMappings"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "tableMappings should be an array, got: {}",
                body["source"]["postgres"]["tableMappings"]
            )
        });
    assert_eq!(
        mappings.len(),
        3,
        "expected 3 table mappings (minimal default + 2 added), got {}: {:?}",
        mappings.len(),
        mappings
    );
    let target_tables: Vec<&str> = mappings
        .iter()
        .filter_map(|m| m["targetTable"].as_str())
        .collect();
    assert!(target_tables.contains(&"t"));
    assert!(target_tables.contains(&"t2_dst"));
    assert!(target_tables.contains(&"t3_dst"));
}

// ── Postgres JSON table mappings (issue #566) ──────────────────────────────
//
// `--table-mapping-json` takes the API's table mapping object verbatim. These
// shape the destination table ClickPipes creates and cannot be changed later,
// so the body must reproduce the JSON exactly, with no field invented and none
// dropped.

#[tokio::test]
async fn postgres_table_mapping_json_reproduces_every_field_in_the_body() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--table-mapping-json".into());
    args.push(
        serde_json::json!({
            "sourceSchemaName": "public",
            "sourceTable": "users",
            "targetTable": "users_raw",
            "excludedColumns": ["ssn", "dob"],
            "sortingKeys": ["created_at", "id"],
            "partitionByExpr": "toYYYYMM(created_at)",
            "partitionKey": "id",
            "tableEngine": "ReplacingMergeTree",
        })
        .to_string(),
    );
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    let mappings = body["source"]["postgres"]["tableMappings"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "tableMappings should be an array, got: {}",
                body["source"]["postgres"]["tableMappings"]
            )
        });
    // The simple mapping from the minimal args comes first, then the JSON one.
    assert_eq!(mappings.len(), 2, "{mappings:?}");
    assert_eq!(
        mappings[0],
        serde_json::json!({
            "sourceSchemaName": "public",
            "sourceTable": "t",
            "targetTable": "t",
            "excludedColumns": [],
            "sortingKeys": [],
            "useCustomSortingKey": false,
            "partitionByExpr": "",
            "partitionKey": "",
            "tableEngine": "MergeTree",
        }),
        "the simple form's wire shape must not change",
    );
    assert_eq!(
        mappings[1],
        serde_json::json!({
            "sourceSchemaName": "public",
            "sourceTable": "users",
            "targetTable": "users_raw",
            "excludedColumns": ["ssn", "dob"],
            "sortingKeys": ["created_at", "id"],
            // Set for the caller, because the API ignores the keys without it.
            "useCustomSortingKey": true,
            "partitionByExpr": "toYYYYMM(created_at)",
            "partitionKey": "id",
            "tableEngine": "ReplacingMergeTree",
        }),
    );
}

#[tokio::test]
async fn postgres_table_mapping_json_alone_satisfies_the_mapping_requirement() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    let simple = args
        .iter()
        .position(|arg| arg == "--table-mapping")
        .expect("baseline table mapping");
    args.drain(simple..=simple + 1);
    args.push("--table-mapping-json".into());
    args.push(
        r#"{"sourceSchemaName":"audit","sourceTable":"events","targetTable":"audit_events","tableEngine":"Null"}"#
            .into(),
    );
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    assert_eq!(
        body["source"]["postgres"]["tableMappings"],
        serde_json::json!([{
            "sourceSchemaName": "audit",
            "sourceTable": "events",
            "targetTable": "audit_events",
            "excludedColumns": [],
            "sortingKeys": [],
            "useCustomSortingKey": false,
            "partitionByExpr": "",
            "partitionKey": "",
            "tableEngine": "Null",
        }]),
    );
}

#[tokio::test]
async fn postgres_invalid_table_mapping_json_is_a_usage_error_before_any_request() {
    let mock = MockServer::start().await;

    for (mapping, diagnostic) in [
        ("{ nope", "--table-mapping-json #1: invalid JSON"),
        (
            r#"{"sourceSchemaName":"public","sourceTable":"users"}"#,
            "targetTable is required and must not be empty",
        ),
        (
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","excludeColumns":["ssn"]}"#,
            "unknown field excludeColumns",
        ),
        (
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":["id"],"useCustomSortingKey":false}"#,
            "sortingKeys is set but useCustomSortingKey is false",
        ),
        (
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","tableEngine":"MergeTre"}"#,
            "invalid tableEngine: unknown value 'MergeTre'",
        ),
    ] {
        let mut args = postgres_args_minimal();
        args.push("--table-mapping-json".into());
        args.push(mapping.into());
        let output = invoke_cli_without_cloud_credentials(&mock, &args);

        assert_eq!(
            output.status.code(),
            Some(2),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(diagnostic), "{stderr}");
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── MySQL, MongoDB and BigQuery JSON table mappings (issue #691) ───────────

#[tokio::test]
async fn mysql_table_mapping_json_reproduces_every_field_after_simple_mappings() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "mysql-mappings",
            "--host",
            "mysql.example",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "source.events:events",
            "--table-mapping-json",
            r#"{"sourceSchemaName":"sales","sourceTable":"orders","targetTable":"orders_raw","excludedColumns":["private_note"],"sortingKeys":["created_at","id"],"partitionKey":"id","partitionByExpr":"toYYYYMM(created_at)","tableEngine":"ReplacingMergeTree"}"#,
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["source"]["mysql"]["tableMappings"],
        serde_json::json!([
            {
                "sourceSchemaName": "source",
                "sourceTable": "events",
                "targetTable": "events",
            },
            {
                "sourceSchemaName": "sales",
                "sourceTable": "orders",
                "targetTable": "orders_raw",
                "excludedColumns": ["private_note"],
                "sortingKeys": ["created_at", "id"],
                "useCustomSortingKey": true,
                "partitionKey": "id",
                "partitionByExpr": "toYYYYMM(created_at)",
                "tableEngine": "ReplacingMergeTree",
            }
        ])
    );
}

#[tokio::test]
async fn mongodb_table_mapping_json_can_be_the_only_mapping() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "mongodb-mappings",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping-json",
            r#"{"sourceDatabaseName":"sales","sourceCollection":"orders","targetTable":"orders_raw","tableEngine":"Null"}"#,
            "--org-id",
            "org",
        ],
    )
    .await;

    assert_eq!(
        body["source"]["mongodb"]["tableMappings"],
        serde_json::json!([{
            "sourceDatabaseName": "sales",
            "sourceCollection": "orders",
            "targetTable": "orders_raw",
            "tableEngine": "Null",
        }])
    );
}

#[tokio::test]
async fn bigquery_service_account_table_mapping_json_preserves_optional_shapes() {
    let mock = start_mock_clickpipes_api().await;
    let directory = tempfile::tempdir().unwrap();
    let key = directory.path().join("service-account.json");
    std::fs::write(&key, "{}").unwrap();
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "bigquery",
            "svc-id",
            "--name",
            "bigquery-service-account",
            "--service-account-file",
            key.to_str().unwrap(),
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping-json",
            r#"{"sourceDatasetName":"sales","sourceTable":"orders","targetTable":"orders_raw","excludedColumns":[],"sortingKeys":[],"useCustomSortingKey":false}"#,
            "--org-id",
            "org",
        ],
    )
    .await;

    let source = &body["source"]["bigquery"];
    assert!(source.get("authentication").is_none());
    assert!(source.get("credentials").is_some());
    assert_eq!(
        source["tableMappings"],
        serde_json::json!([{
            "sourceDatasetName": "sales",
            "sourceTable": "orders",
            "targetTable": "orders_raw",
            "excludedColumns": [],
            "sortingKeys": [],
            "useCustomSortingKey": false,
        }])
    );
    let mapping = &source["tableMappings"][0];
    assert!(mapping.get("tableEngine").is_none());
}

#[tokio::test]
async fn bigquery_workload_identity_table_mapping_json_stays_in_the_union_arm() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "bigquery",
            "svc-id",
            "--name",
            "bigquery-workload-identity",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            "--project-id",
            "source-project",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping-json",
            r#"{"sourceDatasetName":"sales","sourceTable":"orders","targetTable":"orders_raw","excludedColumns":["private_note"],"sortingKeys":["id"],"tableEngine":"MergeTree"}"#,
            "--org-id",
            "org",
        ],
    )
    .await;

    let source = &body["source"]["bigquery"];
    assert_eq!(
        source["authentication"],
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    assert_eq!(source["projectId"], "source-project");
    assert!(source.get("credentials").is_none());
    assert_eq!(
        source["tableMappings"],
        serde_json::json!([{
            "sourceDatasetName": "sales",
            "sourceTable": "orders",
            "targetTable": "orders_raw",
            "excludedColumns": ["private_note"],
            "sortingKeys": ["id"],
            "useCustomSortingKey": true,
            "tableEngine": "MergeTree",
        }])
    );
}

#[tokio::test]
async fn non_postgres_invalid_table_mapping_json_is_a_usage_error_before_files_or_http() {
    let mock = MockServer::start().await;
    let cases = [
        (
            "mysql",
            vec![
                "clickpipe",
                "create",
                "mysql",
                "svc-id",
                "--name",
                "invalid-mysql",
                "--host",
                "mysql.example",
                "--username",
                "u",
                "--password",
                "p",
                "--ca-certificate",
                "/missing/mysql-ca.pem",
                "--table-mapping-json",
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","partitionKey":"snapshot_id","partitionByExpr":"toYYYYMM(ts)","tableEngine":"MergeTre"}"#,
                "--org-id",
                "org",
            ],
            "invalid tableEngine",
        ),
        (
            "mongodb",
            vec![
                "clickpipe",
                "create",
                "mongodb",
                "svc-id",
                "--name",
                "invalid-mongodb",
                "--uri",
                "mongodb://mongo.example/source",
                "--username",
                "u",
                "--password",
                "p",
                "--ca-certificate",
                "/missing/mongodb-ca.pem",
                "--table-mapping-json",
                r#"{"sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t","sortingKeys":[]}"#,
                "--org-id",
                "org",
            ],
            "unknown field sortingKeys",
        ),
        (
            "bigquery",
            vec![
                "clickpipe",
                "create",
                "bigquery",
                "svc-id",
                "--name",
                "invalid-bigquery",
                "--service-account-file",
                "/missing/bigquery-key.json",
                "--staging-path",
                "gs://bucket/staging",
                "--table-mapping-json",
                r#"{"sourceDatasetName":"ds","sourceTable":"t","targetTable":"t","useCustomSortingKey":true}"#,
                "--org-id",
                "org",
            ],
            "useCustomSortingKey is true but sortingKeys is empty",
        ),
    ];

    for (source, args, diagnostic) in cases {
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{source} stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(diagnostic), "{source}: {stderr}");
        assert!(
            !stderr.contains("No such file"),
            "{source} read a sensitive file before validation: {stderr}"
        );
    }

    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── Postgres CDC pipe settings (issue #565) ────────────────────────────────
//
// The snapshot and initial-load settings are create-time-only at the API, so
// the create body is the only chance to set them. These tests pin the exact
// key set of `source.postgres.settings`: no flag may leak a zero value, and
// no passed flag may be dropped.

fn postgres_settings_keys(settings: &Value) -> Vec<String> {
    let mut keys: Vec<String> = settings
        .as_object()
        .unwrap_or_else(|| panic!("settings should be an object, got: {settings}"))
        .keys()
        .cloned()
        .collect();
    keys.sort_unstable();
    keys
}

#[tokio::test]
async fn postgres_cdc_settings_are_absent_unless_their_flags_are_passed() {
    let mock = start_mock_clickpipes_api().await;
    let args = postgres_args_minimal();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    // `allowNullableColumns`, `deleteOnMerge` and `enableFailoverSlots` are
    // required by the schema, so they always serialize; every other setting
    // must be omitted when its flag is omitted.
    assert_eq!(
        postgres_settings_keys(&body["source"]["postgres"]["settings"]),
        [
            "allowNullableColumns",
            "deleteOnMerge",
            "enableFailoverSlots",
            "replicationMode",
        ],
        "unexpected settings shape: {}",
        body["source"]["postgres"]["settings"]
    );
    let settings = &body["source"]["postgres"]["settings"];
    assert_eq!(settings["allowNullableColumns"], false);
    assert_eq!(settings["deleteOnMerge"], false);
    assert_eq!(settings["enableFailoverSlots"], false);
}

#[tokio::test]
async fn postgres_cdc_settings_serialize_exactly_the_flags_that_were_passed() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    for arg in [
        "--sync-interval-seconds",
        "30",
        "--pull-batch-size",
        "50000",
        "--initial-load-parallelism",
        "4",
        "--snapshot-rows-per-partition",
        "1000000",
        "--snapshot-parallel-tables",
        "3",
        "--allow-nullable-columns",
        "true",
        "--enable-failover-slots",
        "false",
        "--delete-on-merge",
        "true",
    ] {
        args.push(arg.into());
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    let settings = &body["source"]["postgres"]["settings"];
    assert_eq!(
        postgres_settings_keys(settings),
        [
            "allowNullableColumns",
            "deleteOnMerge",
            "enableFailoverSlots",
            "initialLoadParallelism",
            "pullBatchSize",
            "replicationMode",
            "snapshotNumRowsPerPartition",
            "snapshotNumberOfParallelTables",
            "syncIntervalSeconds",
        ],
        "unexpected settings shape: {settings}"
    );
    assert_eq!(settings["replicationMode"], "cdc");
    assert_eq!(settings["syncIntervalSeconds"], 30);
    assert_eq!(settings["pullBatchSize"], 50000);
    assert_eq!(settings["initialLoadParallelism"], 4);
    assert_eq!(settings["snapshotNumRowsPerPartition"], 1000000);
    assert_eq!(settings["snapshotNumberOfParallelTables"], 3);
    assert_eq!(settings["allowNullableColumns"], true);
    // An explicit `false` is sent, not dropped.
    assert_eq!(settings["enableFailoverSlots"], false);
    assert_eq!(settings["deleteOnMerge"], true);
}

// Each --postgres-type value should serialize to the matching enum string.
// The OpenAPI enum has 11 variants; the CLI accepts them via PossibleValuesParser
// and the handler uses parse_enum to convert. A regression here would
// silently change the source type the server uses to route the connection.

macro_rules! postgres_type_test {
    ($test_name:ident, $cli_value:literal, $wire_value:literal) => {
        #[tokio::test]
        async fn $test_name() {
            let mock = start_mock_clickpipes_api().await;
            let mut args = postgres_args_minimal();
            args.push("--postgres-type".into());
            args.push($cli_value.into());
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let body = invoke_cli_capture_body(&mock, &arg_refs).await;
            assert_eq!(
                body["source"]["postgres"]["type"], $wire_value,
                "--postgres-type {} should serialize to wire value {}",
                $cli_value, $wire_value,
            );
        }
    };
}

postgres_type_test!(postgres_type_postgres_serializes, "postgres", "postgres");
postgres_type_test!(postgres_type_supabase_serializes, "supabase", "supabase");
postgres_type_test!(postgres_type_neon_serializes, "neon", "neon");
postgres_type_test!(postgres_type_alloydb_serializes, "alloydb", "alloydb");
postgres_type_test!(
    postgres_type_planetscale_serializes,
    "planetscale",
    "planetscale"
);
postgres_type_test!(
    postgres_type_rdspostgres_serializes,
    "rdspostgres",
    "rdspostgres"
);
postgres_type_test!(
    postgres_type_aurorapostgres_serializes,
    "aurorapostgres",
    "aurorapostgres"
);
postgres_type_test!(
    postgres_type_cloudsqlpostgres_serializes,
    "cloudsqlpostgres",
    "cloudsqlpostgres"
);
postgres_type_test!(
    postgres_type_azurepostgres_serializes,
    "azurepostgres",
    "azurepostgres"
);
postgres_type_test!(
    postgres_type_crunchybridge_serializes,
    "crunchybridge",
    "crunchybridge"
);
postgres_type_test!(postgres_type_tigerdata_serializes, "tigerdata", "tigerdata");

// ── Dotenv ─────────────────────────────────────────────────────────────────
//
// A `.env` file in the current working directory supplying
// `CLICKHOUSE_CLOUD_API_KEY` + `CLICKHOUSE_CLOUD_API_SECRET` should produce
// the exact same `Authorization: Basic <base64>` header as exporting those
// vars in the shell. End-to-end proof that the resolver picks up `.env` and
// hands them to the lib client's basic-auth path.

#[tokio::test]
async fn dotenv_creds_produce_basic_auth_request() {
    use std::io::Write;

    let mock = MockServer::start().await;

    let stub_orgs = serde_json::json!({
        "result": [],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_orgs))
        .mount(&mock)
        .await;

    // Put `.env` in the working directory and run the binary cd'd into it,
    // because the loader reads `cwd/.env` rather than walking ancestor
    // directories. The parent process's env vars are cleared in the child so
    // the .env is the only source of credentials — otherwise the test could
    // silently pass for the wrong reason.
    let dir = tempfile::tempdir().unwrap();
    let mut env_file = std::fs::File::create(dir.path().join(".env")).unwrap();
    env_file
        .write_all(
            b"CLICKHOUSE_CLOUD_API_KEY=dotenv-key\nCLICKHOUSE_CLOUD_API_SECRET=dotenv-secret\n",
        )
        .unwrap();
    drop(env_file);

    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .current_dir(dir.path())
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let auth = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::GET)
        .and_then(|r| r.headers.get("Authorization"))
        .expect("no Authorization header recorded");
    let auth_str = auth.to_str().expect("non-utf8 auth header");
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "dotenv-key:dotenv-secret",
        )
    );
    assert_eq!(
        auth_str, expected,
        "Authorization header should match the .env credentials exactly"
    );
}

// ── Service query auth modes (issue #247) ──────────────────────────────────
//
// `cloud service query` has two auth paths:
//   - API key auth: a stored per-service key is preferred; otherwise the
//     active API key is tried directly before a new key is auto-provisioned.
//   - OAuth: the user's own bearer token is sent directly to the query host
//     — no key lookup and, crucially, NO provisioning calls (key creation
//     and endpoint upsert need write access an OAuth token doesn't have).
// Both tests run the binary against two mocks: one impersonating the
// control plane (service lookup), one impersonating the query host (wired
// up via CLICKHOUSE_CLOUD_QUERY_HOST, which overrides host derivation).

const QUERY_TEST_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";

async fn start_mock_control_plane_with_service() -> MockServer {
    let mock = MockServer::start().await;
    let stub_service = serde_json::json!({
        "result": { "id": QUERY_TEST_SERVICE_ID, "name": "demo" },
        "status": 200,
        "requestId": "stub-service-get",
    });
    let stub_services = serde_json::json!({
        "result": [{ "id": QUERY_TEST_SERVICE_ID, "name": "demo" }],
        "status": 200,
        "requestId": "stub-service-list",
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_service))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_services))
        .mount(&mock)
        .await;
    mock
}

async fn start_mock_query_host() -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .mount(&mock)
        .await;
    mock
}

async fn start_mock_query_host_for_provisioning() -> MockServer {
    let mock = MockServer::start().await;
    let basic_auth = |credentials: &str| {
        format!(
            "Basic {}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials)
        )
    };
    let primary_auth = basic_auth("fake-key-for-tests:fake-secret-for-tests");
    let provisioned_auth = basic_auth("provisioned-key-id:provisioned-key-secret");
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .and(header("authorization", primary_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .with_priority(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .and(header("authorization", provisioned_auth.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .with_priority(5)
        .mount(&mock)
        .await;
    mock
}

async fn invoke_oauth_service_query_response(
    body: Vec<u8>,
    extra_args: &[&str],
    agent: bool,
) -> (std::process::Output, MockServer) {
    let control = start_mock_control_plane_with_service().await;
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .expect(1)
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &control.uri());

    let url = control.uri();
    let mut args = vec![
        "cloud".to_string(),
        "--url".to_string(),
        url,
        "service".to_string(),
        "query".to_string(),
        "--id".to_string(),
        QUERY_TEST_SERVICE_ID.to_string(),
        "--org-id".to_string(),
        "org-1".to_string(),
        "--query".to_string(),
        "SELECT 1".to_string(),
    ];
    args.extend(extra_args.iter().map(|arg| (*arg).to_string()));

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    if agent {
        command.env("AGENT", "opencode");
    }
    let output = command
        .env("DO_NOT_TRACK", "1")
        .args(args)
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        // Pin stdin: `--query` now refuses to run when bytes are waiting on a
        // non-terminal stdin (#641), so these tests must not inherit whatever
        // the test runner happens to have there.
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");

    (output, query_host)
}

// ── `--query` versus stdin (issue #641) ────────────────────────────────────
//
// `--query` sends one request body, so a redirected data file can never be
// part of it. Rather than discard it silently, the CLI refuses before any
// network call. An empty non-terminal stdin (`</dev/null`, CI, an agent
// harness) is not data and must keep working, including when the writer is
// held open and never writes.

/// How a test drives the child's stdin.
enum StdinPlan<'a> {
    /// Closed immediately, the `< /dev/null` shape.
    Null,
    /// A regular file with content already in it, the `< data.csv` shape.
    File(std::fs::File),
    /// Written and closed as soon as the child is spawned.
    Write(&'a [u8]),
    /// A pipe whose writer waits before producing, the `producer |
    /// clickhousectl` race: the shell starts both at once, so the data can
    /// arrive after the CLI has already looked at stdin.
    WriteAfter(&'a [u8], std::time::Duration),
    /// A pipe that is held open and never written to: a harness that wired up
    /// stdin and produced nothing. The CLI must not hang on it.
    IdleOpen,
}

/// Run `cloud service query` over OAuth with a caller-chosen stdin. Returns
/// the process output plus both mocks so a test can assert that nothing was
/// requested at all.
async fn invoke_oauth_service_query_with_stdin(
    sql_args: &[&str],
    stdin_plan: StdinPlan<'_>,
) -> (std::process::Output, MockServer, MockServer) {
    let control = start_mock_control_plane_with_service().await;
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &control.uri());

    let mut args = vec![
        "cloud".to_string(),
        "--url".to_string(),
        control.uri(),
        "service".to_string(),
        "query".to_string(),
        "--id".to_string(),
        QUERY_TEST_SERVICE_ID.to_string(),
        "--org-id".to_string(),
        "org-1".to_string(),
    ];
    args.extend(sql_args.iter().map(|arg| (*arg).to_string()));

    let stdin = match &stdin_plan {
        StdinPlan::Null => Stdio::null(),
        StdinPlan::File(file) => Stdio::from(file.try_clone().unwrap()),
        StdinPlan::Write(_) | StdinPlan::WriteAfter(..) | StdinPlan::IdleOpen => Stdio::piped(),
    };

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let mut child = command
        .env("DO_NOT_TRACK", "1")
        .args(args)
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");

    // `wait_with_output` closes the child's stdin, so the writer has to be
    // taken out of the `Child` for any plan that must outlive the wait.
    let held_open = match stdin_plan {
        StdinPlan::Write(bytes) => {
            let mut pipe = child.stdin.take().expect("piped stdin");
            pipe.write_all(bytes).unwrap();
            None
        }
        StdinPlan::WriteAfter(bytes, delay) => {
            let mut pipe = child.stdin.take().expect("piped stdin");
            std::thread::sleep(delay);
            pipe.write_all(bytes).unwrap();
            None
        }
        StdinPlan::IdleOpen => Some(child.stdin.take().expect("piped stdin")),
        StdinPlan::Null | StdinPlan::File(_) => None,
    };
    let output = child.wait_with_output().expect("failed to wait for output");
    drop(held_open);

    (output, control, query_host)
}

/// The `sql` field of a recorded Query API request body.
fn query_sql_of(request: &wiremock::Request) -> String {
    let body: Value = serde_json::from_slice(&request.body).expect("query body is JSON");
    body["sql"].as_str().expect("sql field").to_string()
}

#[tokio::test]
async fn service_query_refuses_data_on_stdin_before_any_request() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data.csv");
    std::fs::write(&data, "1,alice\n2,bob\n").unwrap();
    let file = std::fs::File::open(&data).unwrap();

    let (output, control, query_host) = invoke_oauth_service_query_with_stdin(
        &["--query", "INSERT INTO trips FORMAT CSV"],
        StdinPlan::File(file),
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--query cannot be combined with SQL or data on stdin"),
        "{stderr}"
    );
    assert!(stderr.contains("cat - data.csv"), "{stderr}");
    assert!(stderr.contains("--queries-file -"), "{stderr}");
    assert!(output.stdout.is_empty(), "{:?}", output.stdout);
    // The refusal happens while reading the SQL, so neither the control plane
    // nor the query host is contacted.
    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_refuses_data_that_arrives_after_the_check_starts() {
    // `cat data.csv | clickhousectl ... --query ...`: the shell starts both
    // processes at once, so the first byte can land after the CLI has begun
    // looking at stdin. A zero-timeout readiness check would call that "no
    // input" and send the empty INSERT, which is #641 all over again. The
    // interleaving here depends on process startup, so the readiness timeout
    // itself is pinned deterministically by the pipe-level unit tests in
    // `cloud::services`; this is the end-to-end guard.
    let (output, control, query_host) = invoke_oauth_service_query_with_stdin(
        &["--query", "INSERT INTO trips FORMAT CSV"],
        StdinPlan::WriteAfter(b"1,alice\n2,bob\n", std::time::Duration::from_millis(150)),
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--query cannot be combined with SQL or data on stdin"),
        "{stderr}"
    );
    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_does_not_hang_on_a_pipe_nobody_writes_to() {
    // The writer stays open for the whole run and never produces a byte. The
    // bounded first-byte wait must expire and the query must still be sent.
    let (output, _control, query_host) =
        invoke_oauth_service_query_with_stdin(&["--query", "SELECT 1"], StdinPlan::IdleOpen).await;

    assert_success(&output);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(query_sql_of(&requests[0]), "SELECT 1");
}

#[tokio::test]
async fn service_query_with_query_and_empty_stdin_still_runs() {
    let (output, _control, query_host) =
        invoke_oauth_service_query_with_stdin(&["--query", "SELECT 1"], StdinPlan::Null).await;

    assert_success(&output);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(query_sql_of(&requests[0]), "SELECT 1");
}

#[tokio::test]
async fn service_query_still_reads_sql_piped_on_bare_stdin() {
    let (output, _control, query_host) =
        invoke_oauth_service_query_with_stdin(&[], StdinPlan::Write(b"SELECT 1\n")).await;

    assert_success(&output);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(query_sql_of(&requests[0]), "SELECT 1\n");
}

#[tokio::test]
async fn service_query_still_reads_sql_from_queries_file_dash() {
    let (output, _control, query_host) = invoke_oauth_service_query_with_stdin(
        &["--queries-file", "-"],
        StdinPlan::Write(b"SELECT 1\n"),
    )
    .await;

    assert_success(&output);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(query_sql_of(&requests[0]), "SELECT 1\n");
}

#[tokio::test]
async fn service_query_completes_a_text_response_line_without_duplication() {
    let (output, _) = invoke_oauth_service_query_response(b"OK".to_vec(), &[], false).await;
    assert_success(&output);
    assert_eq!(output.stdout, b"OK\n");

    let (output, _) = invoke_oauth_service_query_response(b"1\n".to_vec(), &[], false).await;
    assert_success(&output);
    assert_eq!(output.stdout, b"1\n");
}

#[tokio::test]
async fn service_query_acknowledges_an_empty_success_without_changing_stdout() {
    let (output, _) = invoke_oauth_service_query_response(Vec::new(), &[], false).await;
    assert_success(&output);
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"OK\n");
}

#[tokio::test]
async fn service_query_does_not_append_to_binary_output() {
    let body = vec![0, 1, 2, 3];
    let (output, _) =
        invoke_oauth_service_query_response(body.clone(), &["--format", "RowBinary"], false).await;
    assert_success(&output);
    assert_eq!(output.stdout, body);
}

#[tokio::test]
async fn service_query_json_selects_json_each_row_on_the_wire() {
    let (output, query_host) = invoke_oauth_service_query_response(
        br#"{"value":1}
"#
        .to_vec(),
        &["--json"],
        false,
    )
    .await;
    assert_success(&output);

    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), Some("format=JSONEachRow"));
}

#[tokio::test]
async fn service_query_agent_json_uses_json_each_row_unless_format_is_explicit() {
    let (output, query_host) =
        invoke_oauth_service_query_response(b"1\n".to_vec(), &[], true).await;
    assert_success(&output);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), Some("format=JSONEachRow"));

    let (output, query_host) =
        invoke_oauth_service_query_response(b"1\n".to_vec(), &["--format", "CSV"], true).await;
    assert_success(&output);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), Some("format=CSV"));
}

#[tokio::test]
async fn service_query_rejects_json_with_an_explicit_format_before_network_access() {
    let control = MockServer::start().await;
    let url = control.uri();
    let cases = [
        vec![
            "cloud",
            "--url",
            &url,
            "--json",
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--query",
            "SELECT 1",
            "--format",
            "CSV",
        ],
        vec![
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--query",
            "SELECT 1",
            "--json",
            "--format",
            "CSV",
        ],
    ];

    for args in cases {
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        let output = command
            .env("DO_NOT_TRACK", "1")
            .env("CLICKHOUSE_CLOUD_API_KEY", "unused-key")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "unused-secret")
            .args(args)
            .output()
            .expect("failed to spawn clickhousectl");

        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--json"), "{stderr}");
        assert!(stderr.contains("--format"), "{stderr}");
        assert!(
            stderr.contains("clickhousectl cloud service query"),
            "{stderr}"
        );
    }

    assert!(control.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_requires_exactly_one_selector_before_network_access() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;
    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    std::fs::create_dir(&home_dir).unwrap();
    let url = control.uri();
    let missing_query_file = dir.path().join("must-not-be-read.sql");

    let invoke = |selector: &[&str], query_input: &[&str], authenticated: bool| {
        let mut args = vec![
            "cloud",
            "--url",
            url.as_str(),
            "service",
            "query",
            "--org-id",
            "org-1",
        ];
        args.extend_from_slice(query_input);
        args.extend_from_slice(selector);

        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home_dir)
            .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
            .current_dir(dir.path())
            .stdin(Stdio::null())
            .args(args);
        if authenticated {
            command
                .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
                .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests");
        }
        command.output().expect("failed to spawn clickhousectl")
    };

    let missing = invoke(
        &[],
        &["--queries-file", missing_query_file.to_str().unwrap()],
        false,
    );
    assert_eq!(missing.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("--name <NAME>"), "{stderr}");
    assert!(stderr.contains("--id <ID>"), "{stderr}");
    assert!(
        !stderr.contains("must-not-be-read.sql"),
        "query input was read before selector validation: {stderr}"
    );

    let both = invoke(
        &["--name", "demo", "--id", QUERY_TEST_SERVICE_ID],
        &["--query", "SELECT 1"],
        false,
    );
    assert_eq!(both.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&both.stderr);
    assert!(stderr.contains("--name <NAME>"), "{stderr}");
    assert!(stderr.contains("--id <ID>"), "{stderr}");

    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());

    for selector in [["--name", "demo"], ["--id", QUERY_TEST_SERVICE_ID]] {
        let output = invoke(&selector, &["--query", "SELECT 1"], true);
        assert_success(&output);
        assert_eq!(output.stdout, b"1\n");
    }

    assert_eq!(control.received_requests().await.unwrap().len(), 2);
    assert_eq!(query_host.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn service_query_accepts_independent_sql_sources_and_stdin_fallback() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("query.sql"), "SELECT 2\n").unwrap();
    let control_url = control.uri();
    let query_host_url = query_host.uri();

    let command = || {
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        command
            .env("DO_NOT_TRACK", "1")
            .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
            .env("CLICKHOUSE_CLOUD_QUERY_HOST", &query_host_url)
            .current_dir(dir.path())
            // Explicit so the inline and file cases below never inherit the
            // test runner's stdin, which `--query` now refuses (#641).
            .stdin(Stdio::null())
            .args([
                "cloud",
                "--url",
                control_url.as_str(),
                "service",
                "query",
                "--id",
                QUERY_TEST_SERVICE_ID,
                "--org-id",
                "org-1",
            ]);
        command
    };

    let inline = command()
        .args(["--query", "SELECT 1"])
        .output()
        .expect("failed to run inline query");
    assert_success(&inline);

    let file = command()
        .args(["--queries-file", "query.sql"])
        .output()
        .expect("failed to run query file");
    assert_success(&file);

    let mut child = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run stdin query");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"SELECT 3\n")
        .unwrap();
    let stdin = child.wait_with_output().unwrap();
    assert_success(&stdin);

    let sql: Vec<String> = query_host
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| {
            serde_json::from_slice::<Value>(&request.body).unwrap()["sql"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(sql, ["SELECT 1", "SELECT 2\n", "SELECT 3\n"]);
}

#[tokio::test]
async fn service_query_rejects_conflicting_sql_sources_before_file_or_network_access() {
    let control = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let missing_query_file = dir.path().join("must-not-be-read.sql");
    let url = control.uri();

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .current_dir(dir.path())
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--query",
            "SELECT 1",
            "--queries-file",
            missing_query_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--query <QUERY>"), "{stderr}");
    assert!(stderr.contains("--queries-file <QUERIES_FILE>"), "{stderr}");
    assert!(!stderr.contains("No such file or directory"), "{stderr}");
    assert!(control.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_with_oauth_sends_bearer_and_never_provisions() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;

    // OAuth tokens are the lowest-precedence credential tier, so clear the
    // API key env vars. Tokens now live in the global ~/.clickhouse/tokens.json
    // (issue #277), so point HOME at a temp dir and write them there.
    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    let tokens = serde_json::json!({
        "access_token": "test-bearer-token",
        "refresh_token": "unused",
        "expires_at": 4102444800u64, // 2100-01-01: never expires in tests
        "api_url": format!("{}/v1", control.uri()),
    });
    std::fs::write(
        ch_dir.join("tokens.json"),
        serde_json::to_vec(&tokens).unwrap(),
    )
    .unwrap();

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    // The query request must carry the OAuth bearer token, not Basic auth,
    // and no `auth-provider: custom` marker (that header means "custom
    // Query API key").
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    let run = &query_requests[0];
    let auth = run.headers.get("authorization").unwrap().to_str().unwrap();
    assert_eq!(auth, "Bearer test-bearer-token");
    assert!(
        run.headers.get("auth-provider").is_none(),
        "auth-provider header must not accompany a bearer token",
    );

    // No provisioning: key creation and query-endpoint upsert are both
    // POSTs, so the control plane must see only GETs.
    let control_requests = control.received_requests().await.unwrap();
    assert!(
        control_requests
            .iter()
            .all(|r| r.method == wiremock::http::Method::GET),
        "OAuth service query made non-GET control-plane calls: {:?}",
        control_requests
            .iter()
            .map(|r| format!("{} {}", r.method, r.url.path()))
            .collect::<Vec<_>>(),
    );

    // And no Query API key may be written locally for the OAuth path.
    assert!(
        !ch_dir.join("credentials.json").exists(),
        "OAuth service query wrote .clickhouse/credentials.json",
    );

    // The query result streams through to stdout untouched.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
}

#[tokio::test]
async fn service_query_uses_an_already_authorized_api_key_without_provisioning() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &control.uri(),
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "assigned-key-id")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "assigned-key-secret")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(output.stdout, b"1\n");

    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    let auth = query_requests[0]
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "assigned-key-id:assigned-key-secret",
        )
    );
    assert_eq!(auth, expected);

    let control_requests = control.received_requests().await.unwrap();
    assert!(
        control_requests
            .iter()
            .all(|request| request.method == wiremock::http::Method::GET),
        "an authorized API key must not trigger provisioning: {:?}",
        control_requests
            .iter()
            .map(|request| format!("{} {}", request.method, request.url.path()))
            .collect::<Vec<_>>(),
    );
    assert!(!dir.path().join(".clickhouse/credentials.json").exists());
}

#[tokio::test]
async fn service_query_with_stored_key_sends_basic_auth_with_that_key() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;

    // A stored per-service Query API key short-circuits provisioning; the
    // control-plane creds come from the env tier (the credentials file
    // carries only service_query_keys, no api_key/api_secret).
    let dir = tempfile::tempdir().unwrap();
    let ch_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    let creds = serde_json::json!({
        "service_query_keys": {
            QUERY_TEST_SERVICE_ID: {
                "key_id": "stored-key-id",
                "key_secret": "stored-key-secret",
                "endpoint_id": "ep-1",
                "service_name": "demo",
                "created_at": "2026-05-11T12:00:00Z",
            }
        }
    });
    std::fs::write(
        ch_dir.join("credentials.json"),
        serde_json::to_vec(&creds).unwrap(),
    )
    .unwrap();

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    // The query request authenticates with the stored per-service key, not
    // the org-level env creds, and keeps the custom-key marker header.
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    let run = &query_requests[0];
    let auth = run.headers.get("authorization").unwrap().to_str().unwrap();
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "stored-key-id:stored-key-secret",
        )
    );
    assert_eq!(auth, expected);
    assert_eq!(run.headers.get("auth-provider").unwrap(), "custom");

    // Stored key present → no provisioning POSTs on the control plane.
    let control_requests = control.received_requests().await.unwrap();
    assert!(
        control_requests
            .iter()
            .all(|r| r.method == wiremock::http::Method::GET),
        "stored-key service query made non-GET control-plane calls: {:?}",
        control_requests
            .iter()
            .map(|r| format!("{} {}", r.method, r.url.path()))
            .collect::<Vec<_>>(),
    );
}

// ── Provisioning cleanup (issue #314) ──────────────────────────────────────
//
// Every field of a key-creation or endpoint-upsert response is `Option<T>`,
// so provisioning can fail *after* the key exists. Each of those failures
// must delete the key it created, otherwise every retry leaves another
// orphaned key in the org.

const QUERY_TEST_KEY_UUID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const OLD_QUERY_TEST_KEY_UUID: &str = "11111111-2222-3333-4444-555555555555";
const PRESERVED_QUERY_SERVICE_ID: &str = "preserved-service";

fn write_repair_query_credentials(
    project_dir: &Path,
    organization_id: Option<&str>,
    api_key_id: Option<&str>,
    endpoint_id: Option<&str>,
    pending_cleanup_api_key_ids: &[&str],
) -> Value {
    let mut target = serde_json::json!({
        "key_id": "stored-key-id",
        "key_secret": "stored-key-secret",
        "service_name": "demo",
        "created_at": "2026-05-11T12:00:00Z"
    });
    if let Some(organization_id) = organization_id {
        target["organization_id"] = Value::String(organization_id.to_string());
    }
    if let Some(api_key_id) = api_key_id {
        target["api_key_id"] = Value::String(api_key_id.to_string());
    }
    if let Some(endpoint_id) = endpoint_id {
        target["endpoint_id"] = Value::String(endpoint_id.to_string());
    }
    if !pending_cleanup_api_key_ids.is_empty() {
        target["pending_cleanup_api_key_ids"] = serde_json::json!(pending_cleanup_api_key_ids);
    }

    let credentials = serde_json::json!({
        "api_key": "project-management-key",
        "api_secret": "project-management-secret",
        "service_query_keys": {
            PRESERVED_QUERY_SERVICE_ID: {
                "organization_id": "org-1",
                "api_key_id": "preserved-api-key-uuid",
                "key_id": "preserved-key-id",
                "key_secret": "preserved-key-secret",
                "endpoint_id": "preserved-endpoint",
                "service_name": "preserved",
                "created_at": "2026-05-11T12:00:00Z"
            },
            QUERY_TEST_SERVICE_ID: target
        }
    });
    let clickhouse_dir = project_dir.join(".clickhouse");
    std::fs::create_dir_all(&clickhouse_dir).unwrap();
    std::fs::write(
        clickhouse_dir.join("credentials.json"),
        serde_json::to_vec(&credentials).unwrap(),
    )
    .unwrap();
    credentials
}

fn service_query_key_repair_process(
    project_dir: &Path,
    control: &MockServer,
) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        // The post-repair probe (#658) goes to the query host; pin it to the
        // control mock so no test can reach a real one. A test that wants the
        // probe mounts `GET service` (state running) and `POST /service/{id}/run`.
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", control.uri())
        .current_dir(project_dir)
        .args([
            "cloud",
            "--url",
            &control.uri(),
            "--json",
            "service",
            "repair-query-key",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
        ]);
    command
}

async fn mount_repair_endpoint_get(control: &MockServer) -> String {
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "ep-1",
                "openApiKeys": ["unrelated-key", OLD_QUERY_TEST_KEY_UUID],
                "roles": ["sql_console_read_only"],
                "allowedOrigins": "https://example.com"
            },
            "status": 200,
            "requestId": "stub-endpoint-get"
        })))
        .expect(1)
        .mount(control)
        .await;
    endpoint_path
}

async fn mount_replacement_key_create(control: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": { "id": QUERY_TEST_KEY_UUID },
                "keyId": "replacement-key-id",
                "keySecret": "replacement-key-secret"
            },
            "status": 200,
            "requestId": "stub-key-create"
        })))
        .expect(1)
        .mount(control)
        .await;
}

// ── Rejected stored key classification (issue #528) ────────────────────────
//
// A Query API 401/403 for a stored key is classified against the key's
// management record and the endpoint binding before anything is touched.
// Only a key that no longer exists (GET key -> 404) makes the local record
// disposable; every other verdict, and every lookup failure, leaves the
// credentials file byte-for-byte unchanged and makes no write to the control
// plane. `OLD_QUERY_TEST_KEY_UUID` is the stored key's management ID.

fn stored_key_path() -> String {
    format!("/v1/organizations/org-1/keys/{OLD_QUERY_TEST_KEY_UUID}")
}

fn query_endpoint_path() -> String {
    format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint")
}

/// A `GET /keys/{id}` body for the stored key. Only the fields the classifier
/// reads are set; the key's secret is never part of a management record.
fn stored_key_record(state: &str, expire_at: Option<&str>, cidrs: &[&str]) -> ResponseTemplate {
    let mut result = serde_json::json!({
        "id": OLD_QUERY_TEST_KEY_UUID,
        "name": "clickhousectl-query-demo",
        "state": state,
        "ipAccessList": cidrs
            .iter()
            .map(|cidr| serde_json::json!({ "source": cidr, "description": "test" }))
            .collect::<Vec<_>>(),
    });
    if let Some(expire_at) = expire_at {
        result["expireAt"] = Value::String(expire_at.to_string());
    }
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-key-get"
    }))
}

async fn mount_stored_key_get(control: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(stored_key_path()))
        .respond_with(response)
        .mount(control)
        .await;
}

async fn mount_query_endpoint_get(control: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(query_endpoint_path()))
        .respond_with(response)
        .mount(control)
        .await;
}

fn query_endpoint_record(open_api_keys: &[&str]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": {
            "id": "ep-1",
            "openApiKeys": open_api_keys,
            "roles": ["sql_console_admin"],
            "allowedOrigins": "*"
        },
        "status": 200,
        "requestId": "stub-endpoint-get"
    }))
}

/// Run `cloud service query` with a stored key the query host rejects with
/// `status`. Returns the process output, the credentials file as written
/// before the run, and the project directory (kept alive for later reads).
async fn run_rejected_stored_key_query(
    control: &MockServer,
    status: u16,
    api_key_id: Option<&str>,
    pending_cleanup_api_key_ids: &[&str],
    extra_args: &[&str],
) -> (std::process::Output, Value, tempfile::TempDir) {
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(status).set_body_string("key rejected"))
        .expect(1)
        .mount(&query_host)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        api_key_id,
        Some("ep-1"),
        pending_cleanup_api_key_ids,
    );
    let mut command = service_query_process(project.path(), control, &query_host);
    command.args(extra_args);
    let output = command
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    (output, original, project)
}

fn read_credentials(project: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(project.join(".clickhouse/credentials.json")).unwrap())
        .unwrap()
}

/// Every request the run made to the control plane was a read.
async fn assert_control_plane_only_read(control: &MockServer, context: &str) {
    let requests = control.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .all(|request| request.method == wiremock::http::Method::GET),
        "{context}: a rejected stored key must never create, bind or delete anything: {:?}",
        requests
            .iter()
            .map(|request| format!("{} {}", request.method, request.url.path()))
            .collect::<Vec<_>>(),
    );
}

async fn control_plane_requests_to(control: &MockServer, path: &str) -> usize {
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == path)
        .count()
}

fn repair_hint() -> String {
    format!("clickhousectl cloud service repair-query-key {QUERY_TEST_SERVICE_ID} --org-id org-1")
}

#[tokio::test]
async fn a_disabled_stored_query_key_is_reported_and_never_replaced() {
    for status in [401, 403] {
        let control = start_mock_control_plane_with_service().await;
        mount_stored_key_get(
            &control,
            stored_key_record("disabled", None, &["0.0.0.0/0"]),
        )
        .await;
        // No endpoint stub on purpose: a disabled key needs no binding check,
        // and an unmounted route would be a visible 404 in the request log.
        let (output, original, project) = run_rejected_stored_key_query(
            &control,
            status,
            Some(OLD_QUERY_TEST_KEY_UUID),
            &[],
            &[],
        )
        .await;

        assert_eq!(output.status.code(), Some(1), "{status}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("rejected with HTTP {status}")),
            "{stderr}"
        );
        assert!(
            stderr.contains(&format!(
                "management API key {OLD_QUERY_TEST_KEY_UUID} is disabled"
            )),
            "{stderr}"
        );
        assert!(stderr.contains("no replacement was created"), "{stderr}");
        assert!(
            stderr.contains(&format!(
                "clickhousectl cloud key update {OLD_QUERY_TEST_KEY_UUID} --state enabled --org-id org-1"
            )),
            "{stderr}"
        );
        assert!(stderr.contains(&repair_hint()), "{stderr}");
        assert!(
            !stderr.contains("stored-key-secret"),
            "secret leaked: {stderr}"
        );

        assert_control_plane_only_read(&control, "disabled").await;
        assert_eq!(
            control_plane_requests_to(&control, &query_endpoint_path()).await,
            0,
            "a disabled key is classified from its record alone"
        );
        assert_eq!(read_credentials(project.path()), original, "{status}");
    }
}

#[tokio::test]
async fn an_expired_stored_query_key_is_reported_and_never_replaced() {
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        stored_key_record("enabled", Some("2020-01-01T00:00:00Z"), &["0.0.0.0/0"]),
    )
    .await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "management API key {OLD_QUERY_TEST_KEY_UUID} expired at 2020-01-01T00:00:00Z"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("no replacement was created"), "{stderr}");
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert_control_plane_only_read(&control, "expired").await;
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn a_deleted_stored_query_key_is_reported_and_never_replaced() {
    // The key is gone from the organization, so the stored secret can never
    // work again. The record is still kept: it is what lets repair-query-key
    // find the key's UUID and drop it from the endpoint binding while binding
    // the replacement. Rerunning the query fails identically until then.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        ResponseTemplate::new(404).set_body_string("NOT_FOUND"),
    )
    .await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "management API key {OLD_QUERY_TEST_KEY_UUID} no longer exists in organization org-1"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("no replacement was created"), "{stderr}");
    assert!(
        stderr.contains("still listed on the service's Query API endpoint"),
        "{stderr}"
    );
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert!(
        !stderr.contains("has been removed") && !stderr.contains("Rerun the query"),
        "{stderr}"
    );
    assert_control_plane_only_read(&control, "deleted").await;
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn a_deleted_stored_query_key_with_pending_cleanup_keeps_its_record() {
    // The record still names a superseded key awaiting deletion (#527). The
    // query first retries that deletion; here the retry fails, so a warning
    // names the leftover key. The deleted verdict then keeps the record like
    // always, and repair is the way to finish both.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        ResponseTemplate::new(404).set_body_string("NOT_FOUND"),
    )
    .await;
    mount_key_delete(
        &control,
        PENDING_QUERY_TEST_KEY_UUID,
        ResponseTemplate::new(500).set_body_string("boom"),
    )
    .await;
    let (output, original, project) = run_rejected_stored_key_query(
        &control,
        401,
        Some(OLD_QUERY_TEST_KEY_UUID),
        &[PENDING_QUERY_TEST_KEY_UUID],
        &[],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Warning:"), "{stderr}");
    assert!(stderr.contains(PENDING_QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(
        stderr.contains("no longer exists in organization org-1"),
        "{stderr}"
    );
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert_eq!(
        key_deletes_received(&control).await,
        [PENDING_QUERY_TEST_KEY_UUID],
        "the retry is the only write, and it names the stored retired key"
    );
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn a_deleted_stored_query_key_whose_pending_retry_succeeds_keeps_its_record() {
    // Same record, but the retried deletion succeeds: the pending list is
    // emptied quietly, and the deleted verdict still keeps the record itself,
    // so repair can drop the dead key's endpoint binding.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        ResponseTemplate::new(404).set_body_string("NOT_FOUND"),
    )
    .await;
    mount_key_delete(
        &control,
        PENDING_QUERY_TEST_KEY_UUID,
        key_deleted_response(),
    )
    .await;
    let (output, original, project) = run_rejected_stored_key_query(
        &control,
        401,
        Some(OLD_QUERY_TEST_KEY_UUID),
        &[PENDING_QUERY_TEST_KEY_UUID],
        &[],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Warning:"), "{stderr}");
    assert!(
        stderr.contains("no longer exists in organization org-1"),
        "{stderr}"
    );
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert_eq!(
        key_deletes_received(&control).await,
        [PENDING_QUERY_TEST_KEY_UUID]
    );
    let stored = read_credentials(project.path());
    // An empty pending list is omitted from the file, so the record differs
    // from the original in exactly that key.
    let mut expected = original.clone();
    expected["service_query_keys"][QUERY_TEST_SERVICE_ID]
        .as_object_mut()
        .unwrap()
        .remove("pending_cleanup_api_key_ids");
    assert_eq!(stored, expected, "only the pending list changed");
}

// ── Pending retirement retry on the query path (issue #527) ────────────────
//
// A repair whose final key deletion failed leaves the retired key's ID on the
// record. The next query for the service retries the deletion before it runs:
// quietly when it works, with a warning when it does not, and never at the
// expense of the query itself.

/// Run `cloud service query` with a working stored key whose record lists
/// `pending_cleanup_api_key_ids`. Returns the output and the project dir.
async fn run_query_with_pending_retirements(
    control: &MockServer,
    pending_cleanup_api_key_ids: &[&str],
) -> (std::process::Output, tempfile::TempDir) {
    let query_host = start_mock_query_host().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        pending_cleanup_api_key_ids,
    );
    let output = service_query_process(project.path(), control, &query_host)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    (output, project)
}

#[tokio::test]
async fn a_query_retries_pending_key_retirements_quietly_before_running() {
    let control = start_mock_control_plane_with_service().await;
    mount_key_delete(&control, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;
    mount_key_delete(
        &control,
        PENDING_QUERY_TEST_KEY_UUID,
        key_deleted_response(),
    )
    .await;

    let (output, project) = run_query_with_pending_retirements(
        &control,
        &[OLD_QUERY_TEST_KEY_UUID, PENDING_QUERY_TEST_KEY_UUID],
    )
    .await;
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    assert_eq!(
        stderr_without_notes(&output),
        "",
        "a successful retry is silent"
    );
    assert_eq!(
        key_deletes_received(&control).await,
        [OLD_QUERY_TEST_KEY_UUID, PENDING_QUERY_TEST_KEY_UUID]
    );
    let record = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(record["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(record["key_id"], "stored-key-id");
    assert!(
        record.get("pending_cleanup_api_key_ids").is_none(),
        "{record}"
    );
}

#[tokio::test]
async fn a_failed_retirement_retry_warns_keeps_the_id_and_still_runs_the_query() {
    let control = start_mock_control_plane_with_service().await;
    mount_key_delete(
        &control,
        OLD_QUERY_TEST_KEY_UUID,
        ResponseTemplate::new(500).set_body_string("still failing"),
    )
    .await;
    mount_key_delete(
        &control,
        PENDING_QUERY_TEST_KEY_UUID,
        key_deleted_response(),
    )
    .await;

    let (output, project) = run_query_with_pending_retirements(
        &control,
        &[OLD_QUERY_TEST_KEY_UUID, PENDING_QUERY_TEST_KEY_UUID],
    )
    .await;
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    let stderr = stderr_without_notes(&output);
    assert!(stderr.starts_with("Warning:"), "{stderr}");
    assert!(stderr.contains(OLD_QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(
        !stderr.contains(PENDING_QUERY_TEST_KEY_UUID),
        "a key that was deleted is not reported as failed: {stderr}"
    );
    assert!(stderr.contains("still failing"), "{stderr}");
    assert!(
        stderr.contains("clickhousectl cloud key delete <key-id> --org-id org-1"),
        "{stderr}"
    );
    assert!(!stderr.contains("stored-key-secret"), "{stderr}");
    // Only the key that could not be deleted stays pending.
    let record = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(
        record["pending_cleanup_api_key_ids"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
}

/// #598, for the retirement-retry warning: the retry runs before the query, so
/// a stderr nobody is reading must not panic the process at exit 101 and leave
/// the query unrun.
#[tokio::test]
async fn a_failed_retirement_retry_survives_a_closed_stderr() {
    let control = start_mock_control_plane_with_service().await;
    mount_key_delete(
        &control,
        OLD_QUERY_TEST_KEY_UUID,
        ResponseTemplate::new(500).set_body_string("still failing"),
    )
    .await;

    let query_host = start_mock_query_host().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[OLD_QUERY_TEST_KEY_UUID],
    );
    let mut child = service_query_process(project.path(), &control, &query_host)
        // The fixture's env credentials are shadowed by its file credentials,
        // which emits the precedence `note:` first; dropping them makes the
        // retry warning the first thing written to the closed stderr.
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");
    // The reader of stderr walks away before the warning is written.
    drop(child.stderr.take().expect("stderr was piped"));
    let output = child
        .wait_with_output()
        .await
        .expect("failed to wait for clickhousectl");

    assert_eq!(
        output.status.code(),
        Some(0),
        "a closed stderr must not turn a warned-about retry into a panic"
    );
    // Not just "didn't panic": the query the warning precedes still ran, and
    // the ID that could not be deleted is still pending.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    let record = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(
        record["pending_cleanup_api_key_ids"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
}

#[tokio::test]
async fn a_retirement_retry_never_deletes_the_active_key() {
    // A record that lists its own active key as pending is contradictory.
    // The retry deletes the genuinely retired key and leaves the active one
    // alone, whatever the list says.
    let control = start_mock_control_plane_with_service().await;
    mount_key_delete(&control, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;

    let (output, project) = run_query_with_pending_retirements(
        &control,
        &[QUERY_TEST_KEY_UUID, OLD_QUERY_TEST_KEY_UUID],
    )
    .await;
    assert_success(&output);
    assert_eq!(
        key_deletes_received(&control).await,
        [OLD_QUERY_TEST_KEY_UUID]
    );
    let record = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(record["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(
        record["pending_cleanup_api_key_ids"],
        serde_json::json!([QUERY_TEST_KEY_UUID])
    );
}

#[tokio::test]
async fn a_query_without_pending_retirements_makes_no_control_plane_writes() {
    let control = start_mock_control_plane_with_service().await;
    let (output, _project) = run_query_with_pending_retirements(&control, &[]).await;
    assert_success(&output);
    assert!(key_deletes_received(&control).await.is_empty());
    assert_control_plane_only_read(&control, "no pending retirements").await;
}

#[tokio::test]
async fn an_unbound_stored_query_key_is_reported_and_never_replaced() {
    // The endpoint exists but lists another key.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(&control, stored_key_record("enabled", None, &["0.0.0.0/0"])).await;
    mount_query_endpoint_get(&control, query_endpoint_record(&["unrelated-key"])).await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 403, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is not bound to Query API endpoint ep-1"),
        "{stderr}"
    );
    assert!(stderr.contains("no replacement was created"), "{stderr}");
    assert!(
        stderr.contains(&format!(
            "clickhousectl cloud service query-endpoint get {QUERY_TEST_SERVICE_ID} --org-id org-1"
        )),
        "{stderr}"
    );
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert_control_plane_only_read(&control, "unbound").await;
    assert_eq!(read_credentials(project.path()), original);

    // No endpoint at all: still unbound, still nothing recreated.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(&control, stored_key_record("enabled", None, &["0.0.0.0/0"])).await;
    mount_query_endpoint_get(
        &control,
        ResponseTemplate::new(404).set_body_string("NOT_FOUND"),
    )
    .await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("has no Query API endpoint"), "{stderr}");
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert_control_plane_only_read(&control, "no endpoint").await;
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn an_enabled_bound_stored_query_key_that_is_still_rejected_lists_its_allowlist() {
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        stored_key_record("enabled", None, &["203.0.113.0/24", "198.51.100.7/32"]),
    )
    .await;
    mount_query_endpoint_get(
        &control,
        query_endpoint_record(&["unrelated-key", OLD_QUERY_TEST_KEY_UUID]),
    )
    .await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is enabled, unexpired and bound to the Query API endpoint"),
        "{stderr}"
    );
    assert!(
        stderr.contains("IP access list (203.0.113.0/24, 198.51.100.7/32)"),
        "{stderr}"
    );
    assert!(
        stderr.contains("stored secret no longer matches"),
        "{stderr}"
    );
    assert!(stderr.contains("Nothing was changed"), "{stderr}");
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert!(
        !stderr.contains("stored-key-secret"),
        "secret leaked: {stderr}"
    );
    assert_control_plane_only_read(&control, "rejected").await;
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn an_unverifiable_stored_query_key_rejection_changes_nothing() {
    // The key lookup itself fails: the verdict is "unknown", not "stale".
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        ResponseTemplate::new(500).set_body_string("upstream unavailable"),
    )
    .await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "management API key {OLD_QUERY_TEST_KEY_UUID} could not be read"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("upstream unavailable"), "{stderr}");
    assert!(stderr.contains("Nothing was changed"), "{stderr}");
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert_control_plane_only_read(&control, "key lookup failed").await;
    assert_eq!(read_credentials(project.path()), original);

    // The key reads fine but the endpoint lookup fails: same treatment.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(&control, stored_key_record("enabled", None, &["0.0.0.0/0"])).await;
    mount_query_endpoint_get(
        &control,
        ResponseTemplate::new(503).set_body_string("endpoint service unavailable"),
    )
    .await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, Some(OLD_QUERY_TEST_KEY_UUID), &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "the Query API endpoint binding of service {QUERY_TEST_SERVICE_ID} could not be read"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("Nothing was changed"), "{stderr}");
    assert_control_plane_only_read(&control, "endpoint lookup failed").await;
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn a_legacy_stored_query_key_rejection_changes_nothing() {
    // No management key ID in the record: there is nothing to look up, so no
    // key GET is even attempted, and nothing is removed.
    let control = start_mock_control_plane_with_service().await;
    let (output, original, project) =
        run_rejected_stored_key_query(&control, 401, None, &[], &[]).await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("predates key-ownership metadata"),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!("service_query_keys.{QUERY_TEST_SERVICE_ID}")),
        "{stderr}"
    );
    assert!(stderr.contains("Nothing was changed"), "{stderr}");
    assert_control_plane_only_read(&control, "legacy").await;
    assert_eq!(
        control_plane_requests_to(&control, &stored_key_path()).await,
        0
    );
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn stored_query_key_rejections_emit_structured_json_errors() {
    fn parse_error(output: &std::process::Output) -> Value {
        assert!(output.stdout.is_empty(), "{:?}", output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // The credentials file written by `write_repair_query_credentials`
        // also holds project management credentials, so the CLI's existing
        // "note: ... env vars are set but ignored" line precedes the object.
        let object = &stderr[stderr.find('{').unwrap_or(0)..];
        serde_json::from_str(object.trim())
            .unwrap_or_else(|e| panic!("stderr does not end in one JSON object ({e}): {stderr}"))
    }

    // Disabled: the code, the key ID and the deliberate repair command.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        stored_key_record("disabled", None, &["0.0.0.0/0"]),
    )
    .await;
    let (output, original, project) = run_rejected_stored_key_query(
        &control,
        401,
        Some(OLD_QUERY_TEST_KEY_UUID),
        &[],
        &["--json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    let error = parse_error(&output);
    assert_eq!(error["error"]["code"], "query_key_disabled");
    assert_eq!(error["error"]["api_key_id"], OLD_QUERY_TEST_KEY_UUID);
    assert_eq!(error["error"]["command"], repair_hint());
    assert!(error["error"].get("ip_access_list").is_none(), "{error}");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("is disabled")
    );
    assert_eq!(read_credentials(project.path()), original);

    // Rejected while enabled and bound: the allowlist travels as data.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        stored_key_record("enabled", None, &["203.0.113.0/24"]),
    )
    .await;
    mount_query_endpoint_get(&control, query_endpoint_record(&[OLD_QUERY_TEST_KEY_UUID])).await;
    let (output, original, project) = run_rejected_stored_key_query(
        &control,
        403,
        Some(OLD_QUERY_TEST_KEY_UUID),
        &[],
        &["--json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    let error = parse_error(&output);
    assert_eq!(error["error"]["code"], "query_key_rejected");
    assert_eq!(
        error["error"]["ip_access_list"],
        serde_json::json!(["203.0.113.0/24"])
    );
    assert!(
        !error.to_string().contains("stored-key-secret"),
        "secret leaked: {error}"
    );
    assert_eq!(read_credentials(project.path()), original);

    // Deleted: read-only like every other verdict, and the command is repair.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(
        &control,
        ResponseTemplate::new(404).set_body_string("NOT_FOUND"),
    )
    .await;
    let (output, original, project) = run_rejected_stored_key_query(
        &control,
        401,
        Some(OLD_QUERY_TEST_KEY_UUID),
        &[],
        &["--json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    let error = parse_error(&output);
    assert_eq!(error["error"]["code"], "query_key_deleted");
    assert_eq!(error["error"]["api_key_id"], OLD_QUERY_TEST_KEY_UUID);
    assert_eq!(error["error"]["command"], repair_hint());
    assert_eq!(read_credentials(project.path()), original);

    // Unverified: no command is pushed for an ambiguous verdict.
    let control = start_mock_control_plane_with_service().await;
    mount_stored_key_get(&control, ResponseTemplate::new(500).set_body_string("boom")).await;
    let (output, original, project) = run_rejected_stored_key_query(
        &control,
        401,
        Some(OLD_QUERY_TEST_KEY_UUID),
        &[],
        &["--json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    let error = parse_error(&output);
    assert_eq!(error["error"]["code"], "query_key_unverified");
    assert!(error["error"].get("command").is_none(), "{error}");
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn service_query_key_repair_replaces_exact_binding_and_preserves_credentials() {
    let control = MockServer::start().await;
    mount_running_service_accepting_probes(&control, 1).await;
    let endpoint_path = mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .and(body_json(serde_json::json!({
            "roles": ["sql_console_read_only"],
            "openApiKeys": ["unrelated-key", QUERY_TEST_KEY_UUID],
            "allowedOrigins": "https://example.com"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "ep-1",
                "openApiKeys": ["unrelated-key", QUERY_TEST_KEY_UUID]
            },
            "status": 200,
            "requestId": "stub-endpoint-repair"
        })))
        .expect(1)
        .mount(&control)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{OLD_QUERY_TEST_KEY_UUID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-old-key-delete"
        })))
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["serviceId"], QUERY_TEST_SERVICE_ID);
    assert_eq!(result["organizationId"], "org-1");
    assert_eq!(result["replacedApiKeyId"], OLD_QUERY_TEST_KEY_UUID);
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["endpointId"], "ep-1");
    assert_eq!(result["verification"], "verified");
    assert_eq!(probes_received(&control).await.len(), 1);

    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["api_key"], original["api_key"]);
    assert_eq!(stored["api_secret"], original["api_secret"]);
    assert_eq!(
        stored["service_query_keys"][PRESERVED_QUERY_SERVICE_ID],
        original["service_query_keys"][PRESERVED_QUERY_SERVICE_ID]
    );
    let repaired = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["organization_id"], "org-1");
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(repaired["key_id"], "replacement-key-id");
    assert_eq!(repaired["key_secret"], "replacement-key-secret");
    assert_eq!(repaired["endpoint_id"], "ep-1");
    assert!(repaired.get("pending_cleanup_api_key_ids").is_none());
}

#[tokio::test]
async fn repair_retains_exact_old_key_id_when_final_cleanup_fails() {
    // The replacement is active and bound, so a failed delete of the
    // superseded key is a warning, not a failure (#527): the exact ID stays
    // on the record for the next query to retry, and the warning says so.
    let control = MockServer::start().await;
    mount_running_service_accepting_probes(&control, 1).await;
    let endpoint_path = mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1" },
            "status": 200,
            "requestId": "stub-endpoint-repair"
        })))
        .expect(1)
        .mount(&control)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{OLD_QUERY_TEST_KEY_UUID}"
        )))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "temporary cleanup failure",
            "status": 500,
            "requestId": "stub-old-key-delete-failure"
        })))
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["replacedApiKeyId"], OLD_QUERY_TEST_KEY_UUID);
    assert_eq!(
        result["pendingCleanupApiKeyIds"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
    assert!(result.get("deletedApiKeyIds").is_none(), "{result}");
    let stderr = stderr_without_notes(&output);
    assert!(stderr.starts_with("Warning:"), "{stderr}");
    assert!(stderr.contains(OLD_QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(stderr.contains("temporary cleanup failure"), "{stderr}");
    assert!(
        stderr.contains(&format!(
            "clickhousectl cloud service query --id {QUERY_TEST_SERVICE_ID} --org-id org-1"
        )),
        "{stderr}"
    );
    assert!(
        stderr.contains("clickhousectl cloud key delete <key-id> --org-id org-1"),
        "{stderr}"
    );

    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][PRESERVED_QUERY_SERVICE_ID],
        original["service_query_keys"][PRESERVED_QUERY_SERVICE_ID]
    );
    let repaired = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(repaired["key_id"], "replacement-key-id");
    assert_eq!(
        repaired["pending_cleanup_api_key_ids"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
}

/// #598, for the repair's cleanup warning: the replacement key is already
/// created, bound and committed to the record by the time the warning is
/// written, so a stderr nobody is reading must not turn a completed repair
/// into a panic and exit 101.
#[tokio::test]
async fn repair_with_a_failed_final_cleanup_survives_a_closed_stderr() {
    let control = MockServer::start().await;
    let endpoint_path = mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1" },
            "status": 200,
            "requestId": "stub-endpoint-repair"
        })))
        .expect(1)
        .mount(&control)
        .await;
    mount_key_delete(
        &control,
        OLD_QUERY_TEST_KEY_UUID,
        ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "temporary cleanup failure",
            "status": 500,
            "requestId": "stub-old-key-delete-failure"
        })),
    )
    .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let mut child = service_query_key_repair_process(project.path(), &control)
        // As above: only file credentials, so the cleanup warning is the first
        // thing written to the closed stderr.
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");
    // The reader of stderr walks away before the warning is written.
    drop(child.stderr.take().expect("stderr was piped"));
    let output = child
        .wait_with_output()
        .await
        .expect("failed to wait for clickhousectl");

    assert_eq!(
        output.status.code(),
        Some(0),
        "a closed stderr must not turn a committed repair into a panic"
    );
    // Not just "didn't panic": the repair still reported its result, and the
    // exact retired ID is still stored for the next query to retry.
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(
        result["pendingCleanupApiKeyIds"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
    let repaired = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(
        repaired["pending_cleanup_api_key_ids"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
}

#[tokio::test]
async fn failed_repair_restores_bindings_deletes_new_key_and_preserves_records() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let control = MockServer::start().await;
    let endpoint_path = mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    let call_index = Arc::new(AtomicUsize::new(0));
    let responder_index = Arc::clone(&call_index);
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if responder_index.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(500).set_body_json(serde_json::json!({
                    "error": "binding replacement failed",
                    "status": 500,
                    "requestId": "stub-repair-failure"
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": "ep-1" },
                    "status": 200,
                    "requestId": "stub-rollback"
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-new-key-cleanup"
        })))
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("binding replacement failed"));
    // A 5xx is not key propagation: no wait, no notice, straight to rollback
    // (#658). The two upserts below are the attempt and the rollback.
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Waiting for the new API key"));

    let requests = control.received_requests().await.unwrap();
    let endpoint_posts = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
        })
        .collect::<Vec<_>>();
    assert_eq!(endpoint_posts.len(), 2);
    let replacement: Value = serde_json::from_slice(&endpoint_posts[0].body).unwrap();
    let rollback: Value = serde_json::from_slice(&endpoint_posts[1].body).unwrap();
    assert_eq!(
        replacement["openApiKeys"],
        serde_json::json!(["unrelated-key", QUERY_TEST_KEY_UUID])
    );
    assert_eq!(
        rollback["openApiKeys"],
        serde_json::json!(["unrelated-key", OLD_QUERY_TEST_KEY_UUID])
    );
    assert!(requests.iter().all(|request| {
        request.url.path() != format!("/v1/organizations/org-1/keys/{OLD_QUERY_TEST_KEY_UUID}")
    }));
    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored, original);
}

#[tokio::test]
async fn failed_repair_retains_new_key_when_endpoint_rollback_fails() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let control = MockServer::start().await;
    let endpoint_path = mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    let call_index = Arc::new(AtomicUsize::new(0));
    let responder_index = Arc::clone(&call_index);
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |_: &wiremock::Request| {
            let (error, request_id) = if responder_index.fetch_add(1, Ordering::SeqCst) == 0 {
                ("binding replacement failed", "stub-repair-failure")
            } else {
                ("endpoint rollback failed", "stub-rollback-failure")
            };
            ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": error,
                "status": 500,
                "requestId": request_id
            }))
        })
        .expect(2)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("binding replacement failed"), "{stderr}");
    assert!(stderr.contains("endpoint rollback failed"), "{stderr}");
    assert!(stderr.contains(QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(stderr.contains("was retained"), "{stderr}");

    let requests = control.received_requests().await.unwrap();
    assert!(requests.iter().all(|request| {
        request.url.path() != format!("/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}")
    }));
    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored, original);
}

#[tokio::test]
async fn failed_repair_records_the_new_key_for_retry_when_its_rollback_delete_fails() {
    // The upsert fails, the original binding is restored, and then the delete
    // of the never-bound replacement key fails. The key exists and grants
    // nothing, and its ID must not live only in the error text (#527): it is
    // appended to the untouched record's pending list so the next query
    // retries the deletion (#658). Everything else on disk stays as it was.
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let control = MockServer::start().await;
    let endpoint_path = mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    let attempts = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |_: &wiremock::Request| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(500).set_body_json(serde_json::json!({
                    "error": "binding replacement failed",
                    "status": 500,
                    "requestId": "stub-repair-failure"
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": "ep-1" },
                    "status": 200,
                    "requestId": "stub-rollback"
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;
    mount_key_delete(
        &control,
        QUERY_TEST_KEY_UUID,
        ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "key service unavailable",
            "status": 500,
            "requestId": "stub-new-key-delete-failure"
        })),
    )
    .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_without_notes(&output);
    assert!(stderr.contains("binding replacement failed"), "{stderr}");
    assert!(stderr.contains("key service unavailable"), "{stderr}");
    assert!(
        stderr.contains(&format!(
            "failed to delete newly created API key {QUERY_TEST_KEY_UUID}"
        )),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "service_query_keys.{QUERY_TEST_SERVICE_ID}.pending_cleanup_api_key_ids"
        )),
        "the error says where the ID went: {stderr}"
    );
    assert!(
        stderr.contains("retried automatically by the next `clickhousectl cloud service query"),
        "{stderr}"
    );

    assert_eq!(
        key_deletes_received(&control).await,
        vec![QUERY_TEST_KEY_UUID.to_string()],
        "only the rolled-back key was attempted; the superseded key stays"
    );
    let mut expected = original.clone();
    expected["service_query_keys"][QUERY_TEST_SERVICE_ID]["pending_cleanup_api_key_ids"] =
        serde_json::json!([QUERY_TEST_KEY_UUID]);
    assert_eq!(
        read_credentials(project.path()),
        expected,
        "the record still names the old key and now lists the rolled-back one as pending"
    );
}

const PENDING_QUERY_TEST_KEY_UUID: &str = "77777777-6666-5555-4444-333333333333";
const THIRD_QUERY_TEST_KEY_UUID: &str = "33333333-4444-5555-6666-777777777777";

/// `GET serviceQueryEndpoint` listing exactly `open_api_keys`, with the
/// read-only role and origin the repair must preserve.
async fn mount_repair_endpoint_get_listing(control: &MockServer, open_api_keys: &[&str]) {
    Mock::given(method("GET"))
        .and(path(query_endpoint_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "ep-1",
                "openApiKeys": open_api_keys,
                "roles": ["sql_console_read_only"],
                "allowedOrigins": "https://example.com"
            },
            "status": 200,
            "requestId": "stub-endpoint-get"
        })))
        .expect(1)
        .mount(control)
        .await;
}

/// `POST /keys` answering with `api_key_uuid` as the new key's resource ID.
async fn mount_key_create_returning(control: &MockServer, api_key_uuid: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": { "id": api_key_uuid },
                "keyId": format!("key-id-for-{api_key_uuid}"),
                "keySecret": format!("key-secret-for-{api_key_uuid}")
            },
            "status": 200,
            "requestId": "stub-key-create"
        })))
        .expect(1)
        .mount(control)
        .await;
}

/// The endpoint upsert the repair must send: the read-only role and origin
/// preserved, exactly `open_api_keys` bound.
async fn expect_repair_endpoint_upsert(control: &MockServer, open_api_keys: &[&str]) {
    Mock::given(method("POST"))
        .and(path(query_endpoint_path()))
        .and(body_json(serde_json::json!({
            "roles": ["sql_console_read_only"],
            "openApiKeys": open_api_keys,
            "allowedOrigins": "https://example.com"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1", "openApiKeys": open_api_keys },
            "status": 200,
            "requestId": "stub-endpoint-repair"
        })))
        .expect(1)
        .mount(control)
        .await;
}

async fn mount_key_delete(control: &MockServer, api_key_uuid: &str, response: ResponseTemplate) {
    Mock::given(method("DELETE"))
        .and(path(format!("/v1/organizations/org-1/keys/{api_key_uuid}")))
        .respond_with(response)
        .expect(1)
        .mount(control)
        .await;
}

fn key_deleted_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "stub-key-delete"
    }))
}

/// The `DELETE /keys/{id}` paths the control plane received, in order.
async fn key_deletes_received(control: &MockServer) -> Vec<String> {
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.method == wiremock::http::Method::DELETE)
        .filter_map(|request| {
            request
                .url
                .path()
                .strip_prefix("/v1/organizations/org-1/keys/")
                .map(str::to_string)
        })
        .collect()
}

/// Stderr without the credential-precedence `note:` line the fixture's file
/// credentials plus the env credentials always produce.
fn stderr_without_notes(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|line| !line.starts_with("note:"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn repair_with_a_pending_retirement_replaces_the_key_and_deletes_every_retired_key() {
    // An earlier repair could not delete its superseded key, so the record
    // lists it as pending (#527). This repair replaces the current key,
    // unbinds both retired keys in the one endpoint upsert, deletes both, and
    // leaves nothing pending. The unrelated binding survives. The service is
    // running and accepts the new key at once, so the run is entirely quiet.
    let control = MockServer::start().await;
    mount_repair_service_state(&control, "running").await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .expect(1)
        .mount(&control)
        .await;
    mount_repair_endpoint_get_listing(
        &control,
        &[
            "unrelated-key",
            OLD_QUERY_TEST_KEY_UUID,
            PENDING_QUERY_TEST_KEY_UUID,
        ],
    )
    .await;
    mount_key_create_returning(&control, QUERY_TEST_KEY_UUID).await;
    expect_repair_endpoint_upsert(&control, &["unrelated-key", QUERY_TEST_KEY_UUID]).await;
    mount_key_delete(
        &control,
        PENDING_QUERY_TEST_KEY_UUID,
        key_deleted_response(),
    )
    .await;
    mount_key_delete(&control, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[PENDING_QUERY_TEST_KEY_UUID],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(
        stderr_without_notes(&output),
        "",
        "a fully successful cleanup prints no warning"
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["replacedApiKeyId"], OLD_QUERY_TEST_KEY_UUID);
    assert_eq!(result["verification"], "verified");
    assert_eq!(
        result["deletedApiKeyIds"],
        serde_json::json!([PENDING_QUERY_TEST_KEY_UUID, OLD_QUERY_TEST_KEY_UUID])
    );
    assert!(result.get("pendingCleanupApiKeyIds").is_none(), "{result}");
    assert_eq!(
        key_deletes_received(&control).await,
        [PENDING_QUERY_TEST_KEY_UUID, OLD_QUERY_TEST_KEY_UUID],
        "exactly the stored retired keys are deleted, never the unrelated or the new one"
    );

    let stored = read_credentials(project.path());
    assert_eq!(stored["api_key"], original["api_key"]);
    assert_eq!(
        stored["service_query_keys"][PRESERVED_QUERY_SERVICE_ID],
        original["service_query_keys"][PRESERVED_QUERY_SERVICE_ID]
    );
    let repaired = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
    assert!(
        repaired.get("pending_cleanup_api_key_ids").is_none(),
        "{repaired}"
    );
}

#[tokio::test]
async fn repeated_repairs_do_not_grow_the_endpoint_binding_or_the_pending_list() {
    // Two repairs in a row against the same project: each one retires the key
    // the previous one installed, so the endpoint always binds exactly the
    // unrelated key plus the current one and nothing is ever left pending.
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );

    let first = MockServer::start().await;
    mount_running_service_accepting_probes(&first, 1).await;
    mount_repair_endpoint_get_listing(&first, &["unrelated-key", OLD_QUERY_TEST_KEY_UUID]).await;
    mount_key_create_returning(&first, QUERY_TEST_KEY_UUID).await;
    expect_repair_endpoint_upsert(&first, &["unrelated-key", QUERY_TEST_KEY_UUID]).await;
    mount_key_delete(&first, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;
    let output = service_query_key_repair_process(project.path(), &first)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(
        key_deletes_received(&first).await,
        [OLD_QUERY_TEST_KEY_UUID]
    );

    let second = MockServer::start().await;
    mount_running_service_accepting_probes(&second, 1).await;
    mount_repair_endpoint_get_listing(&second, &["unrelated-key", QUERY_TEST_KEY_UUID]).await;
    mount_key_create_returning(&second, THIRD_QUERY_TEST_KEY_UUID).await;
    expect_repair_endpoint_upsert(&second, &["unrelated-key", THIRD_QUERY_TEST_KEY_UUID]).await;
    mount_key_delete(&second, QUERY_TEST_KEY_UUID, key_deleted_response()).await;
    let output = service_query_key_repair_process(project.path(), &second)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["replacedApiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["apiKeyId"], THIRD_QUERY_TEST_KEY_UUID);
    assert_eq!(result["verification"], "verified");
    assert_eq!(
        result["deletedApiKeyIds"],
        serde_json::json!([QUERY_TEST_KEY_UUID])
    );
    assert_eq!(
        key_deletes_received(&second).await,
        [QUERY_TEST_KEY_UUID],
        "the second repair deletes only the key the first one installed"
    );

    let repaired = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], THIRD_QUERY_TEST_KEY_UUID);
    assert_eq!(
        repaired["key_id"],
        format!("key-id-for-{THIRD_QUERY_TEST_KEY_UUID}")
    );
    assert!(
        repaired.get("pending_cleanup_api_key_ids").is_none(),
        "{repaired}"
    );
}

// ── Key propagation (issue #658) ────────────────────────────────────────────
//
// `POST /keys` and the endpoint upsert are answered by different services, and
// the upsert can reject a key created moments earlier with
// `400 OpenAPI key <id> does not belong to the organization` while
// `GET /keys/{id}` already returns it. Provisioning and repair both create a
// key and then bind it, so both wait that out: the upsert alone is retried,
// with the same body, inside a bounded window; the notice below is printed
// once; a success ends the wait. The condition is structural (a typed
// `Error::Api` with status 400), never the message text.

const KEY_PROPAGATION_NOTICE: &str =
    "Waiting for the new API key to become visible to the Query API endpoint...";

/// The control plane's answer while the new key has not propagated yet.
fn key_not_in_organization_response(api_key_uuid: &str) -> ResponseTemplate {
    ResponseTemplate::new(400).set_body_json(serde_json::json!({
        "error": format!("OpenAPI key {api_key_uuid} does not belong to the organization"),
        "status": 400,
        "requestId": "stub-key-propagation"
    }))
}

/// The endpoint upsert bodies the control plane received, in order.
async fn endpoint_upserts_received(control: &MockServer) -> Vec<Value> {
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST
                && request.url.path() == query_endpoint_path()
        })
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

/// The endpoint upsert answering `first` on the first attempt and binding
/// `open_api_keys` on every later one.
async fn mount_endpoint_upsert_failing_once(
    control: &MockServer,
    first: ResponseTemplate,
    open_api_keys: Vec<&str>,
    expected_calls: u64,
) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let attempts = Arc::new(AtomicUsize::new(0));
    let open_api_keys: Vec<String> = open_api_keys.iter().map(|key| key.to_string()).collect();
    Mock::given(method("POST"))
        .and(path(query_endpoint_path()))
        .respond_with(move |_: &wiremock::Request| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                first.clone()
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": "ep-1", "openApiKeys": open_api_keys },
                    "status": 200,
                    "requestId": "stub-endpoint-upsert"
                }))
            }
        })
        .expect(expected_calls)
        .mount(control)
        .await;
}

#[tokio::test]
async fn repair_waits_for_the_new_key_to_propagate_before_binding_it() {
    let control = MockServer::start().await;
    mount_running_service_accepting_probes(&control, 1).await;
    mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    mount_endpoint_upsert_failing_once(
        &control,
        key_not_in_organization_response(QUERY_TEST_KEY_UUID),
        vec!["unrelated-key", QUERY_TEST_KEY_UUID],
        2,
    )
    .await;
    mount_key_delete(&control, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["replacedApiKeyId"], OLD_QUERY_TEST_KEY_UUID);
    assert_eq!(
        result["deletedApiKeyIds"],
        serde_json::json!([OLD_QUERY_TEST_KEY_UUID])
    );
    assert_eq!(result["verification"], "verified");
    let stderr = stderr_without_notes(&output);
    assert_eq!(
        stderr.matches(KEY_PROPAGATION_NOTICE).count(),
        1,
        "the notice is printed exactly once: {stderr}"
    );
    assert!(!stderr.contains("Warning:"), "{stderr}");

    // Exactly one key was created; the retried upsert carried the same body;
    // the superseded key was retired and the new one was never deleted.
    let requests = control.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| {
                request.method == wiremock::http::Method::POST
                    && request.url.path() == "/v1/organizations/org-1/keys"
            })
            .count(),
        1
    );
    let upserts = endpoint_upserts_received(&control).await;
    assert_eq!(upserts.len(), 2);
    assert_eq!(upserts[0], upserts[1], "the retry resends the same body");
    assert_eq!(
        upserts[0]["openApiKeys"],
        serde_json::json!(["unrelated-key", QUERY_TEST_KEY_UUID])
    );
    assert_eq!(
        key_deletes_received(&control).await,
        vec![OLD_QUERY_TEST_KEY_UUID.to_string()]
    );
    let stored = read_credentials(project.path());
    let repaired = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(repaired["key_id"], "replacement-key-id");
    assert!(repaired.get("pending_cleanup_api_key_ids").is_none());
}

#[tokio::test]
async fn a_400_that_outlives_the_propagation_window_rolls_the_repair_back() {
    // The control plane keeps refusing the new key for the whole window (this
    // test waits out the real 30 s deadline, so the shipped policy is what is
    // pinned). The repair then fails exactly as an unretried failure did:
    // original binding restored, new key deleted, record untouched, and the
    // error the user sees is the upsert's own last answer.
    let control = MockServer::start().await;
    mount_repair_endpoint_get(&control).await;
    mount_replacement_key_create(&control).await;
    Mock::given(method("POST"))
        .and(path(query_endpoint_path()))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            let binds_new_key = body["openApiKeys"]
                .as_array()
                .unwrap()
                .iter()
                .any(|key| key == QUERY_TEST_KEY_UUID);
            if binds_new_key {
                key_not_in_organization_response(QUERY_TEST_KEY_UUID)
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": "ep-1" },
                    "status": 200,
                    "requestId": "stub-rollback"
                }))
            }
        })
        .mount(&control)
        .await;
    mount_key_delete(&control, QUERY_TEST_KEY_UUID, key_deleted_response()).await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let started = std::time::Instant::now();
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        stderr_without_notes(&output)
    );
    assert!(
        started.elapsed() >= std::time::Duration::from_secs(30),
        "the whole window is waited out before giving up"
    );
    let stderr = stderr_without_notes(&output);
    assert_eq!(
        stderr.matches(KEY_PROPAGATION_NOTICE).count(),
        1,
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "OpenAPI key {QUERY_TEST_KEY_UUID} does not belong to the organization"
        )),
        "the last 400 is the error reported: {stderr}"
    );

    let upserts = endpoint_upserts_received(&control).await;
    assert!(
        upserts.len() >= 3,
        "several attempts then a rollback: {upserts:?}"
    );
    let (rollback, attempts) = upserts.split_last().unwrap();
    for attempt in attempts {
        assert_eq!(
            attempt["openApiKeys"],
            serde_json::json!(["unrelated-key", QUERY_TEST_KEY_UUID])
        );
    }
    assert_eq!(
        rollback["openApiKeys"],
        serde_json::json!(["unrelated-key", OLD_QUERY_TEST_KEY_UUID])
    );
    assert_eq!(
        key_deletes_received(&control).await,
        vec![QUERY_TEST_KEY_UUID.to_string()],
        "only the new key is deleted; the superseded key is untouched"
    );
    assert_eq!(read_credentials(project.path()), original);
}

#[tokio::test]
async fn first_use_provisioning_waits_for_the_new_key_to_propagate_before_binding_it() {
    // The first-use path creates a key, reads the endpoint and binds the key:
    // the same create-then-bind the repair does, so the same wait applies.
    let control = start_mock_control_plane_with_service().await;
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret"
        }),
    )
    .await;
    Mock::given(method("GET"))
        .and(path(query_endpoint_path()))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not found",
            "status": 404,
            "requestId": "stub-endpoint-get"
        })))
        .expect(1)
        .mount(&control)
        .await;
    mount_endpoint_upsert_failing_once(
        &control,
        key_not_in_organization_response(QUERY_TEST_KEY_UUID),
        vec![QUERY_TEST_KEY_UUID],
        2,
    )
    .await;
    let query_host = start_mock_query_host_for_provisioning().await;

    let project = tempfile::tempdir().unwrap();
    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(project.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches(KEY_PROPAGATION_NOTICE).count(),
        1,
        "{stderr}"
    );

    let upserts = endpoint_upserts_received(&control).await;
    assert_eq!(upserts.len(), 2);
    assert_eq!(upserts[0], upserts[1]);
    assert_eq!(
        upserts[0]["openApiKeys"],
        serde_json::json!([QUERY_TEST_KEY_UUID])
    );
    assert!(
        recorded_key_deletes(&control).await.is_empty(),
        "a key that was eventually bound is never deleted"
    );
    let stored = read_credentials(project.path());
    assert_eq!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]["api_key_id"],
        QUERY_TEST_KEY_UUID
    );
}

// ── Post-repair verification (issue #658) ───────────────────────────────────
//
// The new binding reaches the Query API host a moment after the upsert. A
// query in between is rejected and the stored-key classifier (#528) would
// call an enabled, bound key "IP allowlist or secret mismatch". So a repair
// on a running service probes the endpoint with the new key, the way
// first-use provisioning does, and exits 0 only once the key works. Idle and
// stopped services are not probed (the probe would wake one and the other
// cannot answer); the next query verifies the key instead.

const QUERY_READINESS_NOTICE: &str = "Waiting for the Query API endpoint to become ready...";

async fn mount_repair_service_state(control: &MockServer, state: &str) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": QUERY_TEST_SERVICE_ID, "name": "demo", "state": state },
            "status": 200,
            "requestId": "stub-service-get"
        })))
        .mount(control)
        .await;
}

/// Everything a clean repair needs on the control plane: the endpoint, the
/// replacement key, the upsert and the retirement of the old key.
async fn mount_clean_repair(control: &MockServer) {
    mount_repair_endpoint_get(control).await;
    mount_replacement_key_create(control).await;
    expect_repair_endpoint_upsert(control, &["unrelated-key", QUERY_TEST_KEY_UUID]).await;
    mount_key_delete(control, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;
}

/// A `running` service whose query host accepts every probe: what every
/// successful repair sees. `expected_probes` pins that the probe ran.
async fn mount_running_service_accepting_probes(control: &MockServer, expected_probes: u64) {
    mount_repair_service_state(control, "running").await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .expect(expected_probes)
        .mount(control)
        .await;
}

/// The one JSON error object on stderr, after any notice lines.
fn structured_error(output: &std::process::Output) -> Value {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let object = &stderr[stderr.find('{').unwrap_or(0)..];
    serde_json::from_str(object.trim())
        .unwrap_or_else(|e| panic!("stderr does not end in one JSON object ({e}): {stderr}"))
}

/// The probe requests the query host received with the replacement key.
async fn probes_received(control: &MockServer) -> Vec<Value> {
    let auth = query_test_basic_auth("replacement-key-id:replacement-key-secret");
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| {
            request.url.path() == format!("/service/{QUERY_TEST_SERVICE_ID}/run")
                && request
                    .headers
                    .get("authorization")
                    .is_some_and(|value| value == auth.as_str())
        })
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

#[tokio::test]
async fn repair_exits_zero_only_once_the_query_api_accepts_the_new_key() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let control = MockServer::start().await;
    mount_repair_service_state(&control, "running").await;
    mount_clean_repair(&control).await;
    let attempts = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(move |_: &wiremock::Request| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(401).set_body_string("API key is not authorized")
            } else {
                ResponseTemplate::new(200).set_body_string("1\n")
            }
        })
        .expect(2)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["verification"], "verified");
    let stderr = stderr_without_notes(&output);
    assert_eq!(
        stderr.matches(QUERY_READINESS_NOTICE).count(),
        1,
        "{stderr}"
    );
    assert!(!stderr.contains("Note:"), "{stderr}");

    let probes = probes_received(&control).await;
    assert_eq!(
        probes.len(),
        2,
        "one rejected probe, one accepted: {probes:?}"
    );
    for probe in &probes {
        assert_eq!(probe["sql"], "SELECT 1");
    }
    assert_eq!(
        key_deletes_received(&control).await,
        vec![OLD_QUERY_TEST_KEY_UUID.to_string()],
        "the probe never triggers a rollback"
    );
}

#[tokio::test]
async fn repair_does_not_probe_a_service_that_is_not_running() {
    for state in ["idle", "stopped"] {
        let control = MockServer::start().await;
        mount_repair_service_state(&control, state).await;
        mount_clean_repair(&control).await;

        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join("home")).unwrap();
        write_repair_query_credentials(
            project.path(),
            Some("org-1"),
            Some(OLD_QUERY_TEST_KEY_UUID),
            Some("ep-1"),
            &[],
        );
        let output = service_query_key_repair_process(project.path(), &control)
            .output()
            .await
            .expect("failed to spawn clickhousectl");
        assert_success(&output);
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["status"], "repaired");
        assert_eq!(result["verification"], "skipped");
        let stderr = stderr_without_notes(&output);
        assert!(
            stderr.contains(&format!("Note: service {QUERY_TEST_SERVICE_ID} is {state}")),
            "{stderr}"
        );
        assert!(stderr.contains("is verified by the next"), "{stderr}");
        assert!(
            probes_received(&control).await.is_empty(),
            "{state}: probed"
        );
    }
}

#[tokio::test]
async fn repair_skips_verification_when_the_service_state_cannot_be_read() {
    // No `GET service` is mounted, so the state lookup answers 404. The repair
    // is complete and reported; the key is simply not probed.
    let control = MockServer::start().await;
    mount_clean_repair(&control).await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["verification"], "skipped");
    let stderr = stderr_without_notes(&output);
    assert!(
        stderr.contains(&format!(
            "Note: could not read service {QUERY_TEST_SERVICE_ID}:"
        )),
        "{stderr}"
    );
    assert!(
        stderr.contains(QUERY_TEST_KEY_UUID),
        "the note names the stored key: {stderr}"
    );
    assert!(probes_received(&control).await.is_empty());
}

#[tokio::test]
async fn repair_skips_verification_without_a_configured_query_host() {
    // `--url` points at a host the Query API host cannot be derived from and
    // `CLICKHOUSE_CLOUD_QUERY_HOST` is unset: the library would fall back to
    // the production host, so the CLI does not probe at all. Every request
    // the run made went to the mock, and none of them was a probe.
    let control = MockServer::start().await;
    mount_running_service_accepting_probes(&control, 0).await;
    mount_clean_repair(&control).await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .env_remove("CLICKHOUSE_CLOUD_QUERY_HOST")
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["verification"], "skipped");
    let stderr = stderr_without_notes(&output);
    assert!(
        stderr.contains("Note: no Query API host is configured for"),
        "{stderr}"
    );
    assert!(stderr.contains("CLICKHOUSE_CLOUD_QUERY_HOST"), "{stderr}");
    let requests = control.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .all(|request| request.url.path().starts_with("/v1/")),
        "only control-plane requests, no probe: {:?}",
        requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect::<Vec<_>>()
    );
    // Old key retired, new key kept: the repair itself is untouched.
    assert_eq!(
        key_deletes_received(&control).await,
        vec![OLD_QUERY_TEST_KEY_UUID.to_string()]
    );
}

#[tokio::test]
async fn a_failed_probe_reports_the_committed_repair_and_exits_zero() {
    // The probe fails for a reason that is not "not ready yet". The repair
    // itself is complete and consistent, so nothing is undone and the exit
    // code stays 0: the result names the new key with `verification: failed`
    // and a warning on stderr says what failed and how the key gets verified.
    let control = MockServer::start().await;
    mount_repair_service_state(&control, "running").await;
    mount_clean_repair(&control).await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(
            ResponseTemplate::new(502).set_body_json(serde_json::json!({ "error": "bad gateway" })),
        )
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["verification"], "failed");
    let stderr = stderr_without_notes(&output);
    assert!(
        stderr.contains(&format!(
            "Warning: the query key for service {QUERY_TEST_SERVICE_ID} was replaced (new API \
             key {QUERY_TEST_KEY_UUID}) and stored in .clickhouse/credentials.json, but probing \
             it failed"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("bad gateway"), "{stderr}");
    assert!(stderr.contains("is verified by the next"), "{stderr}");

    assert_eq!(
        key_deletes_received(&control).await,
        vec![OLD_QUERY_TEST_KEY_UUID.to_string()],
        "the old key is retired and the new key is never deleted"
    );
    let stored = read_credentials(project.path());
    let repaired = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(repaired["key_id"], "replacement-key-id");
}

#[tokio::test]
async fn repair_skips_verification_when_the_probe_finds_the_service_stopped() {
    // The service was running when its state was read and stopped by the time
    // the probe arrived (HTTP 206 `Service is stopped`). Not a failure of the
    // key: the probe is skipped like any other not-running service.
    let control = MockServer::start().await;
    mount_repair_service_state(&control, "running").await;
    mount_clean_repair(&control).await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(
            ResponseTemplate::new(206)
                .set_body_json(serde_json::json!({ "data": "Service is stopped" })),
        )
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["verification"], "skipped");
    let stderr = stderr_without_notes(&output);
    assert!(
        stderr.contains(&format!("Note: service {QUERY_TEST_SERVICE_ID} is stopped")),
        "{stderr}"
    );
    assert!(!stderr.contains("Warning:"), "{stderr}");
}

#[tokio::test]
#[ignore = "waits out the real 120 s Query API readiness window; run explicitly"]
async fn a_key_the_query_api_never_accepts_reports_the_repair_then_fails_with_its_own_code() {
    // The probe is rejected for the whole readiness window. The repair is
    // committed and printed first, with `verification: failed`; then the
    // structured error follows on stderr with its own code, the new key's ID
    // and the query command to run — never a repair rerun, which would rotate
    // a key that may only be slow to propagate. Nothing is rolled back.
    let control = MockServer::start().await;
    mount_repair_service_state(&control, "running").await;
    mount_clean_repair(&control).await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        stderr_without_notes(&output)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "repaired");
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);
    assert_eq!(result["verification"], "failed");
    let error = structured_error(&output);
    assert_eq!(error["error"]["code"], "query_key_repair_unverified");
    assert_eq!(error["error"]["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(
        error["error"]["command"],
        format!(
            "clickhousectl cloud service query --id {QUERY_TEST_SERVICE_ID} --org-id org-1 \
             --query \"SELECT 1\""
        )
    );
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("was replaced (new API key"), "{message}");
    assert!(
        message.contains("Do not rerun repair-query-key"),
        "{message}"
    );
    assert!(
        probes_received(&control).await.len() > 1,
        "the whole window was used"
    );
    assert_eq!(
        key_deletes_received(&control).await,
        vec![OLD_QUERY_TEST_KEY_UUID.to_string()],
        "no rollback: the old key is retired, the new one kept"
    );
    let repaired = &read_credentials(project.path())["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(repaired["api_key_id"], QUERY_TEST_KEY_UUID);
}

#[tokio::test]
async fn repair_refuses_legacy_query_key_without_any_cloud_mutation() {
    let control = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(project.path(), None, None, Some("ep-1"), &[]);
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("legacy or non-owned record"));
    assert!(control.received_requests().await.unwrap().is_empty());
    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored, original);
}

fn service_query_process(
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
) -> tokio::process::Command {
    service_query_process_with_sql(project_dir, control, query_host, "SELECT 1")
}

fn service_query_process_with_sql(
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
    sql: &str,
) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .current_dir(project_dir)
        .stdin(Stdio::null())
        .args([
            "cloud",
            "--url",
            &control.uri(),
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            sql,
        ]);
    command
}

async fn run_concurrent_service_queries(
    count: usize,
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
) -> Vec<std::process::Output> {
    let tasks = (0..count)
        .map(|_| {
            let mut command = service_query_process(project_dir, control, query_host);
            tokio::spawn(async move {
                command
                    .output()
                    .await
                    .expect("failed to spawn clickhousectl")
            })
        })
        .collect::<Vec<_>>();

    let mut outputs = Vec::with_capacity(count);
    for task in tasks {
        outputs.push(task.await.expect("clickhousectl task panicked"));
    }
    outputs
}

fn write_preserved_query_credentials(project_dir: &Path) -> Value {
    let credentials = serde_json::json!({
        "service_query_keys": {
            PRESERVED_QUERY_SERVICE_ID: {
                "organization_id": "org-1",
                "api_key_id": "preserved-api-key-uuid",
                "key_id": "preserved-key-id",
                "key_secret": "preserved-key-secret",
                "endpoint_id": "preserved-endpoint",
                "service_name": "preserved",
                "created_at": "2026-05-11T12:00:00Z"
            }
        }
    });
    let clickhouse_dir = project_dir.join(".clickhouse");
    std::fs::create_dir_all(&clickhouse_dir).unwrap();
    std::fs::write(
        clickhouse_dir.join("credentials.json"),
        serde_json::to_vec(&credentials).unwrap(),
    )
    .unwrap();
    credentials
}

async fn provision_while_project_auth_changes(auth_args: &[&str]) -> tempfile::TempDir {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": { "id": QUERY_TEST_KEY_UUID },
                "keyId": "provisioned-key-id",
                "keySecret": "provisioned-key-secret"
            },
            "status": 200,
            "requestId": "stub-key-create"
        })))
        .expect(1)
        .mount(&control)
        .await;
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not found",
            "status": 404,
            "requestId": "stub-endpoint-get"
        })))
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    write_preserved_query_credentials(project.path());
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let auth_args = auth_args
        .iter()
        .map(|argument| (*argument).to_string())
        .collect::<Vec<_>>();
    let auth_project = project.path().to_path_buf();
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |_: &wiremock::Request| {
            let mut command = Command::new(clickhousectl_binary());
            clear_inherited_env(&mut command);
            let output = command
                .env("DO_NOT_TRACK", "1")
                .env("HOME", &home)
                .current_dir(&auth_project)
                .args(&auth_args)
                .output()
                .expect("failed to spawn concurrent auth command");
            assert_success(&output);
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": "ep-1", "openApiKeys": [QUERY_TEST_KEY_UUID] },
                "status": 200,
                "requestId": "stub-endpoint-upsert"
            }))
        })
        .expect(1)
        .mount(&control)
        .await;

    let output = service_query_process(project.path(), &control, &query_host)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    project
}

#[tokio::test]
async fn provisioning_merges_a_concurrent_api_key_login() {
    let project = provision_while_project_auth_changes(&[
        "cloud",
        "auth",
        "login",
        "--api-key",
        "concurrent-key",
        "--api-secret",
        "concurrent-secret",
    ])
    .await;

    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["api_key"], "concurrent-key");
    assert_eq!(stored["api_secret"], "concurrent-secret");
    assert_eq!(
        stored["service_query_keys"][PRESERVED_QUERY_SERVICE_ID]["key_id"],
        "preserved-key-id"
    );
    assert_eq!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]["key_id"],
        "provisioned-key-id"
    );
}

#[tokio::test]
async fn provisioning_does_not_restore_credentials_cleared_by_concurrent_logout() {
    let project =
        provision_while_project_auth_changes(&["cloud", "auth", "logout", "--api-keys"]).await;

    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert!(stored.get("api_key").is_none());
    assert!(stored.get("api_secret").is_none());
    assert!(
        stored["service_query_keys"]
            .get(PRESERVED_QUERY_SERVICE_ID)
            .is_none()
    );
    assert_eq!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]["key_id"],
        "provisioned-key-id"
    );
    assert_eq!(stored["service_query_keys"].as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn concurrent_service_queries_provision_once_and_reuse_atomically_saved_credentials() {
    const PROCESS_COUNT: usize = 6;

    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    let project = tempfile::tempdir().unwrap();
    let original_credentials = write_preserved_query_credentials(project.path());
    std::fs::create_dir(project.path().join("home")).unwrap();

    // Lock ownership is held by the OS file handle. Contents left by a dead
    // process must not make the lock stale or block the next provisioner.
    std::fs::write(
        project.path().join(".clickhouse/query-provisioning.lock"),
        "stale owner metadata",
    )
    .unwrap();
    let provisioning_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(project.path().join(".clickhouse/query-provisioning.lock"))
        .unwrap();
    provisioning_lock.lock().unwrap();

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": { "id": QUERY_TEST_KEY_UUID },
                "keyId": "provisioned-key-id",
                "keySecret": "provisioned-key-secret"
            },
            "status": 200,
            "requestId": "stub-key-create"
        })))
        .mount(&control)
        .await;
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not found",
            "status": 404,
            "requestId": "stub-endpoint-get"
        })))
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1", "openApiKeys": [QUERY_TEST_KEY_UUID] },
            "status": 200,
            "requestId": "stub-endpoint-upsert"
        })))
        .mount(&control)
        .await;

    let mut children = Vec::with_capacity(PROCESS_COUNT);
    for _ in 0..PROCESS_COUNT {
        children.push(
            service_query_process(project.path(), &control, &query_host)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn clickhousectl"),
        );
    }

    let primary_auth = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "fake-key-for-tests:fake-secret-for-tests",
        )
    );
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let started = query_host
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|request| {
                    request
                        .headers
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        == Some(primary_auth.as_str())
                })
                .count();
            if started == PROCESS_COUNT {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("concurrent queries did not all reach provisioning");

    drop(provisioning_lock);
    let mut outputs = Vec::with_capacity(PROCESS_COUNT);
    for child in children {
        outputs.push(
            child
                .wait_with_output()
                .await
                .expect("failed to wait for clickhousectl"),
        );
    }
    for output in &outputs {
        assert_success(output);
        assert_eq!(output.stdout, b"1\n");
    }

    // A later process proves the persisted result is immediately reusable and
    // does not enter provisioning again.
    let reuse_output = service_query_process(project.path(), &control, &query_host)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&reuse_output);

    let requests = control.received_requests().await.unwrap();
    let key_creates = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST
                && request.url.path() == "/v1/organizations/org-1/keys"
        })
        .count();
    let endpoint_gets = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::GET && request.url.path() == endpoint_path
        })
        .count();
    let endpoint_upserts = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
        })
        .collect::<Vec<_>>();
    assert_eq!(key_creates, 1, "only the lock holder may create a key");
    assert_eq!(
        endpoint_gets, 1,
        "only the lock holder may inspect the endpoint"
    );
    assert_eq!(endpoint_upserts.len(), 1, "the endpoint must be bound once");
    assert!(
        requests
            .iter()
            .all(|request| request.method != wiremock::http::Method::DELETE),
        "successful provisioning must not delete a key",
    );
    let upsert_body: Value = serde_json::from_slice(&endpoint_upserts[0].body).unwrap();
    assert_eq!(
        upsert_body["openApiKeys"],
        serde_json::json!([QUERY_TEST_KEY_UUID])
    );

    let credentials_bytes =
        std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap();
    let credentials: Value = serde_json::from_slice(&credentials_bytes)
        .expect("atomic replacement must leave valid credential JSON");
    assert_eq!(
        credentials["service_query_keys"][PRESERVED_QUERY_SERVICE_ID],
        original_credentials["service_query_keys"][PRESERVED_QUERY_SERVICE_ID],
        "the under-lock merge must preserve unrelated credentials",
    );
    let stored = &credentials["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(stored["organization_id"], "org-1");
    assert_eq!(stored["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(stored["key_id"], "provisioned-key-id");
    assert_eq!(stored["key_secret"], "provisioned-key-secret");
    assert_eq!(stored["endpoint_id"], "ep-1");
    assert!(stored["created_at"].is_string());
    assert_eq!(
        std::fs::read_to_string(project.path().join(".clickhouse/.gitignore")).unwrap(),
        "*\n",
        "a pre-existing project metadata directory must still get ignored",
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(project.path().join(".clickhouse/credentials.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

async fn mount_successful_query_provisioning(control: &MockServer) -> String {
    mount_key_create_and_delete(
        control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret"
        }),
    )
    .await;
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not found",
            "status": 404,
            "requestId": "stub-endpoint-get"
        })))
        .expect(1)
        .mount(control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1", "openApiKeys": [QUERY_TEST_KEY_UUID] },
            "status": 200,
            "requestId": "stub-endpoint-upsert"
        })))
        .expect(1)
        .mount(control)
        .await;
    endpoint_path
}

fn query_test_basic_auth(credentials: &str) -> String {
    format!(
        "Basic {}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials)
    )
}

#[tokio::test]
async fn just_provisioned_service_query_retries_readiness_errors_with_the_same_key() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{Duration, Instant};

    let control = start_mock_control_plane_with_service().await;
    let endpoint_path = mount_successful_query_provisioning(&control).await;
    let query_host = MockServer::start().await;
    let query_path = format!("/service/{QUERY_TEST_SERVICE_ID}/run");
    let primary_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    Mock::given(method("POST"))
        .and(path(query_path.clone()))
        .and(header("authorization", primary_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .expect(1)
        .mount(&query_host)
        .await;

    let provisioned_auth = query_test_basic_auth("provisioned-key-id:provisioned-key-secret");
    let response_index = Arc::new(AtomicUsize::new(0));
    let delivered_statuses = Arc::new(Mutex::new(Vec::new()));
    let responder_index = Arc::clone(&response_index);
    let responder_statuses = Arc::clone(&delivered_statuses);
    Mock::given(method("POST"))
        .and(path(query_path))
        .and(header("authorization", provisioned_auth.as_str()))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            match body["sql"].as_str() {
                Some("SELECT 1") => {
                    let index = responder_index.fetch_add(1, Ordering::SeqCst);
                    let status = [401, 403, 404, 206].get(index).copied().unwrap_or(500);
                    responder_statuses.lock().unwrap().push(status);
                    if status == 206 {
                        ResponseTemplate::new(status)
                            .set_body_string(r#"{"data":"Confirm wake service"}"#)
                    } else {
                        ResponseTemplate::new(status).set_body_string("query endpoint is not ready")
                    }
                }
                Some("SELECT 42") => ResponseTemplate::new(200).set_body_string("42\n"),
                sql => ResponseTemplate::new(400)
                    .set_body_string(format!("unexpected SQL in readiness test: {sql:?}")),
            }
        })
        .expect(5)
        .mount(&query_host)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let started = Instant::now();
    let output = service_query_process_with_sql(project.path(), &control, &query_host, "SELECT 42")
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    let elapsed = started.elapsed();

    assert_success(&output);
    assert_eq!(output.stdout, b"42\n");
    assert_eq!(*delivered_statuses.lock().unwrap(), [401, 403, 404, 206]);
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Waiting for the Query API endpoint to become ready")
    );
    assert!(
        elapsed >= Duration::from_millis(1_200),
        "readiness retries did not back off: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "readiness retries exceeded their expected timing bound: {elapsed:?}"
    );

    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 6);
    let provisioned_requests: Vec<_> = query_requests
        .iter()
        .filter(|request| {
            request
                .headers
                .get("authorization")
                .is_some_and(|value| value == provisioned_auth.as_str())
        })
        .collect();
    assert_eq!(
        provisioned_requests.len(),
        5,
        "every retry must reuse the new key"
    );
    let sql: Vec<_> = provisioned_requests
        .iter()
        .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap()["sql"].clone())
        .collect();
    assert_eq!(
        sql.iter().filter(|value| **value == "SELECT 1").count(),
        4,
        "readiness retries must use the harmless probe"
    );
    let user_queries: Vec<_> = provisioned_requests
        .iter()
        .filter(|request| {
            serde_json::from_slice::<Value>(&request.body).unwrap()["sql"] == "SELECT 42"
        })
        .collect();
    assert_eq!(user_queries.len(), 1, "user SQL must run exactly once");
    assert_eq!(user_queries[0].headers.get("wake-service").unwrap(), "true");

    let control_requests = control.received_requests().await.unwrap();
    let request_count = |request_method: wiremock::http::Method, request_path: &str| {
        control_requests
            .iter()
            .filter(|request| {
                request.method == request_method && request.url.path() == request_path
            })
            .count()
    };
    assert_eq!(
        request_count(wiremock::http::Method::POST, "/v1/organizations/org-1/keys"),
        1,
        "readiness retries must not reprovision the key"
    );
    assert_eq!(
        request_count(wiremock::http::Method::POST, &endpoint_path),
        1,
        "readiness retries must not upsert the endpoint again"
    );
}

#[tokio::test]
async fn just_provisioned_service_query_fails_immediately_when_the_service_is_stopped() {
    use std::time::{Duration, Instant};

    let control = start_mock_control_plane_with_service().await;
    mount_successful_query_provisioning(&control).await;
    let query_host = MockServer::start().await;
    let query_path = format!("/service/{QUERY_TEST_SERVICE_ID}/run");
    let primary_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    Mock::given(method("POST"))
        .and(path(query_path.clone()))
        .and(header("authorization", primary_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .expect(1)
        .mount(&query_host)
        .await;
    let provisioned_auth = query_test_basic_auth("provisioned-key-id:provisioned-key-secret");
    Mock::given(method("POST"))
        .and(path(query_path))
        .and(header("authorization", provisioned_auth.as_str()))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            r#"{"error":"ClickHouse service is currently unavailable. Please try again later."}"#,
        ))
        .expect(1)
        .mount(&query_host)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let started = Instant::now();
    let output = service_query_process(project.path(), &control, &query_host)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    let elapsed = started.elapsed();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(query_host.received_requests().await.unwrap().len(), 2);
    assert!(
        elapsed < Duration::from_secs(5),
        "stopped-service recognition unexpectedly waited: {elapsed:?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Provisioning Query API endpoint + key for service 'demo'...\nError: service 'demo' is stopped; start it with `clickhousectl cloud service start {QUERY_TEST_SERVICE_ID} --org-id org-1` and retry\n"
        )
    );
}

#[tokio::test]
async fn concurrent_failed_provisioners_delete_only_their_exact_created_key() {
    const PROCESS_COUNT: usize = 3;
    const UNRELATED_BOUND_KEY: &str = "99999999-8888-7777-6666-555555555555";

    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    let project = tempfile::tempdir().unwrap();
    let original_credentials = write_preserved_query_credentials(project.path());
    std::fs::create_dir(project.path().join("home")).unwrap();
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret"
        }),
    )
    .await;
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1", "openApiKeys": [UNRELATED_BOUND_KEY] },
            "status": 200,
            "requestId": "stub-endpoint-get"
        })))
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "upsert failed",
            "status": 500,
            "requestId": "stub-endpoint-upsert"
        })))
        .mount(&control)
        .await;

    let outputs =
        run_concurrent_service_queries(PROCESS_COUNT, project.path(), &control, &query_host).await;
    for output in &outputs {
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("upsert failed"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let requests = control.received_requests().await.unwrap();
    let count = |request_method: wiremock::http::Method, request_path: &str| {
        requests
            .iter()
            .filter(|request| {
                request.method == request_method && request.url.path() == request_path
            })
            .count()
    };
    assert_eq!(
        count(wiremock::http::Method::POST, "/v1/organizations/org-1/keys"),
        PROCESS_COUNT
    );
    assert_eq!(
        count(wiremock::http::Method::GET, &endpoint_path),
        PROCESS_COUNT
    );
    assert_eq!(
        count(wiremock::http::Method::POST, &endpoint_path),
        PROCESS_COUNT
    );
    let deletes = requests
        .iter()
        .filter(|request| request.method == wiremock::http::Method::DELETE)
        .collect::<Vec<_>>();
    assert_eq!(deletes.len(), PROCESS_COUNT);
    assert!(deletes.iter().all(|request| {
        request.url.path() == format!("/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}")
    }));
    for upsert in requests.iter().filter(|request| {
        request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
    }) {
        let body: Value = serde_json::from_slice(&upsert.body).unwrap();
        assert_eq!(
            body["openApiKeys"],
            serde_json::json!([UNRELATED_BOUND_KEY, QUERY_TEST_KEY_UUID]),
            "the unrelated endpoint binding must be preserved",
        );
    }

    let credentials: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(credentials, original_credentials);
    assert!(
        credentials["service_query_keys"]
            .get(QUERY_TEST_SERVICE_ID)
            .is_none(),
        "a failed provision must not leave an untracked local record",
    );
}

/// Mount a key-creation POST returning `result`, plus a key DELETE, on the
/// control plane. `result` lets each test omit exactly the field under test.
async fn mount_key_create_and_delete(control: &MockServer, result: Value) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-key-create",
        })))
        .mount(control)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-key-delete",
        })))
        .mount(control)
        .await;
}

/// Run `cloud service query` in an empty project dir (no stored key, so the
/// provisioning path runs) against `control`, with API key env creds.
async fn invoke_service_query_provisioning(control: &MockServer) -> (tempfile::TempDir, String) {
    let query_host = start_mock_query_host_for_provisioning().await;
    let dir = tempfile::tempdir().unwrap();
    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");

    assert!(
        !output.status.success(),
        "provisioning with an incomplete response must fail\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    (dir, String::from_utf8_lossy(&output.stderr).to_string())
}

/// The key UUIDs the control plane was asked to delete.
async fn recorded_key_deletes(control: &MockServer) -> Vec<String> {
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.method == wiremock::http::Method::DELETE)
        .map(|r| r.url.path().to_string())
        .collect()
}

#[tokio::test]
async fn service_query_unbinds_before_deleting_key_when_credential_persistence_fails() {
    const EXISTING_BOUND_KEY: &str = "99999999-8888-7777-6666-555555555555";

    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret"
        }),
    )
    .await;

    let project = tempfile::tempdir().unwrap();
    let credentials_dir = project.path().join(".clickhouse");
    std::fs::create_dir(&credentials_dir).unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    let endpoint_get_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            let result =
                if endpoint_get_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    serde_json::json!({
                        "id": "ep-1",
                        "allowedOrigins": "https://before.example",
                        "openApiKeys": [EXISTING_BOUND_KEY],
                        "roles": ["sql_console_read_only"]
                    })
                } else {
                    serde_json::json!({
                        "id": "ep-1",
                        "allowedOrigins": "*",
                        "openApiKeys": [EXISTING_BOUND_KEY, QUERY_TEST_KEY_UUID],
                        "roles": ["sql_console_admin"]
                    })
                };
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-endpoint-get"
            }))
        })
        .expect(2)
        .mount(&control)
        .await;
    let endpoint_upsert_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if endpoint_upsert_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                // Provisioning has already loaded credentials. A directory at
                // the destination makes the atomic replacement fail after bind.
                std::fs::create_dir(&credentials_path).unwrap();
            }
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": "ep-1" },
                "status": 200,
                "requestId": "stub-endpoint-upsert"
            }))
        })
        .expect(2)
        .mount(&control)
        .await;

    let output = service_query_process(project.path(), &control, &query_host)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert!(!output.status.success());

    let requests = control.received_requests().await.unwrap();
    let endpoint_upserts = requests
        .iter()
        .enumerate()
        .filter(|(_, request)| {
            request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
        })
        .collect::<Vec<_>>();
    assert_eq!(endpoint_upserts.len(), 2);
    let bind_body: Value = serde_json::from_slice(&endpoint_upserts[0].1.body).unwrap();
    assert_eq!(
        bind_body["openApiKeys"],
        serde_json::json!([EXISTING_BOUND_KEY, QUERY_TEST_KEY_UUID])
    );
    let unbind_body: Value = serde_json::from_slice(&endpoint_upserts[1].1.body).unwrap();
    assert_eq!(
        unbind_body,
        serde_json::json!({
            "allowedOrigins": "*",
            "openApiKeys": [EXISTING_BOUND_KEY],
            "roles": ["sql_console_admin"]
        }),
        "compensation must preserve the current endpoint while removing only its own key",
    );
    let key_delete = requests
        .iter()
        .position(|request| {
            request.method == wiremock::http::Method::DELETE
                && request.url.path()
                    == format!("/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}")
        })
        .expect("the unbound key must be deleted");
    assert!(
        endpoint_upserts[1].0 < key_delete,
        "the endpoint must be repaired before its key is deleted",
    );
    assert!(credentials_dir.join("credentials.json").is_dir());
}

#[tokio::test]
async fn service_query_retains_key_when_persistence_and_unbind_both_fail() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret"
        }),
    )
    .await;

    let project = tempfile::tempdir().unwrap();
    let credentials_dir = project.path().join(".clickhouse");
    std::fs::create_dir(&credentials_dir).unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    let endpoint_get_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if endpoint_get_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                ResponseTemplate::new(404).set_body_json(serde_json::json!({
                    "error": "not found",
                    "status": 404,
                    "requestId": "stub-endpoint-get"
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": {
                        "id": "ep-1",
                        "allowedOrigins": "*",
                        "openApiKeys": [QUERY_TEST_KEY_UUID],
                        "roles": ["sql_console_admin"]
                    },
                    "status": 200,
                    "requestId": "stub-endpoint-get-after-bind"
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;
    let endpoint_upsert_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |_: &wiremock::Request| {
            if endpoint_upsert_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                std::fs::create_dir(&credentials_path).unwrap();
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": "ep-1" },
                    "status": 200,
                    "requestId": "stub-endpoint-bind"
                }))
            } else {
                ResponseTemplate::new(500).set_body_json(serde_json::json!({
                    "error": "endpoint unbind rejected",
                    "status": 500,
                    "requestId": "stub-endpoint-unbind"
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;

    let output = service_query_process(project.path(), &control, &query_host)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert!(!output.status.success());
    assert!(
        recorded_key_deletes(&control).await.is_empty(),
        "a still-bound key must be retained",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("local credential persistence failed")
            && stderr.contains("endpoint unbind rejected")
            && stderr.contains(QUERY_TEST_KEY_UUID)
            && stderr.contains("retained for recovery"),
        "the error must report both failures and the recoverable key ID:\n{stderr}",
    );
}

#[tokio::test]
async fn service_query_deletes_the_key_when_the_create_response_omits_the_secret() {
    let control = start_mock_control_plane_with_service().await;
    // `keySecret` absent: the key exists but cannot authenticate anything.
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
        }),
    )
    .await;

    let (dir, stderr) = invoke_service_query_provisioning(&control).await;
    assert!(
        stderr.contains("keySecret"),
        "stderr should name the missing field:\n{stderr}",
    );

    assert_eq!(
        recorded_key_deletes(&control).await,
        vec![format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )],
        "the unusable key must be deleted exactly once",
    );

    // The key was never bound, so no endpoint upsert was attempted, and
    // nothing was persisted locally.
    let upserts = control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path().ends_with("/serviceQueryEndpoint"))
        .count();
    assert_eq!(upserts, 0, "a keyless credential must not be bound");
    assert!(!dir.path().join(".clickhouse/credentials.json").exists());
}

#[tokio::test]
async fn service_query_keeps_the_key_when_the_endpoint_response_omits_the_id() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret",
        }),
    )
    .await;
    // No endpoint configured yet (404), and the upsert succeeds but answers
    // without `id`. The key is bound and usable: the echoed id is diagnostic
    // only, so provisioning completes rather than discarding the credential.
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not found",
            "status": 404,
            "requestId": "stub-endpoint-get",
        })))
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "roles": ["sql_console_admin"] },
            "status": 200,
            "requestId": "stub-endpoint-upsert",
        })))
        .mount(&control)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    assert!(
        recorded_key_deletes(&control).await.is_empty(),
        "a bound, usable key must not be discarded over an unused echoed id",
    );

    // The credential is persisted, with `endpoint_id` omitted rather than
    // written as a placeholder.
    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    let key = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(key["organization_id"], "org-1");
    assert_eq!(key["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(key["key_id"], "provisioned-key-id");
    assert_eq!(key["key_secret"], "provisioned-key-secret");
    assert!(
        key.get("endpoint_id").is_none(),
        "an absent endpoint id must not be stored: {stored}",
    );
}

#[tokio::test]
async fn service_query_deletes_the_key_when_the_endpoint_get_omits_open_api_keys() {
    let control = start_mock_control_plane_with_service().await;
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret",
        }),
    )
    .await;
    // A 200 endpoint GET whose `openApiKeys` is absent leaves the currently
    // bound keys unknown. The upsert replaces the list wholesale, so binding
    // on top of an assumed-empty list would revoke them.
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1", "roles": ["sql_console_admin"] },
            "status": 200,
            "requestId": "stub-endpoint-get",
        })))
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1" },
            "status": 200,
            "requestId": "stub-endpoint-upsert",
        })))
        .mount(&control)
        .await;

    let (dir, stderr) = invoke_service_query_provisioning(&control).await;

    let upserts = control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.method == wiremock::http::Method::POST && r.url.path() == endpoint_path)
        .count();
    assert_eq!(
        upserts, 0,
        "the endpoint must not be rebound from an unknown key list",
    );
    assert_eq!(
        recorded_key_deletes(&control).await,
        vec![format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )],
        "the unbindable key must be deleted exactly once",
    );
    assert!(!dir.path().join(".clickhouse/credentials.json").exists());
    assert!(
        stderr.contains("'openApiKeys'"),
        "stderr should name the omitted field:\n{stderr}",
    );
}

/// Provision against an endpoint GET that reports `existing_keys`, and return
/// the `openApiKeys` the upsert was sent, plus the project dir.
async fn provision_against_endpoint_with_keys(existing_keys: Value) -> (tempfile::TempDir, Value) {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    mount_key_create_and_delete(
        &control,
        serde_json::json!({
            "key": { "id": QUERY_TEST_KEY_UUID },
            "keyId": "provisioned-key-id",
            "keySecret": "provisioned-key-secret",
        }),
    )
    .await;
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1", "openApiKeys": existing_keys },
            "status": 200,
            "requestId": "stub-endpoint-get",
        })))
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1" },
            "status": 200,
            "requestId": "stub-endpoint-upsert",
        })))
        .mount(&control)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    assert!(
        recorded_key_deletes(&control).await.is_empty(),
        "a successfully bound key must not be discarded",
    );
    let upsert = control
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.method == wiremock::http::Method::POST && r.url.path() == endpoint_path)
        .expect("the endpoint upsert must be sent");
    let body: Value = serde_json::from_slice(&upsert.body).unwrap();
    assert_eq!(body["roles"], serde_json::json!(["sql_console_admin"]));
    (dir, body["openApiKeys"].clone())
}

#[tokio::test]
async fn service_query_binds_the_new_key_when_the_endpoint_reports_no_keys() {
    // An explicitly empty `openApiKeys` is a real answer, not an omission.
    let (dir, sent_keys) = provision_against_endpoint_with_keys(serde_json::json!([])).await;
    assert_eq!(sent_keys, serde_json::json!([QUERY_TEST_KEY_UUID]));
    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]["endpoint_id"], "ep-1",
        "an echoed endpoint id is recorded",
    );
}

#[tokio::test]
async fn service_query_merges_the_new_key_into_the_reported_keys() {
    let existing = "99999999-8888-7777-6666-555555555555";
    let (_dir, sent_keys) =
        provision_against_endpoint_with_keys(serde_json::json!([existing])).await;
    assert_eq!(
        sent_keys,
        serde_json::json!([existing, QUERY_TEST_KEY_UUID]),
        "an existing binding must survive the upsert",
    );
}

// ── Idled / stopped services ───────────────────────────────────────────────
//
// An idled service answers 206 `Confirm wake service`, which the CLI retries
// with `wake-service: true`. A stopped service currently answers 404 with an
// unavailable-service error; the CLI must not treat that 404 as a missing
// query endpoint and must instead fail with a hint to start the service.

/// Write an OAuth tokens.json into `ch_dir` (the caller's `$HOME/.clickhouse`)
/// so the binary authenticates with a bearer token against the given control
/// plane. Callers must also set `HOME` to the parent of `ch_dir`.
fn write_oauth_tokens(ch_dir: &std::path::Path, control_uri: &str) {
    let tokens = serde_json::json!({
        "access_token": "test-bearer-token",
        "refresh_token": "unused",
        "expires_at": 4102444800u64, // 2100-01-01: never expires in tests
        "api_url": format!("{control_uri}/v1"),
    });
    std::fs::write(
        ch_dir.join("tokens.json"),
        serde_json::to_vec(&tokens).unwrap(),
    )
    .unwrap();
}

async fn invoke_oauth_service_query_error(body: &str) -> std::process::Output {
    let control = start_mock_control_plane_with_service().await;
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(400).set_body_string(body))
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &control.uri());

    let url = control.uri();
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT broken FROM",
        ])
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn service_query_renders_documented_sql_error() {
    let output = invoke_oauth_service_query_error(
        r#"{"error":{"code":"62","details":"Syntax error near FROM"}}"#,
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: SQL error 62: Syntax error near FROM\n"
    );
}

#[tokio::test]
async fn service_query_preserves_malformed_json_error_and_status() {
    let body = r#"{"error":{"code":"62","details":"truncated"#;
    let output = invoke_oauth_service_query_error(body).await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("Error: Query API returned HTTP 400 Bad Request: {body}\n")
    );
}

#[tokio::test]
async fn service_query_preserves_non_json_error_and_status() {
    let output = invoke_oauth_service_query_error("upstream proxy failed").await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Query API returned HTTP 400 Bad Request: upstream proxy failed\n"
    );
}

#[tokio::test]
async fn service_query_resends_with_wake_header_when_service_is_idle() {
    let control = start_mock_control_plane_with_service().await;

    // Query host that refuses attempts without the wake confirmation: the
    // header-matched mock (higher priority) runs the query, the fallback
    // answers 206 `Confirm wake service`.
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .and(header("wake-service", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(
            ResponseTemplate::new(206).set_body_string(r#"{"data":"Confirm wake service"}"#),
        )
        .with_priority(5)
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &control.uri());

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    // Exactly two attempts: the refused one without the wake header, then
    // the resend carrying the wake confirmation.
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 2);
    assert!(
        query_requests[0].headers.get("wake-service").is_none(),
        "first attempt must not pre-emptively wake the service",
    );
    assert_eq!(
        query_requests[1].headers.get("wake-service").unwrap(),
        "true"
    );

    // The 206 body must not leak into the query output, and the user is
    // told about the wake on stderr.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("idle"),
        "stderr should mention the service is idle:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[tokio::test]
async fn service_query_fails_with_start_hint_when_service_is_stopped() {
    let control = start_mock_control_plane_with_service().await;

    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            r#"{"error":"ClickHouse service is currently unavailable. Please try again later."}"#,
        ))
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    std::fs::create_dir(&home_dir).unwrap();

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("HOME", home_dir)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: service 'demo' is stopped; start it with `clickhousectl cloud service start {QUERY_TEST_SERVICE_ID} --org-id org-1` and retry\n"
        ),
    );

    // The stopped response is terminal: no wake resend and no endpoint/key
    // provisioning despite its HTTP 404 status.
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    let control_requests = control.received_requests().await.unwrap();
    assert!(
        control_requests
            .iter()
            .all(|request| request.method == wiremock::http::Method::GET),
        "stopped service query attempted provisioning: {:?}",
        control_requests
            .iter()
            .map(|request| format!("{} {}", request.method, request.url.path()))
            .collect::<Vec<_>>(),
    );
    assert!(!dir.path().join(".clickhouse/credentials.json").exists());
}

// Shell env vars must win over `.env` — if both are set, the request is
// signed with the shell values, never the file values.

#[tokio::test]
async fn shell_env_overrides_dotenv_creds_in_request() {
    use std::io::Write;

    let mock = MockServer::start().await;

    let stub_orgs = serde_json::json!({
        "result": [],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_orgs))
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut env_file = std::fs::File::create(dir.path().join(".env")).unwrap();
    env_file
        .write_all(
            b"CLICKHOUSE_CLOUD_API_KEY=dotenv-key\nCLICKHOUSE_CLOUD_API_SECRET=dotenv-secret\n",
        )
        .unwrap();
    drop(env_file);

    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "shell-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "shell-secret")
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    let auth = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::GET)
        .and_then(|r| r.headers.get("Authorization"))
        .expect("no Authorization header recorded");
    let auth_str = auth.to_str().expect("non-utf8 auth header");
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "shell-key:shell-secret",
        )
    );
    assert_eq!(
        auth_str, expected,
        "shell env vars must override .env values on the wire"
    );
}

// ── Issue #267: agent session/trace headers land on outbound requests ────────
//
// When invoked under a detected AI agent that publishes a session id /
// traceparent to its subprocesses (Claude Code uses CLAUDE_CODE_SESSION_ID;
// TRACEPARENT is the W3C standard var), `clickhousectl` forwards them as the
// `agent-session-id` and `traceparent` request headers via the default headers
// on the shared HTTP client (`crate::http::client_builder`). This proves they
// reach the wire through the client the Cloud library actually uses.

#[tokio::test]
async fn agent_session_and_trace_headers_are_forwarded() {
    let mock = MockServer::start().await;

    let stub_orgs = serde_json::json!({
        "result": [],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_orgs))
        .mount(&mock)
        .await;

    let url = mock.uri();
    let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        // Mark this invocation as Claude Code and expose the session/trace ids.
        .env("AGENT", "claude-code")
        .env("CLAUDE_CODE_SESSION_ID", "sess-test-267")
        .env("TRACEPARENT", traceparent)
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let req = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::GET)
        .expect("no GET request recorded");

    assert_eq!(
        req.headers
            .get("agent-session-id")
            .expect("agent-session-id header missing")
            .to_str()
            .unwrap(),
        "sess-test-267",
    );
    assert_eq!(
        req.headers
            .get("traceparent")
            .expect("traceparent header missing")
            .to_str()
            .unwrap(),
        traceparent,
    );
}

// ── ClickPipe object-storage ingestion-control flags (#289) ─────────────────
//
// `--skip-initial-load` and `--start-after` must serialize to
// `skipInitialLoad` / `startAfter` on the object-storage source body when
// passed, and stay absent when omitted. `--skip-initial-load` requires
// `--queue-url`; `--start-after` conflicts with `--skip-initial-load`.

#[tokio::test]
async fn s3_skip_initial_load_serializes_when_passed() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://bucket.s3.us-east-1.amazonaws.com/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--continuous",
            "--queue-url",
            "https://sqs.us-east-1.amazonaws.com/123/q",
            "--skip-initial-load",
            "--org-id",
            "org",
        ],
    )
    .await;
    let s3 = &body["source"]["objectStorage"];
    assert_eq!(s3["skipInitialLoad"], true);
    assert_eq!(s3["queueUrl"], "https://sqs.us-east-1.amazonaws.com/123/q");
    // startAfter is absent when --start-after not passed.
    assert!(
        s3.get("startAfter").is_none(),
        "startAfter leaked when --start-after not passed: {s3}",
    );
}

#[tokio::test]
async fn s3_start_after_serializes_when_passed() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://bucket.s3.us-east-1.amazonaws.com/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--continuous",
            "--queue-url",
            "https://sqs.us-east-1.amazonaws.com/123/q",
            "--start-after",
            "obj-key-001",
            "--org-id",
            "org",
        ],
    )
    .await;
    let s3 = &body["source"]["objectStorage"];
    assert_eq!(s3["startAfter"], "obj-key-001");
    // skipInitialLoad is absent when --skip-initial-load not passed.
    assert!(
        s3.get("skipInitialLoad").is_none(),
        "skipInitialLoad leaked when --skip-initial-load not passed: {s3}",
    );
}

// ── ClickPipe MySQL --server-id (#289) ─────────────────────────────────────
//
// `--server-id` must serialize to `serverId` on the MySQL source body when
// passed, and stay absent when omitted.

#[tokio::test]
async fn mysql_server_id_serializes_when_passed() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "mysql",
            "--port",
            "3306",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--replication-mode",
            "cdc",
            "--server-id",
            "4242",
            "--org-id",
            "org",
        ],
    )
    .await;
    let mysql = &body["source"]["mysql"];
    assert_eq!(mysql["serverId"], 4242);
}

#[tokio::test]
async fn mysql_server_id_absent_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "mysql",
            "--port",
            "3306",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;
    let mysql = &body["source"]["mysql"];
    assert!(
        mysql.get("serverId").is_none(),
        "serverId leaked when --server-id not passed: {mysql}",
    );
}

// ── ClickPipe settings updates are source-aware (#602) ─────────────────────
//
// `kafka_read_committed` is only supported for Kafka pipes: the API fails the
// whole PUT with "Setting 'kafka_read_committed' is only supported for Kafka
// ClickPipes" for any other source. The handler therefore fetches the pipe to
// classify its source, and only reads back and re-sends the Kafka-only setting
// when the source is Kafka.

const CLICKPIPE_PATH: &str = "/v1/organizations/org/services/svc-id/clickpipes/pipe-id";
const CLICKPIPE_SETTINGS_PATH: &str =
    "/v1/organizations/org/services/svc-id/clickpipes/pipe-id/settings";

/// Stub the pipe GET with a source of the given shape, e.g.
/// `json!({ "objectStorage": { "type": "s3" } })`.
async fn mount_clickpipe_get(mock: &MockServer, source: Value) {
    let stub_pipe = serde_json::json!({
        "result": {
            "id": "00000000-0000-0000-0000-0000000000aa",
            "name": "test-pipe",
            "state": "Running",
            "source": source,
        },
        "status": 200,
        "requestId": "stub-clickpipe-get",
    });
    Mock::given(method("GET"))
        .and(path(CLICKPIPE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_pipe))
        .mount(mock)
        .await;
}

async fn mount_clickpipe_settings_put(mock: &MockServer, result: Value) {
    let updated_settings = serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-settings-update",
    });
    Mock::given(method("PUT"))
        .and(path(CLICKPIPE_SETTINGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(updated_settings))
        .mount(mock)
        .await;
}

/// The (method, path) pairs the mock saw, in order.
async fn recorded_request_shape(mock: &MockServer) -> Vec<(String, String)> {
    mock.received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| {
            (
                request.method.as_str().to_string(),
                request.url.path().to_string(),
            )
        })
        .collect()
}

async fn recorded_put_body(mock: &MockServer) -> Value {
    let requests = mock.received_requests().await.unwrap();
    let put = requests
        .iter()
        .find(|request| request.method == wiremock::http::Method::PUT)
        .expect("no settings PUT request recorded by mock");
    serde_json::from_slice::<Value>(&put.body).unwrap()
}

#[tokio::test]
async fn clickpipe_settings_update_omits_kafka_only_settings_for_non_kafka_pipes() {
    for source in [
        serde_json::json!({ "objectStorage": { "type": "s3", "format": "JSONEachRow" } }),
        serde_json::json!({ "kinesis": { "stream": "events" } }),
        // A response that drops `source` entirely is treated as non-Kafka.
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source.clone()).await;
        mount_clickpipe_settings_put(
            &mock,
            serde_json::json!({ "object_storage_max_file_count": 200 }),
        )
        .await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--object-storage-max-file-count",
                "200",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);

        // No settings GET: nothing needs reading back when no Kafka-only
        // setting is being re-sent.
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".to_string(), CLICKPIPE_PATH.to_string()),
                ("PUT".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
            ],
            "unexpected requests for source {source}"
        );
        assert_eq!(
            recorded_put_body(&mock).await,
            serde_json::json!({ "object_storage_max_file_count": 200 }),
            "kafka-only settings leaked for source {source}"
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_preserves_kafka_read_committed() {
    for (current_settings, expected) in [
        (serde_json::json!({ "kafka_read_committed": true }), true),
        (serde_json::json!({ "kafka_read_committed": false }), false),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(
            &mock,
            serde_json::json!({ "kafka": { "type": "kafka", "brokers": "b:9092" } }),
        )
        .await;
        let current_settings = serde_json::json!({
            "result": current_settings,
            "status": 200,
            "requestId": "stub-settings-get",
        });
        Mock::given(method("GET"))
            .and(path(CLICKPIPE_SETTINGS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(current_settings))
            .mount(&mock)
            .await;
        mount_clickpipe_settings_put(
            &mock,
            serde_json::json!({
                "streaming_max_insert_wait_ms": 1000,
                "kafka_read_committed": expected,
            }),
        )
        .await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--streaming-max-insert-wait-ms",
                "1000",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);

        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".to_string(), CLICKPIPE_PATH.to_string()),
                ("GET".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
                ("PUT".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
            ]
        );
        assert_eq!(
            recorded_put_body(&mock).await,
            serde_json::json!({
                "streaming_max_insert_wait_ms": 1000,
                "kafka_read_committed": expected,
            })
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_rejects_empty_or_invalid_updates_before_http() {
    for flags in [
        vec![],
        vec!["--clickhouse-max-download-threads", "33"],
        vec!["--clickhouse-min-insert-block-size-bytes", "10737418241"],
        vec!["--clickhouse-parallel-distributed-insert-select", "3"],
        vec!["--kafka-read-committed", "yes"],
    ] {
        let mock = MockServer::start().await;
        let mut args = vec![
            "clickpipe",
            "settings",
            "update",
            "svc-id",
            "pipe-id",
            "--org-id",
            "org",
        ];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn clickpipe_settings_update_sends_new_settings_including_zero() {
    for (download_threads, block_bytes, distributed_mode) in [(7, 20971520_u64, 1), (0, 0, 0)] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(
            &mock,
            serde_json::json!({ "objectStorage": { "type": "s3" } }),
        )
        .await;
        let expected = serde_json::json!({
            "clickhouse_max_download_threads": download_threads,
            "clickhouse_min_insert_block_size_bytes": block_bytes,
            "clickhouse_parallel_distributed_insert_select": distributed_mode,
        });
        mount_clickpipe_settings_put(&mock, expected.clone()).await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--clickhouse-max-download-threads",
                &download_threads.to_string(),
                "--clickhouse-min-insert-block-size-bytes",
                &block_bytes.to_string(),
                "--clickhouse-parallel-distributed-insert-select",
                &distributed_mode.to_string(),
            ],
        );
        assert_success(&output);
        assert_eq!(recorded_put_body(&mock).await, expected);
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            expected
        );
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".into(), CLICKPIPE_PATH.into()),
                ("PUT".into(), CLICKPIPE_SETTINGS_PATH.into()),
            ]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_can_explicitly_set_kafka_read_committed() {
    for requested in [true, false] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, serde_json::json!({ "kafka": { "type": "kafka" } })).await;
        let expected = serde_json::json!({ "kafka_read_committed": requested });
        mount_clickpipe_settings_put(&mock, expected.clone()).await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--kafka-read-committed",
                &requested.to_string(),
            ],
        );
        assert_success(&output);
        assert_eq!(recorded_put_body(&mock).await, expected);
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".into(), CLICKPIPE_PATH.into()),
                ("PUT".into(), CLICKPIPE_SETTINGS_PATH.into()),
            ]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_refuses_kafka_setting_for_other_or_unknown_sources() {
    for source in [
        serde_json::json!({ "objectStorage": { "type": "s3" } }),
        serde_json::json!({ "kinesis": {} }),
        serde_json::json!({ "pubsub": {} }),
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source).await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--kafka-read-committed",
                "false",
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("confirmed Kafka source"));
        assert!(output.stdout.is_empty());
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![("GET".into(), CLICKPIPE_PATH.into())]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_never_guesses_an_absent_kafka_setting() {
    for current in [
        serde_json::json!({}),
        serde_json::json!({"kafka_read_committed": null}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, serde_json::json!({ "kafka": {} })).await;
        Mock::given(method("GET"))
            .and(path(CLICKPIPE_SETTINGS_PATH))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": current})),
            )
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--streaming-max-insert-wait-ms",
                "1000",
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("pass --kafka-read-committed"));
        assert!(output.stdout.is_empty());
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".into(), CLICKPIPE_PATH.into()),
                ("GET".into(), CLICKPIPE_SETTINGS_PATH.into()),
            ]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_stops_when_the_preservation_read_fails() {
    let mock = MockServer::start().await;
    mount_clickpipe_get(&mock, serde_json::json!({ "kafka": {} })).await;
    Mock::given(method("GET"))
        .and(path(CLICKPIPE_SETTINGS_PATH))
        .respond_with(
            ResponseTemplate::new(503).set_body_json(serde_json::json!({"error": "unavailable"})),
        )
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "settings",
            "update",
            "svc-id",
            "pipe-id",
            "--org-id",
            "org",
            "--streaming-max-insert-wait-ms",
            "1000",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        recorded_request_shape(&mock).await,
        vec![
            ("GET".into(), CLICKPIPE_PATH.into()),
            ("GET".into(), CLICKPIPE_SETTINGS_PATH.into()),
        ]
    );
}

// ── ClickPipe ingestion settings apply to some pipe types only (#643) ──────
//
// The ingestion settings endpoints exist for streaming and object-storage pipes
// only. For a database CDC pipe the API answers `NOT_FOUND: ingestion for pipe
// "<id>" not found`, which reads as "the pipe is gone". Both settings commands
// therefore classify the pipe's source first and refuse before touching the
// settings endpoint.

/// The settings GET the CLI must not reach for a database CDC pipe. Mounted so
/// the assertion is "the CLI never called it", not "the mock had no route".
async fn mount_clickpipe_settings_get(mock: &MockServer) {
    let settings = serde_json::json!({
        "result": { "streaming_max_insert_wait_ms": 1000 },
        "status": 200,
        "requestId": "stub-settings-get",
    });
    Mock::given(method("GET"))
        .and(path(CLICKPIPE_SETTINGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(settings))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_settings_get_refuses_database_pipes_before_calling_the_endpoint() {
    for (source, label) in [
        (
            serde_json::json!({ "postgres": { "host": "db.example.com" } }),
            "Postgres CDC",
        ),
        (
            serde_json::json!({ "mysql": { "host": "db.example.com" } }),
            "MySQL CDC",
        ),
        (
            serde_json::json!({ "mongodb": { "host": "db.example.com" } }),
            "MongoDB CDC",
        ),
        (
            serde_json::json!({ "bigquery": { "projectId": "proj" } }),
            "BigQuery",
        ),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source.clone()).await;
        mount_clickpipe_settings_get(&mock).await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "get",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
            ],
        );

        assert_eq!(output.status.code(), Some(1), "source {source}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "Error: ClickPipe pipe-id is a {label} pipe; `clickpipe settings get` and \
                 `settings update` apply only to streaming (Kafka, Kinesis) and object-storage \
                 pipes. CDC pipe settings (sync interval, pull batch size) live on the pipe \
                 itself: see `clickhousectl cloud clickpipe get svc-id pipe-id`.\n"
            ),
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).is_empty(),
            "a refusal must print no settings for source {source}"
        );
        // Only the pipe read: the settings endpoint is never called.
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![("GET".to_string(), CLICKPIPE_PATH.to_string())],
            "unexpected requests for source {source}"
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_refuses_database_pipes_before_calling_the_endpoint() {
    let mock = MockServer::start().await;
    mount_clickpipe_get(
        &mock,
        serde_json::json!({ "postgres": { "host": "db.example.com" } }),
    )
    .await;
    mount_clickpipe_settings_get(&mock).await;
    mount_clickpipe_settings_put(&mock, serde_json::json!({})).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "settings",
            "update",
            "svc-id",
            "pipe-id",
            "--streaming-max-insert-wait-ms",
            "1000",
            "--org-id",
            "org",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("ClickPipe pipe-id is a Postgres CDC pipe;"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        recorded_request_shape(&mock).await,
        vec![("GET".to_string(), CLICKPIPE_PATH.to_string())],
    );
}

#[tokio::test]
async fn clickpipe_settings_get_reads_settings_for_pipes_that_have_them() {
    for source in [
        serde_json::json!({ "kafka": { "type": "kafka", "brokers": "b:9092" } }),
        serde_json::json!({ "kinesis": { "stream": "events" } }),
        serde_json::json!({ "objectStorage": { "type": "s3", "format": "JSONEachRow" } }),
        // An unclassifiable pipe proceeds: the API stays the authority.
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source.clone()).await;
        mount_clickpipe_settings_get(&mock).await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "get",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);

        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            serde_json::json!({ "streaming_max_insert_wait_ms": 1000 }),
            "unexpected output for source {source}"
        );
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".to_string(), CLICKPIPE_PATH.to_string()),
                ("GET".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
            ],
            "unexpected requests for source {source}"
        );
    }
}

// ── ClickPipe schema discovery (#289, beta) ────────────────────────────────
//
// `clickpipe schema-discover` POSTs to .../clickpipes/schemaDiscovery with a
// `source` containing the kafka/kinesis source built from the CLI args. The
// request body shape is asserted; the stubbed response is rendered as a table
// (or JSON with --json).

/// Start a wiremock server that accepts a schema-discovery POST and records
/// the request body. Returns inferred fields the CLI renders.
async fn start_mock_schema_discovery_api() -> MockServer {
    let mock = MockServer::start().await;
    let stub_response = serde_json::json!({
        "result": {
            "fields": [
                { "name": "id", "type": "Int64", "optional": false },
                { "name": "event", "type": "String", "optional": true },
            ],
        },
        "status": 200,
        "requestId": "stub-schema-discovery",
    });
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes/schemaDiscovery$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_response))
        .mount(&mock)
        .await;
    mock
}

#[tokio::test]
async fn schema_discover_kafka_posts_source_body() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "svc-id",
            "--org-id",
            "org",
            "kafka",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123:role/x",
        ],
    )
    .await;
    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["brokers"], "broker:9092");
    assert_eq!(kafka["topics"], "topic");
    assert_eq!(kafka["format"], "JSONEachRow");
    // Kinesis is absent for a Kafka discovery request.
    assert!(
        body["source"].get("kinesis").is_none(),
        "kinesis leaked into kafka schema-discovery body: {}",
        body["source"],
    );
}

#[tokio::test]
async fn issue_593_schema_discover_kafka_sends_base64_protobuf_schema() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body_with_stdin(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "svc-id",
            "--org-id",
            "org",
            "kafka",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "Protobuf",
            "--kafka-type",
            "azureeventhub",
            "--event-hubs-connection-string",
            "Endpoint=sb://events.example/;SharedAccessKey=secret",
            "--protobuf-schema-file",
            "-",
        ],
        &[0_u8, 1, 2, 3],
    )
    .await;

    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["protobufSchema"], "AAECAw==");
    assert_eq!(kafka["authentication"], "PLAIN");
    assert_eq!(
        kafka["credentials"],
        serde_json::json!({
            "connectionString": "Endpoint=sb://events.example/;SharedAccessKey=secret"
        })
    );
    assert!(kafka.get("exactlyOnce").is_none());
}

#[tokio::test]
async fn schema_discover_kinesis_posts_source_body() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "svc-id",
            "--org-id",
            "org",
            "kinesis",
            "--stream-name",
            "mystream",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
        ],
    )
    .await;
    let kinesis = &body["source"]["kinesis"];
    assert_eq!(kinesis["streamName"], "mystream");
    assert_eq!(kinesis["region"], "us-east-1");
    assert_eq!(kinesis["format"], "JSONEachRow");
    // Kafka is absent for a Kinesis discovery request.
    assert!(
        body["source"].get("kafka").is_none(),
        "kafka leaked into kinesis schema-discovery body: {}",
        body["source"],
    );
}

#[tokio::test]
async fn schema_discover_object_storage_posts_source_body() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "svc-id",
            "--org-id",
            "org",
            "object-storage",
            "--source-url",
            "https://bucket.s3.us-east-1.amazonaws.com/data/*.csv",
            "--format",
            "CSV",
            "--compression",
            "gzip",
            "--delimiter",
            ",",
            "--iam-role",
            "arn:aws:iam::123:role/x",
        ],
    )
    .await;
    let object_storage = &body["source"]["objectStorage"];
    assert_eq!(
        object_storage["url"],
        "https://bucket.s3.us-east-1.amazonaws.com/data/*.csv"
    );
    assert_eq!(object_storage["format"], "CSV");
    assert_eq!(object_storage["type"], "s3");
    assert_eq!(object_storage["compression"], "gzip");
    assert_eq!(object_storage["delimiter"], ",");
    assert_eq!(object_storage["authentication"], "IAM_ROLE");
    assert_eq!(object_storage["iamRole"], "arn:aws:iam::123:role/x");
    // No credential the user did not pass, and no other source key.
    assert!(
        object_storage.get("accessKey").is_none(),
        "accessKey leaked into object-storage schema-discovery body: {object_storage}",
    );
    for key in ["kafka", "kinesis", "pubsub"] {
        assert!(
            body["source"].get(key).is_none(),
            "{key} leaked into object-storage schema-discovery body: {}",
            body["source"],
        );
    }
}

// ── Google Cloud Pub/Sub source (issue #587) ───────────────────────────────
//
// `clickpipe create pubsub` and `clickpipe schema-discover <SERVICE_ID> pubsub`
// build the same `source.pubsub` object. The service account key is read from a
// file (or from stdin for `-`) and sent base64-encoded under
// `serviceAccountKey.serviceAccountFile`, so the path never goes on the wire —
// and neither the key nor its encoding may reach output or an error message.

const PUBSUB_SERVICE_ACCOUNT_KEY: &str =
    r#"{"type":"service_account","private_key":"FAKE_PRIVATE_KEY"}"#;

fn encoded_service_account_key() -> String {
    base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        PUBSUB_SERVICE_ACCOUNT_KEY.as_bytes(),
    )
}

fn write_service_account_key(dir: &Path) -> PathBuf {
    let path = dir.join("sa-key.json");
    std::fs::File::create(&path)
        .expect("create service account key file")
        .write_all(PUBSUB_SERVICE_ACCOUNT_KEY.as_bytes())
        .expect("write service account key file");
    path
}

/// Minimal `clickpipe create pubsub` invocation reading the key from
/// `service_account_file`.
fn pubsub_create_args(service_account_file: &str) -> Vec<String> {
    [
        "clickpipe",
        "create",
        "pubsub",
        "svc-id",
        "--name",
        "pubsub-pipe",
        "--topic",
        "events",
        "--project-id",
        "my-gcp-project",
        "--format",
        "JSONEachRow",
        "--seek-type",
        "earliest",
        "--service-account-file",
        service_account_file,
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "event_id:Int64",
        "--org-id",
        "org",
    ]
    .iter()
    .map(|arg| arg.to_string())
    .collect()
}

fn as_str_args(args: &[String]) -> Vec<&str> {
    args.iter().map(|arg| arg.as_str()).collect()
}

/// Spawn the binary with a piped stdin, write `stdin_data`, and return the
/// finished output. Used for `--service-account-file -`.
fn invoke_cli_with_piped_stdin(
    mock: &MockServer,
    cli_args: &[String],
    stdin_data: &str,
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let mut args = vec![
        "cloud".to_string(),
        "--url".to_string(),
        mock.uri(),
        "--json".to_string(),
    ];
    args.extend_from_slice(cli_args);
    let mut child = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin_data.as_bytes())
        .expect("write service account key to stdin");
    child
        .wait_with_output()
        .expect("failed to wait for clickhousectl")
}

/// The JSON body of the first POST the mock recorded.
async fn recorded_post_body(mock: &MockServer) -> Value {
    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let post = requests
        .iter()
        .find(|request| request.method == wiremock::http::Method::POST)
        .expect("no POST request recorded by mock");
    serde_json::from_slice(&post.body).expect("POST body wasn't valid JSON")
}

#[tokio::test]
async fn pubsub_create_posts_source_body_with_the_key_read_from_the_file() {
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let key_path = write_service_account_key(dir.path());
    let mut args = pubsub_create_args(key_path.to_str().expect("utf-8 temp path"));
    // Swap the minimal seek for the timestamp form and add every optional flag.
    let seek = args
        .iter()
        .position(|arg| arg == "earliest")
        .expect("baseline seek type");
    args[seek] = "timestamp".to_string();
    args.extend(
        [
            "--seek-timestamp",
            "2026-04-10T12:00:00Z",
            "--filter",
            r#"attributes.region = "eu""#,
            "--enable-ordering",
            "--ack-deadline",
            "120",
            "--role",
            "analytics_reader",
        ]
        .iter()
        .map(|arg| arg.to_string()),
    );

    let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;

    let pubsub = &body["source"]["pubsub"];
    assert_eq!(pubsub["topic"], "events");
    assert_eq!(pubsub["projectId"], "my-gcp-project");
    assert_eq!(pubsub["format"], "JSONEachRow");
    assert_eq!(pubsub["authentication"], "SERVICE_ACCOUNT");
    assert_eq!(pubsub["seekType"], "timestamp");
    assert_eq!(pubsub["seekTimestamp"], "2026-04-10T12:00:00Z");
    assert_eq!(pubsub["filter"], r#"attributes.region = "eu""#);
    assert_eq!(pubsub["enableOrdering"], true);
    assert_eq!(pubsub["ackDeadline"], 120);
    // The key came from the file, base64-encoded, and the path never went out.
    assert_eq!(
        pubsub["serviceAccountKey"]["serviceAccountFile"],
        Value::String(encoded_service_account_key()),
    );
    let serialized = body.to_string();
    assert!(
        !serialized.contains(key_path.to_str().expect("utf-8 temp path")),
        "the key file path leaked into the request body: {serialized}",
    );
    assert!(
        !serialized.contains("FAKE_PRIVATE_KEY"),
        "the raw key leaked into the request body unencoded: {serialized}",
    );
    // Destination and roles behave exactly as on the other create subcommands.
    assert_eq!(body["destination"]["database"], "default");
    assert_eq!(body["destination"]["table"], "events");
    assert_eq!(
        body["destination"]["roles"],
        serde_json::json!(["analytics_reader"]),
    );
    // No other source arm is populated.
    for key in ["kafka", "kinesis", "objectStorage", "postgres", "bigquery"] {
        assert!(
            body["source"].get(key).is_none(),
            "{key} leaked into the pubsub create body: {}",
            body["source"],
        );
    }
}

#[tokio::test]
async fn pubsub_create_posts_the_typed_destination_table_definition() {
    let mock = start_mock_clickpipes_api().await;
    let directory = tempfile::tempdir().unwrap();
    let key_path = write_service_account_key(directory.path());
    let definition_path = directory.path().join("table-definition.json");
    std::fs::write(
        &definition_path,
        serde_json::json!({
            "engine": {
                "columnIds": [],
                "type": "Null",
                "versionColumnId": null
            },
            "partitionBy": "tuple()",
            "primaryKey": "event_id",
            "sortingKey": ["event_id"]
        })
        .to_string(),
    )
    .unwrap();
    let mut args = pubsub_create_args(key_path.to_str().expect("utf-8 temp path"));
    args.extend([
        "--table-definition-file".into(),
        definition_path.to_string_lossy().into_owned(),
        "--managed-table".into(),
        "false".into(),
    ]);

    let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;

    assert_eq!(body["destination"]["managedTable"], false);
    assert_eq!(
        body["destination"]["tableDefinition"],
        serde_json::json!({
            "engine": {"type": "Null"},
            "partitionBy": "tuple()",
            "primaryKey": "event_id",
            "sortingKey": ["event_id"]
        })
    );
    assert_eq!(body["source"]["pubsub"]["topic"], "events");
}

#[tokio::test]
async fn pubsub_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let key_path = write_service_account_key(dir.path());
    let args = pubsub_create_args(key_path.to_str().expect("utf-8 temp path"));

    let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;

    let pubsub = &body["source"]["pubsub"];
    assert_eq!(pubsub["seekType"], "earliest");
    for field in ["seekTimestamp", "filter", "enableOrdering", "ackDeadline"] {
        assert!(
            pubsub.get(field).is_none(),
            "{field} leaked into the pubsub source body: {pubsub}",
        );
    }
    assert!(
        body["destination"].get("roles").is_none(),
        "roles leaked into the destination body when --role was omitted: {}",
        body["destination"],
    );
}

#[tokio::test]
async fn pubsub_service_account_key_can_be_read_from_stdin() {
    let mock = start_mock_clickpipes_api().await;
    let args = pubsub_create_args("-");

    let output = invoke_cli_with_piped_stdin(&mock, &args, PUBSUB_SERVICE_ACCOUNT_KEY);
    assert_success(&output);

    let body = recorded_post_body(&mock).await;
    assert_eq!(
        body["source"]["pubsub"]["serviceAccountKey"]["serviceAccountFile"],
        Value::String(encoded_service_account_key()),
    );
}

#[tokio::test]
async fn pubsub_service_account_key_never_reaches_output_on_an_api_error() {
    let mock = MockServer::start().await;
    // A rejected create: the API error body is surfaced to the user, so this is
    // where an echoed credential would show up.
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "pubsub source is not enabled for this organization",
            "status": 400,
            "requestId": "stub-request-id",
        })))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let key_path = write_service_account_key(dir.path());
    let args = pubsub_create_args(key_path.to_str().expect("utf-8 temp path"));

    let output = invoke_cli_with_cloud_credentials(&mock, &as_str_args(&args));

    assert!(
        !output.status.success(),
        "a rejected create must fail: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("not enabled for this organization"),
        "the API error should still be reported: {stderr}",
    );
    for secret in [
        PUBSUB_SERVICE_ACCOUNT_KEY,
        "FAKE_PRIVATE_KEY",
        &encoded_service_account_key(),
    ] {
        assert!(
            !stderr.contains(secret),
            "the service account key leaked into stderr: {stderr}",
        );
        assert!(
            !stdout.contains(secret),
            "the service account key leaked into stdout: {stdout}",
        );
    }
}

#[tokio::test]
async fn schema_discover_pubsub_posts_source_body() {
    let mock = start_mock_schema_discovery_api().await;
    let dir = tempfile::tempdir().unwrap();
    let key_path = write_service_account_key(dir.path());
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "svc-id",
            "--org-id",
            "org",
            "pubsub",
            "--topic",
            "events",
            "--project-id",
            "my-gcp-project",
            "--format",
            "Avro",
            "--seek-type",
            "latest",
            "--service-account-file",
            key_path.to_str().expect("utf-8 temp path"),
            "--ack-deadline",
            "30",
        ],
    )
    .await;

    let pubsub = &body["source"]["pubsub"];
    assert_eq!(pubsub["topic"], "events");
    assert_eq!(pubsub["projectId"], "my-gcp-project");
    assert_eq!(pubsub["format"], "Avro");
    assert_eq!(pubsub["authentication"], "SERVICE_ACCOUNT");
    assert_eq!(pubsub["seekType"], "latest");
    assert_eq!(pubsub["ackDeadline"], 30);
    assert_eq!(
        pubsub["serviceAccountKey"]["serviceAccountFile"],
        Value::String(encoded_service_account_key()),
    );
    for key in ["kafka", "kinesis", "objectStorage"] {
        assert!(
            body["source"].get(key).is_none(),
            "{key} leaked into pubsub schema-discovery body: {}",
            body["source"],
        );
    }
}

// ── ClickPipes GCP workload identity (issue #690) ──────────────────────────

fn assert_workload_identity_source(source: &Value) {
    assert_eq!(
        source["authentication"],
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    for field in [
        "iamRole",
        "accessKey",
        "connectionString",
        "serviceAccountKey",
    ] {
        assert!(source.get(field).is_none(), "{field} leaked into {source}");
    }
}

#[tokio::test]
async fn workload_identity_create_requests_cover_every_supported_gcp_source() {
    let cases: Vec<(&str, Vec<&str>)> = vec![
        (
            "kafka",
            vec![
                "clickpipe",
                "create",
                "kafka",
                "svc-id",
                "--name",
                "gcmk-pipe",
                "--brokers",
                "broker:9092",
                "--topics",
                "events",
                "--format",
                "JSONEachRow",
                "--kafka-type",
                "gcmk",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:Int64",
                "--org-id",
                "org",
            ],
        ),
        (
            "objectStorage",
            vec![
                "clickpipe",
                "create",
                "object-storage",
                "svc-id",
                "--name",
                "gcs-pipe",
                "--source-url",
                "gs://bucket/events/*.json",
                "--format",
                "JSONEachRow",
                "--storage-type",
                "gcs",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:Int64",
                "--org-id",
                "org",
            ],
        ),
        (
            "pubsub",
            vec![
                "clickpipe",
                "create",
                "pubsub",
                "svc-id",
                "--name",
                "pubsub-pipe",
                "--topic",
                "events",
                "--project-id",
                "project-1",
                "--format",
                "JSONEachRow",
                "--seek-type",
                "earliest",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:Int64",
                "--org-id",
                "org",
            ],
        ),
        (
            "bigquery",
            vec![
                "clickpipe",
                "create",
                "bigquery",
                "svc-id",
                "--name",
                "bigquery-pipe",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
                "--project-id",
                "project-1",
                "--staging-path",
                "gs://bucket/staging",
                "--table-mapping",
                "dataset.events:events",
                "--destination-database",
                "analytics",
                "--org-id",
                "org",
            ],
        ),
    ];

    for (source_name, args) in cases {
        let mock = start_mock_clickpipes_api().await;
        let body = invoke_cli_capture_body(&mock, &args).await;
        let source = &body["source"][source_name];
        assert_workload_identity_source(source);
        assert!(
            !body.to_string().contains("must-not-be-read"),
            "local credential material leaked into {body}"
        );
        if source_name == "kafka" {
            assert_eq!(source["type"], "gcmk");
            assert!(source["credentials"].is_null());
        }
        if source_name == "bigquery" {
            assert_eq!(source["projectId"], "project-1");
            assert_eq!(body["destination"]["database"], "analytics");
            assert!(source.get("credentials").is_none());
        }
    }
}

#[tokio::test]
async fn workload_identity_schema_discovery_covers_every_supported_arm() {
    let cases: Vec<(&str, Vec<&str>)> = vec![
        (
            "kafka",
            vec![
                "clickpipe",
                "schema-discover",
                "svc-id",
                "--org-id",
                "org",
                "kafka",
                "--brokers",
                "broker:9092",
                "--topics",
                "events",
                "--format",
                "JSONEachRow",
                "--kafka-type",
                "gcmk",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ],
        ),
        (
            "objectStorage",
            vec![
                "clickpipe",
                "schema-discover",
                "svc-id",
                "--org-id",
                "org",
                "object-storage",
                "--source-url",
                "gs://bucket/events/*.json",
                "--format",
                "JSONEachRow",
                "--storage-type",
                "gcs",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ],
        ),
        (
            "pubsub",
            vec![
                "clickpipe",
                "schema-discover",
                "svc-id",
                "--org-id",
                "org",
                "pubsub",
                "--topic",
                "events",
                "--project-id",
                "project-1",
                "--format",
                "JSONEachRow",
                "--seek-type",
                "earliest",
                "--auth",
                "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ],
        ),
    ];

    for (source_name, args) in cases {
        let mock = start_mock_schema_discovery_api().await;
        let body = invoke_cli_capture_body(&mock, &args).await;
        let source = &body["source"][source_name];
        assert_workload_identity_source(source);
        if source_name == "kafka" {
            assert!(source["credentials"].is_null());
        }
    }
}

#[tokio::test]
async fn contradictory_workload_identity_credentials_fail_before_http() {
    let mock = start_mock_clickpipes_api().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "gcs-pipe",
            "--source-url",
            "gs://bucket/events.json",
            "--format",
            "JSONEachRow",
            "--storage-type",
            "gcs",
            "--auth",
            "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            "--service-account-file",
            "/missing/must-not-be-read.json",
            "--database",
            "default",
            "--table",
            "events",
            "--org-id",
            "org",
        ],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot be combined"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        mock.received_requests()
            .await
            .expect("request log")
            .is_empty(),
        "invalid credentials reached HTTP"
    );
}

async fn mount_clickpipe_context(mock: &MockServer, result: Value) {
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/context",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-clickpipes-context",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_context_get_renders_full_and_sparse_responses() {
    let mock = MockServer::start().await;
    mount_clickpipe_context(
        &mock,
        serde_json::json!({
            "gcpWorkloadIdentity": {
                "supported": true,
                "ready": true,
                "principal": "clickpipes@project.iam.gserviceaccount.com",
            }
        }),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["clickpipe", "context", "get", "svc-id", "--org-id", "org"],
    );
    assert_success(&output);
    let output_json: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(
        output_json["gcpWorkloadIdentity"]["principal"],
        "clickpipes@project.iam.gserviceaccount.com"
    );

    let human = MockServer::start().await;
    mount_clickpipe_context(
        &human,
        serde_json::json!({
            "gcpWorkloadIdentity": {
                "supported": true,
                "ready": true,
                "principal": "clickpipes@project.iam.gserviceaccount.com",
            }
        }),
    )
    .await;
    let output = invoke_cli_human(
        &human,
        &["clickpipe", "context", "get", "svc-id", "--org-id", "org"],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "gcpWorkloadIdentity:\n  supported: true\n  ready: true\n  principal: clickpipes@project.iam.gserviceaccount.com\n"
    );

    let sparse = MockServer::start().await;
    mount_clickpipe_context(&sparse, serde_json::json!({})).await;
    let output = invoke_cli_human(
        &sparse,
        &["clickpipe", "context", "get", "svc-id", "--org-id", "org"],
    );
    assert_success(&output);
    assert!(output.stdout.is_empty());

    let not_ready = MockServer::start().await;
    mount_clickpipe_context(
        &not_ready,
        serde_json::json!({
            "gcpWorkloadIdentity": { "supported": true, "ready": false }
        }),
    )
    .await;
    let output = invoke_cli_human(
        &not_ready,
        &["clickpipe", "context", "get", "svc-id", "--org-id", "org"],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "gcpWorkloadIdentity:\n  supported: true\n  ready: false\n"
    );
}

#[tokio::test]
async fn clickpipe_context_get_accepts_oauth_bearer_auth() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/context",
        ))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "gcpWorkloadIdentity": { "supported": false } },
            "status": 200,
            "requestId": "stub-clickpipes-context",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .current_dir(dir.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "clickpipe",
            "context",
            "get",
            "svc-id",
            "--org-id",
            "org",
        ])
        .output()
        .expect("run OAuth context get");
    assert_success(&output);
}

// ── Generated service passwords are never silently dropped ─────────────────
//
// `service reset-password` without either hash flag sends an empty PATCH
// body, which asks the API to generate a password returned exactly once. A
// response without one loses that credential, so both output modes must fail
// instead of reporting a successful reset.

async fn run_service_reset_password(
    password_response: Value,
    extra_args: &[&str],
) -> std::process::Output {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/password$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": password_response,
            "status": 200,
            "requestId": "stub-reset-password",
        })))
        .mount(&mock)
        .await;

    let url = mock.uri();
    let mut args: Vec<&str> = vec![
        "cloud",
        "--url",
        &url,
        "service",
        "reset-password",
        "svc-id",
        "--org-id",
        "org-1",
    ];
    args.extend(extra_args);

    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(&args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn service_reset_password_fails_when_the_generated_password_is_absent() {
    for extra_args in [&[][..], &["--json"][..]] {
        let output = run_service_reset_password(serde_json::json!({}), extra_args).await;
        assert!(
            !output.status.success(),
            "a generation reset with no password must fail for args {extra_args:?}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("omitted the generated password"),
            "stderr should name the omitted password for args {extra_args:?}:\n{stderr}",
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("Password reset for service")
                && !stdout.contains("no plaintext password returned"),
            "no success output may precede the failure for args {extra_args:?}:\n{stdout}",
        );
    }
}

#[tokio::test]
async fn service_reset_password_succeeds_for_a_hash_reset_without_a_password() {
    let output =
        run_service_reset_password(serde_json::json!({}), &["--new-password-hash", "e3b0c442"])
            .await;
    assert_success(&output);
    // Agent detection can force --json here, so assert only what holds in
    // both output modes: the reset succeeds and shows no password.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("New password"),
        "a hash reset must not report a password:\n{stdout}",
    );
}

#[tokio::test]
async fn service_reset_password_treats_a_double_sha1_only_reset_as_generation() {
    // The API ignores `newDoubleSha1Hash` unless `newPasswordHash` is also
    // sent, and generates a password instead — so the mode is generation and a
    // response without a password loses the new credential.
    let output = run_service_reset_password(
        serde_json::json!({}),
        &["--new-double-sha1-hash", "aabbccdd"],
    )
    .await;
    assert!(
        !output.status.success(),
        "a double-SHA1-only reset with no password must fail\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("omitted the generated password"),
        "stderr should name the omitted password:\n{stderr}",
    );
}

#[tokio::test]
async fn service_reset_password_json_prints_the_generated_password() {
    let output =
        run_service_reset_password(serde_json::json!({ "password": "s3cret" }), &["--json"]).await;
    assert_success(&output);
    let body: Value =
        serde_json::from_slice(&output.stdout).expect("--json output wasn't valid JSON");
    assert_eq!(body["password"], "s3cret");
}

// ── Generated API key material is never silently dropped ───────────────────
//
// `key create` without the pre-hash flags asks the API to generate a key pair
// returned exactly once. A response missing `keyId`/`keySecret` loses that
// credential, so validation runs before either output branch — `--json` (which
// agent detection also turns on by itself) must not print the incomplete
// response and exit zero.

async fn run_key_create(result: Value, extra_args: &[&str]) -> std::process::Output {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-key-create",
        })))
        .mount(&mock)
        .await;

    let url = mock.uri();
    let mut args: Vec<&str> = vec![
        "cloud", "--url", &url, "key", "create", "--name", "ci", "--org-id", "org-1",
    ];
    args.extend(extra_args);

    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .args(&args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn key_create_fails_when_the_generated_material_is_absent() {
    for extra_args in [&[][..], &["--json"][..]] {
        let output = run_key_create(
            serde_json::json!({
                "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" },
                "keyId": "generated-key-id",
            }),
            extra_args,
        )
        .await;
        assert!(
            !output.status.success(),
            "a generated key create with no secret must fail for args {extra_args:?}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("omitted the generated key material"),
            "stderr should name the omitted material for args {extra_args:?}:\n{stderr}",
        );
        // Neither the human confirmation nor the incomplete response body may
        // reach stdout ahead of the failure.
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("API key created!") && !stdout.contains("generated-key-id"),
            "no success output may precede the failure for args {extra_args:?}:\n{stdout}",
        );
    }
}

#[tokio::test]
async fn key_create_json_prints_the_raw_response() {
    let result = serde_json::json!({
        "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" },
        "keyId": "generated-key-id",
        "keySecret": "generated-key-secret",
    });
    let output = run_key_create(result.clone(), &["--json"]).await;
    assert_success(&output);
    let body: Value =
        serde_json::from_slice(&output.stdout).expect("--json output wasn't valid JSON");
    // --json reflects the key set the API sent; nothing is synthesized from
    // the resolved material.
    assert_eq!(body, result);
}

#[tokio::test]
async fn key_create_succeeds_for_a_pre_hashed_key_without_generated_material() {
    let output = run_key_create(
        serde_json::json!({ "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" } }),
        &[
            "--hash-key-id",
            "0f1e2d3c",
            "--hash-key-id-suffix",
            "3c",
            "--hash-key-secret",
            "4b5a6978",
        ],
    )
    .await;
    assert_success(&output);
    // Agent detection can force --json here, so assert only what holds in
    // both output modes: no key material is reported.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Key Secret"),
        "a pre-hashed create must not report key material:\n{stdout}",
    );
}

// ── `service update --remove-*` warns on unmatched entries (issue #612) ────

/// Mount a `GET` returning the given `ipAccessList`/`privateEndpointIds`/`tags`
/// snapshot and a `PATCH` that echoes it back unchanged, both scoped to
/// `svc-1` in `org-1`.
async fn mount_service_update_round_trip(mock: &MockServer, current: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": current,
            "status": 200,
            "requestId": "stub-service-get",
        })))
        .expect(1)
        .mount(mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": current,
            "status": 200,
            "requestId": "stub-service-update",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn service_update_warns_when_remove_ip_allow_matches_nothing() {
    let mock = MockServer::start().await;
    mount_service_update_round_trip(
        &mock,
        serde_json::json!({
            "id": "22222222-3333-4444-5555-666666666666",
            "name": "demo",
            "ipAccessList": [{ "source": "10.0.0.0/8" }],
        }),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--remove-ip-allow",
            "10.99.99.99/32",
        ],
    );

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Warning: --remove-ip-allow 10.99.99.99/32 did not match any entry in the service's \
         current IP allow list; no entry was removed\n"
    );
}

#[tokio::test]
async fn service_update_does_not_warn_when_remove_ip_allow_matches() {
    let mock = MockServer::start().await;
    mount_service_update_round_trip(
        &mock,
        serde_json::json!({
            "id": "22222222-3333-4444-5555-666666666666",
            "name": "demo",
            "ipAccessList": [
                { "source": "10.0.0.0/8" },
                { "source": "2001:db8::/32" },
            ],
        }),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--remove-ip-allow",
            " 10.0.0.0/8 =retired office",
            "--remove-ip-allow",
            " 2001:db8::/32 =retired ipv6 range",
        ],
    );

    assert_success(&output);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let requests = mock.received_requests().await.unwrap();
    let patch = requests
        .iter()
        .find(|request| request.method.as_str() == "PATCH")
        .unwrap();
    let body: Value = serde_json::from_slice(&patch.body).unwrap();
    assert_eq!(
        body["ipAccessList"]["remove"],
        serde_json::json!([
            { "source": "10.0.0.0/8", "description": "retired office" },
            { "source": "2001:db8::/32", "description": "retired ipv6 range" },
        ])
    );
}

#[tokio::test]
async fn service_update_warns_on_unmatched_private_endpoint_and_tag_removal() {
    let mock = MockServer::start().await;
    mount_service_update_round_trip(
        &mock,
        serde_json::json!({
            "id": "22222222-3333-4444-5555-666666666666",
            "name": "demo",
            "privateEndpointIds": ["pe-1"],
            "tags": [{ "key": "env", "value": "prod" }],
        }),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--remove-private-endpoint-id",
            "pe-missing",
            "--remove-tag",
            "missing",
        ],
    );

    assert_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        "Warning: --remove-private-endpoint-id pe-missing did not match any private endpoint \
         on the service; nothing was removed\n\
         Warning: --remove-tag missing did not match any tag on the service; nothing was \
         removed\n"
    );
}

#[tokio::test]
async fn service_update_skips_get_when_no_removals_requested() {
    let mock = MockServer::start().await;
    // No removal flags: the handler must not issue the pre-update GET at all,
    // since idempotent adds/renames never risk a silent no-op.
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "22222222-3333-4444-5555-666666666666", "name": "renamed" },
            "status": 200,
            "requestId": "stub-service-update",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service", "update", "svc-1", "--org-id", "org-1", "--name", "renamed",
        ],
    );

    assert_success(&output);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── Private endpoint ID format validation (issue #611) ─────────────────────

#[tokio::test]
async fn private_endpoint_create_sends_well_formed_endpoint_id() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/privateEndpoint",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "vpce-0123456789abcdef0", "description": "prod" },
            "status": 200,
            "requestId": "stub-private-endpoint-create",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "private-endpoint",
            "create",
            "svc-1",
            "--org-id",
            "org-1",
            "--endpoint-id",
            "vpce-0123456789abcdef0",
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["id"], "vpce-0123456789abcdef0");
}

/// A malformed ID must fail as a clap usage error (exit 2) before any request
/// is sent: registering one is org-wide and has to be unpicked by hand.
#[tokio::test]
async fn private_endpoint_create_rejects_malformed_endpoint_id_without_calling_api() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/privateEndpoint",
        ))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "private-endpoint",
            "create",
            "svc-1",
            "--org-id",
            "org-1",
            "--endpoint-id",
            "vpce-bogus",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid AWS VPC endpoint ID 'vpce-bogus'"),
        "stderr should explain the format: {stderr}"
    );
    assert!(
        mock.received_requests().await.unwrap().is_empty(),
        "a rejected endpoint ID must not reach the API"
    );
}

#[tokio::test]
async fn service_update_rejects_malformed_added_private_endpoint_id_without_calling_api() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--add-private-endpoint-id",
            "vpce-bogus",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// GCP (numeric PSC connection ID) and Azure (Resource ID) formats are not
/// AWS-shaped and must still be forwarded verbatim.
#[tokio::test]
async fn private_endpoint_create_forwards_non_aws_endpoint_ids() {
    for endpoint_id in [
        "102600141743718403",
        "/subscriptions/11111111-2222-3333-4444-555555555555/resourceGroups/rg/providers/Microsoft.Network/privateEndpoints/pe-demo",
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/privateEndpoint",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": endpoint_id, "description": "" },
                "status": 200,
                "requestId": "stub-private-endpoint-create",
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "service",
                "private-endpoint",
                "create",
                "svc-1",
                "--org-id",
                "org-1",
                "--endpoint-id",
                endpoint_id,
            ],
        );

        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["id"], endpoint_id);
    }
}

// ── `clickpipe scale` requires at least one target (issue #605) ────────────
//
// Without --replicas/--cpu-millicores/--memory-gb the CLI used to send an
// empty PATCH body, which the API 400s on and the CLI surfaced as a generic
// "Internal error" (exit 1). It must now be a clap usage error (exit 2)
// raised before any request is sent.

#[tokio::test]
async fn clickpipe_scale_without_any_flag_is_a_usage_error() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes/[^/]+/scaling$",
        ))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["clickpipe", "scale", "svc-1", "pipe-1", "--org-id", "org-1"],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("replicas")
            || stderr.contains("cpu-millicores")
            || stderr.contains("memory-gb"),
        "stderr should name the scale flags: {stderr}"
    );
    assert!(
        mock.received_requests().await.unwrap().is_empty(),
        "a rejected scale command must not reach the API"
    );
}

#[tokio::test]
async fn clickpipe_scale_with_a_single_flag_sends_the_request() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes/[^/]+/scaling$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "pipe-1",
                "scaling": { "replicas": 4 },
            },
            "status": 200,
            "requestId": "stub-clickpipe-scale",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "scale",
            "svc-1",
            "pipe-1",
            "--org-id",
            "org-1",
            "--replicas",
            "4",
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["replicas"], 4);
}

// ── service-wide ClickPipes CDC scaling (issue #586) ───────────────────────

const CDC_SCALING_PATH: &str = "/v1/organizations/org-1/services/svc-1/clickpipesCdcScaling";

fn cdc_scaling_envelope(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-cdc-scaling",
    }))
}

#[tokio::test]
async fn clickpipe_cdc_scaling_get_supports_oauth_and_preserves_sparse_json() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CDC_SCALING_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(cdc_scaling_envelope(serde_json::json!({
            "replicaCpuMillicores": 2000,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "clickpipe",
            "cdc-scaling",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .expect("failed to run CDC scaling get with OAuth");

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "replicaCpuMillicores": 2000 })
    );
}

#[tokio::test]
async fn clickpipe_cdc_scaling_get_prints_sparse_human_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CDC_SCALING_PATH))
        .respond_with(cdc_scaling_envelope(serde_json::json!({
            "replicaMemoryGb": 8,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "clickpipe",
            "cdc-scaling",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("replicaMemoryGb: 8"), "{stdout}");
    assert!(!stdout.contains("replicaCpuMillicores"), "{stdout}");
}

#[tokio::test]
async fn clickpipe_cdc_scaling_update_preserves_omitted_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(CDC_SCALING_PATH))
        .and(body_json(serde_json::json!({
            "replicaCpuMillicores": 1000,
        })))
        .respond_with(cdc_scaling_envelope(serde_json::json!({
            "replicaCpuMillicores": 1000,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "cdc-scaling",
            "update",
            "svc-1",
            "--cpu-millicores",
            "1000",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "replicaCpuMillicores": 1000 })
    );
    let requests = mock.received_requests().await.unwrap();
    let authorization = requests[0]
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(authorization.starts_with("Basic "), "{authorization}");
}

#[tokio::test]
async fn clickpipe_cdc_scaling_update_sends_maximum_allocation() {
    let mock = MockServer::start().await;
    let scaling = serde_json::json!({
        "replicaCpuMillicores": 32000,
        "replicaMemoryGb": 128.0,
    });
    Mock::given(method("PATCH"))
        .and(path(CDC_SCALING_PATH))
        .and(body_json(scaling.clone()))
        .respond_with(cdc_scaling_envelope(scaling.clone()))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "cdc-scaling",
            "update",
            "svc-1",
            "--cpu-millicores",
            "32000",
            "--memory-gb",
            "128",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        scaling
    );
}

#[tokio::test]
async fn clickpipe_cdc_scaling_update_rejects_oauth_before_http() {
    let mock = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "clickpipe",
            "cdc-scaling",
            "update",
            "svc-1",
            "--memory-gb",
            "4",
            "--org-id",
            "org-1",
        ])
        .output()
        .expect("failed to run CDC scaling update with OAuth");

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn clickpipe_cdc_scaling_get_scopes_not_found_errors_to_the_organization() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CDC_SCALING_PATH))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "NOT_FOUND",
            "requestId": "stub-cdc-scaling-error",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "cdc-scaling",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: NOT_FOUND: request scoped to organization org-1\n"
    );
}

// ── the Query API gateway timeout (issue #644) ─────────────────────────────
//
// The gateway stops waiting after roughly 30 seconds and answers HTTP 500
// with `{"error":"Timeout error."}`. The statement keeps running on the
// service, so the CLI must (a) say so, (b) point at `system.processes` before
// a rerun, (c) hand over the native-protocol command built from the service's
// own `nativesecure` endpoint, and (d) never resend the statement itself.

/// The gateway's timeout body, verbatim.
const QUERY_GATEWAY_TIMEOUT_BODY: &str = r#"{"error":"Timeout error."}"#;

const QUERY_TEST_NATIVE_HOST: &str = "demo.gcp.clickhouse.cloud";

/// A control plane whose `GET service` response carries both endpoints, the
/// shape a real service has.
async fn start_mock_control_plane_with_native_endpoint() -> MockServer {
    let mock = MockServer::start().await;
    let stub_service = serde_json::json!({
        "result": {
            "id": QUERY_TEST_SERVICE_ID,
            "name": "demo",
            "endpoints": [
                { "protocol": "https", "host": QUERY_TEST_NATIVE_HOST, "port": 8443 },
                {
                    "protocol": "nativesecure",
                    "host": QUERY_TEST_NATIVE_HOST,
                    "port": 9440,
                    "username": "default",
                },
            ],
        },
        "status": 200,
        "requestId": "stub-service-get",
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_service))
        .mount(&mock)
        .await;
    mock
}

/// Run `cloud service query` with API-key auth against a query host that
/// fails with `status`/`body`. Returns the process output and the query host,
/// so a test can count how many times the statement was actually sent.
async fn invoke_service_query_against_failing_query_host(
    control: &MockServer,
    status: u16,
    body: &str,
    extra_args: &[&str],
) -> (std::process::Output, MockServer) {
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("home")).unwrap();
    let mut command = service_query_process(dir.path(), control, &query_host);
    command.args(extra_args);
    let output = command
        .output()
        .await
        .expect("failed to spawn clickhousectl");

    (output, query_host)
}

#[tokio::test]
async fn query_gateway_timeout_hints_the_native_client_without_retrying() {
    let control = start_mock_control_plane_with_native_endpoint().await;
    let (output, query_host) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        QUERY_GATEWAY_TIMEOUT_BODY,
        &[],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stops waiting after about 30 seconds"),
        "{stderr}"
    );
    assert!(
        stderr.contains("The statement may still be running on the service"),
        "{stderr}"
    );
    assert!(
        stderr.contains("SELECT query_id, elapsed FROM system.processes"),
        "{stderr}"
    );
    assert!(
        stderr.contains("clickhousectl local use latest"),
        "{stderr}"
    );
    // The host and port come from the mocked service response, not from a
    // placeholder or a guess.
    assert!(
        stderr.contains(&format!(
            "clickhouse client --host {QUERY_TEST_NATIVE_HOST} --secure --port 9440 --user \
             default --password '<password>' --query '<your SQL>'"
        )),
        "{stderr}"
    );
    // The user's SQL is never echoed back, and no credential appears.
    assert!(!stderr.contains("SELECT 1"), "{stderr}");
    assert!(!stderr.contains("fake-secret-for-tests"), "{stderr}");
    // The anonymous 500 the old code surfaced is gone.
    assert!(
        !stderr.contains("Internal Server Error"),
        "the gateway timeout must not be reported as a bare 500: {stderr}"
    );
    assert!(output.stdout.is_empty(), "{:?}", output.stdout);

    // Exactly one attempt: a statement that may still be running is never
    // resent.
    assert_eq!(
        query_host.received_requests().await.unwrap().len(),
        1,
        "the gateway timeout must not be retried"
    );
}

#[tokio::test]
async fn query_gateway_timeout_without_endpoints_points_at_service_get() {
    // The default stub service carries no `endpoints` at all.
    let control = start_mock_control_plane_with_service().await;
    let (output, query_host) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        QUERY_GATEWAY_TIMEOUT_BODY,
        &[],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("clickhouse client --host <host> --secure --port 9440"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "clickhousectl cloud service get 11111111-2222-3333-4444-555555555555 --org-id org-1"
        ),
        "{stderr}"
    );
    assert_eq!(query_host.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn query_gateway_timeout_json_emits_a_structured_error() {
    let control = start_mock_control_plane_with_native_endpoint().await;
    let (output, _query_host) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        QUERY_GATEWAY_TIMEOUT_BODY,
        &["--json"],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "{:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|e| panic!("stderr is not one JSON object ({e}): {stderr}"));
    assert_eq!(error["error"]["code"], "query_timeout");
    assert_eq!(error["error"]["host"], QUERY_TEST_NATIVE_HOST);
    assert_eq!(error["error"]["port"], 9440);
    assert_eq!(
        error["error"]["command"],
        format!(
            "clickhouse client --host {QUERY_TEST_NATIVE_HOST} --secure --port 9440 --user \
             default --password '<password>' --query '<your SQL>'"
        )
    );
    let message = error["error"]["message"]
        .as_str()
        .expect("message is a string");
    assert!(message.contains("about 30 seconds"), "{message}");
    assert!(
        message.contains("SELECT query_id, elapsed FROM system.processes"),
        "{message}"
    );
}

#[tokio::test]
async fn query_gateway_timeout_json_omits_an_absent_endpoint() {
    let control = start_mock_control_plane_with_service().await;
    let (output, _query_host) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        QUERY_GATEWAY_TIMEOUT_BODY,
        &["--json"],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|e| panic!("stderr is not one JSON object ({e}): {stderr}"));
    assert_eq!(error["error"]["code"], "query_timeout");
    // Absent means omitted, never a fabricated host or a `null`.
    assert!(error["error"].get("host").is_none(), "{stderr}");
    assert!(error["error"].get("port").is_none(), "{stderr}");
    assert!(
        error["error"]["command"]
            .as_str()
            .is_some_and(|command| command.contains("--host <host>")),
        "{stderr}"
    );
}

/// The control case: a 500 that is *not* the gateway timeout keeps the
/// generic API error, in both output modes.
#[tokio::test]
async fn other_query_500s_keep_the_generic_api_error() {
    let control = start_mock_control_plane_with_native_endpoint().await;
    let (output, query_host) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        r#"{"error":"Internal error."}"#,
        &[],
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            r#"Query API returned HTTP 500 Internal Server Error: {"error":"Internal error."}"#
        ),
        "{stderr}"
    );
    assert!(!stderr.contains("30 seconds"), "{stderr}");
    assert!(!stderr.contains("clickhouse client"), "{stderr}");
    assert_eq!(query_host.received_requests().await.unwrap().len(), 1);

    // In JSON mode it stays prose too: no structured detail was resolved for
    // it, so no code is invented.
    let (json_output, _) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        r#"{"error":"Internal error."}"#,
        &["--json"],
    )
    .await;
    let stderr = String::from_utf8_lossy(&json_output.stderr);
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(
        serde_json::from_str::<Value>(stderr.trim()).is_err(),
        "{stderr}"
    );
}

// ── `clickpipe reverse-private-endpoint` CRUD (issue #567) ─────────────────
//
// PrivateLink connectivity for ClickPipes is a reverse private endpoint the
// user creates on the service and then references from a pipe: by ID for
// Kafka, or by DNS name as `--host` for Postgres/MySQL CDC. These tests pin
// the five requests, the request bodies (including which fields stay off the
// wire), and the client-side type/flag validation that must cost no request.

const RPE_COLLECTION_PATH: &str =
    "/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints";
const RPE_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

fn reverse_private_endpoint_path() -> String {
    format!("{RPE_COLLECTION_PATH}/{RPE_ID}")
}

/// One endpoint as the API returns it, with the fields the CLI renders.
fn reverse_private_endpoint_json() -> serde_json::Value {
    serde_json::json!({
        "id": RPE_ID,
        "serviceId": "11111111-2222-3333-4444-555555555555",
        "type": "VPC_ENDPOINT_SERVICE",
        "description": "warehouse",
        "status": "PendingAcceptance",
        "endpointId": "vpce-12345678901234567",
        "dnsNames": ["vpce-1-abc.vpce.amazonaws.com"],
        "vpcEndpointServiceName": "com.amazonaws.vpce.us-east-1.vpce-svc-1",
    })
}

fn reverse_private_endpoint_envelope(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-reverse-private-endpoint",
    }))
}

/// Run the binary with credentials but *without* `--json`, so human output can
/// be asserted. `invoke_cli_with_cloud_credentials` always adds `--json`.
fn invoke_cli_human(mock: &MockServer, cli_args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url];
    args.extend(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    // Agent detection turns on `--json` on its own, so the inherited
    // environment has to go for human output to be observable at all.
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args);
    command.output().expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn reverse_private_endpoint_list_returns_the_resource_array() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(serde_json::json!([
            reverse_private_endpoint_json()
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource array as JSON");
    assert_eq!(stdout[0]["id"], RPE_ID);
    assert_eq!(stdout[0]["status"], "PendingAcceptance");
    assert!(
        stdout.get("status").is_none() && stdout.get("requestId").is_none(),
        "must not emit the raw API envelope, got: {stdout}"
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), RPE_COLLECTION_PATH.to_string())]
    );
}

/// Every response field is `Option`, so a row with nothing but an ID must
/// render placeholders rather than fabricate values.
#[tokio::test]
async fn reverse_private_endpoint_list_table_renders_absent_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(serde_json::json!([
            { "id": RPE_ID }
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_human(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| ID"), "{stdout}");
    assert!(stdout.contains("DNS Names"), "{stdout}");
    assert!(stdout.contains(RPE_ID), "{stdout}");
    // Type, Description, Status and DNS Names are all absent here.
    assert_eq!(stdout.matches(" - ").count(), 4, "{stdout}");
}

#[tokio::test]
async fn reverse_private_endpoint_list_reports_an_empty_collection() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(serde_json::json!([])))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_human(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "No reverse private endpoints found"
    );
}

#[tokio::test]
async fn reverse_private_endpoint_get_reads_the_single_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(reverse_private_endpoint_path()))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "get",
            "svc-1",
            RPE_ID,
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource object as JSON");
    assert_eq!(stdout["endpointId"], "vpce-12345678901234567");
    assert_eq!(stdout["dnsNames"][0], "vpce-1-abc.vpce.amazonaws.com");
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), reverse_private_endpoint_path())]
    );
}

/// One create per endpoint type: the POST body carries the type's own fields
/// and nothing else, so an omitted flag never reaches the wire as `""`.
#[tokio::test]
async fn reverse_private_endpoint_create_sends_the_body_for_each_type() {
    let cases: Vec<(Vec<&str>, serde_json::Value)> = vec![
        (
            vec![
                "--type",
                "VPC_ENDPOINT_SERVICE",
                "--vpc-endpoint-service-name",
                "com.amazonaws.vpce.us-east-1.vpce-svc-1",
            ],
            serde_json::json!({
                "description": "warehouse",
                "type": "VPC_ENDPOINT_SERVICE",
                "vpcEndpointServiceName": "com.amazonaws.vpce.us-east-1.vpce-svc-1",
            }),
        ),
        (
            vec![
                "--type",
                "VPC_RESOURCE",
                "--vpc-resource-configuration-id",
                "rcfg-12345678901234567",
                "--vpc-resource-share-arn",
                "arn:aws:ram:us-east-1:123456789012:resource-share/share-1",
            ],
            serde_json::json!({
                "description": "warehouse",
                "type": "VPC_RESOURCE",
                "vpcResourceConfigurationId": "rcfg-12345678901234567",
                "vpcResourceShareArn":
                    "arn:aws:ram:us-east-1:123456789012:resource-share/share-1",
            }),
        ),
        (
            vec![
                "--type",
                "MSK_MULTI_VPC",
                "--msk-cluster-arn",
                "arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster",
                "--msk-authentication",
                "SASL_IAM",
            ],
            serde_json::json!({
                "description": "warehouse",
                "type": "MSK_MULTI_VPC",
                "mskClusterArn": "arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster",
                "mskAuthentication": "SASL_IAM",
            }),
        ),
        (
            vec![
                "--type",
                "GCP_PSC_SERVICE_ATTACHMENT",
                "--gcp-service-attachment",
                "projects/p/regions/us-central1/serviceAttachments/s",
                "--custom-private-dns-mapping",
                "db.example.com",
                "--custom-private-dns-mapping",
                "*.example.com",
            ],
            serde_json::json!({
                "description": "warehouse",
                "type": "GCP_PSC_SERVICE_ATTACHMENT",
                "gcpServiceAttachment": "projects/p/regions/us-central1/serviceAttachments/s",
                "customPrivateDnsMappings": [
                    { "privateDnsName": "db.example.com" },
                    { "privateDnsName": "*.example.com" },
                ],
            }),
        ),
    ];

    for (flags, expected_body) in cases {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(RPE_COLLECTION_PATH))
            .respond_with(reverse_private_endpoint_envelope(
                reverse_private_endpoint_json(),
            ))
            .expect(1)
            .mount(&mock)
            .await;

        let mut args = vec![
            "clickpipe",
            "reverse-private-endpoint",
            "create",
            "svc-1",
            "--description",
            "warehouse",
            "--org-id",
            "org-1",
        ];
        args.extend_from_slice(&flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(
            received_request_shape(&mock).await,
            vec![("POST".to_string(), RPE_COLLECTION_PATH.to_string())],
            "unexpected requests for {flags:?}"
        );
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body, expected_body, "unexpected body for {flags:?}");
    }
}

#[tokio::test]
async fn reverse_private_endpoint_create_prints_the_status_to_wait_for() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_human(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "create",
            "svc-1",
            "--description",
            "warehouse",
            "--type",
            "VPC_ENDPOINT_SERVICE",
            "--vpc-endpoint-service-name",
            "com.amazonaws.vpce.us-east-1.vpce-svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Reverse private endpoint created"),
        "{stdout}"
    );
    assert!(stdout.contains(RPE_ID), "{stdout}");
    assert!(stdout.contains("Status: PendingAcceptance"), "{stdout}");
    assert!(
        stdout.contains("DNS names: vpce-1-abc.vpce.amazonaws.com"),
        "{stdout}"
    );
}

/// The flags a type needs, and the flags belonging to another type, are
/// checked before any network call: a bad combination is a usage error
/// (exit 2) with no request sent.
#[tokio::test]
async fn reverse_private_endpoint_create_validates_flags_before_any_request() {
    let cases: [(Vec<&str>, &str); 3] = [
        (
            vec![
                "--type",
                "VPC_RESOURCE",
                "--vpc-resource-configuration-id",
                "rcfg-1",
            ],
            "--type VPC_RESOURCE requires --vpc-resource-share-arn",
        ),
        (
            vec![
                "--type",
                "VPC_ENDPOINT_SERVICE",
                "--vpc-endpoint-service-name",
                "com.amazonaws.vpce.us-east-1.vpce-svc-1",
                "--msk-cluster-arn",
                "arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster",
            ],
            "--msk-cluster-arn applies to --type MSK_MULTI_VPC, not --type VPC_ENDPOINT_SERVICE",
        ),
        (
            vec![
                "--type",
                "MSK_MULTI_VPC",
                "--msk-cluster-arn",
                "arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster",
                "--msk-authentication",
                "SASL_IAM",
                "--custom-private-dns-mapping",
                "db.example.com",
            ],
            "--custom-private-dns-mapping is not supported for --type MSK_MULTI_VPC",
        ),
    ];

    for (flags, expected_message) in cases {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(RPE_COLLECTION_PATH))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&mock)
            .await;

        // No --org-id either: a rejected invocation must not even look one up.
        let mut args = vec![
            "clickpipe",
            "reverse-private-endpoint",
            "create",
            "svc-1",
            "--description",
            "warehouse",
        ];
        args.extend_from_slice(&flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_eq!(output.status.code(), Some(2), "expected a usage error");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected_message), "{stderr}");
        assert!(
            mock.received_requests().await.unwrap().is_empty(),
            "a rejected create must not reach the API"
        );
    }
}

#[tokio::test]
async fn reverse_private_endpoint_update_patches_the_full_mapping_list() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(reverse_private_endpoint_path()))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "update",
            "svc-1",
            RPE_ID,
            "--custom-private-dns-mapping",
            "db.example.com",
            "--custom-private-dns-mapping",
            "*.example.com",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), reverse_private_endpoint_path())]
    );
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "customPrivateDnsMappings": [
                { "privateDnsName": "db.example.com" },
                { "privateDnsName": "*.example.com" },
            ],
        })
    );
}

#[tokio::test]
async fn reverse_private_endpoint_delete_confirms_the_removal() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(reverse_private_endpoint_path()))
        .respond_with(successful_delete_response("stub-rpe-delete"))
        .expect(2)
        .mount(&mock)
        .await;

    let args = [
        "clickpipe",
        "reverse-private-endpoint",
        "delete",
        "svc-1",
        RPE_ID,
        "--org-id",
        "org-1",
    ];

    let human = invoke_cli_human(&mock, &args);
    assert_success(&human);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout).trim(),
        format!("Reverse private endpoint {RPE_ID} deleted")
    );

    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&json.stdout).trim())
        .expect("stdout should be JSON");
    assert_eq!(stdout, serde_json::json!({ "deleted": RPE_ID }));

    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("DELETE".to_string(), reverse_private_endpoint_path()),
            ("DELETE".to_string(), reverse_private_endpoint_path()),
        ]
    );
}

// ── PEM certificates are elided from human output (issue #665) ─────────────
//
// A Postgres ClickPipe's `source.postgres.caCertificate` is a whole PEM block.
// Rendered verbatim it pushed every other field of `clickpipe get` off the
// screen. Human output replaces each block with a one-line summary of that
// block; `--json` still carries the bytes, so a caller that needs the
// certificate has not lost anything.

/// A real self-signed EC certificate, the same fixture the `cloud::output`
/// unit tests use. The fingerprint came from
/// `openssl x509 -noout -fingerprint -sha256`.
const CA_CERTIFICATE_PEM: &str = "\
-----BEGIN CERTIFICATE-----
MIIBjDCCATOgAwIBAgIUajES1wl65zexYYPuWX8ShldYw4YwCgYIKoZIzj0EAwIw
HDEaMBgGA1UEAwwRY2hjdGwtcGVtLWZpeHR1cmUwHhcNMjYwOTAyMTc1NDI3WhcN
NDYwODI4MTc1NDI3WjAcMRowGAYDVQQDDBFjaGN0bC1wZW0tZml4dHVyZTBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABNTPygUG2umVvTqod5jJXCgp1o9qwrx2wLf7
p+2PyHYm5ZdIS+kqT25Xm2SGM3th4dB43l3fd5kF0g6CzvGNt42jUzBRMB0GA1Ud
DgQWBBQcL9JNezOJ8vzT0lR1Pj4sMoH2STAfBgNVHSMEGDAWgBQcL9JNezOJ8vzT
0lR1Pj4sMoH2STAPBgNVHRMBAf8EBTADAQH/MAoGCCqGSM49BAMCA0cAMEQCIAhv
iLjfMqcnJ10gmKoyEIMDDRJP2UwtGRcJZU/FnaIEAiBeUmN+nJBGIq0tFHIxz1Xl
LaBMf6qZANMrXRQaETxhIA==
-----END CERTIFICATE-----
";

const CA_CERTIFICATE_SUMMARY: &str = "caCertificate: <PEM CERTIFICATE, SHA-256 fingerprint 5A:6D:67:FD:14:1B:1E:61:4A:F4:E2:7D:F1:F8:67:E2:75:85:DF:92:E3:66:31:85:75:AB:2C:C3:F4:8C:9A:D8>";

fn postgres_source_with_certificate() -> Value {
    serde_json::json!({
        "postgres": {
            "host": "db.example.com",
            "port": 5432,
            "database": "postgres",
            "caCertificate": CA_CERTIFICATE_PEM,
        }
    })
}

#[tokio::test]
async fn clickpipe_get_human_output_summarizes_the_ca_certificate() {
    let mock = MockServer::start().await;
    mount_clickpipe_get(&mock, postgres_source_with_certificate()).await;

    let output = invoke_cli_human(
        &mock,
        &["clickpipe", "get", "svc-id", "pipe-id", "--org-id", "org"],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(CA_CERTIFICATE_SUMMARY),
        "certificate not summarized:\n{stdout}"
    );
    assert!(
        !stdout.contains("-----BEGIN"),
        "certificate body dumped:\n{stdout}"
    );
    // The point of eliding is that the surrounding fields stay readable.
    assert!(stdout.contains("host: db.example.com"), "{stdout}");
    assert!(stdout.contains("name: test-pipe"), "{stdout}");
}

#[tokio::test]
async fn clickpipe_get_json_output_keeps_the_full_ca_certificate() {
    let mock = MockServer::start().await;
    mount_clickpipe_get(&mock, postgres_source_with_certificate()).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["clickpipe", "get", "svc-id", "pipe-id", "--org-id", "org"],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pipe: Value = serde_json::from_str(stdout.trim()).expect("stdout should be JSON");
    assert_eq!(
        pipe["source"]["postgres"]["caCertificate"],
        Value::String(CA_CERTIFICATE_PEM.to_string()),
        "--json must carry the certificate verbatim: {stdout}"
    );
    assert!(
        stdout.contains("-----BEGIN CERTIFICATE-----"),
        "--json must not elide: {stdout}"
    );
}

// ── A well-formed but unknown id reads as not found (issue #666) ───────────
//
// The API answers HTTP 400 `Invalid <thing> id string:"<id>"` for a
// syntactically valid UUID that resolves to nothing, so a by-id read reported
// a bad request instead of a missing resource. The refinement is structural
// (the status, plus whether the identifiers the CLI put in the path parse as
// UUIDs), so a malformed id must still get the server's own answer.

/// A well-formed UUID that no resource has. All-zero is still a syntactically
/// valid UUID.
const UNKNOWN_UUID: &str = "00000000-0000-0000-0000-000000000000";
const LOOKUP_ORG_ID: &str = "00000000-0000-4000-8000-000000000001";

/// The 400 the API really answers for a well-formed id it cannot resolve.
fn invalid_id_400_response(thing: &str, id: &str) -> ResponseTemplate {
    ResponseTemplate::new(400).set_body_json(serde_json::json!({
        "status": 400,
        "error": format!("BAD_REQUEST: Invalid {thing} id string:\"{id}\""),
        "requestId": "stub-invalid-id",
    }))
}

async fn mount_invalid_id_400(mock: &MockServer, api_path: String, thing: &str, id: &str) {
    Mock::given(method("GET"))
        .and(path(api_path))
        .respond_with(invalid_id_400_response(thing, id))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn postgres_get_unknown_well_formed_id_reads_as_not_found() {
    let mock = MockServer::start().await;
    mount_invalid_id_400(
        &mock,
        format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{UNKNOWN_UUID}"),
        "Postgres service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_human(
        &mock,
        &["postgres", "get", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: No such Postgres service: {UNKNOWN_UUID} (organization {LOOKUP_ORG_ID}). \
             The API rejected the identifier: \
             BAD_REQUEST: Invalid Postgres service id string:\"{UNKNOWN_UUID}\"\n"
        )
    );
}

#[tokio::test]
async fn service_get_unknown_well_formed_id_reads_as_not_found() {
    let mock = MockServer::start().await;
    mount_invalid_id_400(
        &mock,
        format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}"),
        "service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_human(
        &mock,
        &["service", "get", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: No such service: {UNKNOWN_UUID} (organization {LOOKUP_ORG_ID}). \
             The API rejected the identifier: \
             BAD_REQUEST: Invalid service id string:\"{UNKNOWN_UUID}\"\n"
        )
    );
}

#[tokio::test]
async fn org_get_unknown_well_formed_id_reads_as_not_found() {
    let mock = MockServer::start().await;
    mount_invalid_id_400(
        &mock,
        format!("/v1/organizations/{UNKNOWN_UUID}"),
        "organization",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_human(&mock, &["org", "get", UNKNOWN_UUID]);

    assert_eq!(output.status.code(), Some(1));
    // The organization is the identifier being looked up, so it is not
    // repeated as request scope.
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: No such organization: {UNKNOWN_UUID}. \
             The API rejected the identifier: \
             BAD_REQUEST: Invalid organization id string:\"{UNKNOWN_UUID}\"\n"
        )
    );
}

#[tokio::test]
async fn by_id_read_not_found_carries_the_resource_not_found_code_in_json_mode() {
    for (api_path, thing, args, expected_command) in [
        (
            format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{UNKNOWN_UUID}"),
            "Postgres service",
            vec!["postgres", "get", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID],
            format!("clickhousectl cloud postgres list --org-id {LOOKUP_ORG_ID}"),
        ),
        (
            format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}"),
            "service",
            vec!["service", "get", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID],
            format!("clickhousectl cloud service list --org-id {LOOKUP_ORG_ID}"),
        ),
        (
            format!("/v1/organizations/{UNKNOWN_UUID}"),
            "organization",
            vec!["org", "get", UNKNOWN_UUID],
            "clickhousectl cloud org list".to_string(),
        ),
    ] {
        let mock = MockServer::start().await;
        mount_invalid_id_400(&mock, api_path, thing, UNKNOWN_UUID).await;

        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_eq!(output.status.code(), Some(1), "args {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error: Value =
            serde_json::from_str(stderr.trim()).unwrap_or_else(|_| panic!("not JSON: {stderr}"));
        assert_eq!(error["error"]["code"], "resource_not_found", "{stderr}");
        assert_eq!(
            error["error"]["command"],
            Value::String(expected_command),
            "{stderr}"
        );
        // The JSON message is the same text human mode prints, server detail
        // and all.
        let message = error["error"]["message"].as_str().expect("a message");
        assert!(message.starts_with("No such "), "{stderr}");
        assert!(
            message.ends_with(&format!(
                "BAD_REQUEST: Invalid {thing} id string:\"{UNKNOWN_UUID}\""
            )),
            "{stderr}"
        );
    }
}

#[tokio::test]
async fn by_id_read_keeps_the_servers_message_for_a_malformed_id() {
    const MALFORMED_ID: &str = "not-a-uuid";
    for (api_path, thing, args) in [
        (
            format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{MALFORMED_ID}"),
            "Postgres service",
            vec!["postgres", "get", MALFORMED_ID, "--org-id", LOOKUP_ORG_ID],
        ),
        (
            format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{MALFORMED_ID}"),
            "service",
            vec!["service", "get", MALFORMED_ID, "--org-id", LOOKUP_ORG_ID],
        ),
        (
            format!("/v1/organizations/{MALFORMED_ID}"),
            "organization",
            vec!["org", "get", MALFORMED_ID],
        ),
    ] {
        let mock = MockServer::start().await;
        mount_invalid_id_400(&mock, api_path, thing, MALFORMED_ID).await;

        let output = invoke_cli_human(&mock, &args);

        assert_eq!(output.status.code(), Some(1), "args {args:?}");
        // "Invalid" is the truth here, and more useful than "no such".
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("Error: BAD_REQUEST: Invalid {thing} id string:\"{MALFORMED_ID}\"\n"),
        );
    }
}

/// `--json` mode must not invent a code for a failure the CLI did not
/// classify: a malformed id keeps the API's prose, as it does in human mode.
#[tokio::test]
async fn by_id_read_in_json_mode_keeps_the_servers_message_for_a_malformed_id() {
    const MALFORMED_ID: &str = "not-a-uuid";
    for (api_path, thing, args) in [
        (
            format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{MALFORMED_ID}"),
            "Postgres service",
            vec!["postgres", "get", MALFORMED_ID, "--org-id", LOOKUP_ORG_ID],
        ),
        (
            format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{MALFORMED_ID}"),
            "service",
            vec!["service", "get", MALFORMED_ID, "--org-id", LOOKUP_ORG_ID],
        ),
        (
            format!("/v1/organizations/{MALFORMED_ID}"),
            "organization",
            vec!["org", "get", MALFORMED_ID],
        ),
    ] {
        let mock = MockServer::start().await;
        mount_invalid_id_400(&mock, api_path, thing, MALFORMED_ID).await;

        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_eq!(output.status.code(), Some(1), "args {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr,
            format!("Error: BAD_REQUEST: Invalid {thing} id string:\"{MALFORMED_ID}\"\n"),
        );
        assert!(
            !stderr.contains("\"code\""),
            "no structured code for an unclassified failure: {stderr}"
        );
    }
}

/// A `service delete` for an id no service has, with the organization id also
/// a well-formed UUID so every path identifier passes the structural test.
fn invoke_unknown_service_delete(mock: &MockServer, force: bool) -> std::process::Output {
    let mut args = vec!["service", "delete", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID];
    if force {
        args.push("--force");
    }
    invoke_cli_human(mock, &args)
}

/// `--force` reads the service first to decide whether to stop it. That read
/// must treat the 400 the way it treats a 404 (see
/// `forced_service_delete_surfaces_not_found_for_an_absent_service`): carry
/// on to the delete, rather than aborting on an id `service get` reports as
/// missing.
#[tokio::test]
async fn forced_service_delete_proceeds_when_the_read_rejects_a_well_formed_id() {
    let mock = MockServer::start().await;
    let service_path = format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}");
    Mock::given(method("GET"))
        .and(path(service_path.clone()))
        .respond_with(invalid_id_400_response("service", UNKNOWN_UUID))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(service_path.clone()))
        .respond_with(invalid_id_400_response("service", UNKNOWN_UUID))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_unknown_service_delete(&mock, true);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: No such service: {UNKNOWN_UUID} (organization {LOOKUP_ORG_ID}). \
             The API rejected the identifier: \
             BAD_REQUEST: Invalid service id string:\"{UNKNOWN_UUID}\"\n"
        )
    );
    // The read did not abort the command: the delete was still attempted.
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("GET".to_string(), service_path.clone()),
            ("DELETE".to_string(), service_path),
        ]
    );
}

/// Without `--force` there is no read at all, so the delete's own 400 is what
/// has to be refined.
#[tokio::test]
async fn service_delete_reports_a_well_formed_unknown_id_as_not_found() {
    let mock = MockServer::start().await;
    let service_path = format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}");
    Mock::given(method("DELETE"))
        .and(path(service_path.clone()))
        .respond_with(invalid_id_400_response("service", UNKNOWN_UUID))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_unknown_service_delete(&mock, false);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: No such service: {UNKNOWN_UUID} (organization {LOOKUP_ORG_ID}). \
             The API rejected the identifier: \
             BAD_REQUEST: Invalid service id string:\"{UNKNOWN_UUID}\"\n"
        )
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("DELETE".to_string(), service_path)]
    );
}

// PgBouncer parameters must survive all advertised file-input paths (#697).
fn pgbouncer_write_commands<'a>(
    map_file: &'a str,
    config_file: &'a str,
) -> Vec<(Vec<&'a str>, &'static str, &'static str)> {
    vec![
        (
            vec![
                "postgres",
                "create",
                "--name",
                "pg-test",
                "--region",
                "us-east-1",
                "--size",
                "c6gd.large",
                "--pg-bouncer-config-file",
                map_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres",
        ),
        (
            vec![
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "replica-test",
                "--pg-bouncer-config-file",
                map_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/readReplica",
        ),
        (
            vec![
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "restored-test",
                "--restore-target",
                "2026-08-01T00:00:00Z",
                "--pg-bouncer-config-file",
                map_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/restoredService",
        ),
        (
            vec!["postgres", "config", "patch", "pg-1", "--file", config_file],
            "PATCH",
            "/v1/organizations/org-1/postgres/pg-1/config",
        ),
        (
            vec![
                "postgres",
                "config",
                "replace",
                "pg-1",
                "--file",
                config_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/config",
        ),
    ]
}

fn invoke_pgbouncer_cli(mock: &MockServer, args: &[&str]) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("HOME", directory.path())
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(directory.path())
        .args(["cloud", "--url", &mock.uri(), "--json"])
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn pgbouncer_file_inputs_preserve_string_maps_in_every_write_path() {
    let directory = tempfile::tempdir().unwrap();
    let map_file = directory.path().join("pgbouncer.json");
    let config_file = directory.path().join("config.json");
    let parameters =
        serde_json::json!({"default_pool_size": "16", "future_parameter": "on", "empty": ""});
    let configuration =
        serde_json::json!({"pgConfig": {"work_mem": "64MB"}, "pgBouncerConfig": parameters});
    std::fs::write(&map_file, parameters.to_string()).unwrap();
    std::fs::write(&config_file, configuration.to_string()).unwrap();

    for (mut args, verb, endpoint) in
        pgbouncer_write_commands(map_file.to_str().unwrap(), config_file.to_str().unwrap())
    {
        let mock = MockServer::start().await;
        Mock::given(method(verb))
            .and(path(endpoint))
            .and(wiremock::matchers::basic_auth(
                "fake-key-for-tests",
                "fake-secret-for-tests",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": configuration, "status": 200,
            })))
            .expect(1)
            .mount(&mock)
            .await;
        args.extend(["--org-id", "org-1"]);
        let output = invoke_pgbouncer_cli(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["pgBouncerConfig"], parameters, "{args:?}");
        if endpoint.ends_with("/config") {
            assert_eq!(body, configuration);
            let output: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output["pgBouncerConfig"], parameters);
        }
    }
}

#[tokio::test]
async fn pgbouncer_invalid_file_values_fail_before_any_api_request() {
    let directory = tempfile::tempdir().unwrap();
    let map_file = directory.path().join("pgbouncer.json");
    let config_file = directory.path().join("config.json");
    for value in [
        serde_json::json!(16),
        serde_json::json!(false),
        serde_json::json!(null),
    ] {
        let parameters = serde_json::json!({"default_pool_size": value});
        std::fs::write(&map_file, parameters.to_string()).unwrap();
        std::fs::write(
            &config_file,
            serde_json::json!({"pgConfig": {}, "pgBouncerConfig": parameters}).to_string(),
        )
        .unwrap();
        for (args, _, _) in
            pgbouncer_write_commands(map_file.to_str().unwrap(), config_file.to_str().unwrap())
        {
            let mock = MockServer::start().await;
            // No --org-id: validation must precede even organization discovery.
            let output = invoke_pgbouncer_cli(&mock, &args);
            assert_eq!(output.status.code(), Some(1), "{args:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("string"), "{args:?}: {error}");
            assert!(
                error.contains("pgBouncerConfig") || error.contains("pgbouncer.json"),
                "{error}"
            );
            assert!(
                mock.received_requests().await.unwrap().is_empty(),
                "{args:?}"
            );
        }
    }
}

#[tokio::test]
async fn pgbouncer_config_get_json_preserves_values_and_tolerates_absent_sections() {
    for result in [
        serde_json::json!({"pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        serde_json::json!({"pgBouncerConfig": {}}),
        serde_json::json!({"pgBouncerConfig": null}),
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_pgbouncer_cli(
            &mock,
            &["postgres", "config", "get", "pg-1", "--org-id", "org-1"],
        );
        assert_success(&output);
        let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
        if result.get("pgBouncerConfig").is_none_or(Value::is_null) {
            assert!(actual.get("pgBouncerConfig").is_none());
        } else {
            assert_eq!(actual["pgBouncerConfig"], result["pgBouncerConfig"]);
        }
    }
}

// IP allowlist descriptions (#589).

#[tokio::test]
async fn service_allowlist_descriptions_reach_create_and_additive_update_requests() {
    let create = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .and(body_json(serde_json::json!({
            "name": "described-service",
            "provider": "aws",
            "region": "us-east-1",
            "ipAccessList": [
                {"source": "192.0.2.0/24"},
                {"source": "2001:db8::/32", "description": "\u{6771}\u{4eac} \u{1f5fc}"},
            ],
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "service": {"id": "22222222-3333-4444-5555-666666666666", "name": "described-service"},
                "password": "generated-password"
            },
            "status": 200,
            "requestId": "stub-service-create"
        })))
        .expect(1)
        .mount(&create)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &create,
        &[
            "service",
            "create",
            "--name",
            "described-service",
            "--ip-allow",
            "192.0.2.0/24",
            "--ip-allow",
            "2001:db8::/32=\u{6771}\u{4eac} \u{1f5fc}",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);

    let update = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .and(body_json(serde_json::json!({
            "ipAccessList": {
                "add": [{"source": "2001:db8:1::/48", "description": "branch office"}],
                "remove": []
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "22222222-3333-4444-5555-666666666666", "name": "described-service"},
            "status": 200,
            "requestId": "stub-service-update"
        })))
        .expect(1)
        .mount(&update)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &update,
        &[
            "service",
            "update",
            "svc-1",
            "--add-ip-allow",
            "2001:db8:1::/48=branch office",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
}

#[tokio::test]
async fn api_key_allowlist_descriptions_reach_create_and_update_requests() {
    let create = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .and(body_json(serde_json::json!({
            "name": "described-key",
            "state": "enabled",
            "assignedRoleIds": [],
            "ipAccessList": [
                {"source": "198.51.100.4"},
                {"source": "2001:db8::/32", "description": "production"},
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": {"id": "11111111-2222-3333-4444-555555555555", "name": "described-key"},
                "keyId": "generated-key-id",
                "keySecret": "generated-key-secret"
            },
            "status": 200,
            "requestId": "stub-key-create"
        })))
        .expect(1)
        .mount(&create)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &create,
        &[
            "key",
            "create",
            "--name",
            "described-key",
            "--ip-allow",
            "198.51.100.4",
            "--ip-allow",
            "2001:db8::/32=production",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);

    let update = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .and(body_json(serde_json::json!({
            "ipAccessList": [{"source": "203.0.113.0/24", "description": ""}]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "11111111-2222-3333-4444-555555555555", "name": "described-key"},
            "status": 200,
            "requestId": "stub-key-update"
        })))
        .expect(1)
        .mount(&update)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &update,
        &[
            "key",
            "update",
            "key-1",
            "--ip-allow",
            "203.0.113.0/24=",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
}

#[tokio::test]
async fn invalid_allowlist_sources_fail_before_service_or_key_requests() {
    for args in [
        vec![
            "service",
            "create",
            "--name",
            "invalid-service",
            "--ip-allow",
            "2001:db8::/129=invalid",
            "--org-id",
            "org-1",
        ],
        vec![
            "key",
            "update",
            "key-1",
            "--ip-allow",
            "not-an-ip=invalid",
            "--org-id",
            "org-1",
        ],
        vec![
            "service",
            "update",
            "svc-1",
            "--remove-ip-allow",
            "2001:db8::/129=invalid",
            "--org-id",
            "org-1",
        ],
    ] {
        let server = MockServer::start().await;
        let output = invoke_cli_with_cloud_credentials(&server, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("invalid IP allowlist entry"),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

// API key nullable expiry (#698) and explicit list clearing (#597).
async fn invoke_key_update(server: &MockServer, flags: &[&str]) -> std::process::Output {
    let project = tempfile::tempdir().unwrap();
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "expiry-test-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "expiry-test-secret")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &server.uri(),
            "--json",
            "key",
            "update",
            "key-1",
            "--org-id",
            "org-1",
        ])
        .args(flags)
        .stdin(Stdio::null())
        .output()
        .expect("failed to run key update")
}

#[tokio::test]
async fn key_update_sends_exact_omitted_set_and_clear_bodies() {
    let cases: &[(&[&str], Value)] = &[
        (&[], serde_json::json!({})),
        (
            &["--name", "renamed"],
            serde_json::json!({"name": "renamed"}),
        ),
        (&["--clear-expiry"], serde_json::json!({"expireAt": null})),
        (
            &["--clear-roles", "--clear-ip-allow"],
            serde_json::json!({"assignedRoleIds": [], "ipAccessList": []}),
        ),
        (
            &["--expires-at", "2030-01-01T00:00:00Z"],
            serde_json::json!({"expireAt": "2030-01-01T00:00:00Z"}),
        ),
        (
            &[
                "--clear-expiry",
                "--name",
                "renamed",
                "--role-id",
                "11111111-2222-3333-4444-555555555555",
                "--state",
                "disabled",
                "--ip-allow",
                "10.0.0.0/8",
            ],
            serde_json::json!({
                "expireAt": null, "name": "renamed", "state": "disabled",
                "assignedRoleIds": ["11111111-2222-3333-4444-555555555555"],
                "ipAccessList": [{"source": "10.0.0.0/8"}],
            }),
        ),
    ];
    for (flags, expected) in cases {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/keys/key-1"))
            .and(wiremock::matchers::basic_auth(
                "expiry-test-key",
                "expiry-test-secret",
            ))
            .and(body_json(expected.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"name": "updated", "expireAt": null}, "status": 200,
            })))
            .expect(1)
            .mount(&server)
            .await;
        let output = invoke_key_update(&server, flags).await;
        assert_success(&output);
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result, serde_json::json!({"name": "updated"}));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn key_update_conflicts_send_no_http() {
    for flags in [
        vec!["--clear-expiry", "--expires-at", "2030-01-01T00:00:00Z"],
        vec![
            "--clear-roles",
            "--role-id",
            "11111111-2222-3333-4444-555555555555",
        ],
        vec!["--clear-ip-allow", "--ip-allow", "10.0.0.0/8"],
    ] {
        let server = MockServer::start().await;
        let output = invoke_key_update(&server, &flags).await;
        assert_eq!(output.status.code(), Some(2));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn key_update_preserves_auth_error_conversion() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .and(body_json(serde_json::json!({"assignedRoleIds": []})))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": "not permitted", "status": 403,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let output = invoke_key_update(&server, &["--clear-roles"]).await;
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
}

#[tokio::test]
async fn key_update_expiry_rejects_oauth_before_http() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &server.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &server.uri(),
            "--json",
            "key",
            "update",
            "key-1",
            "--org-id",
            "org-1",
            "--clear-expiry",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("failed to run OAuth key update");
    assert_eq!(output.status.code(), Some(4));
    assert!(server.received_requests().await.unwrap().is_empty());
}

// PgConfig is a closed schema. Every file-input write path must validate it
// before organization discovery or any other HTTP request (#591).
fn pg_config_file_write_commands<'a>(
    pg_config_file: &'a str,
    instance_config_file: &'a str,
) -> Vec<(Vec<&'a str>, &'static str, &'static str)> {
    vec![
        (
            vec![
                "postgres",
                "create",
                "--name",
                "pg-test",
                "--region",
                "us-east-1",
                "--size",
                "c6gd.large",
                "--pg-config-file",
                pg_config_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres",
        ),
        (
            vec![
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "replica-test",
                "--pg-config-file",
                pg_config_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/readReplica",
        ),
        (
            vec![
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "restored-test",
                "--restore-target",
                "2026-08-01T00:00:00Z",
                "--pg-config-file",
                pg_config_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/restoredService",
        ),
        (
            vec![
                "postgres",
                "config",
                "patch",
                "pg-1",
                "--file",
                instance_config_file,
            ],
            "PATCH",
            "/v1/organizations/org-1/postgres/pg-1/config",
        ),
        (
            vec![
                "postgres",
                "config",
                "replace",
                "pg-1",
                "--file",
                instance_config_file,
            ],
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/config",
        ),
    ]
}

#[tokio::test]
async fn pg_config_files_preserve_valid_values_in_every_write_path() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let instance_config_file = directory.path().join("config.json");
    let pg_config = serde_json::json!({
        "max_connections": 0,
        "autovacuum_analyze_scale_factor": false,
        "default_transaction_isolation": "repeatable read",
        "ssl_min_protocol_version": "TLSv1.3",
        "wal_compression": "zstd"
    });
    let instance_config = serde_json::json!({
        "pgConfig": pg_config,
        "pgBouncerConfig": {"future_parameter": "on"}
    });
    std::fs::write(&pg_config_file, pg_config.to_string()).unwrap();
    std::fs::write(&instance_config_file, instance_config.to_string()).unwrap();

    for (mut args, verb, endpoint) in pg_config_file_write_commands(
        pg_config_file.to_str().unwrap(),
        instance_config_file.to_str().unwrap(),
    ) {
        let mock = MockServer::start().await;
        Mock::given(method(verb))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": instance_config,
                "status": 200
            })))
            .expect(1)
            .mount(&mock)
            .await;
        args.extend(["--org-id", "org-1"]);
        let output = invoke_pgbouncer_cli(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "{args:?}");
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["pgConfig"], pg_config, "{args:?}");
        if endpoint.ends_with("/config") {
            assert_eq!(body["pgBouncerConfig"], instance_config["pgBouncerConfig"]);
        }
    }
}

#[tokio::test]
async fn pg_config_unknown_file_keys_fail_before_any_api_request() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let instance_config_file = directory.path().join("config.json");
    let pg_config = serde_json::json!({"max_conections": 500});
    std::fs::write(&pg_config_file, pg_config.to_string()).unwrap();
    std::fs::write(
        &instance_config_file,
        serde_json::json!({"pgConfig": pg_config, "pgBouncerConfig": {}}).to_string(),
    )
    .unwrap();

    for (args, _, _) in pg_config_file_write_commands(
        pg_config_file.to_str().unwrap(),
        instance_config_file.to_str().unwrap(),
    ) {
        let mock = MockServer::start().await;
        let output = invoke_pgbouncer_cli(&mock, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("unknown pgConfig key 'max_conections'"),
            "{error}"
        );
        assert!(error.contains("max_connections"), "{error}");
        assert!(
            mock.received_requests().await.unwrap().is_empty(),
            "{args:?}"
        );
    }
}

#[tokio::test]
async fn pg_config_malformed_file_roots_fail_before_any_api_request() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let instance_config_file = directory.path().join("config.json");
    std::fs::write(&pg_config_file, "[]").unwrap();
    std::fs::write(&instance_config_file, "[]").unwrap();

    for (args, _, _) in pg_config_file_write_commands(
        pg_config_file.to_str().unwrap(),
        instance_config_file.to_str().unwrap(),
    ) {
        let mock = MockServer::start().await;
        let output = invoke_pgbouncer_cli(&mock, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("must be a JSON object"), "{args:?}: {error}");
        assert!(
            mock.received_requests().await.unwrap().is_empty(),
            "{args:?}"
        );
    }
}

#[tokio::test]
async fn pg_config_null_file_values_fail_before_any_api_request() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let instance_config_file = directory.path().join("config.json");
    let pg_config = serde_json::json!({"work_mem": null});
    std::fs::write(&pg_config_file, pg_config.to_string()).unwrap();
    std::fs::write(
        &instance_config_file,
        serde_json::json!({"pgConfig": pg_config, "pgBouncerConfig": {}}).to_string(),
    )
    .unwrap();

    for (args, _, _) in pg_config_file_write_commands(
        pg_config_file.to_str().unwrap(),
        instance_config_file.to_str().unwrap(),
    ) {
        let mock = MockServer::start().await;
        let output = invoke_pgbouncer_cli(&mock, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("null is not supported"), "{args:?}: {error}");
        assert!(
            mock.received_requests().await.unwrap().is_empty(),
            "{args:?}"
        );
    }
}

#[tokio::test]
async fn pg_config_invalid_enum_file_values_fail_before_any_api_request() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let instance_config_file = directory.path().join("config.json");

    for pg_config in [
        serde_json::json!({"default_transaction_isolation": "read uncommitted"}),
        serde_json::json!({"ssl_min_protocol_version": "SSLv3"}),
        serde_json::json!({"wal_compression": "gzip"}),
    ] {
        std::fs::write(&pg_config_file, pg_config.to_string()).unwrap();
        std::fs::write(
            &instance_config_file,
            serde_json::json!({"pgConfig": pg_config, "pgBouncerConfig": {}}).to_string(),
        )
        .unwrap();
        for (args, _, _) in pg_config_file_write_commands(
            pg_config_file.to_str().unwrap(),
            instance_config_file.to_str().unwrap(),
        ) {
            let mock = MockServer::start().await;
            let output = invoke_pgbouncer_cli(&mock, &args);
            assert_eq!(output.status.code(), Some(1), "{args:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("invalid pgConfig value"),
                "{args:?}: {error}"
            );
            assert!(error.contains("expected one of"), "{args:?}: {error}");
            assert!(
                mock.received_requests().await.unwrap().is_empty(),
                "{args:?}"
            );
        }
    }
}

#[tokio::test]
async fn pg_config_set_validates_keys_and_enum_values_before_any_api_request() {
    for setting in [
        "max_conections=500",
        "default_transaction_isolation=read uncommitted",
        "ssl_min_protocol_version=SSLv3",
        "wal_compression=gzip",
    ] {
        let mock = MockServer::start().await;
        let output = invoke_pgbouncer_cli(
            &mock,
            &["postgres", "config", "patch", "pg-1", "--set", setting],
        );
        assert_eq!(output.status.code(), Some(1), "{setting}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("unknown pgConfig key") || error.contains("invalid pgConfig value"),
            "{setting}: {error}"
        );
        assert!(error.contains("expected one of"), "{setting}: {error}");
        assert!(
            mock.received_requests().await.unwrap().is_empty(),
            "{setting}"
        );
    }

    let mock = MockServer::start().await;
    let output = invoke_pgbouncer_cli(
        &mock,
        &[
            "postgres",
            "config",
            "patch",
            "pg-1",
            "--set",
            "work_mem=null",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("null is not supported"), "{error}");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn pg_config_set_preserves_false_zero_and_valid_enums() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"pgConfig": {}, "pgBouncerConfig": {}},
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_pgbouncer_cli(
        &mock,
        &[
            "postgres",
            "config",
            "patch",
            "pg-1",
            "--set",
            "max_connections=0",
            "--set",
            "autovacuum_analyze_scale_factor=false",
            "--set",
            "wal_compression=zstd",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "pgConfig": {
                "max_connections": 0,
                "autovacuum_analyze_scale_factor": false,
                "wal_compression": "zstd"
            },
            "pgBouncerConfig": {}
        })
    );
}

#[tokio::test]
async fn postgres_config_files_reject_missing_or_malformed_sections_before_http() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("config.json");
    for (document, expected) in [
        (serde_json::json!({}), "missing required 'pgConfig'"),
        (
            serde_json::json!({"pgConfig": {}}),
            "missing required 'pgBouncerConfig'",
        ),
        (
            serde_json::json!({"pgBouncerConfig": {}}),
            "missing required 'pgConfig'",
        ),
        (
            serde_json::json!({"pgConfig": null, "pgBouncerConfig": {}}),
            "pgConfig must be a JSON object",
        ),
        (
            serde_json::json!({"pgConfig": {}, "pgBouncerConfig": []}),
            "invalid pgBouncerConfig",
        ),
        (
            serde_json::json!({
                "pgConfig": {}, "pgBouncerConfig": {}, "pgBouncerConfigs": {}
            }),
            "unknown configuration section 'pgBouncerConfigs'",
        ),
    ] {
        std::fs::write(&file, document.to_string()).unwrap();
        for action in ["patch", "replace"] {
            let mock = MockServer::start().await;
            let output = invoke_pgbouncer_cli(
                &mock,
                &[
                    "postgres",
                    "config",
                    action,
                    "pg-1",
                    "--file",
                    file.to_str().unwrap(),
                ],
            );
            assert_eq!(output.status.code(), Some(1), "{action}: {document}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(expected), "{action}: {document}: {error}");
            assert!(
                mock.received_requests().await.unwrap().is_empty(),
                "{action}"
            );
        }
    }
}

#[tokio::test]
async fn postgres_config_files_preserve_explicit_empty_sections() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("config.json");
    let document = serde_json::json!({"pgConfig": {}, "pgBouncerConfig": {}});
    std::fs::write(&file, document.to_string()).unwrap();

    for (action, verb) in [("patch", "PATCH"), ("replace", "POST")] {
        let mock = MockServer::start().await;
        Mock::given(method(verb))
            .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
            .and(body_json(document.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": document,
                "status": 200
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_pgbouncer_cli(
            &mock,
            &[
                "postgres",
                "config",
                action,
                "pg-1",
                "--file",
                file.to_str().unwrap(),
                "--org-id",
                "org-1",
            ],
        );
        assert_success(&output);
    }
}

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
    clear_inherited_env(&mut command);
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

#[tokio::test]
async fn clickstack_all_ten_routes_use_service_org_auth_and_full_bodies() {
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
            "integration": "slack",
            "conditions": {"teamId": "team-1"}
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
                "--config-file",
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
                "--config-file",
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
                "--config-file",
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
                "--config-file",
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
        serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid JSON output for {args:?}: {error}\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
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

#[tokio::test]
async fn clickstack_source_create_accepts_all_variants_and_nested_log_fields() {
    let bodies = vec![
        serde_json::json!({
            "kind":"log", "name":"logs", "connection":"c",
            "from":{"databaseName":"d","tableName":"logs"},
            "defaultTableSelectExpression":"*", "timestampValueExpression":"Timestamp",
            "serviceVersionExpression":"ServiceVersion",
            "filterSettings":{"databaseName":"d","tableName":"filters","columns":[
                {"name":"service","label":"Service","allowAll":true,"valueExpression":"ServiceName"}
            ]},
            "materializedViews":[{"databaseName":"d","tableName":"mv","dimensionColumns":"ServiceName",
                "minGranularity":"1m","timestampColumn":"Timestamp","aggregatedColumns":[
                    {"aggFn":"count","mvColumn":"count"}
                ]}],
            "metadataMaterializedViews":{"keyRollupTable":"keys","kvRollupTable":"kv","granularity":"1m"}
        }),
        serde_json::json!({"kind":"trace","name":"traces","connection":"c","from":{"databaseName":"d","tableName":"traces"},"defaultTableSelectExpression":"*","timestampValueExpression":"ts","durationExpression":"duration","durationPrecision":9,"traceIdExpression":"trace","spanIdExpression":"span","parentSpanIdExpression":"parent","spanNameExpression":"name","spanKindExpression":"kind","serviceVersionExpression":"version"}),
        serde_json::json!({"kind":"metric","name":"metrics","connection":"c","from":{"databaseName":"d"},"metricTables":{"gauge":"g","histogram":"h","sum":"s","summary":"summary","exponential histogram":"eh"},"timestampValueExpression":"ts","resourceAttributesExpression":"resource"}),
        serde_json::json!({"kind":"session","name":"sessions","connection":"c","from":{"databaseName":"d","tableName":"sessions"},"traceSourceId":"traces"}),
        serde_json::json!({"kind":"promql","name":"promql","connection":"c","from":{"databaseName":"d","tableName":"metrics"},"timestampValueExpression":"ts"}),
    ];
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"kind": "promql"}
        })))
        .expect(5)
        .mount(&mock)
        .await;
    let directory = tempfile::tempdir().unwrap();
    for (index, body) in bodies.iter().enumerate() {
        let file = directory.path().join(format!("source-{index}.json"));
        std::fs::write(&file, body.to_string()).unwrap();
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "source",
                "create",
                "svc-1",
                "--config-file",
                file.to_str().unwrap(),
                "--org-id",
                "org-1",
            ],
            None,
            true,
        );
        assert_success(&output);
    }
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.body_json::<Value>().unwrap())
            .collect::<Vec<_>>(),
        bodies
    );
}

#[tokio::test]
async fn clickstack_invalid_config_fails_before_org_discovery_or_resource_request() {
    let invalid = [
        (
            serde_json::json!({"kind":"logs"}),
            "unknown source discriminator",
        ),
        (
            serde_json::json!({"kind":"promql","name":"p","connection":"c","from":{"databaseName":"d","tableName":"t","tableNaem":null},"timestampValueExpression":"ts"}),
            "tableNaem",
        ),
        (
            serde_json::json!({"kind":"log","name":"l"}),
            "missing field",
        ),
        (
            serde_json::json!({"kind":"log","name":"l","connection":"c","from":{"databaseName":"d","tableName":"t"},"defaultTableSelectExpression":"*","timestampValueExpression":"ts","useTextIndexForImplicitColumn":"enabledd"}),
            "unknown useTextIndexForImplicitColumn",
        ),
    ];
    for (body, expected) in invalid {
        let mock = MockServer::start().await;
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("invalid.json");
        std::fs::write(&file, body.to_string()).unwrap();
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "source",
                "create",
                "svc-1",
                "--config-file",
                file.to_str().unwrap(),
            ],
            None,
            false,
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(mock.received_requests().await.unwrap().is_empty());
    }

    let mock = MockServer::start().await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "role",
            "create",
            "svc-1",
            "--config-file",
            "-",
        ],
        Some(
            r#"{"name":"reader","permissions":[{"action":"read","subject":"Dashboard","conditons":null}]}"#,
        ),
        false,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("conditons"));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn clickstack_human_lists_render_sparse_fields_and_api_errors_include_org() {
    for (resource, result) in [
        ("source", serde_json::json!([{}])),
        ("role", serde_json::json!([{}])),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/org-1/services/svc-1/clickstack/{resource}s"
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
        assert!(String::from_utf8_lossy(&output.stdout).contains('-'));
    }

    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-sensitive/services/svc-1/clickstack/roles/role-1",
        ))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error":"NOT_FOUND"})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "role",
            "get",
            "svc-1",
            "role-1",
            "--org-id",
            "org-sensitive",
        ],
        None,
        false,
    );
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("org-sensitive") && error.contains("NOT_FOUND"),
        "{error}"
    );
}

// UDF operations and artifact uploads run against separate hosts so auth and
// presigned-query isolation are exercised by the real executable.
fn udf_test_command(
    server: &MockServer,
    project: &Path,
    oauth: bool,
    json: bool,
    args: &[&str],
) -> Command {
    let home = project.join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    if oauth {
        write_oauth_tokens(&cloud_dir, &server.uri());
    }
    let mut cmd = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut cmd);
    cmd.env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project)
        .args(["cloud", "--url", &server.uri()]);
    if !oauth {
        cmd.args(["--api-key", "udf-key", "--api-secret", "udf-secret"]);
    }
    if json {
        cmd.arg("--json");
    }
    cmd.args(["udf", "--org-id", "org-1"])
        .args(args)
        .stdin(Stdio::null());
    cmd
}

fn udf_definition(kind: &str, create: bool) -> Value {
    let mut value = serde_json::json!({"type": kind, "runtime": "native", "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"});
    if create {
        value["functionName"] = serde_json::json!("my_udf");
    }
    value
}

#[tokio::test]
async fn udf_all_reads_preserve_pagination_and_sparse_responses_with_oauth() {
    for (args, suffix, list) in [
        (vec!["list"], "", true),
        (vec!["get", "my_udf"], "/my_udf", false),
        (
            vec!["attachment", "list", "my_udf"],
            "/my_udf/attachments",
            true,
        ),
        (
            vec!["attachment", "get", "my_udf", "svc-1"],
            "/my_udf/attachments/svc-1",
            false,
        ),
        (vec!["version", "list", "my_udf"], "/my_udf/versions", true),
    ] {
        for sparse in [false, true] {
            let server = MockServer::start().await;
            let project = tempfile::tempdir().unwrap();
            let item = if sparse {
                serde_json::json!({})
            } else {
                serde_json::json!({"functionName":"my_udf", "serviceId":"svc-1", "version":2, "status":"future", "deterministic":false})
            };
            let result = if list {
                serde_json::json!({"items": [item], "pagination":{"nextCursor":"next page", "limit":2, "totalRecords":3}})
            } else {
                item
            };
            Mock::given(method("GET"))
                .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":result})),
                )
                .expect(1)
                .mount(&server)
                .await;
            let mut args = args.clone();
            if list {
                args.extend(["--cursor", "page /+?", "--limit", "2"]);
            }
            let output = udf_test_command(&server, project.path(), true, true, &args)
                .output()
                .unwrap();
            assert_success(&output);
            let output: Value = serde_json::from_slice(&output.stdout).unwrap();
            if list {
                assert_eq!(output["pagination"], result["pagination"]);
                assert_eq!(output["items"][0]["status"], result["items"][0]["status"]);
                let req = server.received_requests().await.unwrap().pop().unwrap();
                let query: std::collections::HashMap<_, _> =
                    req.url.query_pairs().into_owned().collect();
                assert_eq!(query.get("cursor").map(String::as_str), Some("page /+?"));
                assert_eq!(query.get("limit").map(String::as_str), Some("2"));
            } else {
                assert_eq!(output["status"], result["status"]);
            }
        }
    }
}

#[tokio::test]
async fn udf_delete_attach_detach_methods_and_auth() {
    for (args, suffix, verb, expected_body) in [
        (vec!["delete", "my_udf"], "/my_udf", "DELETE", None),
        (
            vec!["version", "delete", "my_udf", "2"],
            "/my_udf/versions/2",
            "DELETE",
            None,
        ),
        (
            vec!["detach", "my_udf", "svc-1"],
            "/my_udf/attachments/svc-1",
            "DELETE",
            None,
        ),
        (
            vec!["attach", "my_udf", "svc-1"],
            "/my_udf/attachments/svc-1",
            "PUT",
            Some(serde_json::json!({})),
        ),
        (
            vec!["attach", "my_udf", "svc-1", "--version", "2"],
            "/my_udf/attachments/svc-1",
            "PUT",
            Some(serde_json::json!({"version":2})),
        ),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method(verb))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(if verb == "DELETE" {
                    serde_json::json!({"status":200,"requestId":"delete-request"})
                } else {
                    serde_json::json!({"result":{}})
                }),
            )
            .expect(1)
            .mount(&server)
            .await;
        assert_success(
            &udf_test_command(&server, project.path(), false, true, &args)
                .output()
                .unwrap(),
        );
        let request = server.received_requests().await.unwrap().pop().unwrap();
        match expected_body {
            Some(body) => assert_eq!(
                serde_json::from_slice::<Value>(&request.body).unwrap(),
                body
            ),
            None => assert!(request.body.is_empty()),
        }
    }
}

#[tokio::test]
async fn udf_create_and_version_upload_full_definitions_without_auth_leakage() {
    for create in [true, false] {
        for kind in ["executable", "executable_pool"] {
            let server = MockServer::start().await;
            let storage = MockServer::start().await;
            let project = tempfile::tempdir().unwrap();
            let archive = b"PK\x03\x04test archive";
            std::fs::write(project.path().join("code.zip"), archive).unwrap();
            let mut definition = udf_definition(kind, create);
            definition.as_object_mut().unwrap().extend(serde_json::json!({
                "commandReadTimeout": 5000, "commandWriteTimeout": 6000, "memoryLimitMib": 128,
                "deterministic": false, "sendChunkHeader": false, "returnName":"result", "format":"JSONEachRow",
                "sandboxType":"netenable", "sandboxVersion":"v3", "maxCommandExecutionTime":20,
                "poolSize": if kind == "executable_pool" { serde_json::json!(4) } else { Value::Null }
            }).as_object().unwrap().clone());
            std::fs::write(
                project.path().join("definition.json"),
                definition.to_string(),
            )
            .unwrap();
            Mock::given(method("POST")).and(path("/v1/organizations/org-1/udfUploads/url"))
                .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
                    "uploadId":"fresh-upload", "uploadUrl":format!("{}/artifact?signature=upload-secret", storage.uri())
                }}))).expect(1).mount(&server).await;
            Mock::given(method("PUT"))
                .and(path("/artifact"))
                .and(header("content-type", "application/zip"))
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount(&storage)
                .await;
            let mut expected = definition;
            expected["uploadId"] = serde_json::json!("fresh-upload");
            if kind == "executable" {
                expected.as_object_mut().unwrap().remove("poolSize");
            }
            let suffix = if create { "" } else { "/my_udf/versions" };
            Mock::given(method("POST")).and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
                .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
                .and(body_json(expected))
                .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"result":{"functionName":"my_udf","version":2,"status":"building","deterministic":false}})))
                .expect(1).mount(&server).await;
            let mut args = if create {
                vec!["create"]
            } else {
                vec!["version", "create", "my_udf"]
            };
            args.extend([
                "--config-file",
                "definition.json",
                "--artifact",
                "code.zip",
                "--debug",
            ]);
            let output = udf_test_command(&server, project.path(), false, true, &args)
                .output()
                .unwrap();
            assert_success(&output);
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap()["deterministic"],
                false
            );
            let upload = storage.received_requests().await.unwrap().pop().unwrap();
            assert_eq!(upload.body, archive);
            assert_eq!(upload.url.query(), Some("signature=upload-secret"));
            assert!(!upload.headers.contains_key("authorization"));
            assert!(!upload.headers.contains_key("cookie"));
            let logged = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            for secret in ["upload-secret", "fresh-upload", "udf-key", "udf-secret"] {
                assert!(!logged.contains(secret), "leaked {secret}");
            }
            let requests = server.received_requests().await.unwrap();
            assert_eq!(requests.len(), 2);
            assert!(requests[0].body.is_empty());
        }
    }
}

#[tokio::test]
async fn udf_upload_failures_never_create_or_leak_presigned_url() {
    for scenario in [
        "missing_id",
        "missing_url",
        "bad_url",
        "credentials",
        "redirect",
        "storage_error",
        "connection_error",
        "session_error",
    ] {
        let server = MockServer::start().await;
        let storage = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
        std::fs::write(
            project.path().join("definition.json"),
            udf_definition("executable", true).to_string(),
        )
        .unwrap();
        let url = match scenario {
            "bad_url" => "not-a-url?signature=upload-secret".to_string(),
            "credentials" => "https://user:upload-secret@example.com/code".to_string(),
            "connection_error" => "http://127.0.0.1:1/code?signature=upload-secret".to_string(),
            _ => format!("{}/code?signature=upload-secret", storage.uri()),
        };
        let mut session = serde_json::json!({"uploadId":"upload-1","uploadUrl":url});
        if scenario == "missing_id" {
            session.as_object_mut().unwrap().remove("uploadId");
        }
        if scenario == "missing_url" {
            session.as_object_mut().unwrap().remove("uploadUrl");
        }
        let response = if scenario == "session_error" {
            ResponseTemplate::new(503).set_body_json(
                serde_json::json!({"error":"session unavailable signature=upload-secret"}),
            )
        } else {
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":session}))
        };
        Mock::given(method("POST"))
            .and(path("/v1/organizations/org-1/udfUploads/url"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let response = if scenario == "redirect" {
            ResponseTemplate::new(307).insert_header(
                "location",
                format!("{}/redirected?signature=upload-secret", storage.uri()),
            )
        } else {
            ResponseTemplate::new(403).set_body_string("signature=upload-secret")
        };
        Mock::given(method("PUT"))
            .and(path("/code"))
            .respond_with(response)
            .mount(&storage)
            .await;
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            true,
            &[
                "create",
                "--config-file",
                "definition.json",
                "--artifact",
                "code.zip",
                "--debug",
            ],
        )
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1), "{scenario}");
        let logged = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!logged.contains("upload-secret"), "{scenario}: {logged}");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert!(storage.received_requests().await.unwrap().len() <= 1);
    }
}

#[tokio::test]
async fn udf_invalid_definition_and_artifact_fail_before_api() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    for definition in [
        serde_json::json!({}),
        serde_json::json!({"functionName":"sparse_get","version":3}),
        serde_json::json!({"type":"future","runtime":"native","arguments":[],"returnType":"UInt64","functionName":"my_udf"}),
        serde_json::json!({"type":"executable","runtime":"native","arguments":[{"name":"x","type":"UInt64","typo":null}],"returnType":"UInt64","functionName":"my_udf"}),
    ] {
        std::fs::write(
            project.path().join("definition.json"),
            definition.to_string(),
        )
        .unwrap();
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            true,
            &[
                "create",
                "--config-file",
                "definition.json",
                "--artifact",
                "code.zip",
            ],
        )
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
    }
    std::fs::write(
        project.path().join("definition.json"),
        udf_definition("executable", true).to_string(),
    )
    .unwrap();
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &[
            "create",
            "--config-file",
            "definition.json",
            "--artifact",
            "missing.zip",
        ],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn udf_every_write_rejects_oauth_before_api() {
    let server = MockServer::start().await;
    for args in [
        vec!["delete", "my_udf"],
        vec!["detach", "my_udf", "svc-1"],
        vec!["attach", "my_udf", "svc-1"],
        vec!["version", "delete", "my_udf", "2"],
        vec![
            "create",
            "--config-file",
            "missing",
            "--artifact",
            "missing",
        ],
        vec![
            "version",
            "create",
            "my_udf",
            "--config-file",
            "missing",
            "--artifact",
            "missing",
        ],
    ] {
        let project = tempfile::tempdir().unwrap();
        assert_eq!(
            udf_test_command(&server, project.path(), true, true, &args)
                .output()
                .unwrap()
                .status
                .code(),
            Some(4)
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn udf_all_read_and_action_errors_preserve_exit_codes() {
    for (args, verb, suffix) in [
        (vec!["list"], "GET", ""),
        (vec!["get", "my_udf"], "GET", "/my_udf"),
        (
            vec!["attachment", "list", "my_udf"],
            "GET",
            "/my_udf/attachments",
        ),
        (
            vec!["attachment", "get", "my_udf", "svc-1"],
            "GET",
            "/my_udf/attachments/svc-1",
        ),
        (vec!["version", "list", "my_udf"], "GET", "/my_udf/versions"),
        (vec!["delete", "my_udf"], "DELETE", "/my_udf"),
        (
            vec!["detach", "my_udf", "svc-1"],
            "DELETE",
            "/my_udf/attachments/svc-1",
        ),
        (
            vec!["version", "delete", "my_udf", "2"],
            "DELETE",
            "/my_udf/versions/2",
        ),
        (
            vec!["attach", "my_udf", "svc-1"],
            "PUT",
            "/my_udf/attachments/svc-1",
        ),
    ] {
        for status in [403, 424] {
            let server = MockServer::start().await;
            let project = tempfile::tempdir().unwrap();
            Mock::given(method(verb))
                .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
                .respond_with(
                    ResponseTemplate::new(status)
                        .set_body_json(serde_json::json!({"error":"dependency unavailable"})),
                )
                .expect(1)
                .mount(&server)
                .await;
            let output = udf_test_command(&server, project.path(), false, true, &args)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(if status == 403 { 4 } else { 1 })
            );
            assert!(String::from_utf8_lossy(&output.stderr).contains("dependency unavailable"));
        }
    }
}

#[tokio::test]
async fn udf_create_api_failures_consume_one_upload_attempt() {
    for create in [true, false] {
        let server = MockServer::start().await;
        let storage = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
        std::fs::write(
            project.path().join("definition.json"),
            udf_definition("executable", create).to_string(),
        )
        .unwrap();
        Mock::given(method("POST")).and(path("/v1/organizations/org-1/udfUploads/url"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
                "uploadId":"one-attempt", "uploadUrl":format!("{}/code?signature=upload-secret", storage.uri())
            }}))).expect(1).mount(&server).await;
        Mock::given(method("PUT"))
            .and(path("/code"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&storage)
            .await;
        let suffix = if create { "" } else { "/my_udf/versions" };
        Mock::given(method("POST"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .respond_with(
                ResponseTemplate::new(424)
                    .set_body_json(serde_json::json!({"error":"build dependency unavailable"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut args = if create {
            vec!["create"]
        } else {
            vec!["version", "create", "my_udf"]
        };
        args.extend(["--config-file", "definition.json", "--artifact", "code.zip"]);
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("build dependency unavailable"));
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn udf_stdin_minimal_definition_and_sparse_human_output() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
                "uploadId":"fresh-upload", "uploadUrl":format!("{}/code", storage.uri())
            }})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/code"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&storage)
        .await;
    let mut expected = udf_definition("executable", true);
    expected["uploadId"] = serde_json::json!("fresh-upload");
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(body_json(expected))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"result":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let mut cmd = udf_test_command(
        &server,
        project.path(),
        false,
        false,
        &["create", "--config-file", "-", "--artifact", "code.zip"],
    );
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(udf_definition("executable", true).to_string().as_bytes())
        .unwrap();
    assert_success(&child.wait_with_output().unwrap());
    for (args, suffix, result) in [
        (
            vec!["get", "my_udf"],
            "/my_udf",
            serde_json::json!({"functionName":null}),
        ),
        (
            vec!["list"],
            "",
            serde_json::json!({"items":[{}],"pagination":{"nextCursor":"next"}}),
        ),
        (
            vec!["attachment", "list", "my_udf"],
            "/my_udf/attachments",
            serde_json::json!({"items":[{}]}),
        ),
        (
            vec!["version", "list", "my_udf"],
            "/my_udf/versions",
            serde_json::json!({"items":null,"pagination":null}),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":result})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let output = udf_test_command(&server, project.path(), false, false, &args)
            .output()
            .unwrap();
        assert_success(&output);
        if args == vec!["list"] {
            assert!(String::from_utf8_lossy(&output.stdout).contains("next"));
        }
        let request = server.received_requests().await.unwrap().pop().unwrap();
        assert!(request.url.query().is_none());
    }
}

// Explicit list clearing (#597).

#[tokio::test]
async fn member_update_preserves_omitted_set_and_clear_role_lists() {
    let cases: &[(&[&str], Value)] = &[
        (&[], serde_json::json!({})),
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

#[tokio::test]
async fn member_update_role_conflict_sends_no_http() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "member",
            "update",
            "user-1",
            "--org-id",
            "org-1",
            "--role-id",
            "role-1",
            "--clear-roles",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

const CLEAR_TAGS_POSTGRES_ID: &str = "11111111-2222-3333-4444-555555555555";

#[tokio::test]
async fn postgres_update_clear_tags_sends_an_empty_array_without_a_get() {
    let mock = MockServer::start().await;
    let postgres_path = format!("/v1/organizations/org-1/postgres/{CLEAR_TAGS_POSTGRES_ID}");
    Mock::given(method("PATCH"))
        .and(path(postgres_path.clone()))
        .and(body_json(serde_json::json!({"tags": []})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": CLEAR_TAGS_POSTGRES_ID, "name": "pg-clear-tags"},
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            CLEAR_TAGS_POSTGRES_ID,
            "--org-id",
            "org-1",
            "--clear-tags",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), postgres_path)],
        "clearing a complete list must not fetch the current service"
    );
}

#[tokio::test]
async fn postgres_update_tag_diff_refuses_a_sparse_get_without_writing() {
    let mock = MockServer::start().await;
    let postgres_path = format!("/v1/organizations/org-1/postgres/{CLEAR_TAGS_POSTGRES_ID}");
    Mock::given(method("GET"))
        .and(path(postgres_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": CLEAR_TAGS_POSTGRES_ID, "name": "pg-sparse"},
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(postgres_path.clone()))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            CLEAR_TAGS_POSTGRES_ID,
            "--org-id",
            "org-1",
            "--add-tag",
            "env=prod",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("omitted the tags field"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), postgres_path)]
    );
}

#[tokio::test]
async fn postgres_update_clear_tags_conflict_sends_no_http() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            CLEAR_TAGS_POSTGRES_ID,
            "--org-id",
            "org-1",
            "--clear-tags",
            "--remove-tag",
            "env",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_update_clear_tags_propagates_api_errors_without_retrying() {
    let mock = MockServer::start().await;
    let postgres_path = format!("/v1/organizations/org-1/postgres/{CLEAR_TAGS_POSTGRES_ID}");
    Mock::given(method("PATCH"))
        .and(path(postgres_path.clone()))
        .and(body_json(serde_json::json!({"tags": []})))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "update failed",
            "status": 500,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            CLEAR_TAGS_POSTGRES_ID,
            "--org-id",
            "org-1",
            "--clear-tags",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), postgres_path)]
    );
}

#[tokio::test]
async fn reverse_private_endpoint_update_clears_the_mapping_list() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(reverse_private_endpoint_path()))
        .and(body_json(serde_json::json!({
            "customPrivateDnsMappings": []
        })))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "update",
            "svc-1",
            RPE_ID,
            "--clear-custom-private-dns-mappings",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), reverse_private_endpoint_path())]
    );
}

#[tokio::test]
async fn reverse_private_endpoint_update_rejects_noop_and_conflict_without_http() {
    for flags in [
        Vec::<&str>::new(),
        vec![
            "--custom-private-dns-mapping",
            "db.example.com",
            "--clear-custom-private-dns-mappings",
        ],
    ] {
        let mock = MockServer::start().await;
        let mut args = vec![
            "clickpipe",
            "reverse-private-endpoint",
            "update",
            "svc-1",
            RPE_ID,
            "--org-id",
            "org-1",
        ];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_eq!(output.status.code(), Some(2));
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

// ── ClickStack saved-search commands (issue #694) ──────────────────────────

#[tokio::test]
async fn clickstack_saved_search_all_five_routes_preserve_bodies_and_json_output() {
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
                "--config-file",
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
                "--config-file",
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
        serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid JSON output for {args:?}: {error}\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
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

#[tokio::test]
async fn clickstack_saved_search_rejects_invalid_file_and_stdin_before_http() {
    let invalid = [
        (
            serde_json::json!({"name":"errors","sourceId":"source-1","whereLanguage":"lucenee"}).to_string(),
            "unknown whereLanguage",
        ),
        (
            serde_json::json!({"name":"errors","sourceId":"source-1","filters":[{"type":"lucene","condition":"x"}]}).to_string(),
            "unknown filters[0].type",
        ),
        (
            serde_json::json!({"name":"errors"}).to_string(),
            "missing field",
        ),
        (
            serde_json::json!({"name":"errors","sourceId":"source-1","selcet":null}).to_string(),
            "selcet",
        ),
        ("{".to_string(), "failed to parse config"),
    ];
    for (index, (body, expected)) in invalid.iter().enumerate() {
        let mock = MockServer::start().await;
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("invalid.json");
        std::fs::write(&file, body).unwrap();
        let (config_file, stdin) = if index % 2 == 0 {
            (file.to_str().unwrap(), None)
        } else {
            ("-", Some(body.as_str()))
        };
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "saved-search",
                "create",
                "svc-1",
                "--config-file",
                config_file,
            ],
            stdin,
            false,
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn clickstack_saved_search_sparse_human_output_and_errors_are_safe() {
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
        .and(path(format!("{searches}/sparse")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {}
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "saved-search",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
        None,
        false,
    );
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains('-'));
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "saved-search",
            "get",
            "svc-1",
            "sparse",
            "--org-id",
            "org-1",
        ],
        None,
        false,
    );
    assert_success(&output);

    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-sensitive/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error":"NOT_FOUND"})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "saved-search",
            "get",
            "svc-1",
            "search-1",
            "--org-id",
            "org-sensitive",
        ],
        None,
        false,
    );
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("org-sensitive") && error.contains("NOT_FOUND"),
        "{error}"
    );
}

#[tokio::test]
async fn clickstack_saved_search_oauth_reads_and_rejects_writes_before_http() {
    let mock = MockServer::start().await;
    let searches = "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches";
    Mock::given(method("GET"))
        .and(path(searches))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": []
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let invoke = |args: &[&str], stdin: Option<&str>| {
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(project.path())
            .args(["cloud", "--url", &mock.uri()])
            .args(args);
        if let Some(input) = stdin {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            child.wait_with_output().unwrap()
        } else {
            command.output().unwrap()
        }
    };
    let output = invoke(
        &[
            "clickstack",
            "saved-search",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
        None,
    );
    assert_success(&output);
    let output = invoke(
        &[
            "clickstack",
            "saved-search",
            "create",
            "svc-1",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
        ],
        Some(r#"{"name":"errors","sourceId":"source-1"}"#),
    );
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

// ── ClickStack dashboard commands (issue #693) ─────────────────────────────

#[tokio::test]
async fn clickstack_dashboard_all_six_routes_preserve_typed_bodies_and_auth() {
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
        "name":"replacement", "tiles":[], "tags":[], "filters":[],
        "savedFilterValues":[], "containers":[]
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
                "--config-file",
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
                "--config-file",
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
                "--config-file",
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
        serde_json::from_slice::<Value>(&output.stdout).unwrap();
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

#[tokio::test]
async fn clickstack_dashboard_validation_surfaces_invalid_and_sparse_results() {
    for (result, expected) in [
        (
            serde_json::json!({"valid":false,"errors":[{"path":"tiles.0.config","message":"Required"}],"normalized":null}),
            Some(false),
        ),
        (serde_json::json!({}), None),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/validate",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":result})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "dashboard",
                "validate",
                "svc-1",
                "--config-file",
                "-",
                "--org-id",
                "org-1",
            ],
            Some(r#"{"name":"dashboard","tiles":[]}"#),
            true,
        );
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.get("valid").and_then(Value::as_bool), expected);
        if expected == Some(false) {
            assert_eq!(output["errors"][0]["path"], "tiles.0.config");
        }
    }
}

#[tokio::test]
async fn clickstack_dashboard_sparse_human_list_and_errors_are_safe() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":[{}]})))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "dashboard",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
        None,
        false,
    );
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains('-'));

    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-sensitive/services/svc-1/clickstack/dashboards/missing",
        ))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(serde_json::json!({"error":"DASHBOARD_NOT_FOUND"})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "dashboard",
            "get",
            "svc-1",
            "missing",
            "--org-id",
            "org-sensitive",
        ],
        None,
        false,
    );
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("DASHBOARD_NOT_FOUND"), "{error}");
}

#[tokio::test]
async fn clickstack_dashboard_validate_fails_fast_for_oauth() {
    let mock = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("home");
    let ch_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &mock.uri());
    let config = directory.path().join("dashboard.json");
    std::fs::write(&config, r#"{"name":"dashboard","tiles":[]}"#).unwrap();
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "clickstack",
            "dashboard",
            "validate",
            "svc-1",
            "--config-file",
            config.to_str().unwrap(),
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("read-only"));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// ── ClickStack alert and webhook commands (issue #695) ─────────────────────

#[tokio::test]
async fn clickstack_alert_and_webhook_all_nine_routes_preserve_bodies_and_auth() {
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
                "--config-file",
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
                "--config-file",
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
                "--config-file",
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
                "--config-file",
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
        serde_json::from_slice::<Value>(&output.stdout).unwrap();
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

#[tokio::test]
async fn clickstack_alert_and_webhook_invalid_inputs_fail_before_http() {
    let invalid = [
        (
            "alert",
            serde_json::json!({"source":"saved_search","savedSearchId":"s","threshold":1,"interval":"1m","thresholdType":"above","channel":{"type":"email","emailRecipients":[]},"channels":[]}),
        ),
        (
            "alert",
            serde_json::json!({"source":"saved_search","savedSearchId":"s","threshold":1,"interval":"1m","thresholdType":"above","channel":{"type":"webhook","webhookId":"h","severty":"warning"},"channels":[{"type":"email","emailRecipients":[]}]}),
        ),
        (
            "webhook",
            serde_json::json!({"name":"bad","service":"slak","url":"https://example.com"}),
        ),
    ];
    for (resource, body) in invalid {
        let mock = MockServer::start().await;
        let body = body.to_string();
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                resource,
                "create",
                "svc-1",
                "--config-file",
                "-",
            ],
            Some(body.as_str()),
            false,
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn clickstack_webhook_create_sends_every_supported_provider_shape() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result":{"service":"generic"}
        })))
        .expect(3)
        .mount(&mock)
        .await;
    let bodies = [
        serde_json::json!({"name":"Slack","service":"slack","url":"https://hooks.slack.com/example","description":"Slack"}),
        serde_json::json!({"name":"Incident.io","service":"incidentio","url":"https://example.com/incident","body":"incident={{title}}","headers":{"X-Key":"value"}}),
        serde_json::json!({"name":"Generic","service":"generic","url":"https://example.com/generic","body":"{\"title\":\"{{title}}\"}","queryParams":{"team":"ops"}}),
    ];
    for body in &bodies {
        let body = body.to_string();
        let output = invoke_clickstack_cli(
            &mock,
            &[
                "clickstack",
                "webhook",
                "create",
                "svc-1",
                "--config-file",
                "-",
                "--org-id",
                "org-1",
            ],
            Some(&body),
            true,
        );
        assert_success(&output);
    }
    assert_eq!(
        mock.received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| request.body_json::<Value>().unwrap())
            .collect::<Vec<_>>(),
        bodies
    );
}

#[tokio::test]
async fn clickstack_alert_and_webhook_sparse_lists_unknown_variants_and_errors_are_safe() {
    for (resource, result) in [
        ("alert", serde_json::json!([{}])),
        (
            "webhook",
            serde_json::json!([{"service":"future","futureField":true},{}]),
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/org-1/services/svc-1/clickstack/{resource}s"
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":result})),
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
        assert!(String::from_utf8_lossy(&output.stdout).contains('-'));
    }

    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-sensitive/services/svc-1/clickstack/alerts/missing",
        ))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(serde_json::json!({"error":"ALERT_NOT_FOUND"})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "alert",
            "get",
            "svc-1",
            "missing",
            "--org-id",
            "org-sensitive",
        ],
        None,
        false,
    );
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("ALERT_NOT_FOUND"), "{error}");
}

#[tokio::test]
async fn clickstack_alert_detail_preserves_state_channels_and_query_timeout() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "id":"alert-1", "state":"ALERT", "channel":{"type":"future","data":true},
        "channels":[{"type":"email","emailRecipients":null}],
        "executionErrors":[{"type":"QUERY_TIMEOUT","message":"query timed out","timestamp":null}]
    });
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":result})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_clickstack_cli(
        &mock,
        &[
            "clickstack",
            "alert",
            "get",
            "svc-1",
            "alert-1",
            "--org-id",
            "org-1",
        ],
        None,
        true,
    );
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["state"], "ALERT");
    assert_eq!(output["executionErrors"][0]["type"], "QUERY_TIMEOUT");
    assert_eq!(output["channel"]["type"], "future");
}

#[tokio::test]
async fn clickstack_alert_and_webhook_writes_fail_fast_for_oauth() {
    let mock = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("home");
    let ch_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &mock.uri());
    for (resource, body) in [
        (
            "alert",
            r#"{"source":"saved_search","savedSearchId":"s","threshold":1,"interval":"1m","thresholdType":"above","channel":{"type":"email","emailRecipients":[]},"channels":[{"type":"email","emailRecipients":[]}]}"#,
        ),
        (
            "webhook",
            r#"{"name":"hook","service":"generic","url":"https://example.com"}"#,
        ),
    ] {
        let config = directory.path().join(format!("{resource}.json"));
        std::fs::write(&config, body).unwrap();
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        let output = command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .args([
                "cloud",
                "--url",
                &mock.uri(),
                "clickstack",
                resource,
                "create",
                "svc-1",
                "--config-file",
                config.to_str().unwrap(),
                "--org-id",
                "org-1",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4));
        assert!(String::from_utf8_lossy(&output.stderr).contains("read-only"));
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_profile_list_sends_exact_queries_and_preserves_json() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([
        { "profile": "v1-standard-byoc-4", "cpuCores": 4.0, "memoryGi": 16.0 },
        { "profile": "future-profile", "cpuCores": 12.5, "memoryGi": 48.5 }
    ]);
    Mock::given(method("GET"))
        .and(path(SERVICE_PROFILES_PATH))
        .and(query_param("region_id", "us-east-1"))
        .and(query_param("byoc_id", "byoc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-service-profiles"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "profile",
            "list",
            "--region",
            "us-east-1",
            "--byoc-id",
            "byoc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2);
    assert!(
        query
            .iter()
            .any(|(key, value)| key == "region_id" && value == "us-east-1")
    );
    assert!(
        query
            .iter()
            .any(|(key, value)| key == "byoc_id" && value == "byoc-1")
    );
    assert!(
        requests[0]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("Basic ")
    );
}

#[tokio::test]
async fn service_profile_list_omits_byoc_and_accepts_empty_oauth_result() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVICE_PROFILES_PATH))
        .and(query_param("region_id", "eu-west-1"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [],
            "status": 200,
            "requestId": "stub-empty-service-profiles"
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
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "service",
            "profile",
            "list",
            "--region",
            "eu-west-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!([])
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query, vec![("region_id".into(), "eu-west-1".into())]);
}

#[tokio::test]
async fn service_profile_list_renders_sparse_unknown_profiles() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVICE_PROFILES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                { "profile": "future-profile", "memoryGi": 48.5 },
                { "cpuCores": 8.0, "memoryGi": null }
            ],
            "status": 200,
            "requestId": "stub-sparse-service-profiles"
        })))
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "service",
            "profile",
            "list",
            "--region",
            "us-east-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("| Profile        | CPU cores | Memory GiB |"),
        "{stdout}"
    );
    assert!(
        stdout.contains("| future-profile | -         | 48.5       |"),
        "{stdout}"
    );
    assert!(
        stdout.contains("| -              | 8         | -          |"),
        "{stdout}"
    );
}

#[tokio::test]
async fn service_profile_list_routes_auth_errors() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVICE_PROFILES_PATH))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "status": 401,
            "error": "Unauthorized",
            "requestId": "stub-profile-auth"
        })))
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "profile",
            "list",
            "--region",
            "us-east-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Unauthorized\n"
    );
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

    let update = invoke_cli_with_cloud_credentials(
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

#[tokio::test]
async fn byoc_create_rejects_unknown_zone_before_any_request() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
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
            "z",
            "--vpc-cidr-range",
            "10.0.0.0/16",
            "--display-name",
            "production",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid availability zone suffix"));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn byoc_api_errors_keep_auth_classification() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/byoc-forbidden",
        ))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Forbidden",
            "requestId": "stub-byoc-forbidden"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "byoc",
            "delete",
            "byoc-forbidden",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Forbidden\n"
    );
}

#[tokio::test]
async fn byoc_update_human_output_tolerates_sparse_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/byoc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"state": "infra-provisioning"},
            "status": 200,
            "requestId": "stub-sparse-byoc-update"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
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
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("infra-provisioning"));
}

#[tokio::test]
async fn service_create_discovers_and_sends_a_dynamic_byoc_profile() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVICE_PROFILES_PATH))
        .and(query_param("region_id", "us-east-1"))
        .and(query_param("byoc_id", "byoc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{
                "profile": "v1-standard-byoc-4",
                "cpuCores": 4.0,
                "memoryGi": 48.0
            }],
            "status": 200,
            "requestId": "stub-byoc-profiles"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .and(body_json(serde_json::json!({
            "name": "byoc-service",
            "provider": "aws",
            "region": "us-east-1",
            "ipAccessList": [{
                "source": "0.0.0.0/0",
                "description": "Allow all (created by clickhousectl)"
            }],
            "minReplicaMemoryGb": 48.0,
            "maxReplicaMemoryGb": 48.0,
            "profile": "v1-standard-byoc-4",
            "byocId": "byoc-1"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "service": {"id": "22222222-3333-4444-5555-666666666666", "name": "byoc-service"},
                "password": "generated-password"
            },
            "status": 200,
            "requestId": "stub-byoc-service-create"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "create",
            "--name",
            "byoc-service",
            "--profile",
            "v1-standard-byoc-4",
            "--byoc-id",
            "byoc-1",
            "--min-replica-memory-gb",
            "48",
            "--max-replica-memory-gb",
            "48",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
}

#[tokio::test]
async fn service_create_rejects_dynamic_profile_memory_mismatch_before_post() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVICE_PROFILES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{"profile": "v1-standard-byoc-4", "memoryGi": 48.0}],
            "status": 200,
            "requestId": "stub-byoc-profiles"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "create",
            "--name",
            "byoc-service",
            "--profile",
            "v1-standard-byoc-4",
            "--byoc-id",
            "byoc-1",
            "--min-replica-memory-gb",
            "16",
            "--max-replica-memory-gb",
            "16",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("requires both replica memory bounds to equal 48 GiB")
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, wiremock::http::Method::GET);
}

fn invoke_service_settings_with_oauth(
    mock: &MockServer,
    cli_args: &[&str],
) -> std::process::Output {
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url, "--json", "service", "settings"];
    args.extend(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args(args)
        .output()
        .unwrap()
}

#[tokio::test]
async fn service_settings_read_routes_support_oauth_and_sparse_responses() {
    let mock = MockServer::start().await;
    let collection = "/v1/organizations/org-1/services/svc-1/clickhouseSettings";
    let item = format!("{collection}/compatibility");
    let schema = format!("{collection}/schema");

    Mock::given(method("GET"))
        .and(path(collection))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"settings": [{"name": "compatibility"}, {"value": "1"}, {}]},
            "status": 200,
            "requestId": "stub-settings-list"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(&item))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"value": "24.8"},
            "status": 200,
            "requestId": "stub-setting-get"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(&schema))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"settings": [
                {"name": "compatibility", "enum": [0, 1]},
                {"description": "future setting"},
                {}
            ]},
            "status": 200,
            "requestId": "stub-settings-schema"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let list = invoke_service_settings_with_oauth(&mock, &["list", "svc-1", "--org-id", "org-1"]);
    assert_success(&list);
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        serde_json::json!({"settings": [
            {"name": "compatibility"}, {"value": "1"}, {}
        ]})
    );

    let get = invoke_service_settings_with_oauth(
        &mock,
        &["get", "svc-1", "compatibility", "--org-id", "org-1"],
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        serde_json::json!({"value": "24.8"})
    );

    let schema_output =
        invoke_service_settings_with_oauth(&mock, &["schema", "svc-1", "--org-id", "org-1"]);
    assert_success(&schema_output);
    assert_eq!(
        serde_json::from_slice::<Value>(&schema_output.stdout).unwrap(),
        serde_json::json!({"settings": [
            {"name": "compatibility", "enum": [0, 1]},
            {"description": "future setting"},
            {}
        ]})
    );
}

#[tokio::test]
async fn service_settings_set_and_unset_send_exact_requests_with_api_key_auth() {
    let mock = MockServer::start().await;
    let collection = "/v1/organizations/org-1/services/svc-1/clickhouseSettings";
    let item = format!("{collection}/compatibility");
    Mock::given(method("PATCH"))
        .and(path(collection))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .and(body_json(serde_json::json!({
            "settings": "{\"compatibility\":\"24.8\",\"enable_analyzer\":1}"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "settings": "{\"compatibility\":\"24.8\",\"enable_analyzer\":1}",
                "warnings": [{"name": "compatibility"}]
            },
            "status": 200,
            "requestId": "stub-settings-set"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(&item))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {},
            "status": 200,
            "requestId": "stub-setting-unset"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let set = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "settings",
            "set",
            "svc-1",
            "--setting",
            "compatibility=\"24.8\"",
            "--setting",
            "enable_analyzer=1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&set);
    assert_eq!(
        serde_json::from_slice::<Value>(&set.stdout).unwrap(),
        serde_json::json!({
            "settings": "{\"compatibility\":\"24.8\",\"enable_analyzer\":1}",
            "warnings": [{"name": "compatibility"}]
        })
    );

    let unset = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "settings",
            "unset",
            "svc-1",
            "compatibility",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&unset);
    assert_eq!(
        serde_json::from_slice::<Value>(&unset.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "stub-setting-unset"})
    );
}

#[tokio::test]
async fn service_settings_set_reads_a_map_from_stdin_and_rejects_bad_json_before_http() {
    let mock = MockServer::start().await;
    let collection = "/v1/organizations/org-1/services/svc-1/clickhouseSettings";
    Mock::given(method("PATCH"))
        .and(path(collection))
        .and(body_json(serde_json::json!({
            "settings": "{\"bool_value\":false,\"future_setting\":{\"nested\":true}}"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"warnings": []},
            "status": 200,
            "requestId": "stub-settings-stdin"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let url = mock.uri();
    let mut child = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .args([
            "cloud",
            "--url",
            &url,
            "--json",
            "service",
            "settings",
            "set",
            "svc-1",
            "--settings-file",
            "-",
            "--org-id",
            "org-1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"future_setting":{"nested":true},"bool_value":false}"#)
        .unwrap();
    let valid = child.wait_with_output().unwrap();
    assert_success(&valid);

    let before_bad = mock.received_requests().await.unwrap().len();
    let mut bad_child = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "settings",
            "set",
            "svc-1",
            "--settings-file",
            "-",
            "--org-id",
            "org-1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    bad_child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{not-json")
        .unwrap();
    let bad = bad_child.wait_with_output().unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("invalid JSON in stdin"));
    assert_eq!(
        mock.received_requests().await.unwrap().len(),
        before_bad,
        "invalid stdin must fail before HTTP"
    );
}

#[tokio::test]
async fn service_settings_api_errors_keep_auth_classification() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickhouseSettings/compatibility",
        ))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Forbidden",
            "requestId": "stub-settings-forbidden"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "settings",
            "get",
            "svc-1",
            "compatibility",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Forbidden\n"
    );
}

#[tokio::test]
async fn service_settings_schema_human_output_renders_nested_sparse_entries() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickhouseSettings/schema",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"settings": [
                {
                    "name": "compatibility",
                    "type": "string",
                    "description": "Compatibility version",
                    "enum": [0, 1]
                },
                {"warning": "future warning"},
                {}
            ]},
            "status": 200,
            "requestId": "stub-settings-schema-human"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "service", "settings", "schema", "svc-1", "--org-id", "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("settings:\n  - description: Compatibility version"));
    assert!(stdout.contains("enum: [0, 1]"));
    assert!(stdout.contains("warning: future warning"));
}
