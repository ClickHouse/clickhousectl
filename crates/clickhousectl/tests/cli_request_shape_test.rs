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
const SCALING_SCHEDULE_PATH: &str = "/v1/organizations/org-1/services/svc-1/scalingSchedule";

/// Inspect the real help path without ambient credentials or a writable real home.
fn permission_help(mock: &MockServer, command: &[&str], help_flag: &str) -> String {
    let project = tempfile::tempdir().unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
        .current_dir(project.path())
        .args(["cloud", "--url", &mock.uri()])
        .args(command)
        .arg(help_flag)
        .output()
        .expect("run permission help");
    assert_success(&output);
    String::from_utf8(output.stdout).unwrap()
}

fn advertised_permission_ids(help: &str) -> std::collections::BTreeSet<&str> {
    help.split(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, ':' | '_' | '-'))
        .filter(|word| word.starts_with("control-plane:"))
        .collect()
}

fn permission_path_matches(template: &str, actual: &str) -> bool {
    let template: Vec<_> = template.split('/').collect();
    let actual: Vec<_> = actual.split('/').collect();
    template.len() == actual.len()
        && template.iter().zip(actual).all(|(expected, actual)| {
            *expected == actual
                || (expected.starts_with('{') && expected.ends_with('}') && !actual.is_empty())
        })
}

/// Check observed HTTP calls rather than duplicating the command's declared list.
async fn assert_observed_permissions_advertised(mock: &MockServer, help: &str) {
    let advertised = advertised_permission_ids(help);
    let requests = mock.received_requests().await.unwrap();
    assert!(
        !requests.is_empty(),
        "call-path coverage must not be vacuous"
    );
    for request in requests {
        let operations: Vec<_> = clickhouse_cloud_api::meta::operations::ALL
            .iter()
            .filter(|operation| {
                operation.method == request.method.as_str()
                    && permission_path_matches(operation.path, request.url.path())
            })
            .collect();
        assert_eq!(
            operations.len(),
            1,
            "unmapped or ambiguous observed operation: {} {}",
            request.method,
            request.url.path()
        );
        for permission in operations[0].required_permissions {
            assert!(
                advertised.contains(permission),
                "help omitted {permission} needed by observed {}",
                operations[0].operation_id
            );
        }
    }
}

#[tokio::test]
async fn permission_help_is_offline_and_exposes_multi_permission_operations() {
    let mock = MockServer::start().await;
    for flag in ["-h", "--help"] {
        let help = permission_help(&mock, &["postgres", "read-replica", "create"], flag);
        let advertised = advertised_permission_ids(&help);
        let required =
            clickhouse_cloud_api::meta::operations::POSTGRES_INSTANCE_CREATE_READ_REPLICA
                .required_permissions;
        assert!(required.len() > 1, "fixture must exercise conjunctions");
        for permission in required {
            assert!(advertised.contains(permission), "missing {permission}");
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn permission_help_does_not_attach_leaf_permissions_to_parent_groups() {
    let mock = MockServer::start().await;
    for command in [&["service"][..], &["postgres", "read-replica"][..]] {
        let help = permission_help(&mock, command, "--help");
        assert!(advertised_permission_ids(&help).is_empty());
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn permission_help_covers_force_delete_and_managed_query_key_cleanup_calls() {
    let mock = MockServer::start().await;
    let help = permission_help(&mock, &["service", "delete"], "--help");
    assert!(mock.received_requests().await.unwrap().is_empty());
    let service_path = format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}");
    Mock::given(method("GET"))
        .and(path(&service_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {"id": DELETE_TEST_SERVICE_ID, "state": "stopped"},
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(&service_path))
        .respond_with(successful_delete_response("permission-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}"
        )))
        .respond_with(successful_delete_response("permission-key-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    let project = tempfile::tempdir().unwrap();
    write_service_query_key(project.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "--org-id",
            "org-1",
            "service",
            "delete",
            DELETE_TEST_SERVICE_ID,
            "--force",
        ])
        .output()
        .expect("run compound service deletion");
    assert_success(&output);
    assert_eq!(mock.received_requests().await.unwrap().len(), 3);
    assert_observed_permissions_advertised(&mock, &help).await;
}

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

/// Turns off the CLI's background update check, which would otherwise send
/// a real request to the release server and wait up to 400 ms for it.
const UPDATE_CHECK_TIMEOUT_ENV: &str = "CHCTL_UPDATE_CHECK_TIMEOUT_MS";

/// `env_clear`, keeping only what every spawned CLI needs: no background
/// update check (see [`UPDATE_CHECK_TIMEOUT_ENV`]).
trait IsolatedEnv {
    fn isolated_env(&mut self) -> &mut Self;
}

impl IsolatedEnv for Command {
    fn isolated_env(&mut self) -> &mut Self {
        self.env_clear().env(UPDATE_CHECK_TIMEOUT_ENV, "0")
    }
}

impl IsolatedEnv for tokio::process::Command {
    fn isolated_env(&mut self) -> &mut Self {
        self.env_clear().env(UPDATE_CHECK_TIMEOUT_ENV, "0")
    }
}

fn clear_inherited_env(command: &mut Command) {
    command.isolated_env();
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .isolated_env()
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
        .isolated_env()
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

/// A saved key pair for `api_url`, the mock the test sends it to (#1202).
fn write_project_api_credentials(root: &Path, key: &str, secret: &str, api_url: &str) {
    let credentials_dir = root.join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(
        credentials_dir.join("credentials.json"),
        serde_json::to_vec(&serde_json::json!({
            "api_key": key,
            "api_secret": secret,
            "api_url": api_url,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn invoke_api_key_login(project_dir: &Path, url: &str) -> std::process::Output {
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
            "--url",
            url,
        ])
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn api_key_login_saves_a_nonempty_pair_and_status_selects_it() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let output = invoke_api_key_login(dir.path(), &mock.uri());
    assert_success(&output);
    let saved: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["api_key"], "new-key");
    assert_eq!(saved["api_secret"], "new-secret");
    // The key was verified against a non-prod API, so that API is saved with
    // it and later commands go there without `--url` (#1202).
    let api_url = format!("{}/v1", mock.uri());
    assert_eq!(saved["api_url"], api_url.as_str());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .current_dir(dir.path())
        .args(["cloud", "auth", "status", "--json"])
        .output()
        .unwrap();
    assert_success(&output);
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["verification"], "verified");
    let rows = status["sources"].as_array().unwrap();
    let saved = rows.iter().find(|row| row["type"] == "API key").unwrap();
    assert_eq!(saved["status"], "Active");
    assert_eq!(saved["scope"], "read/write");
    assert_eq!(saved["active"], "yes");
    assert_eq!(saved["api_url"], api_url.as_str());
}

/// #1202: a command without `--url` uses the API saved with the key and says
/// so on stderr; a `--url` naming another API is refused with exit 4 before
/// the key is sent anywhere, even with env credentials to fall back to.
#[tokio::test]
async fn saved_key_is_sent_only_to_its_saved_api() {
    let saved_api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .expect(1)
        .mount(&saved_api)
        .await;
    let other_api = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    write_project_api_credentials(dir.path(), "file-key", "file-secret", &saved_api.uri());
    let command = |args: &[&str]| {
        Command::new(clickhousectl_binary())
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap()
    };

    let output = command(&["cloud", "auth", "whoami"]);
    assert_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&saved_api.uri()), "stderr: {stderr}");

    let other_url = other_api.uri();
    for json in [true, false] {
        let mut args = vec!["cloud", "--url", &other_url, "auth", "whoami"];
        if json {
            args.push("--json");
        }
        let output = command(&args);
        assert_eq!(output.status.code(), Some(4), "json={json}");
        if json {
            assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
        }
    }
    assert!(other_api.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_never_selects_empty_saved_credentials() {
    let mock = MockServer::start().await;
    // The empty-field forms are `client::empty_saved_credentials_do_not_shadow_usable_sources`.
    for (key, secret, with_env) in [("", "saved-secret", false), ("saved-key", "", true)] {
        let dir = tempfile::tempdir().unwrap();
        write_project_api_credentials(dir.path(), key, secret, &mock.uri());
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .current_dir(dir.path())
            .args(["cloud", "auth", "status", "--json", "--url", &mock.uri()]);
        if with_env {
            command.env("CLICKHOUSE_CLOUD_API_KEY", "env-key");
            command.env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let status: Value = serde_json::from_slice(&output.stdout).unwrap();
        let rows = status["sources"].as_array().unwrap();
        let saved = rows.iter().find(|row| row["type"] == "API key").unwrap();
        assert!(saved["status"].as_str().unwrap().starts_with("Incomplete"));
        assert_eq!(saved["scope"], "-");
        assert_eq!(saved["active"], "-");
        let active: Vec<_> = rows.iter().filter(|row| row["active"] == "yes").collect();
        if with_env {
            assert_eq!(active.len(), 1);
            assert_eq!(active[0]["type"], "Env vars");
        } else {
            assert!(active.is_empty());
        }
    }
}

#[tokio::test]
async fn api_key_login_preserves_malformed_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    let credentials_path = credentials_dir.join("credentials.json");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    let original = b"{malformed credentials\n";
    std::fs::write(&credentials_path, original).unwrap();

    let output = invoke_api_key_login(dir.path(), &mock.uri());

    assert_eq!(output.status.code(), Some(1));
    // The parser's own diagnostic for the file, naming it.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let source = serde_json::from_slice::<Value>(original).unwrap_err();
    assert!(
        stderr.starts_with("Error: ")
            && stderr.contains(".clickhouse/credentials.json")
            && stderr.contains(&source.to_string()),
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
    assert!(output.stdout.is_empty(), "no success confirmation");
    assert!(credentials_dir.join("credentials.json").is_dir());
}

// ── Backup configuration validation (issue #425) ────────────────────────────

#[tokio::test]
async fn backup_list_uses_utc_created_times_only_in_human_output() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([
        {
            "id": "11111111-2222-3333-4444-555555555555",
            "sizeInBytes": 1024,
            "startedAt": "2026-09-15T08:34:56.123Z"
        },
        {}
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-backup-list"
        })))
        .expect(2)
        .mount(&mock)
        .await;

    let args = ["backup", "list", "svc-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("| Created"), "{stdout}");
    assert!(stdout.contains("2026-09-15T08:34:56.123Z"), "{stdout}");
    assert!(!stdout.contains("+00:00"), "{stdout}");
    assert!(
        stdout.lines().any(|line| {
            line.split('|').map(str::trim).collect::<Vec<_>>() == ["", "-", "-", "-", "-", ""]
        }),
        "{stdout}"
    );

    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!([
            {
                "id": "11111111-2222-3333-4444-555555555555",
                "sizeInBytes": 1024.0,
                "startedAt": "2026-09-15T08:34:56.123Z"
            },
            {}
        ])
    );
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
    // The --config-file and --config aliases are clap's; see
    // `primary_json_file_argument_contract` in `cloud::backups`.
    backup_bucket_create_reads_secret_from_stdin_and_sends_exact_gcp_body_with_file_flag("--file")
        .await;
}

async fn backup_bucket_create_reads_secret_from_stdin_and_sends_exact_gcp_body_with_file_flag(
    file_flag: &str,
) {
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
            "backup", "bucket", "create", "svc-1", "--org-id", "org-1", file_flag, "-",
        ],
        serde_json::to_string(&expected).unwrap().as_bytes(),
    )
    .await;
    assert_eq!(body, expected);
}

#[tokio::test]
async fn backup_bucket_update_reads_file_and_delete_use_the_service_resource_path() {
    backup_bucket_update_reads_file_and_delete_use_the_service_resource_path_with_file_flag(
        "--file",
    )
    .await;
}

async fn backup_bucket_update_reads_file_and_delete_use_the_service_resource_path_with_file_flag(
    file_flag: &str,
) {
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
            file_flag,
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
        cloud_runtime_error(&output)["message"],
        "the stored backup period is 12 hours, but --backup-start-time requires 24 or 48. \
         Pass --backup-period-hours 24 or --backup-period-hours 48 in the same call."
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "other");

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
    assert_eq!(cloud_runtime_error(&output)["message"], "Unauthorized");
    assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
}

#[tokio::test]
async fn dispatched_cloud_403_is_a_request_refusal_not_auth_required() {
    // A 403 refuses the request (quota, precondition, policy, grant), which
    // logging in again cannot fix, so it is not `auth_required` (#1225).
    let output = invoke_service_list_api_error(403, "Forbidden").await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(cloud_runtime_error(&output)["message"], "Forbidden");
    assert_eq!(cloud_runtime_error(&output)["code"], "http_4xx");
}

#[tokio::test]
async fn dispatched_cloud_500_remains_a_generic_error() {
    let output = invoke_service_list_api_error(500, "Internal Server Error").await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        cloud_runtime_error(&output)["message"],
        "Internal Server Error"
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "http_5xx");
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
    // How explicit origins are carried is the builder's rule; one value is
    // enough to show the 404-then-create path and its permissions.
    let origins = "https://app.example.com";
    let mock = manual_query_endpoint_mock(ResponseTemplate::new(404)).await;
    let help = permission_help(&mock, &["service", "query-endpoint", "create"], "--help");
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
    assert_observed_permissions_advertised(&mock, &help).await;
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

/// The read-modify-write keeps existing keys unless the flag says otherwise;
/// this is the only run where `--replace-open-api-keys` reaches the handler
/// with an existing endpoint.
#[tokio::test]
async fn query_endpoint_create_key_replacement_is_explicit() {
    let mock = manual_query_endpoint_mock(manual_query_endpoint_response(serde_json::json!({
        "openApiKeys": ["existing-1", "existing-2"],
        "allowedOrigins": "https://before.example.com",
    })))
    .await;
    let body = invoke_cli_capture_body(
        &mock,
        &manual_query_endpoint_args(&["--open-api-key", "new-key", "--replace-open-api-keys"]),
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
            ResponseTemplate::new(401),
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

fn invoke_service_wake(mock: &MockServer, json: bool) -> std::process::Output {
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

    let output = invoke_service_wake(&mock, true);
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

    let output = invoke_service_wake(&mock, false);
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Service - waking (state: -)\n"
    );
}

#[tokio::test]
async fn service_start_and_stop_human_output_reports_acceptance_and_returned_state() {
    // One run per command, one with and one without a returned state; the
    // `-` placeholder itself is `or_absent`'s.
    for (command, request_command, returned_state) in
        [("start", "start", Some("starting")), ("stop", "stop", None)]
    {
        let mock = MockServer::start().await;
        let mut result = serde_json::json!({"name": "demo"});
        if let Some(state) = returned_state {
            result["state"] = serde_json::json!(state);
        }
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/services/svc-1/state"))
            .and(body_json(serde_json::json!({"command": request_command})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-service-state-change",
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let output = invoke_cli_with_cloud_credentials_human(
            &mock,
            &["service", command, "svc-1", "--org-id", "org-1"],
        );

        // One line naming the service, the command and the returned state.
        assert_success(&output);
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.lines().count(), 1, "{stdout}");
        for identifier in ["demo", command, returned_state.unwrap_or("-")] {
            assert!(stdout.contains(identifier), "{identifier}: {stdout}");
        }
    }
}

/// `service scale` wiring: one PATCH to the service's replicaScaling route,
/// carrying every flag in its own field; JSON mode prints the response.
#[tokio::test]
async fn service_scale_sends_the_builder_body_to_the_replica_scaling_route() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "name": "demo",
        "autoscalingMode": "vertical",
        "minReplicaMemoryGb": 8.0,
        "maxReplicaMemoryGb": 32.0,
        "numReplicas": 3,
    });
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/replicaScaling",
        ))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result, "status": 200, "requestId": "stub-service-scale",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "scale",
            "svc-1",
            "--autoscaling-mode",
            "vertical",
            "--min-replica-memory-gb",
            "8",
            "--max-replica-memory-gb",
            "32",
            "--num-replicas",
            "3",
            "--idle-scaling",
            "false",
            "--idle-timeout-minutes",
            "15",
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
    assert_eq!(
        requests[0].body_json::<Value>().unwrap(),
        serde_json::json!({
            "autoscalingMode": "vertical",
            "minReplicaMemoryGb": 8.0,
            "maxReplicaMemoryGb": 32.0,
            "numReplicas": 3,
            "idleScaling": false,
            "idleTimeoutMinutes": 15.0,
        })
    );
}

#[tokio::test]
async fn service_start_and_stop_preserve_json_responses() {
    for (command, returned_state) in [("start", "running"), ("stop", "stopped")] {
        let mock = MockServer::start().await;
        let result = serde_json::json!({"name": "demo", "state": returned_state});
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/services/svc-1/state"))
            .and(body_json(serde_json::json!({"command": command})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-service-state-change-json",
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["service", command, "svc-1", "--org-id", "org-1"],
        );

        assert_success(&output);
        assert!(output.stderr.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            result
        );
    }
}

#[tokio::test]
async fn scaling_schedule_get_uses_oauth_and_preserves_the_full_response() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "activeEntryId": "11111111-2222-3333-4444-555555555555",
        "baseConfig": {
            "autoscalingMode": "vertical",
            "minReplicaMemoryGb": 8.0,
            "maxReplicaMemoryGb": 32.0,
            "minReplicas": 1,
            "maxReplicas": 1,
            "idleScaling": true,
            "idleTimeoutMinutes": 10
        },
        "entries": [{
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "weekday traffic",
            "weekdays": [1, 2, 3, 4, 5],
            "startHourUtc": 8,
            "endHourUtc": 20,
            "autoscalingMode": "horizontal",
            "minReplicaMemoryGb": 16.0,
            "maxReplicaMemoryGb": 16.0,
            "minReplicas": 2,
            "maxReplicas": 8,
            "idleScaling": false,
            "idleTimeoutMinutes": 0,
            "isActiveNow": true
        }]
    });
    Mock::given(method("GET"))
        .and(path(SCALING_SCHEDULE_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-scaling-schedule-get"
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
            "service",
            "scaling-schedule",
            "get",
            "svc-1",
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
    assert!(requests[0].url.query().is_none());
}

#[tokio::test]
async fn scaling_schedule_set_reads_stdin_and_sends_the_exact_complete_body() {
    let mock = MockServer::start().await;
    let request = serde_json::json!({
        "entries": [
            {
                "name": "minimal",
                "weekdays": [0],
                "startHourUtc": 0,
                "endHourUtc": 24
            },
            {
                "name": "horizontal",
                "weekdays": [1, 2, 3, 4, 5],
                "startHourUtc": 8,
                "endHourUtc": 20,
                "autoscalingMode": "horizontal",
                "idleScaling": false,
                "idleTimeoutMinutes": 0,
                "minReplicaMemoryGb": 16.0,
                "maxReplicaMemoryGb": 16.0,
                "minReplicas": 2,
                "maxReplicas": 8
            }
        ]
    });
    let response = serde_json::json!({"entries": [{"name": "minimal"}], "baseConfig": {}});
    Mock::given(method("POST"))
        .and(path(SCALING_SCHEDULE_PATH))
        .and(wiremock::matchers::basic_auth(
            "schedule-test-key",
            "schedule-test-secret",
        ))
        .and(body_json(request.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": response,
            "status": 200,
            "requestId": "stub-scaling-schedule-set"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let mut child = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "schedule-test-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "schedule-test-secret")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "service",
            "scaling-schedule",
            "set",
            "svc-1",
            "--file",
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
        .write_all(serde_json::to_string(&request).unwrap().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        response
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].url.query().is_none());
}

#[tokio::test]
async fn scaling_schedule_delete_uses_the_exact_route_and_preserves_metadata() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(SCALING_SCHEDULE_PATH))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-scaling-schedule-delete"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "scaling-schedule",
            "delete",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "stub-scaling-schedule-delete"})
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].url.query().is_none());
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
    write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
        .current_dir(dir.path())
        .args(["cloud", "--url", &url, "org", "list"])
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

/// #677: credential-precedence and debug notices are emitted before dispatch.
/// A reader may close stderr before either notice is written; the command must
/// still reach the API and report its outcome through the exit status.
#[tokio::test]
async fn cloud_command_survives_closed_stderr_with_credential_notice_and_debug() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [],
            "status": 200,
            "requestId": "stub-org-list",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let mut child = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
        .current_dir(dir.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "--debug",
            "org",
            "list",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");

    drop(child.stderr.take().expect("stderr was piped"));
    let status = child.wait().expect("failed to wait for clickhousectl");

    assert_eq!(
        status.code(),
        Some(0),
        "a closed stderr must not prevent cloud command dispatch"
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), "/v1/organizations".to_string())]
    );
}

#[tokio::test]
async fn auth_status_explicit_flags_win_without_revealing_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    // Both output formats render the same rows, so each runs once.
    for (configure_other_sources, json) in [(false, true), (true, false)] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        if configure_other_sources {
            write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
            let token_dir = home.join(".clickhouse");
            std::fs::create_dir(&token_dir).unwrap();
            write_oauth_tokens(&token_dir, &mock.uri());
        }
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(dir.path())
            .args([
                "cloud",
                "auth",
                "status",
                "--api-key",
                "flag-key",
                "--api-secret",
                "flag-secret",
                "--url",
                &mock.uri(),
                "--debug",
            ]);
        if configure_other_sources {
            command.env("CLICKHOUSE_CLOUD_API_KEY", "env-key");
            command.env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret");
        }
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        for credential in [
            "flag-key",
            "flag-secret",
            "file-key",
            "file-secret",
            "env-key",
            "env-secret",
            "test-bearer-token",
            "unused",
        ] {
            assert!(!stdout.contains(credential), "credential exposed on stdout");
            assert!(!stderr.contains(credential), "credential exposed on stderr");
        }
        assert!(stderr.contains("CLI flags"), "{stderr}");
        // Compare the same source, scope and selection fields in both formats.
        let rows: Vec<(String, String, String, String)> = if json {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], "verified");
            assert_eq!(status["identity"]["keyId"], "key-123");
            status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    (
                        row["type"].as_str().unwrap().to_owned(),
                        row["status"].as_str().unwrap().to_owned(),
                        row["scope"].as_str().unwrap().to_owned(),
                        row["active"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        } else {
            assert!(stdout.contains("Identity:\n"), "{stdout}");
            assert!(stdout.contains("key-123"), "{stdout}");
            stdout
                .lines()
                .skip(2)
                .take_while(|line| !line.is_empty())
                .map(|line| {
                    let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
                    assert_eq!(cells.len(), 5, "{line}");
                    (
                        cells[0].into(),
                        cells[1].into(),
                        cells[2].into(),
                        cells[3].into(),
                    )
                })
                .collect()
        };
        assert_eq!(rows.len(), 4);
        assert_eq!(rows.iter().filter(|row| row.3 == "yes").count(), 1);
        let flags = rows.iter().find(|row| row.0 == "CLI flags").unwrap();
        assert_eq!(flags.1, "Active");
        assert_eq!(flags.2, "read/write");
        assert_eq!(flags.3, "yes");
        for source in ["API key", "Env vars", "OAuth"] {
            let row = rows.iter().find(|row| row.0 == source).unwrap();
            assert_eq!(row.3, "-");
            if configure_other_sources {
                assert!(row.1.starts_with("Configured"), "{row:?}");
                assert_eq!(
                    row.2,
                    if source == "OAuth" {
                        "read-only"
                    } else {
                        "read/write"
                    }
                );
            } else {
                assert_eq!(row.1, "Not configured");
                assert_eq!(row.2, "-");
            }
        }
    }
    // The identity check uses the winning source: the explicit flags.
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert_eq!(request.url.path(), "/v1/whoami");
        assert_eq!(
            request.headers.get("authorization").unwrap(),
            "Basic ZmxhZy1rZXk6ZmxhZy1zZWNyZXQ=" // flag-key:flag-secret
        );
    }
}

#[tokio::test]
async fn auth_status_incomplete_flags_explain_missing_credential_without_falling_back() {
    let mock = MockServer::start().await;
    // Each configured source once, rotating which half of the pair is missing
    // and the output format.
    for (configured_source, (supplied, missing), json) in [
        (None, ("--api-key", "--api-secret"), false),
        (Some("API key"), ("--api-secret", "--api-key"), true),
        (Some("Env vars"), ("--api-key", "--api-secret"), true),
        (Some("OAuth"), ("--api-secret", "--api-key"), false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        if configured_source == Some("API key") {
            write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
        }
        if configured_source == Some("OAuth") {
            let token_dir = home.join(".clickhouse");
            std::fs::create_dir(&token_dir).unwrap();
            write_oauth_tokens(&token_dir, &mock.uri());
        }
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(dir.path())
            .args([
                "cloud",
                "auth",
                "status",
                supplied,
                "flag-credential",
                "--url",
                &mock.uri(),
                "--debug",
            ]);
        if configured_source == Some("Env vars") {
            command.env("CLICKHOUSE_CLOUD_API_KEY", "env-key");
            command.env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret");
        }
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        // One debug line naming the missing flag.
        let context =
            format!("supplied={supplied}, configured_source={configured_source:?}, json={json}");
        assert_eq!(stderr.lines().count(), 1, "{context}: {stderr}");
        assert!(stderr.starts_with("[debug] "), "{context}: {stderr}");
        assert!(stderr.contains(missing), "{context}: {stderr}");
        for credential in [
            "flag-credential",
            "file-key",
            "file-secret",
            "env-key",
            "env-secret",
            "test-bearer-token",
            "unused",
        ] {
            assert!(!stdout.contains(credential), "credential exposed on stdout");
            assert!(!stderr.contains(credential), "credential exposed on stderr");
        }
        let rows: Vec<Vec<String>> = if json {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], "skipped");
            assert!(status["identity"].is_null());
            status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    ["type", "status", "scope", "active"]
                        .map(|field| row[field].as_str().unwrap().to_owned())
                        .to_vec()
                })
                .collect()
        } else {
            assert!(!stdout.contains("Identity"), "{stdout}");
            stdout
                .lines()
                .skip(2)
                .map(|line| {
                    // The fifth column, the API URL, has no JSON counterpart
                    // for these rows.
                    let cells: Vec<String> = line
                        .trim_matches('|')
                        .split('|')
                        .map(|cell| cell.trim().to_owned())
                        .collect();
                    assert_eq!(cells.len(), 5, "{line}");
                    cells.into_iter().take(4).collect()
                })
                .collect()
        };
        assert_eq!(rows.len(), 4);
        for row in rows {
            assert_eq!(row.len(), 4);
            assert_eq!(row[3], "-", "no source should be active: {row:?}");
            if row[0] == "CLI flags" {
                assert_eq!(row[1], format!("Incomplete (missing {missing})"));
                assert_eq!(row[2], "-");
            } else if Some(row[0].as_str()) == configured_source {
                assert_eq!(row[1], "Configured (inactive)");
                assert_eq!(
                    row[2],
                    if row[0] == "OAuth" {
                        "read-only"
                    } else {
                        "read/write"
                    }
                );
            } else {
                assert_eq!(row[1], "Not configured");
                assert_eq!(row[2], "-");
            }
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_marks_outranked_environment_credentials_inactive() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
        .current_dir(dir.path())
        .args(["cloud", "--json", "--url", &mock.uri(), "auth", "status"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);
    assert!(output.stderr.is_empty());
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    let env = status["sources"]
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
async fn service_delete_conflict_suggests_force_without_inspecting_prose() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "status": 409,
            "error": "opaque conflict response"
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
        cloud_runtime_error(&output)["message"],
        "service could not be deleted because of a conflict. If it is running, use \
         --force to stop it first, or `clickhousectl cloud service stop svc-1`. API response: \
         opaque conflict response"
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "http_4xx");
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
        .and(query_param("from_date", "2026-04-16T11:00:00.000Z"))
        .and(query_param("to_date", "2026-04-16T12:00:00.000Z"))
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
        .isolated_env()
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
                "2026-04-16T11:00:00.000Z".to_string()
            ),
            (
                "to_date".to_string(),
                "2026-04-16T12:00:00.000Z".to_string()
            ),
            ("bucket_size_seconds".to_string(), "60".to_string()),
        ]
    );
}

/// The metrics handler, not clap, checks the range order
/// (`validate_datetime_range`), so a reverse range must stop before any request.
#[tokio::test]
async fn postgres_metrics_handler_rejects_a_reverse_range_before_any_request() {
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T13:00:00Z",
            "--to-date",
            "2026-04-16T12:00:00Z",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_metrics_omits_bucket_and_renders_every_sparse_series_and_point() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/metrics"))
        .and(query_param("from_date", "2026-04-16T12:00:00.000Z"))
        .and(query_param("to_date", "2026-04-16T13:00:00.000Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "metrics": [{
                    "key": "connections",
                    "name": "Open connections",
                    "description": "Connections by role",
                    "unit": "connections",
                    "series": [{
                        "label": "{role=\"primary\",zone=\"eu-west-1\"}",
                        "dataPoints": [
                            { "timestamp": 1776337200, "value": 4 },
                            { "timestamp": 1776337200 }
                        ]
                    }, {
                        "label": "custom label / tenant=a,b",
                        "dataPoints": []
                    }, {}]
                }, {
                    "key": "empty-series",
                    "series": []
                }, {
                    "key": "absent-series"
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
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "metrics:\n\
         \x20 - description: Connections by role\n\
         \x20   key: connections\n\
         \x20   name: Open connections\n\
         \x20   series:\n\
         \x20     - dataPoints:\n\
         \x20         1: 2026-04-16T11:00:00Z  4 connections\n\
         \x20         2: 2026-04-16T11:00:00Z  -\n\
         \x20       label: {role=\"primary\",zone=\"eu-west-1\"}\n\
         \x20     - dataPoints: []\n\
         \x20       label: custom label / tenant=a,b\n\
         \x20     - dataPoints: -\n\
         \x20   unit: connections\n\
         \x20 - key: empty-series\n\
         \x20   series: []\n\
         \x20 - key: absent-series\n\
         \x20   series: -\n"
    );
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2, "unexpected query parameters: {query:?}");
    assert!(
        query.iter().all(|(key, _)| key != "bucket_size_seconds"),
        "bucket size must be omitted when the flag is absent: {query:?}"
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
            "body": "{\"message\":\"checkpoint complete\",\"detail\":\"all buffers written\"}"
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
            "result": [
                {
                    "timestamp": "2026-08-01T12:00:00Z",
                    "severity": "LOG",
                    "body": "{\"message\":\"checkpoint complete\",\"detail\":\"all buffers written\"}"
                },
                { "severity": "WARNING", "body": "recovery complete" },
                { "body": "{\"message\":42,\"detail\":\"kept\"}" },
                {}
            ],
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
    for heading in ["Timestamp", "Severity", "Message"] {
        assert!(stdout.contains(heading), "{heading} missing from: {stdout}");
    }
    assert!(stdout.contains("2026-08-01T12:00:00Z"), "{stdout}");
    assert!(stdout.contains("checkpoint complete"), "{stdout}");
    assert!(!stdout.contains("all buffers written"), "{stdout}");
    assert!(stdout.contains("recovery complete"), "{stdout}");
    assert!(
        stdout.contains(r#"{"message":42,"detail":"kept"}"#),
        "{stdout}"
    );
    let absent_row = stdout
        .lines()
        .rfind(|line| line.contains('|'))
        .expect("missing final table row");
    assert_eq!(absent_row.matches('-').count(), 3, "{absent_row}");
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.query(),
        Some("from_date=2026-08-01T00%3A00%3A00Z&to_date=2026-08-02T00%3A00%3A00Z")
    );
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
        "totalDurationUs": 950000.25,
        "avgDurationUs": 1190.8419405320817,
        "maxDurationUs": 81000.75,
        "p50DurationUs": 18000.25,
        "p95DurationUs": 70000.5,
        "p99DurationUs": 80000.125,
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
        .isolated_env()
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
                "2026-04-16T11:00:00.000Z".to_string()
            ),
            (
                "to_date".to_string(),
                "2026-04-16T12:00:00.000Z".to_string()
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
            "result": [
                {
                    "queryId": "query-1",
                    "queryText": "SELECT * FROM events WHERE id = $1",
                    "callCount": 2,
                    "avgDurationUs": 1190.8419405320817,
                    "totalDurationUs": 2381.6838810641634,
                    "p95DurationUs": 2000.25
                },
                {
                    "queryId": "query-2",
                    "totalDurationUs": 0.125
                }
            ],
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
    for heading in [
        "Query ID",
        "Query",
        "Calls",
        "Avg duration (µs)",
        "Total duration (µs)",
    ] {
        assert!(
            stdout.contains(heading),
            "missing {heading:?} from:\n{stdout}"
        );
    }
    for value in [
        "query-1",
        "SELECT * FROM events WHERE id = $1",
        "1190.8419405320817",
        "2381.6838810641634",
        "query-2",
        "0.125",
    ] {
        assert!(stdout.contains(value), "missing {value:?} from:\n{stdout}");
    }
    let cells = |id: &str| -> Vec<String> {
        stdout
            .lines()
            .find(|line| line.contains(id))
            .unwrap_or_else(|| panic!("missing {id} row:\n{stdout}"))
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        cells("query-1"),
        [
            "query-1",
            "SELECT * FROM events WHERE id = $1",
            "2",
            "1190.8419405320817",
            "2381.6838810641634"
        ]
    );
    let second_row = stdout
        .lines()
        .find(|line| line.contains("query-2"))
        .expect("second slow-query row");
    assert_eq!(second_row.matches(" - ").count(), 3, "{stdout}");
    assert!(!stdout.contains("p95DurationUs"), "{stdout}");
    let requests = mock.received_requests().await.unwrap();
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(query.len(), 2, "unexpected query parameters: {query:?}");
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
            "callCount": 2,
            "avgDurationUs": 1190.8419405320817,
            "p50DurationUs": 1100.25,
            "p95DurationUs": 1800.5,
            "p99DurationUs": 1950.125
        },
        "recentExecutions": [{
            "queryId": "query-1",
            "queryText": "SELECT 42",
            "timestamp": "2026-04-16T12:30:00Z",
            "durationUs": 1234.56789,
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
        .isolated_env()
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
                "aggregate": { "queryId": "query-1", "avgDurationUs": 1190.8419405320817, "p99DurationUs": 2000.25 },
                "recentExecutions": [{ "timestamp": "2026-04-16T12:30:00Z", "durationUs": 1234.56789, "rows": 1 }]
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
    assert!(
        stdout.contains("avgDurationUs: 1190.8419405320817"),
        "{stdout}"
    );
    assert!(stdout.contains("p99DurationUs: 2000.25"), "{stdout}");
    assert!(stdout.contains("durationUs: 1234.56789"), "{stdout}");
    assert!(stdout.contains("rows: 1"), "{stdout}");
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
        .isolated_env()
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
        .isolated_env()
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

/// Non-tag updates must echo the current tag snapshot alongside the changed
/// fields. Key-only and empty-value tags remain distinct under the API model's
/// optional-value contract; an empty snapshot is the merge unit test.
#[tokio::test]
async fn postgres_update_preserves_tags_when_changing_other_fields() {
    let mock = MockServer::start().await;
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    let postgres_path = format!("/v1/organizations/org-1/postgres/{postgres_id}");
    let tags = serde_json::json!([
        {"key": "env", "value": "prod"},
        {"key": "owner", "value": "me"},
        {"key": "key-only"},
        {"key": "empty", "value": ""}
    ]);
    Mock::given(method("GET"))
        .and(path(&postgres_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": postgres_id, "tags": tags},
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(&postgres_path))
        .and(body_json(serde_json::json!({
            "name": "renamed-pg", "size": "c6gd.large", "haType": "async",
            "deletionProtection": false, "tags": tags
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": postgres_id, "tags": tags},
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
            "--new-name",
            "renamed-pg",
            "--size",
            "c6gd.large",
            "--ha-type",
            "async",
            "--deletion-protection",
            "false",
        ],
    );
    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("GET".into(), postgres_path.clone()),
            ("PATCH".into(), postgres_path)
        ]
    );
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["tags"], tags);
}

#[tokio::test]
async fn postgres_update_without_tag_flags_refuses_incomplete_or_failed_reads() {
    // Which snapshots are incomplete is `merge_response_tags_refuses_*`; here
    // one incomplete and one failed read each stop before the PATCH.
    for response in [
        ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {}})),
        ResponseTemplate::new(500).set_body_json(serde_json::json!({"error": "read failed"})),
    ] {
        let mock = MockServer::start().await;
        let postgres_id = "11111111-2222-3333-4444-555555555555";
        let postgres_path = format!("/v1/organizations/org-1/postgres/{postgres_id}");
        Mock::given(method("GET"))
            .and(path(&postgres_path))
            .respond_with(response)
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
                "--new-name",
                "renamed",
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            received_request_shape(&mock).await,
            vec![("GET".into(), postgres_path)]
        );
    }
}

// ── Postgres IP access rules (issue #1125) ────────────────────────────────

const ACCESS_RULE_POSTGRES_ID: &str = "11111111-2222-3333-4444-555555555555";

fn postgres_access_rules_response() -> Value {
    serde_json::json!([
        {"id": "rule-1", "source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
        {"id": "rule-2", "source": "198.51.100.7/32", "portRange": "5432..6432"}
    ])
}

/// `--block-public-access` sends `[]`, and each `--ip-allow` becomes one rule
/// without an `id`. A trailing `=` sends an empty description, as ClickHouse
/// `--ip-allow` does. Omitting both is in `postgres_config_files_reach_every_write_path`.
#[tokio::test]
async fn postgres_create_sends_access_rules_only_when_asked() {
    let base = serde_json::json!({
        "name": "pg1", "provider": "aws", "region": "us-east-1", "size": "c6gd.large"
    });
    let with_list = |list: Value| {
        let mut body = base.clone();
        body["ipAccessList"] = list;
        body
    };
    for (flags, expected) in [
        (
            vec!["--block-public-access"],
            with_list(serde_json::json!([])),
        ),
        (
            vec![
                "--ip-allow",
                "203.0.113.0/24:5432=office",
                "--ip-allow",
                "198.51.100.7:5432..6432",
                "--ip-allow",
                "192.0.2.1:6432=",
            ],
            with_list(serde_json::json!([
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "198.51.100.7", "portRange": "5432..6432"},
                {"source": "192.0.2.1", "portRange": "6432", "description": ""}
            ])),
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/organizations/org-1/postgres"))
            .and(body_json(expected.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": ACCESS_RULE_POSTGRES_ID,
                    "ipAccessList": postgres_access_rules_response(),
                },
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec![
            "postgres",
            "create",
            "--org-id",
            "org-1",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "c6gd.large",
        ];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output["ipAccessList"], postgres_access_rules_response());
    }
}

/// Rule changes ride the same PATCH as the preserved tag snapshot. A
/// remove-only update sending `add: []` is the builder's unit test.
#[tokio::test]
async fn postgres_update_sends_access_rule_patch() {
    let postgres_path = format!("/v1/organizations/org-1/postgres/{ACCESS_RULE_POSTGRES_ID}");
    let tags = serde_json::json!([{"key": "env", "value": "prod"}]);
    for (flags, patch) in [(
        vec![
            "--add-ip-allow",
            "203.0.113.0/24:5432=office",
            "--remove-ip-allow",
            "rule-9",
            "--add-ip-allow",
            "203.0.113.0/24:6432",
            "--remove-ip-allow",
            "rule-8",
        ],
        serde_json::json!({
            "add": [
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "203.0.113.0/24", "portRange": "6432"}
            ],
            "remove": [{"id": "rule-9"}, {"id": "rule-8"}]
        }),
    )] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&postgres_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"id": ACCESS_RULE_POSTGRES_ID, "tags": tags},
            })))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("PATCH"))
            .and(path(&postgres_path))
            .and(body_json(
                serde_json::json!({"tags": tags, "ipAccessList": patch}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": ACCESS_RULE_POSTGRES_ID,
                    "tags": tags,
                    "ipAccessList": postgres_access_rules_response(),
                },
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec![
            "postgres",
            "update",
            ACCESS_RULE_POSTGRES_ID,
            "--org-id",
            "org-1",
        ];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output["ipAccessList"], postgres_access_rules_response());
    }
}

#[tokio::test]
async fn postgres_get_human_output_shows_access_rules_and_restore_point() {
    let mock = MockServer::start().await;
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{ACCESS_RULE_POSTGRES_ID}"),
        serde_json::json!({
            "id": ACCESS_RULE_POSTGRES_ID,
            "earliestRestoreTime": "2026-10-01T12:00:00Z",
            "ipAccessList": postgres_access_rules_response(),
        }),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "get",
            ACCESS_RULE_POSTGRES_ID,
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2026-10-01T12:00:00"), "{stdout}");
    for rule in postgres_access_rules_response().as_array().unwrap() {
        let line = stdout
            .lines()
            .find(|line| line.contains(rule["id"].as_str().unwrap()))
            .unwrap_or_else(|| panic!("missing {rule}: {stdout}"));
        for field in ["source", "portRange", "description"] {
            if let Some(value) = rule[field].as_str() {
                assert!(line.contains(value), "{field}: {line}");
            }
        }
    }
}

// ── Postgres restore access rules ─────────────────────────────────────────

const RESTORE_SOURCE_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

/// The allow-all rules the restore endpoint creates on every restored service.
fn postgres_restore_default_rules() -> Value {
    serde_json::json!([
        {"id": "default-5432", "source": "0.0.0.0/0", "portRange": "5432"},
        {"id": "default-6432", "source": "0.0.0.0/0", "portRange": "6432"}
    ])
}

fn postgres_restore_args(extra: &[&'static str]) -> Vec<&'static str> {
    let mut args = vec![
        "postgres",
        "restore",
        RESTORE_SOURCE_ID,
        "--name",
        "restored",
        "--restore-target",
        "2026-09-01T12:00:00Z",
        "--org-id",
        "org-1",
    ];
    args.extend(extra);
    args
}

async fn mount_postgres_restore(mock: &MockServer, rules: Value) {
    Mock::given(method("POST"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}/restoredService"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": ACCESS_RULE_POSTGRES_ID,
                "state": "creating",
                "tags": [{"key": "env", "value": "prod"}],
                "ipAccessList": rules,
            },
        })))
        .expect(1)
        .mount(mock)
        .await;
}

async fn mount_postgres_source(mock: &MockServer, service: Value, expected: u64) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
        })))
        .expect(expected)
        .mount(mock)
        .await;
}

/// Mount the PATCH that replaces the restored service's rules. It must carry
/// the restored tags, because the API clears tags a PATCH omits.
async fn mount_postgres_restored_patch(mock: &MockServer, patch: Value, applied: Value) {
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{ACCESS_RULE_POSTGRES_ID}"
        )))
        .and(body_json(serde_json::json!({
            "tags": [{"key": "env", "value": "prod"}],
            "ipAccessList": patch,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": ACCESS_RULE_POSTGRES_ID, "ipAccessList": applied},
        })))
        .expect(1)
        .mount(mock)
        .await;
}

/// UAT F1: a restored copy of a restricted source must not keep the restore
/// endpoint's allow-all rules. The source's rules are read before restoring,
/// then replace the defaults.
#[tokio::test]
async fn postgres_restore_copies_the_source_access_rules() {
    let mock = MockServer::start().await;
    mount_postgres_source(
        &mock,
        serde_json::json!({"id": RESTORE_SOURCE_ID, "ipAccessList": postgres_access_rules_response()}),
        1,
    )
    .await;
    mount_postgres_restore(&mock, postgres_restore_default_rules()).await;
    let applied = serde_json::json!([
        {"id": "rule-11", "source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
        {"id": "rule-12", "source": "198.51.100.7/32", "portRange": "5432..6432"}
    ]);
    mount_postgres_restored_patch(
        &mock,
        serde_json::json!({
            "add": [
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "198.51.100.7/32", "portRange": "5432..6432"}
            ],
            "remove": [{"id": "default-5432"}, {"id": "default-6432"}]
        }),
        applied.clone(),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&[]));
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["ipAccessList"], applied);
    let source = format!("/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}");
    let restored = format!("/v1/organizations/org-1/postgres/{ACCESS_RULE_POSTGRES_ID}");
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("GET".into(), source.clone()),
            ("POST".into(), format!("{source}/restoredService")),
            ("PATCH".into(), restored),
        ]
    );
}

/// `--ip-allow` and `--block-public-access` replace the defaults without
/// reading the source.
#[tokio::test]
async fn postgres_restore_access_flags_replace_the_default_rules() {
    for (flags, add) in [
        (
            vec!["--ip-allow", "203.0.113.0/24:5432=office"],
            serde_json::json!([
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"}
            ]),
        ),
        (vec!["--block-public-access"], serde_json::json!([])),
    ] {
        let mock = MockServer::start().await;
        mount_postgres_source(&mock, serde_json::json!({}), 0).await;
        mount_postgres_restore(&mock, postgres_restore_default_rules()).await;
        mount_postgres_restored_patch(
            &mock,
            serde_json::json!({
                "add": add,
                "remove": [{"id": "default-5432"}, {"id": "default-6432"}]
            }),
            serde_json::json!([]),
        )
        .await;

        let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&flags));
        assert_success(&output);
    }
}

/// A source whose rules already match the restore's defaults needs no PATCH.
#[tokio::test]
async fn postgres_restore_skips_the_patch_when_rules_already_match() {
    let mock = MockServer::start().await;
    mount_postgres_source(
        &mock,
        serde_json::json!({"ipAccessList": [
            {"id": "source-1", "source": "0.0.0.0/0", "portRange": "5432", "description": ""},
            {"id": "source-2", "source": "0.0.0.0/0", "portRange": "6432"}
        ]}),
        1,
    )
    .await;
    mount_postgres_restore(&mock, postgres_restore_default_rules()).await;

    let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&[]));
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["ipAccessList"], postgres_restore_default_rules());
    assert_eq!(received_request_shape(&mock).await.len(), 2);
}

/// Copying an incomplete rule list would silently drop restrictions, so the
/// restore is refused before anything is created.
#[tokio::test]
async fn postgres_restore_refuses_a_source_without_complete_access_rules() {
    for source in [
        serde_json::json!({"id": RESTORE_SOURCE_ID}),
        serde_json::json!({"ipAccessList": [{"id": "rule-1", "portRange": "5432"}]}),
    ] {
        let mock = MockServer::start().await;
        mount_postgres_source(&mock, source, 1).await;

        let output = invoke_cli_with_cloud_credentials(&mock, &postgres_restore_args(&[]));
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--block-public-access"), "{stderr}");
        assert_eq!(received_request_shape(&mock).await.len(), 1);
    }
}

/// When the rules cannot be applied, the restored service still exists with
/// allow-all rules, so the error names it and how to fix it.
#[tokio::test]
async fn postgres_restore_rule_failure_names_the_restored_service() {
    let mock = MockServer::start().await;
    mount_postgres_restore(&mock, postgres_restore_default_rules()).await;
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{ACCESS_RULE_POSTGRES_ID}"
        )))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "boom",
        })))
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &postgres_restore_args(&["--block-public-access"]),
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(ACCESS_RULE_POSTGRES_ID), "{stderr}");
    assert!(stderr.contains("--remove-ip-allow"), "{stderr}");
}

/// A sparse restore response omits the rule IDs, so they are read back from
/// the restored service before the PATCH.
#[tokio::test]
async fn postgres_restore_reads_rules_omitted_from_a_sparse_response() {
    let mock = MockServer::start().await;
    mount_postgres_service_response(
        &mock,
        "POST",
        &format!("/v1/organizations/org-1/postgres/{RESTORE_SOURCE_ID}/restoredService"),
        serde_json::json!({"id": ACCESS_RULE_POSTGRES_ID}),
    )
    .await;
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{ACCESS_RULE_POSTGRES_ID}"),
        serde_json::json!({
            "id": ACCESS_RULE_POSTGRES_ID,
            "tags": [{"key": "env", "value": "prod"}],
            "ipAccessList": postgres_restore_default_rules(),
        }),
    )
    .await;
    mount_postgres_restored_patch(
        &mock,
        serde_json::json!({
            "add": [],
            "remove": [{"id": "default-5432"}, {"id": "default-6432"}]
        }),
        serde_json::json!([]),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &postgres_restore_args(&["--block-public-access"]),
    );
    assert_success(&output);
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["ipAccessList"], serde_json::json!([]));
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
                "deletionProtection": true,
            },
            {
                "id": "66666666-7777-8888-9999-000000000000",
                "name": "replica-pg",
                "state": "restoring_backup",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": false,
                "deletionProtection": false,
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

    // Each key's matching rules are the `apply_filter_*` unit tests.
    assert_eq!(names("isPrimary=false"), vec!["replica-pg".to_string()]);
    assert_eq!(
        names("state=restoring_backup"),
        vec!["replica-pg".to_string()]
    );
}

#[tokio::test]
async fn postgres_list_shows_provider_in_human_output_and_preserves_json() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(postgres_list_response())
        .expect(2)
        .mount(&mock)
        .await;

    let human =
        invoke_cli_with_cloud_credentials_human(&mock, &["postgres", "list", "--org-id", "org-1"]);
    assert_success(&human);
    let human = String::from_utf8(human.stdout).expect("human output should be UTF-8");
    let row_cells = |name: &str| -> Vec<&str> {
        human
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("missing row for {name}:\n{human}"))
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect()
    };
    assert_eq!(
        row_cells("Name"),
        [
            "Name",
            "ID",
            "State",
            "Provider",
            "Region",
            "Size",
            "PG",
            "HA",
            "Primary",
            "Deletion protection"
        ]
    );
    assert_eq!(row_cells("primary-pg")[3], "aws");
    assert_eq!(row_cells("unknown-pg")[3], "-");
    assert_eq!(row_cells("primary-pg")[9], "yes");
    assert_eq!(row_cells("replica-pg")[9], "no");
    assert_eq!(row_cells("unknown-pg")[9], "-");

    let json = invoke_cli_with_cloud_credentials(&mock, &["postgres", "list", "--org-id", "org-1"]);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).expect("JSON output should be valid"),
        serde_json::json!([
            {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "primary-pg",
                "state": "running",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": true,
                "deletionProtection": true,
            },
            {
                "id": "66666666-7777-8888-9999-000000000000",
                "name": "replica-pg",
                "state": "restoring_backup",
                "region": "us-east-1",
                "provider": "aws",
                "isPrimary": false,
                "deletionProtection": false,
            },
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "unknown-pg",
                "region": "us-east-1",
            },
        ])
    );
}

// ── Postgres promote / switchover role changes (issue #604) ───────────────

const ROLE_TEST_POSTGRES_ID: &str = "11111111-2222-3333-4444-555555555555";

async fn mount_postgres_service_response(
    mock: &MockServer,
    method_name: &str,
    endpoint: &str,
    service: Value,
) {
    Mock::given(method(method_name))
        .and(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
            "status": 200,
            "requestId": "stub-postgres-connection",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn issue_836_postgres_get_human_guidance_handles_sparse_endpoint_fields() {
    let mock = MockServer::start().await;
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}"),
        serde_json::json!({"id": ROLE_TEST_POSTGRES_ID}),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "postgres",
            "get",
            ROLE_TEST_POSTGRES_ID,
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("host=<host-from-get>"), "{stdout}");
    assert!(stdout.contains("user=<username-from-get>"), "{stdout}");
    assert!(stdout.contains(ROLE_TEST_POSTGRES_ID), "{stdout}");
    assert!(stdout.contains("certs get"), "{stdout}");
    assert!(!stdout.contains("password="), "{stdout}");
}

#[tokio::test]
async fn issue_836_postgres_get_json_keeps_the_api_shape_without_connection_defaults() {
    let mock = MockServer::start().await;
    let service = serde_json::json!({
        "id": ROLE_TEST_POSTGRES_ID,
        "hostname": "example.pg.clickhouse.cloud",
        "username": "postgres"
    });
    mount_postgres_service_response(
        &mock,
        "GET",
        &format!("/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}"),
        service.clone(),
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "get",
            ROLE_TEST_POSTGRES_ID,
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout: Value = serde_json::from_slice(&output.stdout).expect("valid JSON response");
    assert_eq!(stdout, service);
}

#[tokio::test]
async fn issue_836_replica_and_restore_human_outputs_point_sparse_results_to_get() {
    for (endpoint, mut args) in [
        (
            "readReplica",
            vec![
                "postgres",
                "read-replica",
                "create",
                "source-id",
                "--name",
                "replica",
            ],
        ),
        (
            "restoredService",
            vec![
                "postgres",
                "restore",
                "source-id",
                "--name",
                "restored",
                "--restore-target",
                "2026-09-01T12:00:00Z",
                "--block-public-access",
            ],
        ),
    ] {
        let mock = MockServer::start().await;
        mount_postgres_service_response(
            &mock,
            "POST",
            &format!("/v1/organizations/org-1/postgres/source-id/{endpoint}"),
            serde_json::json!({"id": ROLE_TEST_POSTGRES_ID}),
        )
        .await;
        mount_sparse_restored_get(&mock, endpoint).await;
        args.extend(["--org-id", "org-1"]);

        let output = invoke_cli_with_cloud_credentials_human(&mock, &args);
        assert_success(&output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("host=<host-from-get>"), "{stdout}");
        assert!(stdout.contains("user=<username-from-get>"), "{stdout}");
        assert!(
            stdout.contains(&format!("postgres get {ROLE_TEST_POSTGRES_ID}")),
            "{stdout}"
        );
        assert!(!stdout.contains("reset-password"), "{stdout}");

        // Read replica `--json` prints the returned service (restore's JSON
        // output is asserted by the `postgres_restore_*` tests).
        if endpoint == "readReplica" {
            let service = serde_json::json!({"id": ROLE_TEST_POSTGRES_ID});
            let json_mock = MockServer::start().await;
            mount_postgres_service_response(
                &json_mock,
                "POST",
                &format!("/v1/organizations/org-1/postgres/source-id/{endpoint}"),
                service.clone(),
            )
            .await;
            let output = invoke_cli_with_cloud_credentials(&json_mock, &args);
            assert_success(&output);
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                service
            );
        }
    }
}

/// A sparse restore response sends the CLI to `get` for the rules it
/// replaces; these already match `--block-public-access`, so nothing is patched.
async fn mount_sparse_restored_get(mock: &MockServer, endpoint: &str) {
    if endpoint == "restoredService" {
        mount_postgres_service_response(
            mock,
            "GET",
            &format!("/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}"),
            serde_json::json!({"id": ROLE_TEST_POSTGRES_ID, "tags": [], "ipAccessList": []}),
        )
        .await;
    }
}

fn postgres_role_service(ha_type: &str) -> serde_json::Value {
    serde_json::json!({
        "id": ROLE_TEST_POSTGRES_ID,
        "name": "my-postgres",
        "state": "running",
        "haType": ha_type,
    })
}

async fn mount_postgres_state(mock: &MockServer, command: &str, service: serde_json::Value) {
    Mock::given(method("PATCH"))
        .and(path(format!(
            "/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}/state"
        )))
        .and(header("authorization", BASIC_TEST_AUTH))
        .and(body_json(serde_json::json!({"command": command})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service,
            "status": 200,
            "requestId": "stub-postgres-state",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

/// Each role command sends one state change and returns its acknowledgment.
/// The exact request list proves the CLI performs no preflight or polling GET.
#[tokio::test]
async fn postgres_role_changes_issue_only_the_expected_state_patch() {
    for (command, ha_type) in [("promote", "async"), ("switchover", "sync")] {
        let mock = MockServer::start().await;
        let service = postgres_role_service(ha_type);
        mount_postgres_state(&mock, command, service.clone()).await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "postgres",
                command,
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
            "{command} must issue only the state change"
        );
        let stdout: Value = serde_json::from_slice(&output.stdout)
            .expect("stdout should be the API acknowledgment as JSON");
        assert_eq!(
            stdout, service,
            "{command} must preserve the response shape"
        );
    }
}

/// Start a human-mode (no `--json`, no agent env) switchover so the test can
/// close its stdout first (see the #598 tests).
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

/// A closed stdout must not turn an accepted switchover into a panic and exit
/// 101 (#598).
#[tokio::test]
async fn postgres_switchover_human_output_survives_a_closed_stdout() {
    let mock = MockServer::start().await;
    mount_postgres_state(&mock, "switchover", postgres_role_service("sync")).await;

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

/// `certs get`, `reset-password` and `restart` each reach their own route:
/// the certificate is written to `--output` with owner-only permissions, the
/// password the caller chose is the PATCH body, and restart is a state PATCH.
#[tokio::test]
async fn postgres_certs_reset_password_and_restart_reach_their_routes() {
    use std::os::unix::fs::PermissionsExt;
    let mock = MockServer::start().await;
    let service = format!("/v1/organizations/org-1/postgres/{ROLE_TEST_POSTGRES_ID}");
    let pem = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";
    Mock::given(method("GET"))
        .and(path(format!("{service}/caCertificates")))
        .respond_with(ResponseTemplate::new(200).set_body_string(pem))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("PATCH"))
        .and(path(format!("{service}/password")))
        .and(body_json(
            serde_json::json!({"password": "Chosen-Pass-123"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {}, "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    mount_postgres_state(&mock, "restart", postgres_role_service("sync")).await;

    let directory = tempfile::tempdir().unwrap();
    let ca_path = directory.path().join("ca.pem");
    let ca = ca_path.to_str().unwrap();
    let id = ROLE_TEST_POSTGRES_ID;
    let certs = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres", "certs", "get", id, "--output", ca, "--org-id", "org-1",
        ],
    );
    assert_success(&certs);
    assert_eq!(std::fs::read_to_string(&ca_path).unwrap(), pem);
    assert_eq!(
        std::fs::metadata(&ca_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&certs.stdout).unwrap(),
        serde_json::json!({"path": ca})
    );

    let reset = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "reset-password",
            id,
            "--password",
            "Chosen-Pass-123",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&reset);
    // The response omitted the password, so the one sent is reported.
    assert_eq!(
        serde_json::from_slice::<Value>(&reset.stdout).unwrap(),
        serde_json::json!({"password": "Chosen-Pass-123"})
    );

    let restart =
        invoke_cli_with_cloud_credentials(&mock, &["postgres", "restart", id, "--org-id", "org-1"]);
    assert_success(&restart);
    assert_eq!(
        serde_json::from_slice::<Value>(&restart.stdout).unwrap(),
        postgres_role_service("sync")
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        // A forced delete of a running service polls the stop; keep the
        // test from waiting out the shipped 5 s interval.
        .env("CHCTL_SERVICE_STOP_POLL_INTERVAL_MS", "100")
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
    // The failed key, with the server's own error for it verbatim.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "{DELETE_TEST_PENDING_API_KEY_ID}: pending cleanup failed"
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
    // The server's code, then the organization the request was scoped to.
    let message = cloud_runtime_error(&output)["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        message.starts_with("NOT_FOUND") && message.ends_with("org-1"),
        "{message}"
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "http_4xx");

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

/// Mock a `--force` delete of a service whose `GET`s answer `states` in
/// order: the first is the pre-stop check, the rest come from the poll loop.
async fn mount_forced_delete_stop_sequence(mock: &MockServer, states: &'static [&'static str]) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

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
        .expect(states.len() as u64)
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

/// `--json` with stderr redirected: the stop notice still reaches stderr, and
/// the poll reports state transitions only, not every poll.
#[tokio::test]
async fn forced_service_delete_reports_only_poll_state_transitions_when_redirected() {
    let mock = MockServer::start().await;
    mount_forced_delete_stop_sequence(&mock, &["running", "stopping", "stopping", "stopped"]).await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), true);
    assert_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    let (notice, polls) = lines.split_first().expect("a stop notice on stderr");
    assert!(notice.contains(DELETE_TEST_SERVICE_ID), "{stderr}");
    let states: Vec<&str> = polls
        .iter()
        .map(|line| line.trim().strip_prefix("state: ").unwrap_or(line))
        .collect();
    assert_eq!(states, ["stopping", "stopped"], "{stderr}");
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
    mount_forced_delete_stop_sequence(&mock, &["running", "stopping", "stopped"]).await;

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
        cloud_runtime_error(&output)["message"],
        format!(
            "failed to delete the auto-provisioned query API key for service \
             {DELETE_TEST_SERVICE_ID} ({DELETE_TEST_API_KEY_ID}: cleanup failed). The local \
             record was kept so the exact IDs are not lost; delete each key with \
             `clickhousectl cloud key delete <key-id> --org-id org-1`"
        )
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "http_5xx");

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
        cloud_runtime_error(&output)["message"],
        "service delete failed"
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "http_5xx");

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
        cloud_runtime_error(&output)["message"],
        format!(
            "the stored query key for service {DELETE_TEST_SERVICE_ID} belongs to \
             organization org-2, not org-1; refusing to delete either resource"
        )
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "other");
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
    // The server's code, then the organization the request was scoped to.
    let message = cloud_runtime_error(&output)["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        message.starts_with("NOT_FOUND") && message.ends_with("org-1"),
        "{message}"
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "http_4xx");

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
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
    let error = cloud_runtime_error(&output);
    assert_eq!(error["code"], "other");
    let message = error["message"].as_str().unwrap();
    let source = serde_json::from_str::<Value>("{").unwrap_err();
    assert!(
        message.contains(".clickhouse/credentials.json") && message.contains(&source.to_string()),
        "{message}"
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

#[tokio::test]
async fn tag_filters_preserve_repeated_equality_and_existence_filters_on_wire() {
    let mock = MockServer::start().await;
    for endpoint in ["services", "usageCost"] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/{endpoint}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": 200,
                "result": if endpoint == "services" { serde_json::json!([]) } else { serde_json::json!({}) }
            })))
            .expect(1)
            .mount(&mock)
            .await;
    }
    let filters = [
        "tag:env=prod",
        "tag:active",
        "tag:empty=",
        "tag:expression=a=b & c",
    ];
    for mut args in [
        vec!["service", "list"],
        vec![
            "org",
            "usage",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ],
    ] {
        args.extend(["--org-id", "org-1"]);
        args.extend(filters.iter().flat_map(|value| ["--filter", *value]));
        assert_success(&invoke_cli_with_cloud_credentials(&mock, &args));
    }
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        let actual: Vec<_> = request
            .url
            .query_pairs()
            .filter(|(key, _)| key == "filter")
            .map(|(_, value)| value.into_owned())
            .collect();
        assert_eq!(actual, filters);
    }
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

// ── ClickPipe create ───────────────────────────────────────────────────────
//
// Field-by-field request shape belongs to the `build_*_request` unit tests in
// `src/cloud/clickpipes.rs`. These tests cover what only the real handler
// reaches: the assembled body on the wire, stdin reads, and error output.

fn kafka_args_minimal() -> Vec<&'static str> {
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
        "--auth",
        "PLAIN",
        "--username",
        "u",
        "--password",
        "p",
    ]
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

/// The controls every streaming create shares. The destination flags go
/// through each streaming handler, which assembles its own request rather than
/// calling a `build_*_request` builder, so only this test sees them on the wire.
fn append_streaming_create_controls(args: &mut Vec<String>, table_definition_file: &str) {
    append_initial_scaling_and_validation(args);
    args.extend([
        "--start-paused".into(),
        "--field-mapping".into(),
        r#"{"sourceField":"source:a=b","destinationField":"destination:x=y"}"#.into(),
        "--clickhouse-max-threads".into(),
        "0".into(),
        "--clickhouse-parallel-view-processing".into(),
        "false".into(),
        "--role".into(),
        "analytics_reader".into(),
        "--managed-table".into(),
        "false".into(),
        "--table-definition-file".into(),
        table_definition_file.into(),
    ]);
}

#[tokio::test]
async fn cross_source_create_controls_reach_all_eight_request_shapes() {
    let directory = tempfile::tempdir().unwrap();
    let service_account = directory.path().join("service-account.json");
    std::fs::write(&service_account, "{}").unwrap();
    let service_account = service_account.to_str().unwrap();
    let table_definition = directory.path().join("table-definition.json");
    std::fs::write(
        &table_definition,
        serde_json::json!({
            "engine": {
                "columnIds": ["amount"],
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
    let table_definition = table_definition.to_str().unwrap();

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
        "--column",
        "id:Int64",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_streaming_create_controls(&mut object_storage, table_definition);
    object_storage.extend([
        "--object-storage-concurrency".into(),
        "1".into(),
        "--object-storage-polling-interval-ms".into(),
        "100".into(),
        "--object-storage-max-insert-bytes".into(),
        "524288000".into(),
        "--object-storage-max-file-count".into(),
        "1".into(),
        "--object-storage-use-cluster-function".into(),
        "false".into(),
    ]);

    let mut kafka = kafka_args_minimal()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    append_streaming_create_controls(&mut kafka, table_definition);
    kafka.extend([
        "--streaming-max-insert-wait-ms".into(),
        "500".into(),
        "--kafka-read-committed".into(),
        "false".into(),
        "--exactly-once".into(),
        "true".into(),
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
    append_streaming_create_controls(&mut kinesis, table_definition);
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
    append_streaming_create_controls(&mut pubsub, table_definition);
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
            assert_eq!(body["startPaused"], true, "{source}");
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
            assert_eq!(
                body["destination"]["roles"],
                serde_json::json!(["analytics_reader"]),
                "{source}"
            );
            assert_eq!(body["destination"]["managedTable"], false, "{source}");
            assert_eq!(
                body["destination"]["tableDefinition"],
                serde_json::json!({
                    "engine": {"columnIds": ["amount"], "type": "SummingMergeTree"},
                    "partitionBy": "toYYYYMM(created_at)",
                    "primaryKey": "event_id",
                    "sortingKey": ["event_id", "created_at"]
                }),
                "{source}"
            );
            if source == "kafka" {
                assert_eq!(body["settings"]["kafka_read_committed"], false, "{source}");
                assert_eq!(body["source"]["kafka"]["exactlyOnce"], true);
            } else {
                assert!(
                    body["settings"].get("kafka_read_committed").is_none(),
                    "{source}: {body}"
                );
            }
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
            assert!(body.get("startPaused").is_none(), "{source}");
            assert!(body.get("scaling").is_none(), "{source}: {body}");
            assert!(body.get("fieldMappings").is_none(), "{source}: {body}");
            assert!(body.get("settings").is_none(), "{source}: {body}");
        }
    }
}

/// The exact body each create subcommand sends for a minimal invocation.
///
/// The builder unit tests assert the library request structs; this is the one
/// wire-level check that an omitted flag stays off the body after the handler
/// assembles and serializes the request. A key that appears here without a
/// flag is a default the CLI chose, so a stray `""`, `[]` or zero (the
/// `4f6c2ba` bug class: `unwrap_or_default()` on an optional field) shows up
/// as a diff against this table.
#[tokio::test]
async fn create_minimal_invocations_send_exact_request_bodies() {
    let directory = tempfile::tempdir().unwrap();
    let bigquery_key = directory.path().join("bigquery-key.json");
    std::fs::write(&bigquery_key, "{}").unwrap();
    let pubsub_key = write_service_account_key(directory.path());
    let owned = |args: &[&str]| args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
    let streaming_destination = |column: &str| {
        serde_json::json!({
            "columns": [{"name": column, "type": "Int64"}],
            "database": "default",
            "managedTable": true,
            "table": "events",
            "tableDefinition": {"engine": {"type": "MergeTree"}},
        })
    };
    let cases: Vec<(Vec<String>, Value)> = vec![
        (
            owned(&[
                "clickpipe",
                "create",
                "object-storage",
                "svc-id",
                "--name",
                "object-pipe",
                "--source-url",
                "https://bucket.example/events/*.json",
                "--format",
                "JSONEachRow",
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:Int64",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "object-pipe",
                "destination": streaming_destination("id"),
                "source": {"objectStorage": {
                    "compression": "auto",
                    "format": "JSONEachRow",
                    "type": "s3",
                    "url": "https://bucket.example/events/*.json",
                }},
            }),
        ),
        (
            owned(&kafka_args_minimal()),
            serde_json::json!({
                "name": "t",
                "destination": streaming_destination("id"),
                "source": {"kafka": {
                    "authentication": "PLAIN",
                    "brokers": "broker:9092",
                    "credentials": {"password": "p", "username": "u"},
                    "format": "JSONEachRow",
                    "offset": {"strategy": "from_beginning"},
                    "reversePrivateEndpointIds": [],
                    "topics": "topic",
                    "type": "kafka",
                }},
            }),
        ),
        (
            owned(&[
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
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:Int64",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "kinesis-pipe",
                "destination": streaming_destination("id"),
                "source": {"kinesis": {
                    "authentication": "IAM_ROLE",
                    "format": "JSONEachRow",
                    "iteratorType": "TRIM_HORIZON",
                    "region": "us-east-1",
                    "streamName": "events",
                }},
            }),
        ),
        (
            postgres_args_minimal(),
            serde_json::json!({
                "name": "t",
                // Database pipes carry only the destination database.
                "destination": {"database": "default"},
                "source": {"postgres": {
                    "authentication": "basic",
                    "credentials": {"password": "p", "username": "u"},
                    "database": "test",
                    "disableTls": false,
                    "host": "pg",
                    "port": 5432,
                    // `cdc` creates the publication and slot server-side, so
                    // neither name is sent.
                    "settings": {
                        "allowNullableColumns": false,
                        "deleteOnMerge": false,
                        "enableFailoverSlots": false,
                        "replicationMode": "cdc",
                    },
                    "skipCertVerification": false,
                    "tableMappings": [{
                        "excludedColumns": [],
                        "partitionByExpr": "",
                        "partitionKey": "",
                        "sortingKeys": [],
                        "sourceSchemaName": "public",
                        "sourceTable": "t",
                        "tableEngine": "ReplacingMergeTree",
                        "targetTable": "t",
                        "useCustomSortingKey": false,
                    }],
                    "type": "postgres",
                }},
            }),
        ),
        (
            mysql_args_minimal(),
            serde_json::json!({
                "name": "t",
                "destination": {"database": "default"},
                "source": {"mysql": {
                    "authentication": "basic",
                    "credentials": {"password": "p", "username": "u"},
                    "host": "mysql",
                    "port": 3306,
                    "settings": {"replicationMechanism": "GTID", "replicationMode": "cdc"},
                    "tableMappings": [{
                        "sourceSchemaName": "mydb",
                        "sourceTable": "t",
                        "targetTable": "t",
                    }],
                    "type": "mysql",
                }},
            }),
        ),
        (
            owned(&[
                "clickpipe",
                "create",
                "mongodb",
                "svc-id",
                "--name",
                "mongo-pipe",
                "--uri",
                "mongodb://mongo.example:27017",
                "--username",
                "u",
                "--password",
                "p",
                "--table-mapping",
                "source.events:events",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "mongo-pipe",
                "destination": {"database": "default"},
                "source": {"mongodb": {
                    "credentials": {"password": "p", "username": "u"},
                    "readPreference": "secondaryPreferred",
                    "settings": {"replicationMode": "cdc"},
                    "tableMappings": [{
                        "sourceCollection": "events",
                        "sourceDatabaseName": "source",
                        "targetTable": "events",
                    }],
                    "uri": "mongodb://mongo.example:27017",
                }},
            }),
        ),
        (
            owned(&[
                "clickpipe",
                "create",
                "bigquery",
                "svc-id",
                "--name",
                "bigquery-pipe",
                "--service-account-file",
                bigquery_key.to_str().unwrap(),
                "--staging-path",
                "gs://bucket/staging",
                "--table-mapping",
                "dataset.events:events",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "bigquery-pipe",
                "destination": {"database": "default"},
                "source": {"bigquery": {
                    // The key file's contents, base64-encoded; never its path.
                    "credentials": {"serviceAccountFile": "e30="},
                    "settings": {"replicationMode": "snapshot"},
                    "snapshotStagingPath": "gs://bucket/staging",
                    "tableMappings": [{
                        "sourceDatasetName": "dataset",
                        "sourceTable": "events",
                        "targetTable": "events",
                    }],
                }},
            }),
        ),
        (
            pubsub_create_args(pubsub_key.to_str().unwrap()),
            serde_json::json!({
                "name": "pubsub-pipe",
                "destination": streaming_destination("event_id"),
                "source": {"pubsub": {
                    "authentication": "SERVICE_ACCOUNT",
                    "format": "JSONEachRow",
                    "projectId": "my-gcp-project",
                    "seekType": "earliest",
                    "serviceAccountKey": {"serviceAccountFile": encoded_service_account_key()},
                    "topic": "events",
                }},
            }),
        ),
    ];

    for (args, expected) in cases {
        let mock = start_mock_clickpipes_api().await;
        let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;
        assert_eq!(body, expected, "clickpipe create {}", args[2]);
    }
}

fn kinesis_auth_args<'a>(operation: &'a str, flags: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "clickpipe",
        operation,
        "kinesis",
        "svc-id",
        "--org-id",
        "org",
        "--stream-name",
        "stream",
        "--region",
        "us-east-1",
        "--format",
        "JSONEachRow",
    ];
    if operation == "create" {
        args.extend([
            "--name",
            "pipe",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
        ]);
    }
    args.extend(flags);
    args
}

#[tokio::test]
async fn kinesis_auth_conflicts_are_secret_free_usage_errors_before_credentials_or_http() {
    // The conflict matrix is unit-tested in `main.rs`; this run checks the
    // process: the usage error wins over missing credentials (exit 2, not 4),
    // nothing reaches HTTP, and neither stream echoes a credential.
    let mock = MockServer::start().await;
    let home = tempfile::tempdir().unwrap();
    let flags = [
        "--iam-role",
        "private-role-arn",
        "--access-key-id",
        "private-access-id",
        "--secret-key",
        "private-secret-key",
    ];
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("HOME", home.path())
        .env("DO_NOT_TRACK", "1")
        .args(["cloud", "--url", &mock.uri(), "--json"])
        .args(kinesis_auth_args("create", &flags))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--iam-role"), "{stderr}");
    for value in [
        "private-role-arn",
        "private-access-id",
        "private-secret-key",
    ] {
        assert!(!stderr.contains(value), "credential leaked: {stderr}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains(value));
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

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
    for secret in [password, ca_contents, ca_path_display.as_str()] {
        assert!(
            !stderr.contains(secret),
            "secret leaked into error: {stderr}"
        );
    }
}

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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
//     active API key is tried directly, and bound to the endpoint only if
//     the endpoint rejects it (#1043).
//   - OAuth: the user's own bearer token is sent directly to the query host
//     — no key lookup and, crucially, NO binding calls (the endpoint upsert
//     needs write access an OAuth token doesn't have).
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
    // The first-use bind tests resolve a UUID organization, as `whoami`
    // reports one (#1043); everything else uses `org-1`.
    for org_id in ["org-1", FIRST_USE_ORG_ID] {
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/{org_id}/services/{QUERY_TEST_SERVICE_ID}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(stub_service.clone()))
            .mount(&mock)
            .await;
    }
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

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--query"), "{stderr}");
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

    assert_eq!(output.status.code(), Some(2));
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

fn assert_invalid_utf8_query_input(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The native-client way out, by its commands.
    for expected in [
        "clickhousectl local use latest",
        "clickhousectl local client --host <nativesecure-host> --port 9440 -- --secure",
        "clickhousectl cloud service get <service-id>",
    ] {
        assert!(
            stderr.contains(expected),
            "missing {expected:?} in {stderr}"
        );
    }
}

#[tokio::test]
async fn service_query_rejects_non_utf8_on_bare_stdin_before_any_request() {
    let (output, control, query_host) =
        invoke_oauth_service_query_with_stdin(&[], StdinPlan::Write(b"SELECT '\xff'\n")).await;

    assert_invalid_utf8_query_input(&output);
    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_rejects_non_utf8_from_queries_file_dash_before_any_request() {
    let (output, control, query_host) = invoke_oauth_service_query_with_stdin(
        &["--queries-file", "-"],
        StdinPlan::Write(b"SELECT '\xff'\n"),
    )
    .await;

    assert_invalid_utf8_query_input(&output);
    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_reports_non_utf8_as_structured_io_without_echoing_input() {
    const PRIVATE_INPUT: &[u8] = b"private-query-fragment-\xff";
    let (output, control, query_host) = invoke_oauth_service_query_with_stdin(
        &["--json", "--queries-file", "-"],
        StdinPlan::Write(PRIVATE_INPUT),
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = std::str::from_utf8(&output.stderr).expect("JSON error output is valid UTF-8");
    let json: Value = serde_json::from_str(stderr).expect("stderr is one JSON error object");
    assert_eq!(json["error"]["code"], "io");
    assert!(json["error"]["message"].is_string(), "{json}");
    assert!(!stderr.contains("private-query-fragment"), "{stderr}");
    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_query_rejects_non_utf8_from_query_file_before_any_request() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"SELECT '\xff'\n").unwrap();
    let query_path = file.path().to_str().unwrap();

    let (output, control, query_host) =
        invoke_oauth_service_query_with_stdin(&["--queries-file", query_path], StdinPlan::Null)
            .await;

    assert_invalid_utf8_query_input(&output);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(query_path),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());
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
async fn service_query_runs_with_each_selector_form() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;
    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    std::fs::create_dir(&home_dir).unwrap();
    let url = control.uri();

    // Selector validation (exactly one of --name, --id or the positional ID)
    // is clap's; see `rejects_service_query_without_name_or_id`.
    for selector in [
        vec!["--name", "demo"],
        vec!["--id", QUERY_TEST_SERVICE_ID],
        vec![QUERY_TEST_SERVICE_ID],
    ] {
        let mut args = vec![
            "cloud",
            "--url",
            url.as_str(),
            "service",
            "query",
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ];
        args.extend_from_slice(&selector);
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        let output = command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home_dir)
            .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
            .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
            .current_dir(dir.path())
            .stdin(Stdio::null())
            .args(args)
            .output()
            .expect("failed to spawn clickhousectl");
        assert_success(&output);
        assert_eq!(output.stdout, b"1\n");
    }

    assert_eq!(control.received_requests().await.unwrap().len(), 3);
    assert_eq!(query_host.received_requests().await.unwrap().len(), 3);
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
        "auth_config": {"auth_url": control.uri(), "client_id": "test-client"},
    });
    std::fs::write(
        ch_dir.join("tokens.json"),
        serde_json::to_vec(&tokens).unwrap(),
    )
    .unwrap();

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
    control_url: &str,
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
        "api_url": control_url,
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
        .isolated_env()
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
        &control.uri(),
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

/// The deleted verdict names the key and the repair, and none of the other
/// verdicts' remedies (re-enable, endpoint inspection, allowlist update). Its
/// code is asserted in `stored_query_key_rejections_emit_structured_json_errors`.
fn assert_deleted_verdict(stderr: &str) {
    assert!(stderr.contains(OLD_QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(stderr.contains(&repair_hint()), "{stderr}");
    assert!(!stderr.contains("--state enabled"), "{stderr}");
    assert!(!stderr.contains("query-endpoint get"), "{stderr}");
    assert!(!stderr.contains("--ip-allow"), "{stderr}");
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

/// The command a skipped or failed repair verification names for the next query.
fn next_query_hint() -> String {
    format!("`clickhousectl cloud service query {QUERY_TEST_SERVICE_ID} --org-id org-1")
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
        // The verdict names the status, the key and its re-enable command;
        // nothing was replaced (the request log and the record below).
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(&format!("HTTP {status}")), "{stderr}");
        assert!(stderr.contains(OLD_QUERY_TEST_KEY_UUID), "{stderr}");
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
    assert!(stderr.contains(OLD_QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(stderr.contains("2020-01-01T00:00:00Z"), "{stderr}");
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
    assert_deleted_verdict(&String::from_utf8_lossy(&output.stderr));
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
    assert_deleted_verdict(&stderr);
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
    assert_deleted_verdict(&stderr);
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
        &control.uri(),
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
        &control.uri(),
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
    assert!(stderr.contains("ep-1"), "{stderr}");
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
    // Still the unbound verdict (it points at the endpoint), but no endpoint ID.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("query-endpoint get"), "{stderr}");
    assert!(!stderr.contains("ep-1"), "{stderr}");
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
    // The allowlist travels as data, with the allowlist fix and the repair.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("203.0.113.0/24, 198.51.100.7/32"),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "`clickhousectl cloud key update {OLD_QUERY_TEST_KEY_UUID} --ip-allow <cidr> --org-id org-1`"
        )),
        "{stderr}"
    );
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
    // The failed lookup's own server text, naming the key.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(OLD_QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(stderr.contains("upstream unavailable"), "{stderr}");
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
    assert!(stderr.contains(QUERY_TEST_SERVICE_ID), "{stderr}");
    assert!(stderr.contains("endpoint service unavailable"), "{stderr}");
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
        stderr.contains(&format!("service_query_keys.{QUERY_TEST_SERVICE_ID}")),
        "{stderr}"
    );
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
            .contains(OLD_QUERY_TEST_KEY_UUID)
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
        &control.uri(),
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
        &control.uri(),
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
            "clickhousectl cloud service query {QUERY_TEST_SERVICE_ID} --org-id org-1"
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
        &control.uri(),
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
        &control.uri(),
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
    assert!(progress_lines(&output).is_empty(), "{output:?}");

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
        &control.uri(),
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
        &control.uri(),
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
    assert!(stderr.contains(QUERY_TEST_KEY_UUID), "{stderr}");
    assert!(
        stderr.contains(&format!(
            "service_query_keys.{QUERY_TEST_SERVICE_ID}.pending_cleanup_api_key_ids"
        )),
        "the error says where the ID went: {stderr}"
    );
    assert!(
        stderr.contains("`clickhousectl cloud service query"),
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
        &control.uri(),
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
    // Both repairs talk to one server, the API the project key is saved for
    // (#1202), which is reset between them.
    let control = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    write_repair_query_credentials(
        project.path(),
        &control.uri(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );

    mount_running_service_accepting_probes(&control, 1).await;
    mount_repair_endpoint_get_listing(&control, &["unrelated-key", OLD_QUERY_TEST_KEY_UUID]).await;
    mount_key_create_returning(&control, QUERY_TEST_KEY_UUID).await;
    expect_repair_endpoint_upsert(&control, &["unrelated-key", QUERY_TEST_KEY_UUID]).await;
    mount_key_delete(&control, OLD_QUERY_TEST_KEY_UUID, key_deleted_response()).await;
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(
        key_deletes_received(&control).await,
        [OLD_QUERY_TEST_KEY_UUID]
    );

    // Check the first repair's expectations before the reset discards them.
    control.verify().await;
    control.reset().await;
    mount_running_service_accepting_probes(&control, 1).await;
    mount_repair_endpoint_get_listing(&control, &["unrelated-key", QUERY_TEST_KEY_UUID]).await;
    mount_key_create_returning(&control, THIRD_QUERY_TEST_KEY_UUID).await;
    expect_repair_endpoint_upsert(&control, &["unrelated-key", THIRD_QUERY_TEST_KEY_UUID]).await;
    mount_key_delete(&control, QUERY_TEST_KEY_UUID, key_deleted_response()).await;
    let output = service_query_key_repair_process(project.path(), &control)
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
        key_deletes_received(&control).await,
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
// `GET /keys/{id}` already returns it. Repair creates a key and then binds
// it, and the first-use bind of the caller's key goes through the same
// upsert, so both wait that out: the upsert alone is retried,
// with the same body, inside a bounded window; the notice below is printed
// once; a success ends the wait. The condition is structural (a typed
// `Error::Api` with status 400), never the message text.

/// The CLI's own progress lines on stderr, in order: every line before an
/// `Error: ` line or a JSON error object, without the credential `note:`
/// lines. Notices are counted and ordered here rather than matched by wording.
fn progress_lines(output: &std::process::Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .take_while(|line| !line.starts_with("Error: ") && !line.starts_with('{'))
        .filter(|line| !line.starts_with("note:") && !line.is_empty())
        .map(str::to_owned)
        .collect()
}

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
        &control.uri(),
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
        progress_lines(&output).len(),
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
    // The control plane keeps refusing the new key for the whole (shortened)
    // window. The repair then fails exactly as an unretried failure did:
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
        &control.uri(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .env("CHCTL_KEY_PROPAGATION_TIMEOUT_MS", "1500")
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        stderr_without_notes(&output)
    );
    let stderr = stderr_without_notes(&output);
    assert_eq!(progress_lines(&output).len(), 1, "{stderr}");
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
async fn first_use_bind_waits_for_the_endpoint_to_accept_the_caller_key() {
    // The first-use bind upserts the caller's key through the same bounded
    // 400 retry the repair uses, so a transient rejection is waited out.
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_first_use_endpoint_get(&control, None).await;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let attempts = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path(first_use_endpoint_path()))
        .respond_with(move |_: &wiremock::Request| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                key_not_in_organization_response(CALLER_KEY_UUID)
            } else {
                endpoint_upsert_ok()
            }
        })
        .expect(2)
        .mount(&control)
        .await;
    let query_host = start_query_host_rejecting_the_first_query().await;

    let (project, output) = run_first_use_query(&control, &query_host, &[]).await;
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    // The bind notice, then the propagation notice once.
    let progress = progress_lines(&output);
    assert_eq!(progress.len(), 2, "{progress:#?}");
    assert!(progress[0].contains("'demo'"), "{progress:#?}");

    let upserts: Vec<Value> = control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST
                && request.url.path() == first_use_endpoint_path()
        })
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect();
    assert_eq!(upserts.len(), 2);
    assert_eq!(upserts[0], upserts[1]);
    assert_eq!(
        upserts[0]["openApiKeys"],
        serde_json::json!([CALLER_KEY_UUID])
    );
    assert_eq!(control_plane_writes(&control).await.len(), 2);
    assert!(!project.path().join(".clickhouse/credentials.json").exists());
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

/// Shortens the 120 s readiness window for tests that wait all of it out.
const QUERY_READINESS_TIMEOUT_ENV: &str = "CHCTL_QUERY_READINESS_TIMEOUT_MS";
const SHORT_QUERY_READINESS_MS: &str = "2000";

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
        &control.uri(),
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
    // One readiness notice, and no skipped-verification note.
    let progress = progress_lines(&output);
    assert_eq!(progress.len(), 1, "{progress:#?}");
    assert!(!progress[0].starts_with("Note:"), "{progress:#?}");

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
            &control.uri(),
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
        // One note naming the service and its state, and the query that verifies the key.
        let stderr = stderr_without_notes(&output);
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        assert!(stderr.starts_with("Note: "), "{stderr}");
        assert!(
            stderr.contains(QUERY_TEST_SERVICE_ID) && stderr.contains(state),
            "{stderr}"
        );
        assert!(stderr.contains(&next_query_hint()), "{stderr}");
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
        &control.uri(),
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
        &control.uri(),
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
    assert!(stderr.starts_with("Note: "), "{stderr}");
    assert!(stderr.contains("CLICKHOUSE_CLOUD_QUERY_HOST"), "{stderr}");
    assert!(stderr.contains(&next_query_hint()), "{stderr}");
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
        &control.uri(),
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
    // A warning naming the service, the new key and where it is stored, the
    // probe's own failure, and the query that verifies the key.
    let stderr = stderr_without_notes(&output);
    assert!(stderr.starts_with("Warning: "), "{stderr}");
    for identifier in [
        QUERY_TEST_SERVICE_ID,
        QUERY_TEST_KEY_UUID,
        ".clickhouse/credentials.json",
    ] {
        assert!(stderr.contains(identifier), "{identifier}: {stderr}");
    }
    assert!(stderr.contains("bad gateway"), "{stderr}");
    assert!(stderr.contains(&next_query_hint()), "{stderr}");

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
        &control.uri(),
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
    assert!(stderr.starts_with("Note: "), "{stderr}");
    assert!(
        stderr.contains(QUERY_TEST_SERVICE_ID) && stderr.contains("stopped"),
        "{stderr}"
    );
    assert!(!stderr.contains("Warning:"), "{stderr}");
}

#[tokio::test]
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
        &control.uri(),
        Some("org-1"),
        Some(OLD_QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .env(QUERY_READINESS_TIMEOUT_ENV, SHORT_QUERY_READINESS_MS)
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
            "clickhousectl cloud service query {QUERY_TEST_SERVICE_ID} --org-id org-1 \
             --query \"SELECT 1\""
        )
    );
    // The message names the committed replacement; the command (above) is the
    // query to rerun, not another repair.
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains(QUERY_TEST_KEY_UUID), "{message}");
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
    let original = write_repair_query_credentials(
        project.path(),
        &control.uri(),
        None,
        None,
        Some("ep-1"),
        &[],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains(QUERY_TEST_SERVICE_ID));
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
    service_query_process_in_org(project_dir, control, query_host, "org-1", sql)
}

fn service_query_process_in_org(
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
    org_id: &str,
    sql: &str,
) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(clickhousectl_binary());
    command
        .isolated_env()
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
            org_id,
            "--query",
            sql,
        ]);
    command
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

// ── First use: bind the caller's own API key (issue #1043) ──────────────────
//
// With no stored record, a query the endpoint rejects for the caller's own
// API key makes the CLI identify that key with `whoami`, merge its resource
// UUID into the endpoint's `openApiKeys` (pruning bindings to keys the org
// reports deleted) and re-run the query with the same credentials. No key is
// created and nothing is written to `credentials.json`. `whoami` answers with
// a UUID organization, so these tests resolve a UUID org rather than `org-1`.

const FIRST_USE_ORG_ID: &str = "5fae43a3-1043-4000-8000-0000000000f1";
const OTHER_ORG_ID: &str = "5fae43a3-1043-4000-8000-0000000000f2";
const CALLER_KEY_UUID: &str = "cccccccc-1043-4000-8000-00000000ca11";
const BOUND_KEY_A: &str = "aaaaaaaa-1043-4000-8000-00000000000a";
const BOUND_KEY_B: &str = "bbbbbbbb-1043-4000-8000-00000000000b";
const DELETED_BOUND_KEY: &str = "dddddddd-1043-4000-8000-00000000000d";
const UNVERIFIABLE_BOUND_KEY: &str = "eeeeeeee-1043-4000-8000-00000000000e";

fn first_use_endpoint_path() -> String {
    format!(
        "/v1/organizations/{FIRST_USE_ORG_ID}/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint"
    )
}

fn first_use_key_path(api_key_uuid: &str) -> String {
    format!("/v1/organizations/{FIRST_USE_ORG_ID}/keys/{api_key_uuid}")
}

fn caller_api_key_identity(organization_id: &str) -> Value {
    serde_json::json!({
        "actorType": "apiKey",
        "keyId": CALLER_KEY_UUID,
        "name": "ci",
        "organizationId": organization_id,
    })
}

async fn mount_whoami(control: &MockServer, identity: Value) {
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(identity))
        .mount(control)
        .await;
}

/// `GET /keys/{uuid}` for a key bound to the endpoint, answering `status`.
async fn mount_bound_key_lookup(control: &MockServer, api_key_uuid: &str, status: u16) {
    let response = if status == 200 {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": api_key_uuid, "state": "enabled" },
            "status": 200,
            "requestId": "stub-key-get",
        }))
    } else {
        ResponseTemplate::new(status).set_body_json(serde_json::json!({
            "error": "key lookup failed",
            "status": status,
            "requestId": "stub-key-get",
        }))
    };
    Mock::given(method("GET"))
        .and(path(first_use_key_path(api_key_uuid)))
        .respond_with(response)
        .expect(1)
        .mount(control)
        .await;
}

/// The endpoint GET: absent (`None`, a 404) or reporting `open_api_keys`.
async fn mount_first_use_endpoint_get(control: &MockServer, open_api_keys: Option<Value>) {
    let response = match open_api_keys {
        None => ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not found",
            "status": 404,
            "requestId": "stub-endpoint-get",
        })),
        Some(keys) => ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "ep-1",
                "allowedOrigins": "*",
                "openApiKeys": keys,
                "roles": ["sql_console_admin"],
            },
            "status": 200,
            "requestId": "stub-endpoint-get",
        })),
    };
    Mock::given(method("GET"))
        .and(path(first_use_endpoint_path()))
        .respond_with(response)
        .mount(control)
        .await;
}

async fn mount_first_use_endpoint_upsert(control: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(first_use_endpoint_path()))
        .respond_with(response)
        .mount(control)
        .await;
}

fn endpoint_upsert_ok() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": { "id": "ep-1" },
        "status": 200,
        "requestId": "stub-endpoint-upsert",
    }))
}

/// Everything a successful first-use bind needs: the caller's identity, an
/// absent endpoint and an upsert that creates it. Returns the endpoint path.
async fn mount_successful_caller_key_bind(control: &MockServer) -> String {
    mount_whoami(control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_first_use_endpoint_get(control, None).await;
    Mock::given(method("POST"))
        .and(path(first_use_endpoint_path()))
        .respond_with(endpoint_upsert_ok())
        .expect(1)
        .mount(control)
        .await;
    first_use_endpoint_path()
}

/// A query host that rejects the caller's own key until it is bound: the
/// first statement is refused with `401`, every later one answers `1`.
async fn start_query_host_rejecting_the_first_query() -> MockServer {
    let mock = MockServer::start().await;
    let caller_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .and(header("authorization", caller_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .and(header("authorization", caller_auth.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .with_priority(5)
        .mount(&mock)
        .await;
    mock
}

fn first_use_query_process(
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
    sql: &str,
) -> tokio::process::Command {
    service_query_process_in_org(project_dir, control, query_host, FIRST_USE_ORG_ID, sql)
}

async fn run_first_use_query(
    control: &MockServer,
    query_host: &MockServer,
    extra_args: &[&str],
) -> (tempfile::TempDir, std::process::Output) {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let mut command = first_use_query_process(project.path(), control, query_host, "SELECT 1");
    command.args(extra_args);
    let output = command
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    (project, output)
}

/// The control-plane requests other than reads: what the run changed.
async fn control_plane_writes(control: &MockServer) -> Vec<String> {
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.method != wiremock::http::Method::GET)
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect()
}

/// Bind against an endpoint reporting `existing_keys` (or none, `None`) and
/// return the single upsert body, after asserting the common contract: the
/// query succeeds with the caller's own key, nothing but the upsert is
/// written, and no local record appears.
async fn assert_first_use_binds(
    control: &MockServer,
    existing_keys: Option<Value>,
) -> (tempfile::TempDir, Value) {
    mount_whoami(control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_first_use_endpoint_get(control, existing_keys).await;
    mount_first_use_endpoint_upsert(control, endpoint_upsert_ok()).await;
    let query_host = start_query_host_rejecting_the_first_query().await;

    let (project, output) = run_first_use_query(control, &query_host, &[]).await;
    assert_success(&output);
    assert_eq!(output.stdout, b"1\n");
    // The bind notice names the service.
    let progress = progress_lines(&output);
    assert!(
        progress.first().is_some_and(|line| line.contains("'demo'")),
        "{progress:#?}"
    );

    // Every query, before and after the bind, used the caller's own key.
    let caller_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    let queries = query_host.received_requests().await.unwrap();
    assert!(queries.len() >= 2, "the query must be re-run after binding");
    assert!(queries.iter().all(|request| {
        request
            .headers
            .get("authorization")
            .is_some_and(|value| value == caller_auth.as_str())
    }));

    let endpoint_path = first_use_endpoint_path();
    assert_eq!(
        control_plane_writes(control).await,
        [format!("POST {endpoint_path}")],
        "the bind is the only write: no key is created or deleted"
    );
    assert!(
        !project.path().join(".clickhouse/credentials.json").exists(),
        "binding the caller's key writes no local record"
    );
    let upsert = control
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| {
            request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
        })
        .unwrap();
    (project, serde_json::from_slice(&upsert.body).unwrap())
}

#[tokio::test]
async fn first_use_binds_the_caller_key_alongside_the_existing_bindings() {
    let control = start_mock_control_plane_with_service().await;
    mount_bound_key_lookup(&control, BOUND_KEY_A, 200).await;
    mount_bound_key_lookup(&control, BOUND_KEY_B, 200).await;
    let (_project, body) = assert_first_use_binds(
        &control,
        Some(serde_json::json!([BOUND_KEY_A, BOUND_KEY_B])),
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({
            "allowedOrigins": "*",
            "openApiKeys": [BOUND_KEY_A, BOUND_KEY_B, CALLER_KEY_UUID],
            "roles": ["sql_console_admin"],
        })
    );

    // Every call the run made is one `service query --help` advertises.
    let help_mock = MockServer::start().await;
    let help = permission_help(&help_mock, &["service", "query"], "--help");
    assert_observed_permissions_advertised(&control, &help).await;
}

#[tokio::test]
async fn first_use_creates_an_absent_endpoint_binding_only_the_caller_key() {
    let control = start_mock_control_plane_with_service().await;
    let (_project, body) = assert_first_use_binds(&control, None).await;
    assert_eq!(
        body,
        serde_json::json!({
            "allowedOrigins": "*",
            "openApiKeys": [CALLER_KEY_UUID],
            "roles": ["sql_console_admin"],
        })
    );
    // An explicitly empty key list is a real answer and binds the same way.
    let control = start_mock_control_plane_with_service().await;
    let (_project, body) = assert_first_use_binds(&control, Some(serde_json::json!([]))).await;
    assert_eq!(body["openApiKeys"], serde_json::json!([CALLER_KEY_UUID]));
}

#[tokio::test]
async fn first_use_prunes_only_bound_keys_the_organization_reports_deleted() {
    // A deleted key's UUID left bound makes every upsert fail (#659), so a
    // confirmed 404 drops it; a lookup that failed for any other reason says
    // nothing about the key, so it stays bound.
    let control = start_mock_control_plane_with_service().await;
    mount_bound_key_lookup(&control, BOUND_KEY_A, 200).await;
    mount_bound_key_lookup(&control, DELETED_BOUND_KEY, 404).await;
    mount_bound_key_lookup(&control, UNVERIFIABLE_BOUND_KEY, 500).await;
    let (_project, body) = assert_first_use_binds(
        &control,
        Some(serde_json::json!([
            BOUND_KEY_A,
            DELETED_BOUND_KEY,
            UNVERIFIABLE_BOUND_KEY
        ])),
    )
    .await;
    assert_eq!(
        body["openApiKeys"],
        serde_json::json!([BOUND_KEY_A, UNVERIFIABLE_BOUND_KEY, CALLER_KEY_UUID])
    );
}

#[tokio::test]
async fn first_use_never_looks_up_the_caller_key_it_is_binding() {
    // The caller is already bound next to a deleted key: pruning that key is
    // a real change, so the upsert runs, and the caller's own key is neither
    // looked up nor dropped.
    let control = start_mock_control_plane_with_service().await;
    mount_bound_key_lookup(&control, DELETED_BOUND_KEY, 404).await;
    let (_project, body) = assert_first_use_binds(
        &control,
        Some(serde_json::json!([CALLER_KEY_UUID, DELETED_BOUND_KEY])),
    )
    .await;
    assert_eq!(body["openApiKeys"], serde_json::json!([CALLER_KEY_UUID]));
    assert_eq!(
        control_plane_requests_to(&control, &first_use_key_path(CALLER_KEY_UUID)).await,
        0
    );
}

#[tokio::test]
async fn first_use_keeps_an_existing_endpoints_roles_and_origins() {
    // Roles are endpoint-wide: binding the caller to an existing read-only
    // endpoint must not grant every bound key `sql_console_admin`.
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_bound_key_lookup(&control, BOUND_KEY_A, 200).await;
    Mock::given(method("GET"))
        .and(path(first_use_endpoint_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "ep-1",
                "allowedOrigins": "https://app.example.com",
                "openApiKeys": [BOUND_KEY_A],
                "roles": ["sql_console_read_only"],
            },
            "status": 200,
            "requestId": "stub-endpoint-get",
        })))
        .mount(&control)
        .await;
    mount_first_use_endpoint_upsert(&control, endpoint_upsert_ok()).await;
    let query_host = start_query_host_rejecting_the_first_query().await;

    let (_project, output) = run_first_use_query(&control, &query_host, &[]).await;
    assert_success(&output);
    let upsert = control
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| {
            request.method == wiremock::http::Method::POST
                && request.url.path() == first_use_endpoint_path()
        })
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&upsert.body).unwrap(),
        serde_json::json!({
            "allowedOrigins": "https://app.example.com",
            "openApiKeys": [BOUND_KEY_A, CALLER_KEY_UUID],
            "roles": ["sql_console_read_only"],
        })
    );
}

#[tokio::test]
async fn first_use_echoes_absent_allowed_origins_as_absent() {
    // `allowedOrigins` is optional on the read and the upsert, so an endpoint
    // that reports none is rebound without one rather than refused or given
    // the new-endpoint default.
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_bound_key_lookup(&control, BOUND_KEY_A, 200).await;
    Mock::given(method("GET"))
        .and(path(first_use_endpoint_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "ep-1",
                "openApiKeys": [BOUND_KEY_A],
                "roles": ["sql_console_admin"],
            },
            "status": 200,
            "requestId": "stub-endpoint-get",
        })))
        .mount(&control)
        .await;
    mount_first_use_endpoint_upsert(&control, endpoint_upsert_ok()).await;
    let query_host = start_query_host_rejecting_the_first_query().await;

    let (_project, output) = run_first_use_query(&control, &query_host, &[]).await;
    assert_success(&output);
    let upsert = control
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| {
            request.method == wiremock::http::Method::POST
                && request.url.path() == first_use_endpoint_path()
        })
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&upsert.body).unwrap(),
        serde_json::json!({
            "openApiKeys": [BOUND_KEY_A, CALLER_KEY_UUID],
            "roles": ["sql_console_admin"],
        })
    );
}

/// The control plane of a first use whose endpoint already binds the caller
/// (listed in uppercase: still the same key) next to a live key, so the bind
/// has nothing to change.
async fn start_control_plane_already_binding_the_caller() -> MockServer {
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_bound_key_lookup(&control, BOUND_KEY_A, 200).await;
    mount_first_use_endpoint_get(
        &control,
        Some(serde_json::json!([
            BOUND_KEY_A,
            CALLER_KEY_UUID.to_ascii_uppercase()
        ])),
    )
    .await;
    mount_first_use_endpoint_upsert(&control, endpoint_upsert_ok()).await;
    control
}

#[tokio::test]
async fn first_use_waits_for_an_already_bound_caller_key_without_rebinding() {
    // A concurrent run may have just bound the key: rebinding would change
    // nothing, so the run writes nothing and waits for the endpoint instead.
    let control = start_control_plane_already_binding_the_caller().await;
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .up_to_n_times(3)
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .with_priority(5)
        .mount(&query_host)
        .await;
    let (project, output) = run_first_use_query(&control, &query_host, &[]).await;

    assert_success(&output);
    assert_eq!(output.stdout, b"1\n");
    // The bind notice, then the readiness wait (the only line not naming the
    // service).
    let progress = progress_lines(&output);
    assert_eq!(progress.len(), 2, "{progress:#?}");
    assert!(!progress[1].contains("'demo'"), "{progress:#?}");
    assert!(control_plane_writes(&control).await.is_empty());
    assert!(!project.path().join(".clickhouse/credentials.json").exists());
}

#[tokio::test]
async fn first_use_fails_when_an_already_bound_caller_key_is_rejected_for_the_whole_window() {
    // Rebinding a key the endpoint already lists changes nothing, so the run
    // makes no upsert, waits out the readiness window, and says where to look.
    let control = start_control_plane_already_binding_the_caller().await;
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .mount(&query_host)
        .await;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let output = first_use_query_process(project.path(), &control, &query_host, "SELECT 1")
        .arg("--json")
        .env(QUERY_READINESS_TIMEOUT_ENV, SHORT_QUERY_READINESS_MS)
        .output()
        .await
        .expect("failed to spawn clickhousectl");

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope: Value = serde_json::from_str(&stderr[stderr.find('{').unwrap()..]).unwrap();
    assert_eq!(envelope["error"]["code"], "query_key_bound_rejected");
    assert_eq!(envelope["error"]["api_key_id"], CALLER_KEY_UUID);
    let message = envelope["error"]["message"].as_str().unwrap();
    // The key, the waited-out window, and where to look.
    for fragment in [
        CALLER_KEY_UUID,
        "2s",
        "cloud service query-endpoint get",
        "cloud key get",
    ] {
        assert!(message.contains(fragment), "{fragment}: {message}");
    }
    assert!(control_plane_writes(&control).await.is_empty());
    assert!(
        query_host.received_requests().await.unwrap().len() > 2,
        "the whole window was used"
    );
    assert!(!project.path().join(".clickhouse/credentials.json").exists());
}

#[tokio::test]
async fn first_use_refuses_a_caller_that_is_not_an_api_key_of_the_resolved_org() {
    let user = serde_json::json!({
        "actorType": "user",
        "userId": "5fae43a3-1043-4000-8000-0000000000aa",
        "email": "ada@example.com",
    });
    let robot = serde_json::json!({ "actorType": "robot", "robotId": "r2" });
    // Which identity was refused is unit-tested beside `caller_key_uuid`; a
    // key of another organization names both organizations.
    for (identity, expected) in [
        (user, vec![]),
        (robot, vec![]),
        (
            caller_api_key_identity(OTHER_ORG_ID),
            vec![OTHER_ORG_ID.to_string(), FIRST_USE_ORG_ID.to_string()],
        ),
    ] {
        for json in [false, true] {
            let control = start_mock_control_plane_with_service().await;
            mount_whoami(&control, identity.clone()).await;
            let query_host = start_query_host_rejecting_the_first_query().await;
            let args: &[&str] = if json { &["--json"] } else { &[] };
            let (project, output) = run_first_use_query(&control, &query_host, args).await;

            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(output.stdout.is_empty());
            let message = if json {
                // The bind notice precedes the one structured error on stderr.
                let stderr = String::from_utf8_lossy(&output.stderr);
                let envelope: Value =
                    serde_json::from_str(&stderr[stderr.find('{').unwrap()..]).unwrap();
                envelope["error"]["message"].as_str().unwrap().to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
            };
            for fragment in &expected {
                assert!(message.contains(fragment.as_str()), "{message}");
            }
            // Refused before the lock and before any endpoint read or write.
            assert!(control_plane_writes(&control).await.is_empty());
            assert_eq!(
                control_plane_requests_to(&control, &first_use_endpoint_path()).await,
                0
            );
            assert_eq!(query_host.received_requests().await.unwrap().len(), 1);
            assert!(!project.path().join(".clickhouse/credentials.json").exists());
        }
    }
}

#[tokio::test]
async fn first_use_with_no_auto_enable_refuses_before_identifying_the_key() {
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_first_use_endpoint_get(&control, None).await;
    mount_first_use_endpoint_upsert(&control, endpoint_upsert_ok()).await;
    let query_host = start_query_host_rejecting_the_first_query().await;
    let (project, output) = run_first_use_query(&control, &query_host, &["--no-auto-enable"]).await;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--no-auto-enable"), "{stderr}");
    assert!(progress_lines(&output).is_empty(), "{stderr}");
    assert_eq!(control_plane_requests_to(&control, "/v1/whoami").await, 0);
    assert!(control_plane_writes(&control).await.is_empty());
    assert!(!project.path().join(".clickhouse/credentials.json").exists());
    assert_eq!(query_host.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_statement_refused_by_an_endpoint_that_accepts_the_key_is_a_query_failure() {
    // The endpoint answers the `SELECT 1` probe, so the 403 refused the
    // statement (a role missing a grant), not the key (#1240): no bind, no
    // re-run, and the same failure with or without `--no-auto-enable`.
    const STATEMENT: &str = "INSERT INTO t VALUES (1)";
    for extra in [None, Some("--no-auto-enable")] {
        let control = start_mock_control_plane_with_service().await;
        mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
        mount_first_use_endpoint_get(&control, Some(serde_json::json!([CALLER_KEY_UUID]))).await;
        let query_host = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
            .respond_with(|request: &wiremock::Request| {
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                if body["sql"] == "SELECT 1" {
                    ResponseTemplate::new(200).set_body_string("1\n")
                } else {
                    ResponseTemplate::new(403)
                        .set_body_string(r#"{"error":"queryEndpoint:ep-1: Not enough privileges"}"#)
                }
            })
            .mount(&query_host)
            .await;
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join("home")).unwrap();
        let mut command = first_use_query_process(project.path(), &control, &query_host, STATEMENT);
        command.arg("--json").args(extra);
        let output = command
            .output()
            .await
            .expect("failed to spawn clickhousectl");

        assert_eq!(output.status.code(), Some(1), "{extra:?}: {output:?}");
        assert!(output.stdout.is_empty());
        assert!(progress_lines(&output).is_empty(), "{extra:?}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let envelope: Value = serde_json::from_str(&stderr[stderr.find('{').unwrap()..]).unwrap();
        assert_eq!(envelope["error"]["code"], "http_4xx", "{extra:?}");
        assert_eq!(control_plane_requests_to(&control, "/v1/whoami").await, 0);
        assert_eq!(
            control_plane_requests_to(&control, &first_use_endpoint_path()).await,
            0
        );
        let sql: Vec<Value> = query_host
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap()["sql"].clone())
            .collect();
        assert_eq!(sql, [STATEMENT, "SELECT 1"], "{extra:?}");
    }
}

#[tokio::test]
async fn first_use_refuses_to_rebind_an_endpoint_whose_settings_are_unknown() {
    // A 200 endpoint GET that omits `openApiKeys` leaves the bound keys
    // unknown: the upsert replaces the list wholesale, so binding on top of an
    // assumed-empty list would revoke them. An omitted `roles` cannot be
    // echoed back, and a default would rewrite it.
    let complete = serde_json::json!({
        "id": "ep-1",
        "allowedOrigins": "*",
        "openApiKeys": [],
        "roles": ["sql_console_admin"],
    });
    for (field, expected) in [("openApiKeys", "'openApiKeys'"), ("roles", "roles")] {
        let mut result = complete.clone();
        result.as_object_mut().unwrap().remove(field);
        let control = start_mock_control_plane_with_service().await;
        mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
        Mock::given(method("GET"))
            .and(path(first_use_endpoint_path()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-endpoint-get",
            })))
            .mount(&control)
            .await;
        let query_host = start_query_host_rejecting_the_first_query().await;
        let (_project, output) = run_first_use_query(&control, &query_host, &[]).await;

        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{field}: {stderr}");
        assert!(control_plane_writes(&control).await.is_empty());
    }
}

#[tokio::test]
async fn a_failed_first_use_bind_has_nothing_to_roll_back() {
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_first_use_endpoint_get(&control, Some(serde_json::json!([]))).await;
    mount_first_use_endpoint_upsert(
        &control,
        ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "upsert failed",
            "status": 500,
            "requestId": "stub-endpoint-upsert",
        })),
    )
    .await;
    let query_host = start_query_host_rejecting_the_first_query().await;
    let (project, output) = run_first_use_query(&control, &query_host, &[]).await;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("upsert failed"));
    // The one write is the failed upsert: no key was created, so none is
    // deleted, and nothing local was recorded.
    assert_eq!(
        control_plane_writes(&control).await,
        [format!("POST {}", first_use_endpoint_path())]
    );
    assert!(!project.path().join(".clickhouse/credentials.json").exists());
    assert_eq!(query_host.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn concurrent_first_use_binds_serialize_and_leave_credentials_untouched() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    const PROCESS_COUNT: usize = 4;

    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    let project = tempfile::tempdir().unwrap();
    let original_credentials = write_preserved_query_credentials(project.path());
    std::fs::create_dir(project.path().join("home")).unwrap();

    // Lock ownership is held by the OS file handle. Contents left by a dead
    // process must not make the lock stale or block the next bind.
    let lock_path = project.path().join(".clickhouse/query-provisioning.lock");
    std::fs::write(&lock_path, "stale owner metadata").unwrap();
    let provisioning_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    provisioning_lock.lock().unwrap();

    // A stateful endpoint: each GET reports what the last upsert bound, so a
    // bind that did not see the previous one's result would show up here.
    let bound: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let endpoint_bound = Arc::new(AtomicBool::new(false));
    let get_state = Arc::clone(&bound);
    Mock::given(method("GET"))
        .and(path(first_use_endpoint_path()))
        .respond_with(
            move |_: &wiremock::Request| match &*get_state.lock().unwrap() {
                None => ResponseTemplate::new(404).set_body_json(serde_json::json!({
                    "error": "not found", "status": 404, "requestId": "stub-endpoint-get"
                })),
                Some(keys) => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": {
                        "id": "ep-1",
                        "allowedOrigins": "*",
                        "openApiKeys": keys,
                        "roles": ["sql_console_admin"],
                    },
                    "status": 200,
                    "requestId": "stub-endpoint-get"
                })),
            },
        )
        .mount(&control)
        .await;
    let upsert_state = Arc::clone(&bound);
    let upsert_flag = Arc::clone(&endpoint_bound);
    Mock::given(method("POST"))
        .and(path(first_use_endpoint_path()))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            *upsert_state.lock().unwrap() = Some(body["openApiKeys"].clone());
            upsert_flag.store(true, Ordering::SeqCst);
            endpoint_upsert_ok()
        })
        .expect(1)
        .mount(&control)
        .await;

    // The query host rejects the caller's key until the endpoint is bound.
    let query_host = MockServer::start().await;
    let query_flag = Arc::clone(&endpoint_bound);
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(move |_: &wiremock::Request| {
            if query_flag.load(Ordering::SeqCst) {
                ResponseTemplate::new(200).set_body_string("1\n")
            } else {
                ResponseTemplate::new(401).set_body_string("API key is not authorized")
            }
        })
        .mount(&query_host)
        .await;

    let mut children = Vec::with_capacity(PROCESS_COUNT);
    for _ in 0..PROCESS_COUNT {
        children.push(
            first_use_query_process(project.path(), &control, &query_host, "SELECT 1")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn clickhousectl"),
        );
    }
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while control_plane_requests_to(&control, "/v1/whoami").await < PROCESS_COUNT {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("concurrent queries did not all reach the bind");
    assert_eq!(
        control_plane_requests_to(&control, &first_use_endpoint_path()).await,
        0,
        "the endpoint is only read under the provisioning lock"
    );

    drop(provisioning_lock);
    for child in children {
        let output = child
            .wait_with_output()
            .await
            .expect("failed to wait for clickhousectl");
        assert_success(&output);
        assert_eq!(output.stdout, b"1\n");
    }

    let requests = control.received_requests().await.unwrap();
    let upserts: Vec<Value> = requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST
                && request.url.path() == first_use_endpoint_path()
        })
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect();
    // The first bind writes; every later one, serialized behind it, finds the
    // caller already bound, writes nothing and retries its query once.
    assert_eq!(upserts.len(), 1);
    assert_eq!(
        upserts[0]["openApiKeys"],
        serde_json::json!([CALLER_KEY_UUID]),
        "serialized binds never duplicate or drop the caller's key"
    );
    assert_eq!(
        control_plane_writes(&control).await.len(),
        1,
        "the upsert is the only write"
    );
    let credentials: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(credentials, original_credentials);
}

fn query_test_basic_auth(credentials: &str) -> String {
    format!(
        "Basic {}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials)
    )
}

#[tokio::test]
async fn first_use_query_retries_readiness_errors_with_the_caller_key() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{Duration, Instant};

    let control = start_mock_control_plane_with_service().await;
    let endpoint_path = mount_successful_caller_key_bind(&control).await;
    let query_host = MockServer::start().await;
    let query_path = format!("/service/{QUERY_TEST_SERVICE_ID}/run");
    let caller_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    Mock::given(method("POST"))
        .and(path(query_path.clone()))
        .and(header("authorization", caller_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&query_host)
        .await;

    let response_index = Arc::new(AtomicUsize::new(0));
    let delivered_statuses = Arc::new(Mutex::new(Vec::new()));
    let responder_index = Arc::clone(&response_index);
    let responder_statuses = Arc::clone(&delivered_statuses);
    Mock::given(method("POST"))
        .and(path(query_path))
        .and(header("authorization", caller_auth.as_str()))
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
        .with_priority(5)
        .expect(5)
        .mount(&query_host)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let started = Instant::now();
    let output = first_use_query_process(project.path(), &control, &query_host, "SELECT 42")
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    let elapsed = started.elapsed();

    assert_success(&output);
    assert_eq!(output.stdout, b"42\n");
    assert_eq!(*delivered_statuses.lock().unwrap(), [401, 403, 404, 206]);
    // Bind, readiness wait (the only line not naming the service), then wake.
    let progress = progress_lines(&output);
    assert_eq!(progress.len(), 3, "{progress:#?}");
    assert!(!progress[1].contains("'demo'"), "{progress:#?}");
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
    assert!(
        query_requests.iter().all(|request| {
            request
                .headers
                .get("authorization")
                .is_some_and(|value| value == caller_auth.as_str())
        }),
        "every attempt uses the caller's own key"
    );
    let sql: Vec<_> = query_requests
        .iter()
        .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap()["sql"].clone())
        .collect();
    assert_eq!(
        sql.iter().filter(|value| **value == "SELECT 1").count(),
        4,
        "readiness retries must use the harmless probe"
    );
    let user_queries: Vec<_> = query_requests
        .iter()
        .skip(1)
        .filter(|request| {
            serde_json::from_slice::<Value>(&request.body).unwrap()["sql"] == "SELECT 42"
        })
        .collect();
    assert_eq!(
        user_queries.len(),
        1,
        "user SQL must run exactly once after the bind"
    );
    assert_eq!(user_queries[0].headers.get("wake-service").unwrap(), "true");

    assert_eq!(
        control_plane_writes(&control).await,
        [format!("POST {endpoint_path}")],
        "readiness retries must not bind the endpoint again"
    );
}

#[tokio::test]
async fn first_use_query_fails_immediately_when_the_service_is_stopped() {
    use std::time::{Duration, Instant};

    let control = start_mock_control_plane_with_service().await;
    mount_successful_caller_key_bind(&control).await;
    let query_host = MockServer::start().await;
    let query_path = format!("/service/{QUERY_TEST_SERVICE_ID}/run");
    let caller_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    Mock::given(method("POST"))
        .and(path(query_path.clone()))
        .and(header("authorization", caller_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(query_path))
        .and(header("authorization", caller_auth.as_str()))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            r#"{"error":"ClickHouse service is currently unavailable. Please try again later."}"#,
        ))
        .with_priority(5)
        .expect(1)
        .mount(&query_host)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let started = Instant::now();
    let output = first_use_query_process(project.path(), &control, &query_host, "SELECT 1")
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
    // The bind notice, then exactly the stopped-service error and its start command.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{stderr}");
    assert!(lines[0].contains("'demo'"), "{stderr}");
    assert!(lines[1].starts_with("Error: "), "{stderr}");
    assert!(
        lines[1].contains(&format!(
            "`clickhousectl cloud service start {QUERY_TEST_SERVICE_ID} --org-id {FIRST_USE_ORG_ID}`"
        )),
        "{stderr}"
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
        "auth_config": {"auth_url": control_uri, "client_id": "test-client"},
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
        .isolated_env()
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
    // One error line carrying the server's code and details.
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(
        stderr.contains("62") && stderr.ends_with("Syntax error near FROM\n"),
        "{stderr}"
    );
}

#[tokio::test]
async fn service_query_preserves_malformed_json_error_and_status() {
    let body = r#"{"error":{"code":"62","details":"truncated"#;
    let output = invoke_oauth_service_query_error(body).await;
    assert_query_http_error(&output, 400, body);
}

#[tokio::test]
async fn service_query_preserves_non_json_error_and_status() {
    let output = invoke_oauth_service_query_error("upstream proxy failed").await;
    assert_query_http_error(&output, 400, "upstream proxy failed");
}

/// One error line carrying the status and the response body verbatim.
fn assert_query_http_error(output: &std::process::Output, status: u16, body: &str) {
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(stderr.contains(&status.to_string()), "{stderr}");
    assert!(stderr.ends_with(&format!("{body}\n")), "{stderr}");
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .isolated_env()
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(
        stderr.starts_with("Error: ") && stderr.contains("'demo'"),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "`clickhousectl cloud service start {QUERY_TEST_SERVICE_ID} --org-id org-1`"
        )),
        "{stderr}"
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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

// ── ClickPipe lifecycle acknowledgments (#842) ──────────────────────────────

#[tokio::test]
async fn clickpipe_lifecycle_human_output_acknowledges_the_immediate_state() {
    for (command, clickpipe_id, name, state, expected) in [
        (
            "start",
            "pipe-start",
            Some("batch-pipe"),
            Some("Stopped"),
            "ClickPipe batch-pipe start request accepted (returned state: Stopped)\n",
        ),
        (
            "stop",
            "pipe-stop",
            Some("cdc-pipe"),
            Some("Pausing"),
            "ClickPipe cdc-pipe stop request accepted (returned state: Pausing)\n",
        ),
        (
            "resync",
            "pipe-resync",
            None,
            None,
            "ClickPipe pipe-resync resync request accepted (returned state: -)\n",
        ),
    ] {
        let mock = MockServer::start().await;
        let mut result = serde_json::Map::new();
        if let Some(name) = name {
            result.insert("name".into(), serde_json::json!(name));
        }
        if let Some(state) = state {
            result.insert("state".into(), serde_json::json!(state));
        }
        Mock::given(method("PATCH"))
            .and(path(format!(
                "/v1/organizations/org/services/svc-id/clickpipes/{clickpipe_id}/state"
            )))
            .and(body_json(serde_json::json!({"command": command})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-clickpipe-state-change",
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let output = invoke_cli_with_cloud_credentials_human(
            &mock,
            &[
                "clickpipe",
                command,
                "svc-id",
                clickpipe_id,
                "--org-id",
                "org",
            ],
        );

        assert_success(&output);
        assert!(output.stderr.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }
}

#[tokio::test]
async fn clickpipe_lifecycle_preserves_the_json_response() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000842",
        "name": "batch-pipe",
        "state": "Stopped",
    });
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/pipe-start/state",
        ))
        .and(body_json(serde_json::json!({"command": "start"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-clickpipe-state-change-json",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "start",
            "svc-id",
            "pipe-start",
            "--org-id",
            "org",
        ],
    );

    assert_success(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
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

#[tokio::test]
async fn clickpipe_get_renders_a_glue_schema_registry_and_keeps_json_verbatim() {
    let source = serde_json::json!({
        "kafka": {
            "type": "msk",
            "format": "Avro",
            "brokers": "broker:9098",
            "topics": "events",
            "authentication": "IAM_ROLE",
            "iamRole": "arn:aws:iam::123456789012:role/Source",
            "schemaRegistry": {
                "type": "glue",
                "glueRegion": "us-east-1",
                "glueRegistryName": "my-registry",
                "glueRoleArn": "arn:aws:iam::123456789012:role/Glue"
            }
        }
    });
    let mock = MockServer::start().await;
    mount_clickpipe_get(&mock, source.clone()).await;
    let args = ["clickpipe", "get", "svc-id", "pipe-id", "--org-id", "org"];

    let output = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["source"], source);

    let output = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in [
        "type: glue",
        "glueRegion: us-east-1",
        "glueRegistryName: my-registry",
        "glueRoleArn: arn:aws:iam::123456789012:role/Glue",
    ] {
        assert!(stdout.contains(line), "{line}\n{stdout}");
    }
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
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            serde_json::json!({ "object_storage_max_file_count": 200 })
        );

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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--kafka-read-committed"), "{stderr}");
        assert!(!stderr.contains("kafka_read_committed"), "{stderr}");
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
        // The omitted settings field, and the flag that supplies it.
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("kafka_read_committed"), "{stderr}");
        assert!(stderr.contains("--kafka-read-committed"), "{stderr}");
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
    // The label for each database source is unit-tested; one source is enough.
    let mock = MockServer::start().await;
    mount_clickpipe_get(
        &mock,
        serde_json::json!({ "postgres": { "host": "db.example.com" } }),
    )
    .await;
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

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        cloud_runtime_error(&output)["message"],
        "ClickPipe pipe-id is a Postgres CDC pipe; `clickpipe settings get` and \
         `settings update` apply only to streaming (Kafka, Kinesis) and object-storage \
         pipes. CDC pipe settings (sync interval, pull batch size) live on the pipe \
         itself: see `clickhousectl cloud clickpipe get svc-id pipe-id`.",
    );
    assert_eq!(cloud_runtime_error(&output)["code"], "other");
    assert!(output.stdout.is_empty(), "a refusal must print no settings");
    // Only the pipe read: the settings endpoint is never called.
    assert_eq!(
        recorded_request_shape(&mock).await,
        vec![("GET".to_string(), CLICKPIPE_PATH.to_string())],
    );
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
        String::from_utf8_lossy(&output.stderr).contains("pipe-id"),
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
// `source` built by the same source builders as `clickpipe create`, which the
// unit tests cover; this wiring test asserts the organization/service path.

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
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/schemaDiscovery",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_response))
        .expect(1)
        .mount(&mock)
        .await;
    mock
}

#[tokio::test]
async fn schema_discover_kinesis_posts_source_body() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "--org-id",
            "org",
            "kinesis",
            "svc-id",
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

/// The Kafka arm assembles its discovery request in the handler, so this is
/// the only check that it sends the shared source without create's
/// `exactlyOnce`.
#[tokio::test]
async fn schema_discover_kafka_posts_the_source_without_create_only_fields() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "--org-id",
            "org",
            "kafka",
            "svc-id",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--iam-role",
            "arn:role",
        ],
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({"source": {"kafka": {
            "authentication": "IAM_ROLE",
            "brokers": "broker:9092",
            "credentials": null,
            "iamRole": "arn:role",
            "format": "JSONEachRow",
            "offset": {"strategy": "from_beginning"},
            "reversePrivateEndpointIds": [],
            "topics": "topic",
            "type": "kafka",
        }}})
    );
}

// ── Google Cloud Pub/Sub source (issue #587) ───────────────────────────────
//
// `clickpipe create pubsub` and `clickpipe schema-discover pubsub <SERVICE_ID>`
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
        .env(UPDATE_CHECK_TIMEOUT_ENV, "0")
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

    assert_eq!(
        output.status.code(),
        Some(1),
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

// ── ClickPipes GCP workload identity (issue #690) ──────────────────────────

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
async fn clickpipe_context_get_renders_json_and_human_output() {
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

    let home = tempfile::tempdir().unwrap();
    Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .current_dir(home.path())
        .args(&args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn service_reset_password_fails_when_the_generated_password_is_absent() {
    // The outcome check runs before either output branch, so one mode shows
    // that nothing is printed; `resolve_reset_password_outcome_*` covers the
    // modes.
    let output = run_service_reset_password(serde_json::json!({}), &[]).await;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "no success output may precede the failure:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    // The wording is unit-tested in `services::lifecycle`.
    assert!(!output.stderr.is_empty(), "{output:?}");
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

/// Both hash flags reach the PATCH body under their own fields, and a hash
/// reset succeeds without a password in the response.
#[tokio::test]
async fn service_reset_password_sends_both_hashes_and_accepts_no_password() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/password"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {}, "status": 200, "requestId": "stub-reset-password",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "reset-password",
            "svc-1",
            "--new-password-hash",
            "e3b0c442",
            "--new-double-sha1-hash",
            "aabbccdd",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({})
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(
        requests[0].body_json::<Value>().unwrap(),
        serde_json::json!({"newPasswordHash": "e3b0c442", "newDoubleSha1Hash": "aabbccdd"})
    );
}

// ── Generated API key material is never silently dropped ───────────────────
//
// `key create` without the pre-hash flags asks the API to generate a key pair
// returned exactly once. A response missing `keyId`/`keySecret` loses that
// credential, so validation runs before either output branch — `--json` (which
// agent detection also turns on by itself) must not print the incomplete
// response and exit zero.

async fn run_key_create(result: Value, extra_args: &[&str]) -> std::process::Output {
    run_key_create_with_requests(result, extra_args).await.0
}

async fn run_key_create_with_requests(
    result: Value,
    extra_args: &[&str],
) -> (std::process::Output, Vec<Value>) {
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

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .args(&args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .output()
        .expect("failed to spawn clickhousectl");
    let bodies = mock
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.body_json().unwrap())
        .collect();
    (output, bodies)
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
        assert_eq!(
            output.status.code(),
            Some(1),
            "a generated key create with no secret must fail for args {extra_args:?}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout),
        );
        // The wording is unit-tested in `api_keys`. Neither the human
        // confirmation nor the incomplete response body may reach stdout
        // ahead of the failure.
        assert!(!output.stderr.is_empty(), "{extra_args:?}");
        assert!(
            output.stdout.is_empty(),
            "no success output may precede the failure for args {extra_args:?}:\n{}",
            String::from_utf8_lossy(&output.stdout),
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
async fn key_create_human_output_shows_the_secret_and_warns_about_deny_all() {
    let output = run_key_create(
        serde_json::json!({
            "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" },
            "keyId": "generated-key-id",
            "keySecret": "generated-key-secret",
        }),
        &[],
    )
    .await;
    // Which label each value sits under is unit-tested on `render_key_created`
    // in `api_keys`; here the confirmation must reach stdout with the secret.
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("generated-key-secret"), "{stdout}");
    // No --ip-allow: one warning naming the flag and the command that adds one.
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("Warning: "), "{stderr}");
    assert!(
        stderr.contains("`cloud key update <resource-id> --ip-allow <IP_OR_CIDR>`"),
        "{stderr}"
    );
}

#[tokio::test]
async fn key_create_succeeds_for_a_pre_hashed_key_without_generated_material() {
    // The response has no key object either, so the absent name and resource
    // ID render as placeholders; an --ip-allow entry suppresses the warning.
    let (output, bodies) = run_key_create_with_requests(
        serde_json::json!({}),
        &[
            "--hash-key-id",
            "0f1e2d3c",
            "--hash-key-id-suffix",
            "3c",
            "--hash-key-secret",
            "4b5a6978",
            "--ip-allow",
            "198.51.100.4/32",
        ],
    )
    .await;
    // Placeholders for the absent name and ID, and no key material at all.
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().filter(|line| line.ends_with(": -")).count(),
        2,
        "{stdout}"
    );
    assert_eq!(stdout.lines().count(), 4, "{stdout}");
    assert!(!stdout.contains("4b5a6978"), "{stdout}");
    assert!(output.stderr.is_empty());
    assert_eq!(
        bodies,
        [serde_json::json!({
            "name": "ci",
            "state": "enabled",
            "assignedRoleIds": [],
            "ipAccessList": [{"source": "198.51.100.4/32"}],
            "hashData": {"keyIdHash": "0f1e2d3c", "keyIdSuffix": "3c", "keySecretHash": "4b5a6978"},
        })]
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
            "--remove-tag",
            "env",
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

    let requests = mock.received_requests().await.unwrap();
    let patch = requests
        .iter()
        .find(|request| request.method.as_str() == "PATCH")
        .unwrap();
    let body: Value = serde_json::from_slice(&patch.body).unwrap();
    assert_eq!(
        body["tags"]["remove"],
        serde_json::json!([{ "key": "missing" }, { "key": "env" }])
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
            "service",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--new-name",
            "renamed",
            "--add-tag",
            "env=prod",
            "--add-tag",
            "team=analytics",
        ],
    );

    assert_success(&output);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["name"], "renamed");
    assert_eq!(
        body["tags"]["add"],
        serde_json::json!([
            { "key": "env", "value": "prod" },
            { "key": "team", "value": "analytics" },
        ])
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
            "--description",
            "prod",
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(
        requests[0].body_json::<Value>().unwrap(),
        serde_json::json!({"id": "vpce-0123456789abcdef0", "description": "prod"})
    );
}

// ── `clickpipe scale` (issue #605) ─────────────────────────────────────────
//
// The at-least-one-target rule and the value ranges are clap rules with their
// own unit tests. The handler builds the PATCH body itself, so these tests pin
// that body on the wire.

#[tokio::test]
async fn clickpipe_scale_sends_exactly_the_requested_targets() {
    for (flags, body) in [
        // An omitted target stays off the wire.
        (vec!["--replicas", "4"], serde_json::json!({"replicas": 4})),
        (
            vec![
                "--replicas",
                "1",
                "--cpu-millicores",
                "125",
                "--memory-gb",
                "0.5",
            ],
            serde_json::json!({
                "replicas": 1, "replicaCpuMillicores": 125, "replicaMemoryGb": 0.5
            }),
        ),
        (
            vec![
                "--replicas",
                "40",
                "--cpu-millicores",
                "2000",
                "--memory-gb",
                "8",
            ],
            serde_json::json!({
                "replicas": 40, "replicaCpuMillicores": 2000, "replicaMemoryGb": 8.0
            }),
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/scaling",
            ))
            .and(body_json(&body))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"id": "11111111-2222-3333-4444-555555555555", "scaling": body},
                "status": 200,
                "requestId": "stub-clickpipe-scale",
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec!["clickpipe", "scale", "svc-1", "pipe-1", "--org-id", "org-1"];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
    }
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
async fn clickpipe_cdc_scaling_update_patches_the_built_body() {
    let mock = MockServer::start().await;
    let scaling = serde_json::json!({ "replicaCpuMillicores": 1000 });
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
            "1000",
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
    start_mock_control_plane_with_query_state(Some("running")).await
}

async fn start_mock_control_plane_with_query_state(state: Option<&str>) -> MockServer {
    let mock = MockServer::start().await;
    let stub_service = serde_json::json!({
        "result": {
            "id": QUERY_TEST_SERVICE_ID,
            "name": "demo",
            "state": state,
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
    for org_id in ["org-1", FIRST_USE_ORG_ID] {
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/{org_id}/services/{QUERY_TEST_SERVICE_ID}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(stub_service.clone()))
            .mount(&mock)
            .await;
    }
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
    // The gateway diagnosis, by the statement it suggests checking and the
    // native-protocol commands.
    let stderr = String::from_utf8_lossy(&output.stderr);
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
    assert!(
        message.contains("SELECT query_id, elapsed FROM system.processes"),
        "{message}"
    );
}

// ── Query timeout diagnosis after an idle/wake observation (#826) ─────────

async fn invoke_service_query_for_timeout_test(
    control: &MockServer,
    query_host: &MockServer,
    auth: &str,
    json: bool,
) -> std::process::Output {
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home/.clickhouse");
    std::fs::create_dir_all(&home).unwrap();
    let sql = if auth == "oauth" {
        "SELECT 826"
    } else {
        "INSERT INTO events VALUES (826)"
    };
    let mut command = service_query_process_with_sql(project.path(), control, query_host, sql);
    match auth {
        "oauth" => {
            write_oauth_tokens(&home, &control.uri());
            command
                .env_remove("CLICKHOUSE_CLOUD_API_KEY")
                .env_remove("CLICKHOUSE_CLOUD_API_SECRET");
        }
        "stored" => {
            write_repair_query_credentials(
                project.path(),
                &control.uri(),
                Some("org-1"),
                Some(QUERY_TEST_KEY_UUID),
                Some("ep-1"),
                &[],
            );
        }
        "management" => {}
        other => panic!("unexpected auth mode: {other}"),
    }
    if json {
        command.arg("--json");
    }
    command
        .output()
        .await
        .expect("failed to spawn clickhousectl")
}

fn assert_query_wake_timeout(output: &std::process::Output, json: bool) {
    assert_query_wake_timeout_in_org(output, json, "org-1");
}

fn assert_query_wake_timeout_in_org(output: &std::process::Output, json: bool, org_id: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = if json {
        // An explicit wake confirmation retains its existing progress notice
        // before the structured error; a pre-query state adds no notice.
        let envelope: Value = serde_json::from_str(&stderr[stderr.find('{').unwrap()..]).unwrap();
        let error = &envelope["error"];
        assert_eq!(error["code"], "query_timeout");
        assert_eq!(
            error["command"],
            format!("clickhousectl cloud service get {QUERY_TEST_SERVICE_ID} --org-id {org_id}")
        );
        assert!(error.get("host").is_none());
        assert!(error.get("port").is_none());
        error["message"].as_str().unwrap().to_string()
    } else {
        stderr.to_string()
    };
    // The wake diagnosis points at the service state, never at the gateway
    // diagnosis's statement check or native client.
    assert!(
        message.contains(&format!(
            "`clickhousectl cloud service get {QUERY_TEST_SERVICE_ID} --org-id {org_id}`"
        )),
        "{message}"
    );
    assert!(!message.contains("clickhouse client"), "{message}");
    assert!(!message.contains("system.processes"), "{message}");
    for sensitive in [
        "SELECT 826",
        "INSERT INTO",
        "stored-key-secret",
        "test-bearer-token",
    ] {
        assert!(!message.contains(sensitive), "{message}");
    }
}

#[tokio::test]
async fn query_timeout_uses_existing_idle_or_awaking_state_without_retry_or_preflight() {
    for state in [
        Some("idle"),
        Some("awaking"),
        Some("running"),
        None,
        Some("future-state"),
    ] {
        for auth in ["management", "stored", "oauth"] {
            for json in [false, true] {
                let control = start_mock_control_plane_with_query_state(state).await;
                let query_host = MockServer::start().await;
                Mock::given(method("POST"))
                    .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
                    .respond_with(
                        ResponseTemplate::new(500).set_body_string(QUERY_GATEWAY_TIMEOUT_BODY),
                    )
                    .expect(1)
                    .mount(&query_host)
                    .await;
                let output =
                    invoke_service_query_for_timeout_test(&control, &query_host, auth, json).await;
                if matches!(state, Some("idle" | "awaking")) {
                    assert_query_wake_timeout(&output, json);
                } else {
                    assert_eq!(output.status.code(), Some(1));
                    assert!(output.stdout.is_empty());
                    // The gateway diagnosis.
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert!(stderr.contains("system.processes"), "{stderr}");
                    if json {
                        assert_eq!(cloud_runtime_error(&output)["code"], "query_timeout");
                    }
                }
                let requests = query_host.received_requests().await.unwrap();
                assert_eq!(requests.len(), 1, "no SQL replay after a timeout");
                assert!(requests[0].headers.get("wake-service").is_none());
                let expected_auth = match auth {
                    "oauth" => "Bearer test-bearer-token".to_string(),
                    "stored" => query_test_basic_auth("stored-key-id:stored-key-secret"),
                    _ => query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests"),
                };
                assert_eq!(
                    requests[0].headers.get("authorization").unwrap(),
                    expected_auth.as_str()
                );
                assert_eq!(
                    control.received_requests().await.unwrap().len(),
                    1,
                    "reuse the existing service lookup"
                );
            }
        }
    }
}

#[tokio::test]
async fn query_timeout_after_explicit_wake_preserves_context_and_never_replays_sql() {
    for auth in ["management", "stored", "oauth"] {
        for json in [false, true] {
            // The newer pre-execution idle response overrides this earlier
            // running snapshot for diagnosis, without another service GET.
            let control = start_mock_control_plane_with_query_state(Some("running")).await;
            let query_host = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
                .respond_with(|request: &wiremock::Request| {
                    if request.headers.contains_key("wake-service") {
                        ResponseTemplate::new(500).set_body_string(QUERY_GATEWAY_TIMEOUT_BODY)
                    } else {
                        ResponseTemplate::new(206)
                            .set_body_string(r#"{"data":"Confirm wake service"}"#)
                    }
                })
                .expect(2)
                .mount(&query_host)
                .await;
            let output =
                invoke_service_query_for_timeout_test(&control, &query_host, auth, json).await;
            assert_query_wake_timeout(&output, json);
            let requests = query_host.received_requests().await.unwrap();
            assert_eq!(
                requests.len(),
                2,
                "one rejected attempt and one accepted wake submission"
            );
            assert!(requests[0].headers.get("wake-service").is_none());
            assert_eq!(requests[1].headers.get("wake-service").unwrap(), "true");
            let first: Value = serde_json::from_slice(&requests[0].body).unwrap();
            let second: Value = serde_json::from_slice(&requests[1].body).unwrap();
            assert_eq!(first["sql"], second["sql"]);
            assert_eq!(control.received_requests().await.unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn query_timeout_after_first_use_bind_idle_probe_submits_user_sql_once() {
    let control = start_mock_control_plane_with_query_state(Some("running")).await;
    mount_successful_caller_key_bind(&control).await;
    let query_host = MockServer::start().await;
    let caller_auth = query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests");
    // The caller's key is refused until the bind; the later attempts use
    // the same key.
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(401).set_body_string("not authorized"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            if body["sql"] == "SELECT 1" {
                ResponseTemplate::new(206).set_body_string(r#"{"data":"Confirm wake service"}"#)
            } else {
                ResponseTemplate::new(500).set_body_string(QUERY_GATEWAY_TIMEOUT_BODY)
            }
        })
        .with_priority(5)
        .expect(2)
        .mount(&query_host)
        .await;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("home")).unwrap();
    let output = first_use_query_process(
        project.path(),
        &control,
        &query_host,
        "INSERT INTO events VALUES (826)",
    )
    .arg("--json")
    .output()
    .await
    .expect("failed to spawn clickhousectl");
    assert_query_wake_timeout_in_org(&output, true, FIRST_USE_ORG_ID);
    let requests = query_host.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        3,
        "one auth rejection, one idle probe, one SQL submission"
    );
    let submitted = &requests[2];
    assert_eq!(submitted.headers.get("wake-service").unwrap(), "true");
    assert_eq!(
        submitted.headers.get("authorization").unwrap(),
        caller_auth.as_str()
    );
    let submitted_body: Value = serde_json::from_slice(&submitted.body).unwrap();
    assert_eq!(submitted_body["sql"], "INSERT INTO events VALUES (826)");
    let service_reads = control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| {
            request.url.path()
                == format!("/v1/organizations/{FIRST_USE_ORG_ID}/services/{QUERY_TEST_SERVICE_ID}")
        })
        .count();
    assert_eq!(service_reads, 1);
}

#[tokio::test]
async fn query_on_idle_service_can_succeed_without_a_wake_confirmation() {
    for auth in ["management", "stored", "oauth"] {
        let control = start_mock_control_plane_with_query_state(Some("idle")).await;
        let query_host = start_mock_query_host().await;
        let output = invoke_service_query_for_timeout_test(&control, &query_host, auth, true).await;
        assert_success(&output);
        assert_eq!(output.stdout, b"1\n");
        assert!(output.stderr.is_empty());
        assert_eq!(query_host.received_requests().await.unwrap().len(), 1);
        assert_eq!(control.received_requests().await.unwrap().len(), 1);
    }
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

    // JSON mode preserves the message and classifies the typed HTTP failure.
    let (json_output, _) = invoke_service_query_against_failing_query_host(
        &control,
        500,
        r#"{"error":"Internal error."}"#,
        &["--json"],
    )
    .await;
    let error = cloud_runtime_error(&json_output);
    assert_eq!(error["code"], "http_5xx");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("500"), "{message}");
    assert!(
        message.ends_with(r#"{"error":"Internal error."}"#),
        "{message}"
    );
}

// ── `clickpipe reverse-private-endpoint` CRUD (issue #567) ─────────────────
//
// PrivateLink connectivity for ClickPipes is a reverse private endpoint the
// user creates on the service and then references from a pipe: by ID for
// Kafka, or by DNS name as `--host` for Postgres/MySQL CDC. These tests pin
// the five requests and what each handler prints; the per-type bodies and
// the type/flag validation are unit tests in `src/cloud/clickpipe_endpoints.rs`
// and `main.rs`.

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
    // The resource array itself, not the API envelope around it.
    assert_eq!(stdout, serde_json::json!([reverse_private_endpoint_json()]));
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
    // Type, Description, Status, DNS Names and DNS Targets are all absent here.
    assert_eq!(stdout.matches(" - ").count(), 5, "{stdout}");
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

/// The create handler posts the builder's body to the service's collection.
/// Each type's field mapping is a `build_create_reverse_private_endpoint_request`
/// unit test.
#[tokio::test]
async fn reverse_private_endpoint_create_posts_the_built_body() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(RPE_COLLECTION_PATH))
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
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("POST".to_string(), RPE_COLLECTION_PATH.to_string())],
    );
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "description": "warehouse",
            "type": "VPC_ENDPOINT_SERVICE",
            "vpcEndpointServiceName": "com.amazonaws.vpce.us-east-1.vpce-svc-1",
        })
    );
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
    assert!(stdout.contains(RPE_ID), "{stdout}");
    assert!(stdout.contains("Status: PendingAcceptance"), "{stdout}");
    assert!(
        stdout.contains("DNS names: vpce-1-abc.vpce.amazonaws.com"),
        "{stdout}"
    );
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
            "*.example.com=rcfg-2",
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
    // A mapping without `=TARGET_ID` leaves `targetId` off the wire.
    assert_eq!(
        body,
        serde_json::json!({
            "customPrivateDnsMappings": [
                { "privateDnsName": "db.example.com" },
                { "privateDnsName": "*.example.com", "targetId": "rcfg-2" },
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
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stdout.contains(RPE_ID), "{stdout}");

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
            vec!["org", "get", "--org-id", UNKNOWN_UUID],
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
        assert!(
            message.ends_with(&format!(
                "BAD_REQUEST: Invalid {thing} id string:\"{UNKNOWN_UUID}\""
            )),
            "{stderr}"
        );
    }
}

// ── Postgres by-id commands report a missing service like `get` (#1156) ──

fn unknown_postgres_path(suffix: &str) -> String {
    format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{UNKNOWN_UUID}{suffix}")
}

/// The not-found message `postgres get` prints for [`UNKNOWN_UUID`]: it names
/// the ID and organization and ends in the rejected request's own server
/// message.
fn assert_postgres_not_found_message(message: &str, server_message: &str) {
    assert!(message.contains(UNKNOWN_UUID), "{message}");
    assert!(message.contains(LOOKUP_ORG_ID), "{message}");
    assert!(message.ends_with(server_message), "{message}");
}

fn assert_postgres_not_found_json(output: &std::process::Output, server_message: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = cloud_runtime_error(output);
    assert_eq!(error["code"], "resource_not_found");
    assert_eq!(
        error["command"],
        format!("clickhousectl cloud postgres list --org-id {LOOKUP_ORG_ID}")
    );
    assert_postgres_not_found_message(error["message"].as_str().unwrap(), server_message);
}

fn assert_postgres_not_found_human(output: &std::process::Output, server_message: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr
        .strip_prefix("Error: ")
        .and_then(|rest| rest.strip_suffix('\n'))
        .unwrap_or_else(|| panic!("one error line: {stderr}"));
    assert_postgres_not_found_message(message, server_message);
}

/// `postgres update` reads the service's tags before it patches. A missing
/// service must stop there with the same answer `postgres get` gives, and
/// must not send the PATCH.
#[tokio::test]
async fn postgres_update_of_a_missing_service_reads_as_not_found() {
    let server_message =
        format!("BAD_REQUEST: Invalid Postgres service id string:\"{UNKNOWN_UUID}\"");
    let args = [
        "postgres",
        "update",
        UNKNOWN_UUID,
        "--org-id",
        LOOKUP_ORG_ID,
        "--new-name",
        "renamed",
    ];
    let mock = MockServer::start().await;
    mount_invalid_id_400(
        &mock,
        unknown_postgres_path(""),
        "Postgres service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_postgres_not_found_json(&output, &server_message);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), unknown_postgres_path(""))]
    );
}

/// `--clear-tags` skips the tag read, so the PATCH is what fails. Its body is
/// user input, so the CLI only calls the service missing after a GET of the
/// service also rejects the id.
#[tokio::test]
async fn postgres_update_clear_tags_of_a_missing_service_reads_as_not_found() {
    let mock = MockServer::start().await;
    let server_message =
        format!("BAD_REQUEST: Invalid Postgres service id string:\"{UNKNOWN_UUID}\"");
    Mock::given(method("PATCH"))
        .and(path(unknown_postgres_path("")))
        .respond_with(invalid_id_400_response("Postgres service", UNKNOWN_UUID))
        .expect(1)
        .mount(&mock)
        .await;
    mount_invalid_id_400(
        &mock,
        unknown_postgres_path(""),
        "Postgres service",
        UNKNOWN_UUID,
    )
    .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "postgres",
            "update",
            UNKNOWN_UUID,
            "--org-id",
            LOOKUP_ORG_ID,
            "--clear-tags",
        ],
    );

    assert_postgres_not_found_json(&output, &server_message);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("PATCH".to_string(), unknown_postgres_path("")),
            ("GET".to_string(), unknown_postgres_path("")),
        ]
    );
}

fn postgres_config_patch_args(setting: &'static str) -> [&'static str; 8] {
    [
        "postgres",
        "config",
        "patch",
        UNKNOWN_UUID,
        "--org-id",
        LOOKUP_ORG_ID,
        "--set",
        setting,
    ]
}

/// A config PATCH for a deleted service answers 404; the confirming GET of
/// the service agrees, so the command reports the service as missing. The
/// JSON form of the answer is in the update and clear-tags tests.
#[tokio::test]
async fn postgres_config_patch_of_a_missing_service_reads_as_not_found() {
    let server_message = "NOT_FOUND: Postgres service not found";
    let args = postgres_config_patch_args("max_connections=500");
    let mock = MockServer::start().await;
    for (http_method, route) in [
        ("PATCH", unknown_postgres_path("/config")),
        ("GET", unknown_postgres_path("")),
    ] {
        Mock::given(method(http_method))
            .and(path(route))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "status": 404,
                "error": server_message,
            })))
            .expect(1)
            .mount(&mock)
            .await;
    }

    let output = invoke_cli_human(&mock, &args);
    assert_postgres_not_found_human(&output, server_message);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("PATCH".to_string(), unknown_postgres_path("/config")),
            ("GET".to_string(), unknown_postgres_path("")),
        ]
    );
}

/// A config PATCH rejected for its body answers 400 too. The confirming GET
/// returns a service that decodes, so the service exists and the server's
/// validation message must reach the user unchanged rather than being
/// misreported as a missing service.
#[tokio::test]
async fn postgres_config_patch_validation_400_keeps_the_servers_message() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(unknown_postgres_path("/config")))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "BAD_REQUEST: max_connections must be at least 10",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    // The success envelope the `postgres get` tests decode.
    mount_postgres_service_response(
        &mock,
        "GET",
        &unknown_postgres_path(""),
        serde_json::json!({
            "id": UNKNOWN_UUID,
            "name": "existing-pg",
            "state": "running",
            "region": "us-east-1",
            "provider": "aws",
            "isPrimary": true,
        }),
    )
    .await;

    let output =
        invoke_cli_with_cloud_credentials(&mock, &postgres_config_patch_args("max_connections=1"));

    assert_eq!(output.status.code(), Some(1));
    let error = cloud_runtime_error(&output);
    assert_eq!(
        error["message"],
        "BAD_REQUEST: max_connections must be at least 10"
    );
    assert_eq!(error["code"], "http_4xx");
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("PATCH".to_string(), unknown_postgres_path("/config")),
            ("GET".to_string(), unknown_postgres_path("")),
        ]
    );
}

/// When the confirming GET itself fails for another reason (a server error,
/// or a permission the caller lacks), it cannot show the service is missing.
/// The original PATCH error stands: its message, JSON code and exit code are
/// unchanged, never not-found and never the GET's own failure.
#[tokio::test]
async fn postgres_config_patch_keeps_the_original_error_when_the_confirming_get_fails() {
    let args = postgres_config_patch_args("max_connections=1");
    for (patch_status, patch_message, get_status, get_message, json) in [
        (
            400,
            "BAD_REQUEST: max_connections must be at least 10",
            500,
            "INTERNAL_ERROR: confirming GET failed",
            true,
        ),
        (
            404,
            "NOT_FOUND: Postgres service not found",
            403,
            "FORBIDDEN: confirming GET not allowed",
            false,
        ),
    ] {
        let mock = MockServer::start().await;
        for (http_method, route, status, message) in [
            (
                "PATCH",
                unknown_postgres_path("/config"),
                patch_status,
                patch_message,
            ),
            ("GET", unknown_postgres_path(""), get_status, get_message),
        ] {
            Mock::given(method(http_method))
                .and(path(route))
                .respond_with(
                    ResponseTemplate::new(status)
                        .set_body_json(serde_json::json!({"status": status, "error": message})),
                )
                .expect(1)
                .mount(&mock)
                .await;
        }

        let case = format!("PATCH {patch_status}, GET {get_status}, json {json}");
        if json {
            let output = invoke_cli_with_cloud_credentials(&mock, &args);
            assert_eq!(output.status.code(), Some(1), "{case}");
            assert!(output.stdout.is_empty(), "{case}");
            let error = cloud_runtime_error(&output);
            assert_eq!(error["message"], patch_message, "{case}");
            assert_eq!(error["code"], "http_4xx", "{case}");
        } else {
            let output = invoke_cli_human(&mock, &args);
            assert_eq!(output.status.code(), Some(1), "{case}");
            assert!(output.stdout.is_empty(), "{case}");
            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                format!("Error: {patch_message}\n"),
                "{case}"
            );
        }
        assert_eq!(
            received_request_shape(&mock).await,
            vec![
                ("PATCH".to_string(), unknown_postgres_path("/config")),
                ("GET".to_string(), unknown_postgres_path("")),
            ],
            "{case}"
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
/// missing. The delete's own 400 is then confirmed by a second read (#1226).
#[tokio::test]
async fn forced_service_delete_proceeds_when_the_read_rejects_a_well_formed_id() {
    let mock = MockServer::start().await;
    let service_path = format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}");
    Mock::given(method("GET"))
        .and(path(service_path.clone()))
        .respond_with(invalid_id_400_response("service", UNKNOWN_UUID))
        .expect(2)
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
            ("DELETE".to_string(), service_path.clone()),
            ("GET".to_string(), service_path),
        ]
    );
}

/// Without `--force`, the delete's own 400 is refined to not-found only once a
/// read of the service rejects the id too (#1226).
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
    mount_invalid_id_400(&mock, service_path.clone(), "service", UNKNOWN_UUID).await;

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
        vec![
            ("DELETE".to_string(), service_path.clone()),
            ("GET".to_string(), service_path),
        ]
    );
}

/// The API answers a bare 400 to a DELETE of a service that is still
/// provisioning. The service exists, so neither `service delete` (with or
/// without `--force`) nor `postgres delete` may report it as
/// `resource_not_found`, which automation reads as already gone (#1226).
#[tokio::test]
async fn delete_rejected_with_400_for_an_existing_service_is_not_resource_not_found() {
    let bare_400 = || {
        ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "BAD_REQUEST: Bad Request",
        }))
    };
    let service_path = format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}");
    let postgres_path = unknown_postgres_path("");
    for (args, resource_path, reads) in [
        (
            vec!["service", "delete", UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID],
            service_path.clone(),
            1,
        ),
        (
            vec![
                "service",
                "delete",
                UNKNOWN_UUID,
                "--org-id",
                LOOKUP_ORG_ID,
                "--force",
            ],
            service_path.clone(),
            2,
        ),
        (
            vec![
                "postgres",
                "delete",
                UNKNOWN_UUID,
                "--org-id",
                LOOKUP_ORG_ID,
            ],
            postgres_path,
            1,
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(resource_path.clone()))
            .respond_with(manual_query_endpoint_response(
                serde_json::json!({"id": UNKNOWN_UUID, "state": "provisioning"}),
            ))
            .expect(reads)
            .mount(&mock)
            .await;
        Mock::given(method("DELETE"))
            .and(path(resource_path.clone()))
            .respond_with(bare_400())
            .expect(1)
            .mount(&mock)
            .await;

        let output = invoke_cli_with_cloud_credentials(&mock, &args);

        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let error = cloud_runtime_error(&output);
        assert_eq!(error["code"], "http_4xx", "{args:?}: {error}");
        assert!(error.get("command").is_none(), "{args:?}: {error}");
        if args[0] == "service" {
            let message = error["message"].as_str().unwrap();
            assert!(message.contains(UNKNOWN_UUID), "{message}");
            assert!(message.contains("'provisioning'"), "{message}");
        }
        // No stop was attempted for a provisioning service.
        assert!(
            received_request_shape(&mock)
                .await
                .iter()
                .all(|(method, _)| method == "GET" || method == "DELETE"),
            "{args:?}"
        );
    }
}

// Postgres config files must survive every advertised file-input path, PgConfig
// (#591) and PgBouncer (#697) alike. Read-replica and restore bodies are
// assembled in their handlers, so their exact bodies are asserted here.
fn postgres_config_write_commands<'a>(
    pg_config_file: &'a str,
    pg_bouncer_file: &'a str,
    config_file: &'a str,
) -> Vec<(Vec<&'a str>, &'static str, &'static str)> {
    let files = [
        "--pg-config-file",
        pg_config_file,
        "--pg-bouncer-config-file",
        pg_bouncer_file,
        "--tag",
        "env=prod",
    ];
    let with_files = |args: &[&'a str]| -> Vec<&'a str> {
        let mut args = args.to_vec();
        args.extend(files);
        args
    };
    vec![
        (
            with_files(&[
                "postgres",
                "create",
                "--name",
                "pg-test",
                "--region",
                "us-east-1",
                "--size",
                "c6gd.large",
            ]),
            "POST",
            "/v1/organizations/org-1/postgres",
        ),
        (
            with_files(&[
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "replica-test",
            ]),
            "POST",
            "/v1/organizations/org-1/postgres/pg-1/readReplica",
        ),
        (
            with_files(&[
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "restored-test",
                "--restore-target",
                "2026-08-01T00:00:00+01:00",
                "--block-public-access",
            ]),
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

/// The write-path response, plus for a restore the ID, tags and (blocked)
/// rules the CLI reads to decide whether the restored rules need a PATCH.
fn restored_service_response(endpoint: &str, result: &Value) -> Value {
    let mut result = result.clone();
    if endpoint.ends_with("/restoredService") {
        result["id"] = serde_json::json!("22222222-3333-4444-5555-666666666666");
        result["tags"] = serde_json::json!([]);
        result["ipAccessList"] = serde_json::json!([]);
    }
    result
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
async fn postgres_config_files_reach_every_write_path() {
    let directory = tempfile::tempdir().unwrap();
    let pg_config_file = directory.path().join("pg.json");
    let pg_bouncer_file = directory.path().join("pgbouncer.json");
    let config_file = directory.path().join("config.json");
    let pg_config = serde_json::json!({
        "max_connections": 0,
        "autovacuum_analyze_scale_factor": false,
        "default_transaction_isolation": "repeatable read",
        "wal_compression": "zstd",
        "bgwriter_delay": "200ms"
    });
    let parameters =
        serde_json::json!({"default_pool_size": "16", "future_parameter": "on", "empty": ""});
    let configuration = serde_json::json!({"pgConfig": pg_config, "pgBouncerConfig": parameters});
    std::fs::write(&pg_config_file, pg_config.to_string()).unwrap();
    std::fs::write(&pg_bouncer_file, parameters.to_string()).unwrap();
    std::fs::write(&config_file, configuration.to_string()).unwrap();
    let tags = serde_json::json!([{"key": "env", "value": "prod"}]);
    let with_config = |body: Value| {
        let mut body = body;
        body["tags"] = tags.clone();
        body["pgConfig"] = pg_config.clone();
        body["pgBouncerConfig"] = parameters.clone();
        body
    };

    for (mut args, verb, endpoint) in postgres_config_write_commands(
        pg_config_file.to_str().unwrap(),
        pg_bouncer_file.to_str().unwrap(),
        config_file.to_str().unwrap(),
    ) {
        let mock = MockServer::start().await;
        Mock::given(method(verb))
            .and(path(endpoint))
            .and(wiremock::matchers::basic_auth(
                "fake-key-for-tests",
                "fake-secret-for-tests",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": restored_service_response(endpoint, &configuration), "status": 200,
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
        // Create omits `ipAccessList` (allow all); `[]` would block public access.
        let expected = match args[1] {
            "create" => with_config(serde_json::json!({
                "name": "pg-test", "provider": "aws", "region": "us-east-1", "size": "c6gd.large"
            })),
            "read-replica" => with_config(serde_json::json!({"name": "replica-test"})),
            "restore" => with_config(serde_json::json!({
                "name": "restored-test", "restoreTarget": "2026-07-31T23:00:00Z"
            })),
            _ => configuration.clone(),
        };
        assert_eq!(body, expected, "{args:?}");
        if endpoint.ends_with("/config") {
            let output: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output["pgBouncerConfig"], parameters);
        }
    }
}

#[tokio::test]
async fn postgres_config_get_honors_human_explicit_json_and_agent_output() {
    let mock = MockServer::start().await;
    let config = serde_json::json!({
        "pgConfig": {"work_mem": "64MB", "max_connections": 500},
        "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(header(
            "authorization",
            "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": config
        })))
        .expect(3)
        .mount(&mock)
        .await;

    let args = ["postgres", "config", "get", "pg-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    assert!(serde_json::from_slice::<Value>(&human.stdout).is_err());
    let human = String::from_utf8(human.stdout).unwrap();
    for line in [
        "pgConfig:",
        "  work_mem: 64MB",
        "  max_connections: 500",
        "pgBouncerConfig:",
        "  default_pool_size: 16",
        "  future_parameter: on",
    ] {
        assert!(human.lines().any(|actual| actual == line), "{human}");
    }

    let explicit = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&explicit);
    assert_eq!(
        serde_json::from_slice::<Value>(&explicit.stdout).unwrap(),
        config
    );

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let agent = Command::new(clickhousectl_binary())
        .isolated_env()
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
        serde_json::from_slice::<Value>(&agent.stdout).unwrap(),
        config
    );
    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), None);
        assert!(request.body.is_empty());
    }
}

// IP allowlist descriptions (#589).

#[tokio::test]
async fn api_key_create_and_update_pass_every_flag_and_allowlist_description() {
    let create = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .and(body_json(serde_json::json!({
            "name": "described-key",
            "state": "disabled",
            "assignedRoleIds": ["11111111-2222-3333-4444-555555555555"],
            "expireAt": "2030-01-01T00:00:00Z",
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
            "--role-id",
            "11111111-2222-3333-4444-555555555555",
            "--expires-at",
            "2030-01-01T00:00:00Z",
            "--state",
            "disabled",
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
async fn key_update_passes_every_flag_to_the_exact_body() {
    // Two runs pass every update flag through the dispatcher; the other
    // combinations are the `build_api_key_update_request_*` unit tests.
    let cases: &[(&[&str], Value)] = &[
        (
            &[
                "--clear-roles",
                "--clear-ip-allow",
                "--expires-at",
                "2030-01-01T00:00:00Z",
            ],
            serde_json::json!({
                "assignedRoleIds": [], "ipAccessList": [], "expireAt": "2030-01-01T00:00:00Z",
            }),
        ),
        (
            &[
                "--clear-expiry",
                "--new-name",
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
async fn postgres_config_file_dash_remains_a_literal_filename() {
    for (action, verb) in [("patch", "PATCH"), ("replace", "POST")] {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let mock = MockServer::start().await;
        let document = serde_json::json!({
            "pgConfig": {"max_connections": 300}, "pgBouncerConfig": {}
        });
        Mock::given(method(verb))
            .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
            .and(body_json(document.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": document, "status": 200
            })))
            .expect(1)
            .mount(&mock)
            .await;
        for exists in [false, true] {
            if exists {
                std::fs::write(directory.path().join("-"), document.to_string()).unwrap();
            }
            let mut child = Command::new(clickhousectl_binary())
                .isolated_env()
                .env("DO_NOT_TRACK", "1")
                .env("HOME", &home)
                .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
                .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
                .current_dir(directory.path())
                .args([
                    "cloud",
                    "--url",
                    &mock.uri(),
                    "--json",
                    "postgres",
                    "config",
                    action,
                    "pg-1",
                    "--file",
                    "-",
                    "--org-id",
                    "org-1",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            // A distinct, valid stdin document must never replace the file input.
            child
                .stdin
                .take()
                .unwrap()
                .write_all(br#"{"pgConfig":{"max_connections":999},"pgBouncerConfig":{}}"#)
                .unwrap();
            let output = child.wait_with_output().unwrap();
            if exists {
                assert_success(&output);
            } else {
                assert_eq!(output.status.code(), Some(1));
                assert!(mock.received_requests().await.unwrap().is_empty());
            }
        }
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
        // Waits poll the service; skip the shipped 5 s gap between polls.
        .env("CHCTL_UDF_POLL_INTERVAL_MS", "100")
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
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        let item = serde_json::json!({"functionName":"my_udf", "serviceId":"svc-1", "version":2, "status":"future", "deterministic":false});
        let result = if list {
            serde_json::json!([item])
        } else {
            item
        };
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Bearer test-bearer-token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(if list { serde_json::json!({"result":result,"nextCursor":"next page","limit":2,"totalCount":3}) } else { serde_json::json!({"result":result}) }),
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
            assert_eq!(output["nextCursor"], "next page");
            assert_eq!(output["limit"], 2);
            assert_eq!(output["totalCount"], 3);
            assert_eq!(output["result"][0]["status"], result[0]["status"]);
            assert!(output.get("items").is_none());
            assert!(output.get("pagination").is_none());
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
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_success(&output);
        if verb == "DELETE" {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                serde_json::json!({"status":200,"requestId":"delete-request"})
            );
        }
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
async fn udf_delete_commands_print_human_confirmations() {
    // One confirmation line naming what was deleted.
    for (args, suffix, expected) in [
        (vec!["delete", "my_udf"], "/my_udf", &["my_udf"][..]),
        (
            vec!["version", "delete", "my_udf", "2"],
            "/my_udf/versions/2",
            &["my_udf", "2"][..],
        ),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method("DELETE"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"status":200,"requestId":"delete-request"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let output = udf_test_command(&server, project.path(), false, false, &args)
            .output()
            .unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 1, "{stdout}");
        for identifier in expected {
            assert!(stdout.contains(identifier), "{identifier}: {stdout}");
        }
    }
}

#[tokio::test]
async fn udf_empty_lists_have_named_empty_states_and_readable_pagination() {
    // Each list has its own empty-state line (no table), then the page data.
    let mut empty_states = std::collections::BTreeSet::new();
    for (args, suffix) in [
        (vec!["list"], ""),
        (vec!["attachment", "list", "my_udf"], "/my_udf/attachments"),
        (vec!["version", "list", "my_udf"], "/my_udf/versions"),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [], "limit":100, "nextCursor":"next page", "totalCount":0
            })))
            .expect(1)
            .mount(&server)
            .await;

        let output = udf_test_command(&server, project.path(), false, false, &args)
            .output()
            .unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 2, "{stdout}");
        assert!(!lines[0].contains('|'), "{stdout}");
        assert!(empty_states.insert(lines[0].to_owned()), "{stdout}");
        assert!(
            lines[1].contains("100") && lines[1].ends_with("next page"),
            "{stdout}"
        );
    }
}

#[tokio::test]
async fn udf_create_and_version_upload_full_definitions_without_auth_leakage() {
    // The `--file` aliases are `primary_json_file_argument_contract`, and the
    // per-kind body is the builder's; one run each of create and version create.
    for (create, kind) in [(true, "executable_pool"), (false, "executable")] {
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
        // Cloud no longer accepts sandboxVersion: it is dropped, with a warning.
        expected.as_object_mut().unwrap().remove("sandboxVersion");
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
            "--file",
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr
                .lines()
                .any(|line| line.starts_with("Warning:") && line.contains("sandboxVersion")),
            "{stderr}"
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
                "--file",
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

// A `udf.json` reused with `version create --file` may keep a functionName
// that matches NAME (stripped from the body); a different one is a usage error.
#[tokio::test]
async fn udf_version_create_file_strips_a_matching_function_name() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    std::fs::write(
        project.path().join("udf.json"),
        udf_definition("executable", true).to_string(),
    )
    .unwrap();
    let args = [
        "version",
        "create",
        "my_udf",
        "--file",
        "udf.json",
        "--artifact",
        "code.zip",
    ];
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
            "uploadId":"fresh-upload", "uploadUrl":format!("{}/artifact?signature=s", storage.uri())
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/artifact"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&storage)
        .await;
    let mut expected = udf_definition("executable", false);
    expected["uploadId"] = serde_json::json!("fresh-upload");
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(body_json(expected))
        .respond_with(ResponseTemplate::new(201).set_body_json(
            serde_json::json!({"result":{"functionName":"my_udf","version":2,"status":"building"}}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(&server, project.path(), false, true, &args)
        .output()
        .unwrap();
    assert_success(&output);

    let mismatch = MockServer::start().await;
    let mut renamed = args;
    renamed[2] = "other_udf";
    let output = udf_test_command(&mismatch, project.path(), false, true, &renamed)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("udf.json"), "{stderr}");
    assert!(mismatch.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn udf_create_api_failures_consume_one_upload_attempt() {
    // Create and version create share `upload_artifact`; one run covers both.
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    std::fs::write(
        project.path().join("definition.json"),
        udf_definition("executable", true).to_string(),
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
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(
            ResponseTemplate::new(424)
                .set_body_json(serde_json::json!({"error":"build dependency unavailable"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let args = [
        "create",
        "--file",
        "definition.json",
        "--artifact",
        "code.zip",
    ];
    let output = udf_test_command(&server, project.path(), false, true, &args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("build dependency unavailable"));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
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
        &["create", "--file", "-", "--artifact", "code.zip"],
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
        (vec!["list"], "", serde_json::json!([{}])),
        (
            vec!["attachment", "list", "my_udf"],
            "/my_udf/attachments",
            serde_json::json!([
                {"functionName": "my_udf", "serviceId": "svc-1", "version": 2, "status": "deployed"},
                {}
            ]),
        ),
        (
            vec!["version", "list", "my_udf"],
            "/my_udf/versions",
            serde_json::json!([]),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"result":result,"nextCursor":"next"})),
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
        if args[0] == "attachment" {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let rows: Vec<Vec<&str>> = stdout
                .lines()
                .filter(|line| line.starts_with('|') && !line.starts_with("|-"))
                .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
                .collect();
            assert_eq!(
                rows,
                [
                    ["name", "service_id", "version", "status"],
                    ["my_udf", "svc-1", "2", "deployed"],
                    ["-", "-", "-", "-"],
                ],
                "{stdout}"
            );
        }
        let request = server.received_requests().await.unwrap().pop().unwrap();
        assert!(request.url.query().is_none());
    }
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

    // Refused after the read and before any PATCH (the request shape below);
    // the wording is unit-tested in `postgres::lifecycle`.
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), postgres_path)]
    );
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

/// The digits in `text`, in order: for messages whose only numbers are counts.
fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
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
            "byocId": "byoc-1",
            "tags": [{"key": "env", "value": "prod"}],
            "endpoints": [{"protocol": "mysql", "enabled": true}]
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
            "--tag",
            "env=prod",
            "--enable-endpoint",
            "mysql",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
}

fn backup_encryption_config_document() -> Value {
    serde_json::json!({
        "schema_version": 1,
        "restore_key_pairs": [{
            "customer_managed_encryption_key": {
                "aws_kms_key_arn": "arn:aws:kms:us-east-1:111122223333:key/1234abcd"
            },
            "encrypted_dek": "d3JhcHBlZA==",
            "future_field": { "nested": [1, null, true] }
        }]
    })
}

async fn mount_service_create_with_backup_encryption_config(mock: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .and(body_json(serde_json::json!({
            "name": "restored",
            "provider": "aws",
            "region": "us-east-1",
            "ipAccessList": [{
                "source": "0.0.0.0/0",
                "description": "Allow all (created by clickhousectl)"
            }],
            "backupId": "a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6",
            "backupEncryptionConfig": backup_encryption_config_document()
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "service": {"id": "22222222-3333-4444-5555-666666666666", "name": "restored"},
                "password": "generated-password"
            },
            "status": 200,
            "requestId": "stub-restore-service-create"
        })))
        .expect(1)
        .mount(mock)
        .await;
}

const BACKUP_RESTORE_ARGS: [&str; 7] = [
    "service",
    "create",
    "--name",
    "restored",
    "--backup-id",
    "a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6",
    "--backup-encryption-config",
];

#[tokio::test]
async fn service_create_reads_backup_encryption_config_from_stdin() {
    let mock = MockServer::start().await;
    mount_service_create_with_backup_encryption_config(&mock).await;

    let mut args = BACKUP_RESTORE_ARGS.to_vec();
    args.extend(["-", "--org-id", "org-1"]);
    let output = invoke_cli_with_cloud_credentials_and_stdin(
        &mock,
        &args,
        &backup_encryption_config_document().to_string(),
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
    // Names the profile and the memory size both bounds must equal.
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("'v1-standard-byoc-4'"), "{stderr}");
    assert!(stderr.contains("48"), "{stderr}");
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

/// `settings get` renders the setting in human mode rather than passing the
/// JSON through; the JSON branch is in the OAuth read-routes test above.
#[tokio::test]
async fn service_settings_get_renders_the_setting_in_human_mode() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickhouseSettings/compatibility",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": "compatibility", "value": "26.2"}, "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
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
    assert_success(&output);
    assert!(
        serde_json::from_slice::<Value>(&output.stdout).is_err(),
        "human mode must not print JSON: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.contains(&"name: compatibility"), "{stdout}");
    assert!(lines.contains(&"value: 26.2"), "{stdout}");
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
            "settings": {"compatibility": "24.8", "enable_analyzer": 1}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "settings": {"compatibility": "24.8", "enable_analyzer": 1},
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
            "settings": {"compatibility": "24.8", "enable_analyzer": 1},
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
async fn service_settings_set_reads_a_map_from_stdin() {
    let mock = MockServer::start().await;
    let collection = "/v1/organizations/org-1/services/svc-1/clickhouseSettings";
    Mock::given(method("PATCH"))
        .and(path(collection))
        .and(body_json(serde_json::json!({
            "settings": {"bool_value": false, "future_setting": {"nested": true}}
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
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .args([
            "cloud", "--url", &url, "--json", "service", "settings", "set", "svc-1", "--file", "-",
            "--org-id", "org-1",
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
}

const JWT_PROVIDERS_PATH: &str = "/v1/organizations/org-1/services/svc-1/jwtProviders";

#[tokio::test]
async fn service_jwt_provider_commands_send_exact_requests_and_print_json() {
    let mock = MockServer::start().await;
    let item = format!("{JWT_PROVIDERS_PATH}/okta");
    let provider = serde_json::json!({
        "name": "okta",
        "issuer": "https://auth.example.com",
        "audience": "analytics",
        "jwksUrl": "https://auth.example.com/jwks.json",
        "rolesClaim": "clickhouse:roles"
    });
    Mock::given(method("POST"))
        .and(path(JWT_PROVIDERS_PATH))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .and(body_json(provider.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": provider, "status": 200, "requestId": "stub-jwt-create"
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(JWT_PROVIDERS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [provider, {"name": "sparse"}], "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(&item))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": provider, "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    // No rolesClaim in the replacement removes the stored one.
    let replaced = serde_json::json!({
        "issuer": "https://auth2.example.com",
        "audience": "analytics",
        "jwksUrl": "https://auth2.example.com/jwks.json"
    });
    Mock::given(method("PUT"))
        .and(path(&item))
        .and(body_json(replaced.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": "okta", "issuer": "https://auth2.example.com"},
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(&item))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "stub-jwt-delete"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let prefix = ["service", "jwt-provider"];
    let run = |args: &[&str]| {
        let mut full = prefix.to_vec();
        full.extend(args);
        full.extend(["--org-id", "org-1"]);
        let output = invoke_cli_with_cloud_credentials(&mock, &full);
        assert_success(&output);
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };

    assert_eq!(
        run(&[
            "create",
            "svc-1",
            "okta",
            "--issuer",
            "https://auth.example.com",
            "--audience",
            "analytics",
            "--jwks-url",
            "https://auth.example.com/jwks.json",
            "--roles-claim",
            "clickhouse:roles",
        ]),
        provider
    );
    assert_eq!(
        run(&["list", "svc-1"]),
        serde_json::json!([provider, {"name": "sparse"}])
    );
    assert_eq!(run(&["get", "svc-1", "okta"]), provider);
    assert_eq!(
        run(&[
            "update",
            "svc-1",
            "okta",
            "--issuer",
            "https://auth2.example.com",
            "--audience",
            "analytics",
            "--jwks-url",
            "https://auth2.example.com/jwks.json",
        ]),
        serde_json::json!({"name": "okta", "issuer": "https://auth2.example.com"})
    );
    assert_eq!(
        run(&["delete", "svc-1", "okta"]),
        serde_json::json!({"status": 200, "requestId": "stub-jwt-delete"})
    );
}

#[tokio::test]
async fn service_jwt_provider_list_renders_a_table_with_absent_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(JWT_PROVIDERS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"name": "okta", "issuer": "https://auth.example.com", "audience": "analytics",
                 "jwksUrl": "https://auth.example.com/jwks.json"},
                {"name": "sparse"}
            ],
            "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "service",
            "jwt-provider",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(serde_json::from_str::<Value>(&stdout).is_err(), "{stdout}");
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.split('|').map(str::trim).collect())
        .collect();
    assert!(
        rows.iter().any(|row| row.contains(&"okta")
            && row.contains(&"https://auth.example.com/jwks.json")),
        "{stdout}"
    );
    assert!(
        rows.iter()
            .any(|row| row.contains(&"sparse") && row.iter().filter(|c| **c == "-").count() == 4),
        "{stdout}"
    );
}

/// The provider name is a request path segment; an invalid one is a usage
/// error before any request, so `..` never reaches the service itself.
#[tokio::test]
async fn service_jwt_provider_rejects_an_invalid_name_before_any_request() {
    let mock = MockServer::start().await;
    for args in [
        vec!["delete", "svc-1", ".."],
        vec!["delete", "--name", "analytics", ".."],
    ] {
        let mut full = vec!["service", "jwt-provider"];
        full.extend(args);
        full.extend(["--org-id", "org-1"]);
        let output = invoke_cli_with_cloud_credentials(&mock, &full);
        assert_eq!(output.status.code(), Some(2), "{full:?}");
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

const UPGRADE_WINDOW_PATH: &str = "/v1/organizations/org-1/services/svc-1/upgradeWindow";

#[tokio::test]
async fn upgrade_window_get_supports_oauth_and_preserves_sparse_json() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(UPGRADE_WINDOW_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "startHourUtc": 3,
                "duration": 8,
                "futureField": "ignored safely"
            },
            "status": 200,
            "requestId": "stub-upgrade-window-get"
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
            "service",
            "upgrade-window",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"startHourUtc": 3, "duration": 8})
    );
}

#[tokio::test]
async fn upgrade_window_get_human_output_names_weekday_and_labels_duration() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(UPGRADE_WINDOW_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"weekday": 3, "startHourUtc": 12, "duration": 6},
            "status": 200,
            "requestId": "stub-upgrade-window-human"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "service",
            "upgrade-window",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "duration: 6 hours\nstartHourUtc: 12\nweekday: Wednesday (3)\n"
    );
}

#[tokio::test]
async fn upgrade_window_set_sends_exact_body_with_basic_auth() {
    let mock = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(UPGRADE_WINDOW_PATH))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .and(body_json(serde_json::json!({
            "weekday": 1,
            "startHourUtc": 18
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"weekday": 1, "startHourUtc": 18, "duration": 6},
            "status": 200,
            "requestId": "stub-upgrade-window-set"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "upgrade-window",
            "set",
            "svc-1",
            "--weekday",
            "1",
            "--start-hour",
            "18",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"weekday": 1, "startHourUtc": 18, "duration": 6})
    );
}

#[tokio::test]
async fn upgrade_window_delete_uses_basic_auth_and_prints_delete_envelope() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(UPGRADE_WINDOW_PATH))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-upgrade-window-delete"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "upgrade-window",
            "delete",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "status": 200,
            "requestId": "stub-upgrade-window-delete"
        })
    );
}

// ── Complete ClickPipe PATCH input (#569) ──────────────────────────────────

const CLICKPIPE_UPDATE_PATH: &str = "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1";

async fn mount_clickpipe_update(mock: &MockServer, patch: Value) {
    Mock::given(method("PATCH"))
        .and(path(CLICKPIPE_UPDATE_PATH))
        .and(body_json(patch))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "updated"
            },
            "status": 200,
            "requestId": "stub-clickpipe-update"
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_update_reads_stdin_and_preserves_explicit_empty_values() {
    let mock = MockServer::start().await;
    let patch = serde_json::json!({"fieldMappings": []});
    mount_clickpipe_update(&mock, patch.clone()).await;
    let output = invoke_cli_with_cloud_credentials_and_stdin(
        &mock,
        &[
            "clickpipe",
            "update",
            "svc-1",
            "pipe-1",
            "--file",
            "-",
            "--org-id",
            "org-1",
        ],
        &patch.to_string(),
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "updated"
        })
    );
}

#[tokio::test]
async fn service_settings_set_rejects_malformed_inputs_before_organization_discovery() {
    // The parse errors themselves are unit-tested in `services::settings`;
    // this pins that the handler parses before it resolves the organization.
    let mock = MockServer::start().await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "settings",
            "set",
            "svc-1",
            "--setting",
            "value=1e3",
        ],
    );
    // The wording is unit-tested in `services::settings`; the error names the
    // flag and the setting.
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--setting") && stderr.contains("'value'"),
        "{output:?}"
    );
    assert!(
        mock.received_requests().await.unwrap().is_empty(),
        "invalid local inputs must fail before even organization discovery"
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
            clear_inherited_env(&mut cmd);
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
        clear_inherited_env(&mut cmd);
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

#[tokio::test]
async fn service_settings_list_renders_mixed_json_types_and_sparse_rows() {
    let mock = MockServer::start().await;
    let settings = serde_json::json!([
        {"name": "max_query_size", "value": 262146},
        {"name": "compatibility", "value": "26.2"},
        {"name": "future_bool", "value": false},
        {"name": "missing_value"},
        {"name": "null_value", "value": null},
        {"value": "unnamed"}
    ]);
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickhouseSettings",
        ))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"settings": settings}, "status": 200
        })))
        .expect(1)
        .mount(&mock)
        .await;
    // JSON mode passes the response through; the OAuth read-routes test
    // covers it. The table is the handler's own rendering.
    let args = ["service", "settings", "list", "svc-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .filter(|line| line.starts_with('|'))
        .map(|line| {
            line.split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect()
        })
        .collect();
    for expected in [
        vec!["max_query_size", "262146"],
        vec!["compatibility", "26.2"],
        vec!["future_bool", "false"],
        vec!["missing_value", "-"],
        vec!["null_value", "-"],
        vec!["-", "unnamed"],
    ] {
        assert!(rows.contains(&expected), "missing {expected:?} in {stdout}");
    }
}

// Query API endpoint management (#766).

const QUERY_API_ENDPOINTS_PATH: &str = "/v1/organizations/org-1/services/svc-1/query-api-endpoints";
const QUERY_API_ENDPOINT_ID: &str = "11111111-2222-3333-8444-555555555555";
const QUERY_API_KEY_ID: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

fn query_api_endpoint_config() -> Value {
    serde_json::json!({
        "name": "orders",
        "sql": "SELECT * FROM orders WHERE id = {id:String}",
        "database": "analytics",
        "parameters": {"id": "default-id", "region": "eu-west"},
        "apiKeyIds": [QUERY_API_KEY_ID],
        "roles": ["query_api", "reader"],
        "allowedOrigins": []
    })
}

fn query_api_endpoint_response() -> Value {
    let mut response = query_api_endpoint_config();
    response.as_object_mut().unwrap().extend(
        serde_json::json!({
            "id": QUERY_API_ENDPOINT_ID,
            "url": format!("https://queries.clickhouse.cloud/run/{QUERY_API_ENDPOINT_ID}"),
            "ownerType": "queryApiEndpoint"
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    response
}

fn invoke_query_api_endpoint(
    server: &MockServer,
    project: &Path,
    oauth: bool,
    json: bool,
    args: &[&str],
    stdin: Option<&str>,
) -> std::process::Output {
    let home = project.join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    if oauth {
        write_oauth_tokens(&cloud_dir, &server.uri());
    }
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project)
        .args(["cloud", "--url", &server.uri()]);
    if !oauth {
        command.args([
            "--api-key",
            "query-endpoint-key",
            "--api-secret",
            "query-endpoint-secret",
        ]);
    }
    if json {
        command.arg("--json");
    }
    command.arg("query-api-endpoint").args(args);
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
        command.stdin(Stdio::null()).output().unwrap()
    }
}

fn query_api_endpoint_envelope(status: u16, result: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(serde_json::json!({
        "status": status,
        "requestId": "query-api-endpoint-request",
        "result": result
    }))
}

fn query_api_endpoint_list_page(
    items: Value,
    next_cursor: Option<&str>,
    total_count: Option<i64>,
    limit: Option<i64>,
) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "query-api-endpoint-request",
        "result": items,
        "nextCursor": next_cursor,
        "totalCount": total_count,
        "limit": limit
    }))
}

#[tokio::test]
async fn query_api_endpoint_all_verbs_use_exact_routes_auth_and_complete_bodies() {
    // The --config-file alias is clap's; see `primary_json_file_argument_contract`
    // in `cloud::query_api_endpoints`.
    query_api_endpoint_all_verbs_use_exact_routes_auth_and_complete_bodies_with_file_flag("--file")
        .await;
}

async fn query_api_endpoint_all_verbs_use_exact_routes_auth_and_complete_bodies_with_file_flag(
    file_flag: &str,
) {
    let server = MockServer::start().await;
    let endpoint = query_api_endpoint_response();
    let list = serde_json::json!({
        "items": [{
            "id": QUERY_API_ENDPOINT_ID,
            "name": "orders",
            "database": "analytics",
            "apiKeyIds": [QUERY_API_KEY_ID],
            "roles": ["query_api", "reader"],
            "allowedOrigins": [],
            "url": format!("https://queries.clickhouse.cloud/run/{QUERY_API_ENDPOINT_ID}"),
            "ownerType": "queryApiEndpoint"
        }],
        "pagination": {"limit": 100, "totalRecords": 1}
    });
    for (verb, suffix, body, status, result) in [
        (
            "POST",
            "",
            Some(query_api_endpoint_config()),
            201,
            endpoint.clone(),
        ),
        (
            "PUT",
            QUERY_API_ENDPOINT_ID,
            Some(query_api_endpoint_config()),
            200,
            endpoint.clone(),
        ),
        ("GET", "", None, 200, list.clone()),
        ("GET", QUERY_API_ENDPOINT_ID, None, 200, endpoint.clone()),
    ] {
        let route = if suffix.is_empty() {
            QUERY_API_ENDPOINTS_PATH.to_string()
        } else {
            format!("{QUERY_API_ENDPOINTS_PATH}/{suffix}")
        };
        let mut mock =
            Mock::given(method(verb))
                .and(path(route))
                .and(wiremock::matchers::basic_auth(
                    "query-endpoint-key",
                    "query-endpoint-secret",
                ));
        if let Some(body) = body {
            mock = mock.and(body_json(body));
        }
        let response = if verb == "GET" && suffix.is_empty() {
            query_api_endpoint_list_page(result["items"].clone(), None, Some(1), Some(100))
        } else {
            query_api_endpoint_envelope(status, result)
        };
        mock.respond_with(response).expect(1).mount(&server).await;
    }
    Mock::given(method("DELETE"))
        .and(path(format!(
            "{QUERY_API_ENDPOINTS_PATH}/{QUERY_API_ENDPOINT_ID}"
        )))
        .and(wiremock::matchers::basic_auth(
            "query-endpoint-key",
            "query-endpoint-secret",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "query-api-endpoint-delete"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let project = tempfile::tempdir().unwrap();
    let config_path = project.path().join("endpoint.json");
    std::fs::write(&config_path, query_api_endpoint_config().to_string()).unwrap();
    let create = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &[
            "--org-id",
            "org-1",
            "create",
            "svc-1",
            file_flag,
            config_path.to_str().unwrap(),
        ],
        None,
    );
    assert_success(&create);
    assert_eq!(
        serde_json::from_slice::<Value>(&create.stdout).unwrap(),
        endpoint
    );

    let request_stdin = query_api_endpoint_config().to_string();
    let update = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &[
            "update",
            "svc-1",
            QUERY_API_ENDPOINT_ID,
            file_flag,
            "-",
            "--org-id",
            "org-1",
        ],
        Some(&request_stdin),
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        endpoint
    );

    let list_output = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    assert_success(&list_output);
    assert_eq!(
        serde_json::from_slice::<Value>(&list_output.stdout).unwrap(),
        list
    );
    let get = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &["get", "svc-1", QUERY_API_ENDPOINT_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        endpoint
    );
    let delete = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &[
            "delete",
            "svc-1",
            QUERY_API_ENDPOINT_ID,
            "--org-id",
            "org-1",
        ],
        None,
    );
    assert_success(&delete);
    assert_eq!(
        serde_json::from_slice::<Value>(&delete.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "query-api-endpoint-delete"})
    );

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    let list_request = requests
        .iter()
        .find(|request| {
            request.method == wiremock::http::Method::GET
                && request.url.path() == QUERY_API_ENDPOINTS_PATH
        })
        .unwrap();
    assert!(list_request.url.query().is_none());
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == wiremock::http::Method::GET)
            .count(),
        2,
        "update must not prefetch the endpoint"
    );
}

#[tokio::test]
async fn query_api_endpoint_reads_support_oauth_and_preserve_reserved_list_cursor() {
    let server = MockServer::start().await;
    let result = serde_json::json!({
        "items": [{}],
        "pagination": {
            "currentCursor": "page /+?",
            "nextCursor": null,
            "limit": 2,
            "totalRecords": 3
        }
    });
    Mock::given(method("GET"))
        .and(path(QUERY_API_ENDPOINTS_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(query_api_endpoint_list_page(
            result["items"].clone(),
            None,
            Some(3),
            Some(2),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "{QUERY_API_ENDPOINTS_PATH}/{QUERY_API_ENDPOINT_ID}"
        )))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(query_api_endpoint_envelope(200, serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let list = invoke_query_api_endpoint(
        &server,
        project.path(),
        true,
        true,
        &[
            "list", "svc-1", "--cursor", "page /+?", "--limit", "2", "--org-id", "org-1",
        ],
        None,
    );
    assert_success(&list);
    let mut expected = result;
    expected["pagination"]
        .as_object_mut()
        .unwrap()
        .remove("nextCursor");
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        expected
    );
    let get = invoke_query_api_endpoint(
        &server,
        project.path(),
        true,
        true,
        &["get", "svc-1", QUERY_API_ENDPOINT_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        serde_json::json!({})
    );

    let request = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| request.url.path() == QUERY_API_ENDPOINTS_PATH)
        .unwrap();
    let query: std::collections::HashMap<_, _> = request.url.query_pairs().into_owned().collect();
    assert_eq!(query.get("cursor").map(String::as_str), Some("page /+?"));
    assert_eq!(query.get("limit").map(String::as_str), Some("2"));
}

#[tokio::test]
async fn query_api_endpoint_human_reads_tolerate_sparse_fields_and_show_next_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "{QUERY_API_ENDPOINTS_PATH}/{QUERY_API_ENDPOINT_ID}"
        )))
        .respond_with(query_api_endpoint_envelope(200, serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(QUERY_API_ENDPOINTS_PATH))
        .respond_with(query_api_endpoint_list_page(
            serde_json::json!([
                {},
                {
                    "id": QUERY_API_ENDPOINT_ID,
                    "name": "orders",
                    "database": "analytics",
                    "ownerType": "futureOwner",
                    "url": "https://queries.clickhouse.cloud/run/example"
                }
            ]),
            Some("next /+?"),
            None,
            None,
        ))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let get = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        false,
        &["get", "svc-1", QUERY_API_ENDPOINT_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(String::from_utf8_lossy(&get.stdout), "{}\n");
    let list = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        false,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .filter(|line| line.starts_with('│'))
        .map(|line| {
            line.split('│')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect()
        })
        .collect();
    assert!(
        rows.contains(&vec!["-", "-", "-", "-", "-"]),
        "the empty item renders every column as absent:\n{stdout}"
    );
    assert!(
        rows.contains(&vec![
            QUERY_API_ENDPOINT_ID,
            "orders",
            "analytics",
            "futureOwner",
            "https://queries.clickhouse.cloud/run/example"
        ]),
        "{stdout}"
    );
    assert!(stdout.contains("Next cursor: next /+?"), "{stdout}");

    let empty_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(QUERY_API_ENDPOINTS_PATH))
        .respond_with(query_api_endpoint_list_page(
            serde_json::json!([]),
            Some("after-empty-page"),
            None,
            None,
        ))
        .expect(1)
        .mount(&empty_server)
        .await;
    let empty_project = tempfile::tempdir().unwrap();
    let empty = invoke_query_api_endpoint(
        &empty_server,
        empty_project.path(),
        false,
        false,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    // No table: one empty-list line, then the cursor.
    assert_success(&empty);
    let stdout = String::from_utf8_lossy(&empty.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(!lines[0].contains('|'), "{stdout}");
    assert!(lines[1].ends_with("after-empty-page"), "{stdout}");
}

#[tokio::test]
async fn clickpipe_list_renders_source_destination_table_and_preserves_json() {
    let mock = MockServer::start().await;
    let sources = [
        ("objectStorage", "Object storage"),
        ("kafka", "Kafka"),
        ("kinesis", "Kinesis"),
        ("pubsub", "Pub/Sub"),
        ("postgres", "Postgres"),
        ("mysql", "MySQL"),
        ("mongodb", "MongoDB"),
        ("bigquery", "BigQuery"),
    ];
    let mut pipes: Vec<Value> = sources
        .iter()
        .enumerate()
        .map(|(index, (source, _))| {
            serde_json::json!({
                "id": format!("00000000-0000-0000-0000-{index:012}"),
                "name": format!("pipe-{index}"),
                "source": { *source: {} },
                "destination": { "database": "analytics", "table": format!("events_{index}") },
                "state": "Running"
            })
        })
        .collect();
    pipes.extend([
        serde_json::json!({}),
        serde_json::json!({"source": {}, "destination": {}}),
        serde_json::json!({"destination": {"database": "database_only"}, "state": "FutureState"}),
        serde_json::json!({"destination": {"table": "table_only"}}),
    ]);
    let result = serde_json::json!(pipes);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .and(header(
            "authorization",
            "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
        )
        .expect(2)
        .mount(&mock)
        .await;
    let args = ["clickpipe", "list", "svc-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect();
    assert_eq!(rows.len(), pipes.len() + 2, "{stdout}");
    assert_eq!(rows[0], ["Name", "ID", "Source", "Destination", "State"]);
    for (index, (_, label)) in sources.iter().enumerate() {
        assert_eq!(
            rows[index + 2],
            [
                format!("pipe-{index}"),
                format!("00000000-0000-0000-0000-{index:012}"),
                label.to_string(),
                format!("analytics.events_{index}"),
                "Running".to_string(),
            ],
            "{stdout}"
        );
    }
    assert_eq!(rows[10], ["-", "-", "-", "-", "-"]);
    assert_eq!(rows[11], ["-", "-", "-", "-", "-"]);
    assert_eq!(rows[12], ["-", "-", "-", "database_only", "FutureState"]);
    assert_eq!(rows[13], ["-", "-", "-", "table_only", "-"]);

    let explicit = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&explicit);
    assert_eq!(
        serde_json::from_slice::<Value>(&explicit.stdout).unwrap(),
        result
    );
    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), None);
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn clickpipe_list_and_name_lookup_read_every_page() {
    // The first request carries no `limit` or `cursor`; a `nextCursor` in any
    // response is followed until absent. Used by `list` and by `--name`.
    const CLICKPIPES_PATH: &str = "/v1/organizations/org-1/services/svc-1/clickpipes";
    const SECOND_ID: &str = "00000000-0000-0000-0000-0000000000c2";
    let mock = MockServer::start().await;
    let first = serde_json::json!({"id": "00000000-0000-0000-0000-0000000000c1", "name": "first"});
    let second = serde_json::json!({"id": SECOND_ID, "name": "target"});
    mount_cursor_page(
        &mock,
        CLICKPIPES_PATH,
        None,
        serde_json::json!({"result": [first], "nextCursor": "pipe page /+?", "limit": 1}),
        3,
    )
    .await;
    mount_cursor_page(
        &mock,
        CLICKPIPES_PATH,
        Some("pipe page /+?"),
        serde_json::json!({"result": [second], "nextCursor": null}),
        3,
    )
    .await;
    Mock::given(method("GET"))
        .and(path(format!("{CLICKPIPES_PATH}/{SECOND_ID}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": second})),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let args = ["clickpipe", "list", "svc-1", "--org-id", "org-1"];
    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!([first, second])
    );
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    for id in ["00000000-0000-0000-0000-0000000000c1", SECOND_ID] {
        assert!(stdout.contains(id), "{stdout}");
    }

    let lookup = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "get",
            "svc-1",
            "--name",
            "target",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&lookup);
    assert_eq!(
        serde_json::from_slice::<Value>(&lookup.stdout).unwrap(),
        second
    );
}

async fn invoke_clickpipe_delete(result: Value, json: bool) -> std::process::Output {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(CLICKPIPE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-clickpipe-delete",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let args = [
        "clickpipe",
        "delete",
        "svc-id",
        "pipe-id",
        "--org-id",
        "org",
    ];
    let output = if json {
        invoke_cli_with_cloud_credentials(&mock, &args)
    } else {
        invoke_cli_human(&mock, &args)
    };
    assert_success(&output);
    assert_eq!(
        recorded_request_shape(&mock).await,
        [("DELETE".to_string(), CLICKPIPE_PATH.to_string())],
    );
    output
}

#[tokio::test]
async fn clickpipe_delete_prints_the_returned_name_or_the_requested_id() {
    for (result, json, expected) in [
        (
            serde_json::json!({"name": "events-pipe"}),
            false,
            "ClickPipe events-pipe deleted\n",
        ),
        (serde_json::json!({}), false, "ClickPipe pipe-id deleted\n"),
        (
            serde_json::json!({"name": ""}),
            false,
            "ClickPipe pipe-id deleted\n",
        ),
        (
            serde_json::json!({"name": "events-pipe"}),
            true,
            "{\"deleted\":\"pipe-id\"}\n",
        ),
    ] {
        let output = invoke_clickpipe_delete(result.clone(), json).await;
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{result} json={json}"
        );
    }
}

// Deleted resources return 404, while never-existing UUIDs may return 400 (#831).
fn deleted_resource_404_response() -> ResponseTemplate {
    ResponseTemplate::new(404).set_body_json(serde_json::json!({
        "status": 404, "error": "NOT_FOUND: Not Found", "requestId": "deleted-resource"
    }))
}

#[tokio::test]
async fn deleted_resource_404_get_and_delete_have_structured_scope_and_guidance() {
    // Each wrapper's lookup refinement once; the output mode alternates because
    // human and JSON rendering of the refined error is shared.
    for (domain, collection, operation, human) in [
        ("service", "services", "get", false),
        ("service", "services", "delete", true),
        ("postgres", "postgres", "get", true),
        ("postgres", "postgres", "delete", false),
        ("org", "", "get", false),
    ] {
        let mock = MockServer::start().await;
        let resource_path = if domain == "org" {
            format!("/v1/organizations/{UNKNOWN_UUID}")
        } else {
            format!("/v1/organizations/{LOOKUP_ORG_ID}/{collection}/{UNKNOWN_UUID}")
        };
        // Postgres deletion first reads the object to render it. Test
        // deletion racing with that read, so the DELETE itself fails.
        if domain == "postgres" && operation == "delete" {
            Mock::given(method("GET"))
                .and(path(resource_path.clone()))
                .respond_with(manual_query_endpoint_response(
                    serde_json::json!({"id": UNKNOWN_UUID}),
                ))
                .expect(1)
                .mount(&mock)
                .await;
        }
        Mock::given(method(if operation == "delete" {
            "DELETE"
        } else {
            "GET"
        }))
        .and(path(resource_path))
        .respond_with(deleted_resource_404_response())
        .expect(1)
        .mount(&mock)
        .await;
        let args = if domain == "org" {
            vec![domain, operation, "--org-id", UNKNOWN_UUID]
        } else {
            vec![domain, operation, UNKNOWN_UUID, "--org-id", LOOKUP_ORG_ID]
        };
        let output = if human {
            invoke_cli_human(&mock, &args)
        } else {
            invoke_cli_with_cloud_credentials(&mock, &args)
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(UNKNOWN_UUID), "{stderr}");
        assert!(stderr.contains("NOT_FOUND: Not Found"), "{stderr}");
        if domain != "org" {
            assert!(stderr.contains(LOOKUP_ORG_ID), "{stderr}");
        }
        if human {
            assert!(stderr.starts_with("Error: "), "{stderr}");
        } else {
            let error: Value = serde_json::from_str(stderr.trim()).expect("structured error");
            assert_eq!(error["error"]["code"], "resource_not_found");
            let command = if domain == "org" {
                "clickhousectl cloud org list".to_string()
            } else {
                format!("clickhousectl cloud {domain} list --org-id {LOOKUP_ORG_ID}")
            };
            assert_eq!(error["error"]["command"], command);
        }
    }
}

#[tokio::test]
async fn deleted_resource_404_query_endpoint_checks_parent_before_naming_missing_service() {
    for parent_status in [200, 400, 404] {
        let mock = MockServer::start().await;
        let service_path = format!("/v1/organizations/{LOOKUP_ORG_ID}/services/{UNKNOWN_UUID}");
        Mock::given(method("GET"))
            .and(path(format!("{service_path}/serviceQueryEndpoint")))
            .respond_with(deleted_resource_404_response())
            .expect(1)
            .mount(&mock)
            .await;
        let parent = match parent_status {
            200 => manual_query_endpoint_response(serde_json::json!({"id": UNKNOWN_UUID})),
            400 => invalid_id_400_response("service", UNKNOWN_UUID),
            _ => deleted_resource_404_response(),
        };
        Mock::given(method("GET"))
            .and(path(service_path.clone()))
            .respond_with(parent)
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "service",
                "query-endpoint",
                "get",
                UNKNOWN_UUID,
                "--org-id",
                LOOKUP_ORG_ID,
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        if parent_status == 200 {
            assert!(!stderr.contains("resource_not_found"), "{stderr}");
            assert!(stderr.contains("NOT_FOUND: Not Found"), "{stderr}");
            assert!(stderr.contains(LOOKUP_ORG_ID), "{stderr}");
        } else {
            let error: Value = serde_json::from_str(stderr.trim()).expect("structured error");
            assert_eq!(error["error"]["code"], "resource_not_found");
            assert!(
                error["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains(UNKNOWN_UUID)
            );
        }
        assert_eq!(
            received_request_shape(&mock).await,
            vec![
                ("GET".into(), format!("{service_path}/serviceQueryEndpoint")),
                ("GET".into(), service_path)
            ]
        );
    }
}

/// Every by-ID and `--name` path the 2026-11-04 sweep found reports a missing
/// resource as `resource_not_found`, naming the list command for whatever is
/// missing (#1237). Unmatched requests answer wiremock's 404, so only the
/// resources that do exist are mounted.
#[tokio::test]
async fn missing_resources_report_resource_not_found_on_by_id_and_name_paths() {
    const SERVICE: &str = "11111111-2222-4333-8444-555555555555";
    let org = LOOKUP_ORG_ID;
    let service_list = format!("clickhousectl cloud service list --org-id {org}");
    let in_service =
        |command: &str| format!("clickhousectl cloud {command} {SERVICE} --org-id {org}");
    let cases: Vec<(Vec<&str>, &[&str], String)> = vec![
        // The service itself is missing.
        (vec!["service", "stop", SERVICE], &[], service_list.clone()),
        (
            vec!["service", "settings", "list", SERVICE],
            &[],
            service_list.clone(),
        ),
        (vec!["backup", "list", SERVICE], &[], service_list.clone()),
        (
            vec!["clickpipe", "list", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["saved-query", "list", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["query-api-endpoint", "list", SERVICE],
            &[],
            service_list.clone(),
        ),
        // A resource inside a missing service names the service.
        (
            vec!["clickpipe", "get", SERVICE, UNKNOWN_UUID],
            &[],
            service_list.clone(),
        ),
        // A resource inside an existing service names the resource.
        (
            vec!["clickpipe", "get", SERVICE, UNKNOWN_UUID],
            &["service"],
            in_service("clickpipe list"),
        ),
        (
            vec!["clickpipe", "delete", SERVICE, UNKNOWN_UUID],
            &["service"],
            in_service("clickpipe list"),
        ),
        (
            vec!["saved-query", "get", SERVICE, UNKNOWN_UUID],
            &["service"],
            in_service("saved-query list"),
        ),
        (
            vec!["query-api-endpoint", "get", SERVICE, UNKNOWN_UUID],
            &["service"],
            in_service("query-api-endpoint list"),
        ),
        (
            vec!["backup", "get", SERVICE, UNKNOWN_UUID],
            &["service"],
            in_service("backup list"),
        ),
        // An attachment names whichever of the UDF and the service is missing.
        (
            vec!["udf", "attach", "my_udf", SERVICE],
            &[],
            format!("clickhousectl cloud udf list --org-id {org}"),
        ),
        (
            vec!["udf", "attach", "my_udf", SERVICE],
            &["udf"],
            service_list.clone(),
        ),
        // An organization-scoped list names the organization.
        (
            vec!["member", "list"],
            &[],
            "clickhousectl cloud org list".into(),
        ),
        // A name or email that matches nothing.
        (
            vec!["service", "delete", "--name", "ghost"],
            &["services"],
            service_list.clone(),
        ),
        (
            vec!["member", "get", "--email", "ghost@example.com"],
            &["members"],
            format!("clickhousectl cloud member list --org-id {org}"),
        ),
    ];
    for (args, existing, expected_command) in cases {
        let mock = MockServer::start().await;
        let service_path = format!("/v1/organizations/{org}/services/{SERVICE}");
        for resource in existing {
            let (api_path, result) = match *resource {
                "service" => (service_path.clone(), serde_json::json!({"id": SERVICE})),
                "udf" => (
                    format!("/v1/organizations/{org}/udfs/my_udf"),
                    serde_json::json!({"name": "my_udf"}),
                ),
                "services" => (
                    format!("/v1/organizations/{org}/services"),
                    serde_json::json!([]),
                ),
                "members" => (
                    format!("/v1/organizations/{org}/members"),
                    serde_json::json!([]),
                ),
                other => unreachable!("{other}"),
            };
            Mock::given(method("GET"))
                .and(path(api_path))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
                )
                .mount(&mock)
                .await;
        }
        let mut cli_args = args.clone();
        cli_args.extend(["--org-id", org]);

        let output = invoke_cli_with_cloud_credentials(&mock, &cli_args);

        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let error = cloud_runtime_error(&output);
        assert_eq!(error["code"], "resource_not_found", "{args:?}: {error}");
        assert_eq!(error["command"], expected_command, "{args:?}: {error}");
    }
}

#[tokio::test]
async fn global_org_id_from_an_ancestor_position_scopes_the_request() {
    // Clap propagates `--org-id` from every position
    // (`cli::org_id_reaches_cloud_args_at_every_command_and_positional_boundary`);
    // one run from the ancestor position shows the value reaches the request.
    let endpoint = "/v1/organizations/org-selected/postgres/pg-1/config";
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(endpoint))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"result": {"pgConfig": {}, "pgBouncerConfig": {}}}),
            ),
        )
        .expect(1)
        .mount(&mock)
        .await;
    assert_success(&invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "--org-id",
            "org-selected",
            "postgres",
            "config",
            "get",
            "pg-1",
        ],
    ));
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), endpoint);
}

#[tokio::test]
async fn global_org_id_does_not_add_lookup_to_auth_status_or_org_list() {
    let mock = MockServer::start().await;
    let status_mock = MockServer::start().await;
    assert_success(&invoke_cli_with_cloud_credentials(
        &status_mock,
        &["--org-id", "ignored-scope", "auth", "status"],
    ));
    // Only the identity check: no organization lookup.
    let requests = status_mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/v1/whoami");
    // A list call must return every org even if a shared selector is supplied.
    let organizations = serde_json::json!([
        {"id": "00000000-0000-0000-0000-000000000001", "name": "First"},
        {"id": "00000000-0000-0000-0000-000000000002", "name": "Second"}
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": organizations})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output =
        invoke_cli_with_cloud_credentials(&mock, &["org", "list", "--org-id", "ignored-scope"]);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        organizations
    );
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

// #891: exact, scoped selectors resolve before the existing target handler.
#[tokio::test]
async fn selectors_use_exact_scoped_names_and_ids_do_not_require_list_access() {
    const ID: &str = "11111111-2222-3333-4444-555555555555";
    type SelectorCase = (
        &'static [&'static str],
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    );
    let cases: &[SelectorCase] = &[
        (
            &["service", "get"],
            "services",
            "services",
            "GET",
            "name",
            "id",
        ),
        (
            &["postgres", "get"],
            "postgres",
            "postgres",
            "GET",
            "name",
            "id",
        ),
        (&["key", "get"], "keys", "keys", "GET", "name", "id"),
        (
            &["org", "role", "get"],
            "roles",
            "roles",
            "GET",
            "name",
            "id",
        ),
        (
            &["member", "get"],
            "members",
            "members",
            "GET",
            "email",
            "userId",
        ),
        (
            &["invitation", "get"],
            "invitations",
            "invitations",
            "GET",
            "email",
            "id",
        ),
        // Destructive commands resolve in their own dispatcher arm, so each
        // has a run of its own.
        (
            &["org", "role", "delete"],
            "roles",
            "roles",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["member", "remove"],
            "members",
            "members",
            "DELETE",
            "email",
            "userId",
        ),
        (
            &["invitation", "delete"],
            "invitations",
            "invitations",
            "DELETE",
            "email",
            "id",
        ),
        (
            &["clickpipe", "get", "parent-service"],
            "services/parent-service/clickpipes",
            "services/parent-service/clickpipes",
            "GET",
            "name",
            "id",
        ),
        (
            &["org", "byoc", "delete"],
            "",
            "byocInfrastructure",
            "DELETE",
            "displayName",
            "id",
        ),
        (
            &["clickstack", "source", "delete", "parent-service"],
            "services/parent-service/clickstack/sources",
            "services/parent-service/clickstack/sources",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "role", "delete", "parent-service"],
            "services/parent-service/clickstack/roles",
            "services/parent-service/clickstack/roles",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "dashboard", "delete", "parent-service"],
            "services/parent-service/clickstack/dashboards",
            "services/parent-service/clickstack/dashboards",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "alert", "delete", "parent-service"],
            "services/parent-service/clickstack/alerts",
            "services/parent-service/clickstack/alerts",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "webhook", "delete", "parent-service"],
            "services/parent-service/clickstack/webhooks",
            "services/parent-service/clickstack/webhooks",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "saved-search", "delete", "parent-service"],
            "services/parent-service/clickstack/saved-searches",
            "services/parent-service/clickstack/saved-searches",
            "DELETE",
            "name",
            "id",
        ),
    ];
    // Every resource's name lookup (its list route and name field) once. A
    // positional ID skips the list in `NameSelector::resolve` for all of them,
    // so only the first case also runs by ID.
    for (index, (prefix, list_suffix, target_suffix, verb, name_field, id_field)) in
        cases.iter().enumerate()
    {
        let forms: &[bool] = if index == 0 { &[false, true] } else { &[true] };
        for &named in forms {
            let mock = MockServer::start().await;
            let list_path = if list_suffix.is_empty() {
                "/v1/organizations/org-1".to_owned()
            } else {
                format!("/v1/organizations/org-1/{list_suffix}")
            };
            let mut row = serde_json::json!({*name_field: "Exact", *id_field: ID});
            if prefix.get(1) == Some(&"source") {
                row["kind"] = "promql".into();
            }
            if prefix.get(1) == Some(&"webhook") {
                row["service"] = "slack".into();
            }
            let mut other = row.clone();
            other[*name_field] = "exact".into();
            other[*id_field] = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into();
            let rows = serde_json::json!([other, row]);
            let result = if list_suffix.is_empty() {
                serde_json::json!({"byocConfig": rows})
            } else {
                rows
            };
            Mock::given(method("GET"))
                .and(path(&list_path))
                .respond_with(if named {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result}))
                } else {
                    ResponseTemplate::new(403).set_body_string("list forbidden")
                })
                .expect(if named { 1 } else { 0 })
                .mount(&mock)
                .await;
            let target_path = format!("/v1/organizations/org-1/{target_suffix}/{ID}");
            Mock::given(method(*verb))
                .and(path(&target_path))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({"result": {}, "status": 200})),
                )
                .expect(1)
                .mount(&mock)
                .await;
            let selector = if *name_field == "email" {
                "--email"
            } else {
                "--name"
            };
            let mut args = prefix.to_vec();
            if named {
                args.extend([selector, "Exact"]);
            } else {
                args.push(ID);
            }
            args.extend(["--org-id", "org-1"]);
            let output = invoke_cli_with_cloud_credentials(&mock, &args);
            assert!(
                output.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                mock.received_requests().await.unwrap().len(),
                if named { 2 } else { 1 }
            );
        }
    }
}

#[tokio::test]
async fn selectors_refuse_bad_lists_before_mutation_and_preserve_list_errors() {
    // Which lists cannot resolve a name is
    // `shared::exact_resolution_rejects_incomplete_or_ambiguous_collections`;
    // here a refused list and a failed list each stop the update.
    for (status, body, exit) in [
        (
            200,
            serde_json::json!({"result": [{"name": "target", "id": "11111111-2222-3333-4444-555555555555"}, {"name": "target", "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}]}),
            1,
        ),
        (401, serde_json::json!({"error": "list forbidden"}), 4),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/services"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "service",
                "update",
                "--name",
                "target",
                "--new-name",
                "replacement",
                "--org-id",
                "org-1",
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
    }
}

#[tokio::test]
async fn selectors_rename_body_and_organization_name_scope_are_independent() {
    let org = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let id = "11111111-2222-3333-4444-555555555555";
    for resource in ["service", "postgres", "key", "org"] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"result": [{"id": org, "name": "Production"}]}),
                ),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let suffix = match resource {
            "service" => "services",
            "postgres" => "postgres",
            "key" => "keys",
            _ => "",
        };
        let target_path = if resource == "org" {
            format!("/v1/organizations/{org}")
        } else {
            format!("/v1/organizations/{org}/{suffix}/{id}")
        };
        if resource != "org" {
            Mock::given(method("GET"))
                .and(path(format!("/v1/organizations/{org}/{suffix}")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"result": [{"id": id, "name": "target"}]}),
                    ),
                )
                .expect(1)
                .mount(&mock)
                .await;
        }
        Mock::given(method("PATCH"))
            .and(path(&target_path))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {}})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec![
            resource,
            "update",
            "--org-name",
            "Production",
            "--new-name",
            "replacement",
        ];
        if resource != "org" {
            args.extend(["--name", "target"]);
        }
        if resource == "postgres" {
            args.push("--clear-tags");
        }
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        let patch = requests.iter().find(|r| r.method == "PATCH").unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert_eq!(body["name"], "replacement");
        assert!(!body.as_object().unwrap().contains_key("newName"));
        assert_eq!(requests.len(), if resource == "org" { 2 } else { 3 });
    }
}

#[tokio::test]
async fn selectors_query_endpoints_read_all_cursor_pages_before_mutating() {
    let first_id = "11111111-2222-3333-4444-555555555555";
    let second_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    for second in [
        serde_json::json!({"result": [], "nextCursor": null}),
        serde_json::json!({"result": [{"id": second_id, "name": "target"}], "nextCursor": null}),
        serde_json::json!({"result": null, "nextCursor": null}),
        serde_json::json!({"result": [], "nextCursor": "page-2"}),
    ] {
        let unique = second["nextCursor"].is_null()
            && second["result"].as_array().is_some_and(Vec::is_empty);
        let mock = MockServer::start().await;
        let list_path = "/v1/organizations/org-1/services/parent/query-api-endpoints";
        Mock::given(method("GET"))
            .and(path(list_path))
            .and(query_param("limit", "100"))
            .and(wiremock::matchers::query_param_is_missing("cursor"))
            .respond_with(query_api_endpoint_list_page(
                serde_json::json!([{"id": first_id, "name": "target"}]),
                Some("page-2"),
                None,
                Some(100),
            ))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .and(path(list_path))
            .and(query_param("cursor", "page-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(second))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("DELETE"))
            .and(path(format!("{list_path}/{first_id}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})),
            )
            .expect(if unique { 1 } else { 0 })
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "query-api-endpoint",
                "delete",
                "parent",
                "--name",
                "target",
                "--org-id",
                "org-1",
            ],
        );
        assert_eq!(
            output.status.success(),
            unique,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            if unique { 3 } else { 2 }
        );
    }
}

#[tokio::test]
async fn selectors_clickstack_exhaust_offset_pages_before_mutating() {
    // Each resource's offset page fetch once; late duplicates are
    // `clickstack::name_pagination_rejects_duplicates_on_later_pages`.
    for (command, resource) in [
        ("alert", "alerts"),
        ("webhook", "webhooks"),
        ("saved-search", "saved-searches"),
    ] {
        let mock = MockServer::start().await;
        let list_path = format!("/v1/organizations/org-1/services/parent/clickstack/{resource}");
        let first: Vec<Value> = (0..1000)
            .map(|index| {
                serde_json::json!({
                    "id": format!("00000000-0000-0000-0000-{index:012}"),
                    "name": format!("other-{index}"),
                    "service": "slack"
                })
            })
            .collect();
        Mock::given(method("GET"))
            .and(path(&list_path))
            .and(query_param("limit", "1000"))
            .and(query_param("offset", "0"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": first})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .and(path(&list_path))
            .and(query_param("limit", "1000"))
            .and(query_param("offset", "1000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": [{"id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "target", "service": "slack"}]})))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("DELETE"))
            .and(path(format!(
                "{list_path}/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickstack",
                command,
                "delete",
                "parent",
                "--name",
                "target",
                "--org-id",
                "org-1",
            ],
        );
        assert_success(&output);
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            3,
            "{command}"
        );
    }
}

#[tokio::test]
async fn selectors_unknown_clickstack_variants_never_mutate() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/parent/clickstack/sources",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"result": [{"id": "id", "name": "target", "kind": "future-variant"}]}),
        ))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickstack",
            "source",
            "delete",
            "parent",
            "--name",
            "target",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

/// The one end-to-end copy of the OAuth write check (`cloud::run`); which
/// commands count as writes is `is_write_command_classifies_reads_and_writes_in_every_domain`.
/// The check runs before any input is read and before organization or target
/// names are resolved, so neither run makes a request.
#[tokio::test]
async fn oauth_writes_are_rejected_before_input_org_or_target_lookups() {
    let mock = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    for (args, json) in [
        (
            vec![
                "udf",
                "create",
                "--file",
                "missing",
                "--artifact",
                "missing",
                "--org-id",
                "org-1",
            ],
            false,
        ),
        (
            vec![
                "service",
                "update",
                "--new-name",
                "new",
                "--name",
                "target",
                "--org-name",
                "production",
            ],
            true,
        ),
    ] {
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(project.path())
            .args(["cloud", "--url", &mock.uri()])
            .args(&args);
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("clickhousectl cloud auth login --api-key"),
            "{stderr}"
        );
        if json {
            assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn selectors_postgres_sources_keep_new_resource_names_in_the_body() {
    // Each command resolves `--source-name` once; a positional ID skips the
    // lookup in `SourceSelector::resolve`.
    let id = "11111111-2222-3333-4444-555555555555";
    for (prefix, endpoint) in [
        (vec!["postgres", "restore"], "restoredService"),
        (vec!["postgres", "read-replica", "create"], "readReplica"),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/postgres"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"result": [{"id": id, "name": "source"}]})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/organizations/org-1/postgres/{id}/{endpoint}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"result": restored_service_response(
                    &format!("/{endpoint}"),
                    &serde_json::json!({}),
                )}),
            ))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = prefix.clone();
        args.extend(["--source-name", "source"]);
        args.extend(["--name", "created", "--org-id", "org-1"]);
        if endpoint == "restoredService" {
            args.extend([
                "--restore-target",
                "2026-09-01T12:00:00Z",
                "--block-public-access",
            ]);
        }
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        let request = requests.iter().find(|r| r.method == "POST").unwrap();
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["name"], "created");
        assert_eq!(requests.len(), 2);
    }
}

#[tokio::test]
async fn selectors_ambiguous_organization_name_fails_without_mutation() {
    // Other incomplete lists: `shared::exact_resolution_rejects_incomplete_or_ambiguous_collections`.
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": [
                { "name": "production", "id": "11111111-2222-3333-4444-555555555555" },
                { "name": "production", "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee" }
            ]})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "update",
            "--org-name",
            "production",
            "--new-name",
            "replacement",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn selectors_endpoint_pagination_integrity_and_later_errors_block_mutation() {
    let target_id = "11111111-2222-3333-4444-555555555555";
    let other_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    for (status, second, exit) in [
        (
            200,
            serde_json::json!({"result": [{"id": other_id, "name": "other"}], "totalCount": 3, "nextCursor": null}),
            1,
        ),
        (
            200,
            serde_json::json!({"result": [], "totalCount": 4, "nextCursor": null}),
            1,
        ),
        (
            200,
            serde_json::json!({"result": [], "totalCount": 3, "nextCursor": null}),
            1,
        ),
        (401, serde_json::json!({"error": "later page forbidden"}), 4),
    ] {
        let mock = MockServer::start().await;
        let list_path = "/v1/organizations/org-1/services/parent/query-api-endpoints";
        Mock::given(method("GET")).and(path(list_path)).and(wiremock::matchers::query_param_is_missing("cursor"))
            .respond_with(query_api_endpoint_list_page(serde_json::json!([{"id": target_id, "name": "target"}, {"id": other_id, "name": "other"}]), Some("page-2"), Some(3), Some(100)))
            .expect(1).mount(&mock).await;
        Mock::given(method("GET"))
            .and(path(list_path))
            .and(query_param("cursor", "page-2"))
            .respond_with(ResponseTemplate::new(status).set_body_json(second))
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "query-api-endpoint",
                "delete",
                "parent",
                "--name",
                "target",
                "--org-id",
                "org-1",
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| request.method == "GET"));
    }
}

fn cloud_runtime_error(output: &std::process::Output) -> Value {
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "stderr is not one JSON object ({error}): {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let error = envelope["error"].clone();
    assert!(error["code"].is_string());
    assert!(error["message"].is_string());
    error
}

// ── All Cloud runtime errors use the JSON envelope (issue #825) ────────────

/// A clean environment makes explicit, automatically detected, and human
/// output observable independently. Each invocation gets isolated credentials.
fn cloud_runtime_error_command(root: &Path, url: &str, mode: &str) -> Command {
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut command = Command::new(clickhousectl_binary());
    command
        .isolated_env()
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "test-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "test-secret")
        .current_dir(root)
        .args(["cloud", "--url", url]);
    match mode {
        "explicit" => {
            command.arg("--json");
        }
        "agent" => {
            command.env("AI_AGENT", "1");
        }
        "human" => {}
        _ => panic!("unknown output mode"),
    }
    command
}

#[tokio::test]
async fn cloud_runtime_errors_classify_http_statuses_in_explicit_and_agent_json() {
    // One run per distinct error code; the status → kind mapping is
    // `failure::api_errors_are_classified_from_the_variant_not_the_message`, and
    // the output mode is independent of the status.
    for (status, code, exit, mode) in [
        (400, "http_4xx", 1, "explicit"),
        (401, "auth_required", 4, "agent"),
        (408, "timeout", 1, "explicit"),
        (429, "rate_limited", 1, "agent"),
        (503, "http_5xx", 1, "explicit"),
        (504, "timeout", 1, "human"),
    ] {
        let mock = MockServer::start().await;
        // Deliberately identical prose: classification must come from status.
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/services"))
            .respond_with(
                ResponseTemplate::new(status).set_body_json(serde_json::json!({
                    "status": status, "error": "same diagnostic", "requestId": "test"
                })),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let output = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit),
            "status {status}, mode {mode}"
        );
        assert!(output.stdout.is_empty());
        if mode == "human" {
            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                "Error: same diagnostic\n"
            );
        } else {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stderr).unwrap(),
                serde_json::json!({"error": {"code": code, "message": "same diagnostic"}})
            );
        }
    }
}

#[tokio::test]
async fn cloud_runtime_errors_cover_missing_auth_validation_transport_and_usage() {
    let mock = MockServer::start().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    for mode in ["explicit", "agent"] {
        let dir = tempfile::tempdir().unwrap();
        let mut missing_auth = cloud_runtime_error_command(dir.path(), &mock.uri(), mode);
        missing_auth
            .env_remove("CLICKHOUSE_CLOUD_API_KEY")
            .env_remove("CLICKHOUSE_CLOUD_API_SECRET");
        let missing_auth = missing_auth.args(["service", "list"]).output().unwrap();
        assert_eq!(missing_auth.status.code(), Some(4));
        assert!(missing_auth.stdout.is_empty());
        assert_eq!(
            structured_error(&missing_auth)["error"]["code"],
            "auth_required"
        );

        let validation = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["auth", "signup", "--url", "invalid-url"])
            .output()
            .unwrap();
        assert_eq!(validation.status.code(), Some(1));
        assert!(validation.stdout.is_empty());
        assert_eq!(structured_error(&validation)["error"]["code"], "other");

        let transport = cloud_runtime_error_command(dir.path(), &closed_url, mode)
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(transport.status.code(), Some(1));
        assert!(transport.stdout.is_empty());
        assert_eq!(structured_error(&transport)["error"]["code"], "transport");

        let usage = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["service", "list", "--not-a-flag"])
            .output()
            .unwrap();
        assert_eq!(usage.status.code(), Some(2));
        assert!(usage.stdout.is_empty());
        assert!(serde_json::from_slice::<Value>(&usage.stderr).is_err());
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cloud_runtime_errors_report_sql_errors_with_the_typed_code() {
    let control = start_mock_control_plane_with_service().await;
    let (output, _) = invoke_service_query_against_failing_query_host(
        &control,
        400,
        r#"{"error":{"code":"62","details":"Syntax error near FROM"}}"#,
        &["--json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(structured_error(&output)["error"]["code"], "sql_error");
    let message = structured_error(&output)["error"]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        message.contains("62") && message.ends_with("Syntax error near FROM"),
        "{message}"
    );
}

#[tokio::test]
async fn cloud_runtime_errors_keep_credential_notices_out_of_json() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400, "error": "request failed", "requestId": "test"
        })))
        .expect(2)
        .mount(&mock)
        .await;
    for mode in ["explicit", "agent"] {
        let dir = tempfile::tempdir().unwrap();
        write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
        let output = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(cloud_runtime_error(&output)["code"], "http_4xx");

        let dir = tempfile::tempdir().unwrap();
        let mut command = cloud_runtime_error_command(dir.path(), &mock.uri(), mode);
        let token_dir = dir.path().join("home/.clickhouse");
        std::fs::create_dir_all(&token_dir).unwrap();
        // Old records without configuration are not usable credentials.
        // Require a new login rather than guessing from the environment.
        std::fs::write(
            token_dir.join("tokens.json"),
            serde_json::to_vec(&serde_json::json!({
                "access_token": "expired", "refresh_token": "refresh", "expires_at": 1,
                "api_url": mock.uri()
            }))
            .unwrap(),
        )
        .unwrap();
        let output = command
            .env_remove("CLICKHOUSE_CLOUD_API_KEY")
            .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
            // Even configured env values must not repair an unknown legacy issuer.
            .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
            .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used")
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
        // Loading rejects the old file without modifying it; login replaces it.
        assert!(token_dir.join("tokens.json").exists());
    }
}

// Old OAuth records must not block higher-precedence API keys.
#[tokio::test]
async fn expired_custom_host_tokens_do_not_block_api_key_auth() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "result": [] })))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    // API keys supplied via env (EnvVars source) outrank OAuth tokens.
    let mut command = cloud_runtime_error_command(dir.path(), &mock.uri(), "explicit");
    let token_dir = dir.path().join("home/.clickhouse");
    std::fs::create_dir_all(&token_dir).unwrap();
    std::fs::write(
        token_dir.join("tokens.json"),
        serde_json::to_vec(&serde_json::json!({
            "access_token": "expired", "refresh_token": "refresh", "expires_at": 1,
            "api_url": mock.uri()
        }))
        .unwrap(),
    )
    .unwrap();
    let output = command
        .args(["service", "list", "--org-id", "org-1"])
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The command reached the API using the API keys instead of failing fast.
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
    // Old files are ignored, not refreshed or modified; API-key auth succeeds.
    assert!(token_dir.join("tokens.json").exists());
}

#[tokio::test]
async fn api_key_precedence_skips_oauth_refresh_and_preserves_tokens() {
    // Every credential source with a refreshable (current, expired) login: the
    // skip in `cloud::run` is inline, and only a current record reaches the
    // refresh. Old and malformed records never refresh, so they run once each
    // to show an API key still wins over an unusable login
    // (`client::credentials_file_overrides_env` and the other resolver unit
    // tests own the ordering itself).
    for (format, source) in [
        ("current", "flags"),
        ("current", "file"),
        ("current", "env"),
        ("current", "dotenv"),
        ("current", "mixed"),
        ("current", "partial-flags"),
        ("old", "file"),
        ("malformed", "env"),
    ] {
        let api = MockServer::start().await;
        let issuer = MockServer::start().await;
        // If refresh is accidentally attempted it would reject and clear tokens.
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(401))
            .expect(0)
            .mount(&issuer)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/services"))
            .and(wiremock::matchers::basic_auth(
                "winning-key",
                "winning-secret",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": []})),
            )
            .expect(if source == "partial-flags" { 0 } else { 1 })
            .mount(&api)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let mut command = custom_oauth_command(dir.path(), &api.uri());
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        let mut record = serde_json::json!({
            "access_token": "expired", "refresh_token": "preserve-refresh", "expires_at": 1,
            "api_url": api.uri(),
            "auth_config": {"auth_url": issuer.uri(), "client_id": "saved-client"}
        });
        if format == "old" {
            record.as_object_mut().unwrap().remove("auth_config");
        }
        let original = if format == "malformed" {
            b"not json".to_vec()
        } else {
            serde_json::to_vec(&record).unwrap()
        };
        std::fs::write(&token_path, &original).unwrap();
        match source {
            "flags" => {
                // Populate lower tiers too, proving the same precedence as the client.
                write_project_api_credentials(
                    dir.path(),
                    "ignored-file",
                    "ignored-secret",
                    &api.uri(),
                );
                command
                    .env("CLICKHOUSE_CLOUD_API_KEY", "ignored-env")
                    .env("CLICKHOUSE_CLOUD_API_SECRET", "ignored-secret")
                    .args(["--api-key", "winning-key", "--api-secret", "winning-secret"]);
            }
            "file" => {
                write_project_api_credentials(
                    dir.path(),
                    "winning-key",
                    "winning-secret",
                    &api.uri(),
                );
                command
                    .env("CLICKHOUSE_CLOUD_API_KEY", "ignored-env")
                    .env("CLICKHOUSE_CLOUD_API_SECRET", "ignored-secret");
            }
            "env" => {
                command
                    .env("CLICKHOUSE_CLOUD_API_KEY", "winning-key")
                    .env("CLICKHOUSE_CLOUD_API_SECRET", "winning-secret");
            }
            "dotenv" => {
                std::fs::write(dir.path().join(".env"), "CLICKHOUSE_CLOUD_API_KEY=winning-key\nCLICKHOUSE_CLOUD_API_SECRET=winning-secret\n").unwrap();
            }
            "mixed" => {
                command.env("CLICKHOUSE_CLOUD_API_KEY", "winning-key");
                std::fs::write(
                    dir.path().join(".env"),
                    "CLICKHOUSE_CLOUD_API_SECRET=winning-secret\n",
                )
                .unwrap();
            }
            "partial-flags" => {
                command.arg("--api-key").arg("winning-key");
            }
            _ => unreachable!(),
        }
        let output = command
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if source == "partial-flags" { 4 } else { 0 }),
            "{source}: {output:?}"
        );
        assert_eq!(std::fs::read(&token_path).unwrap(), original, "{source}");
        assert!(
            issuer.received_requests().await.unwrap().is_empty(),
            "{source}"
        );
        // Replenish Tokio's cooperative budget before wiremock's synchronous
        // drop verification acquires its async lock on the next iteration.
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn incomplete_lower_priority_keys_do_not_prevent_oauth_refresh() {
    for source in ["partial-env", "empty-file"] {
        let api = MockServer::start().await;
        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "refreshed", "expires_in": 3600, "token_type": "Bearer"
            })))
            .expect(1)
            .mount(&issuer)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/services"))
            .and(wiremock::matchers::header(
                "authorization",
                "Bearer refreshed",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": []})),
            )
            .expect(1)
            .mount(&api)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let mut command = custom_oauth_command(dir.path(), &api.uri());
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        std::fs::write(
            &token_path,
            serde_json::to_vec(&serde_json::json!({
                "access_token": "expired", "refresh_token": "refresh", "expires_at": 1,
                "api_url": api.uri(),
                "auth_config": {"auth_url": issuer.uri(), "client_id": "saved-client"}
            }))
            .unwrap(),
        )
        .unwrap();
        match source {
            "partial-env" => {
                command.env("CLICKHOUSE_CLOUD_API_KEY", "incomplete-key");
            }
            "empty-file" => write_project_api_credentials(dir.path(), "", "", &api.uri()),
            _ => unreachable!(),
        }
        let output = command
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_success(&output);
        let saved: Value = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
        assert_eq!(saved["access_token"], "refreshed", "{source}");
    }
}

fn custom_oauth_command(root: &Path, api_url: &str) -> Command {
    let mut command = cloud_runtime_error_command(root, api_url, "explicit");
    command
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET");
    command
}

#[tokio::test]
async fn old_oauth_records_require_login_even_for_known_hosts_and_fresh_tokens() {
    let mock = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let token_dir = dir.path().join("home/.clickhouse");
    std::fs::create_dir_all(&token_dir).unwrap();
    // `auth::load_tokens_rejects_old_records_for_known_and_custom_hosts` covers
    // custom hosts and expired tokens; a fresh token for a known host is the
    // case a refresh could otherwise mask.
    let old = serde_json::json!({
        "access_token": "old-access", "refresh_token": "old-refresh",
        "expires_at": 4102444800_i64, "api_url": "https://api.clickhouse.cloud/v1"
    });
    std::fs::write(
        token_dir.join("tokens.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    let output = custom_oauth_command(dir.path(), &mock.uri())
        .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
        .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used")
        .args(["auth", "whoami"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// Legacy production records cannot be refreshed: issuer configuration must be
// saved by a new login, not reconstructed from the built-in table or current env.
#[tokio::test]
async fn expired_legacy_production_tokens_require_login_instead_of_refresh() {
    let mock = MockServer::start().await;
    // The oldest production format also omitted api_url.
    for (mode, includes_api_url) in [("human", false), ("explicit", true)] {
        let dir = tempfile::tempdir().unwrap();
        let mut command = cloud_runtime_error_command(dir.path(), &mock.uri(), mode);
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        let mut legacy = serde_json::json!({
            "access_token": "expired-production-access",
            "refresh_token": "legacy-production-refresh",
            "expires_at": 1
        });
        if includes_api_url {
            legacy["api_url"] = serde_json::json!("https://api.clickhouse.cloud/v1");
        }
        let original = serde_json::to_vec(&legacy).unwrap();
        std::fs::write(&token_path, &original).unwrap();
        let output = command
            .env_remove("CLICKHOUSE_CLOUD_API_KEY")
            .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
            // No environment or built-in configuration may repair the record.
            .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
            .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used")
            .args(["auth", "whoami"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4), "{mode}: {output:?}");
        assert!(output.stdout.is_empty());
        let message = if mode == "human" {
            String::from_utf8_lossy(&output.stderr).into_owned()
        } else {
            let error = cloud_runtime_error(&output);
            assert_eq!(error["code"], "auth_required");
            error["message"].as_str().unwrap().to_owned()
        };
        assert_unusable_login_message(&message);
        assert_eq!(std::fs::read(&token_path).unwrap(), original);
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// An unusable saved login points only at `cloud auth login`; missing
/// credentials list every source, API-key flags and variables included.
fn assert_unusable_login_message(message: &str) {
    assert!(
        message.contains("`clickhousectl cloud auth login`"),
        "{message}"
    );
    assert!(!message.contains("--api-key"), "{message}");
    assert!(!message.contains("CLICKHOUSE_CLOUD_API_KEY"), "{message}");
}

fn assert_missing_credentials_message(message: &str) {
    assert!(message.contains("--api-key"), "{message}");
    assert!(message.contains("CLICKHOUSE_CLOUD_API_KEY"), "{message}");
}

#[tokio::test]
async fn unusable_oauth_login_is_distinct_from_missing_credentials() {
    let mock = MockServer::start().await;
    // One run per distinct outcome: no login, and an unusable one in each
    // output format and location. Which records are unusable is
    // `auth::load_tokens_reports_unusable_files_without_modifying_them` and
    // `auth::token_deserialization_requires_complete_configuration`.
    for (mode, format, location) in [
        ("human", "absent", "global"),
        ("explicit", "malformed", "legacy"),
        ("human", "incomplete", "global"),
        ("explicit", "old", "global"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join(if location == "global" {
            "home/.clickhouse/tokens.json"
        } else {
            ".clickhouse/tokens.json"
        });
        let original = match format {
            "absent" => None,
            "old" => Some(
                serde_json::to_vec(&serde_json::json!({
                    "access_token": "private-access", "refresh_token": "private-refresh",
                    "expires_at": 4102444800_i64, "api_url": mock.uri()
                }))
                .unwrap(),
            ),
            "incomplete" => Some(
                serde_json::to_vec(&serde_json::json!({
                    "access_token": "private-access", "refresh_token": "private-refresh",
                    "expires_at": 4102444800_i64, "api_url": mock.uri(),
                    "auth_config": {"auth_url": mock.uri()}
                }))
                .unwrap(),
            ),
            "malformed" => Some(b"not json: private-access".to_vec()),
            _ => unreachable!(),
        };
        if let Some(data) = &original {
            std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
            std::fs::write(&token_path, data).unwrap();
        }
        let command = || {
            let mut cmd = cloud_runtime_error_command(dir.path(), &mock.uri(), mode);
            cmd.env_remove("CLICKHOUSE_CLOUD_API_KEY")
                .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
                .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used");
            cmd
        };
        let output = command().args(["auth", "status"]).output().unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let expected = if format == "absent" {
            "Not configured"
        } else {
            "Login required"
        };
        if mode == "human" {
            let row = stdout
                .lines()
                .find(|line| line.contains("| OAuth"))
                .unwrap();
            let cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells, ["OAuth", expected, "-", "-", "-"]);
        } else {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            let row = status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["type"] == "OAuth")
                .unwrap();
            assert_eq!(row["status"], expected);
            assert_eq!(row["scope"], "-");
            assert_eq!(row["active"], "-");
            assert_eq!(status["verification"], "skipped");
            assert!(status.get("warning").is_none());
            if format == "absent" {
                assert!(status.get("oauth_warning").is_none());
            } else {
                assert_unusable_login_message(status["oauth_warning"].as_str().unwrap());
            }
        }
        if format == "absent" {
            assert!(
                !stdout.contains("`clickhousectl cloud auth login`"),
                "{stdout}"
            );
        } else {
            assert!(
                stdout.contains("`clickhousectl cloud auth login`"),
                "{stdout}"
            );
        }
        let output = command()
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        assert!(output.stdout.is_empty());
        let message = if mode == "human" {
            String::from_utf8_lossy(&output.stderr).into_owned()
        } else {
            let error = cloud_runtime_error(&output);
            assert_eq!(error["code"], "auth_required");
            error["message"].as_str().unwrap().to_owned()
        };
        if format == "absent" {
            assert_missing_credentials_message(&message);
        } else {
            assert_unusable_login_message(&message);
        }
        for secret in ["private-access", "private-refresh"] {
            assert!(!stdout.contains(secret));
            assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
        }
        if let Some(data) = original {
            assert_eq!(std::fs::read(&token_path).unwrap(), data);
            if location == "legacy" {
                assert!(!dir.path().join("home/.clickhouse/tokens.json").exists());
            }
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_reports_unusable_oauth_alongside_api_key_identity() {
    for mode in ["human", "explicit"] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .and(wiremock::matchers::basic_auth("test-key", "test-secret"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "status": 401, "error": "key rejected"
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        std::fs::write(&token_path, b"not json").unwrap();
        let output = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["auth", "status", "--debug"])
            .output()
            .unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains("`clickhousectl cloud auth login`"),
            "{stdout}"
        );
        if mode == "human" {
            assert!(stdout.contains("Identity: rejected"));
        } else {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], "rejected");
            assert!(status["warning"].as_str().unwrap().contains("key rejected"));
            assert_unusable_login_message(status["oauth_warning"].as_str().unwrap());
            let sources = status["sources"].as_array().unwrap();
            let env = sources
                .iter()
                .find(|row| row["type"] == "Env vars")
                .unwrap();
            assert_eq!(env["active"], "yes");
            let oauth = sources.iter().find(|row| row["type"] == "OAuth").unwrap();
            assert_eq!(oauth["status"], "Login required");
            assert_eq!(oauth["active"], "-");
        }
        assert_eq!(std::fs::read(&token_path).unwrap(), b"not json");
    }
}

#[tokio::test]
async fn oauth_login_and_refresh_never_follow_redirects() {
    let api = MockServer::start().await;
    let destination = MockServer::start().await;
    // One client builder serves both flows, and its redirect policy treats
    // every 3xx alike; 307/308 are the ones that would replay the body.
    for (status, login) in [(307, true), (308, false)] {
        let issuer = MockServer::start().await;
        let endpoint = if login {
            "/oauth/device/code"
        } else {
            "/oauth/token"
        };
        Mock::given(method("POST"))
            .and(path(endpoint))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Location", format!("{}/stolen", destination.uri())),
            )
            .expect(1)
            .mount(&issuer)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let mut command = custom_oauth_command(dir.path(), &api.uri());
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        if login {
            command
                .env("CLICKHOUSECTL_AUTH_URL", issuer.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "client")
                .args(["auth", "login"]);
        } else {
            std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
            std::fs::write(
                &token_path,
                serde_json::to_vec(&serde_json::json!({
                    "access_token": "expired", "refresh_token": "secret-refresh", "expires_at": 1,
                    "api_url": api.uri(),
                    "auth_config": {"auth_url": issuer.uri(), "client_id": "client"}
                }))
                .unwrap(),
            )
            .unwrap();
            command.args(["auth", "whoami"]);
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(if login { 1 } else { 4 }),
            "{status}: {output:?}"
        );
        assert!(destination.received_requests().await.unwrap().is_empty());
        // A failed refresh clears the login, and in JSON mode its warning
        // stays off stderr, which holds only the error object.
        assert!(!token_path.exists());
        cloud_runtime_error(&output);
    }
    assert!(api.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn oauth_rejects_insecure_authorization_config_before_requests() {
    let api = MockServer::start().await;
    // Other schemes: `auth::authorization_url_requires_https_except_for_loopback`.
    let auth_url = "http://auth.example.invalid";
    let dir = tempfile::tempdir().unwrap();
    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", auth_url)
        .env("CLICKHOUSECTL_CLIENT_ID", "client")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        cloud_runtime_error(&output)["message"]
            .as_str()
            .unwrap()
            .contains("HTTPS")
    );
    let token_path = dir.path().join("home/.clickhouse/tokens.json");
    assert!(!token_path.exists());
    std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
    std::fs::write(
        &token_path,
        serde_json::to_vec(&serde_json::json!({
            "access_token": "expired", "refresh_token": "secret-refresh", "expires_at": 1,
            "api_url": api.uri(),
            "auth_config": {"auth_url": auth_url, "client_id": "client"}
        }))
        .unwrap(),
    )
    .unwrap();
    let output = custom_oauth_command(dir.path(), &api.uri())
        .args(["auth", "whoami"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
    assert!(api.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn custom_oauth_login_pins_config_through_refresh_and_relogin() {
    let api = MockServer::start().await;
    let issuer_a = MockServer::start().await;
    let issuer_b = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let token_path = dir.path().join("home/.clickhouse/tokens.json");
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"actorType": "user", "email": "test@example.com", "organizations": []}
        })))
        .expect(4)
        .mount(&api)
        .await;

    // An absent local path makes the best-effort opener fail without launching
    // a real browser. This test drives the token response, not interactive login.
    let verification_uri = dir.path().join("no-browser-target");
    for (issuer, client_id, access, refresh) in [
        (&issuer_a, "client-a", "access-a", "refresh-a"),
        (&issuer_b, "client-b", "access-b", "refresh-b"),
    ] {
        Mock::given(method("POST"))
            .and(path("/oauth/device/code"))
            .and(wiremock::matchers::body_string_contains(format!(
                "client_id={client_id}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "test-device", "user_code": "TEST",
                "verification_uri": verification_uri.to_string_lossy(),
                "expires_in": 60, "interval": 0
            })))
            .expect(1)
            .mount(issuer)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(wiremock::matchers::body_string_contains(
                "device_code=test-device",
            ))
            .and(wiremock::matchers::body_string_contains(format!(
                "client_id={client_id}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": access, "refresh_token": refresh,
                "expires_in": 3600, "token_type": "Bearer"
            })))
            .expect(1)
            .mount(issuer)
            .await;
    }

    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", format!("{}/", issuer_a.uri()))
        .env("CLICKHOUSECTL_CLIENT_ID", "client-a")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_success(&output);
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
    let config_a = serde_json::json!({"auth_url": issuer_a.uri(), "client_id": "client-a"});
    assert_eq!(saved["auth_config"], config_a);
    assert_eq!(saved["api_url"], format!("{}/v1", api.uri()));
    assert_eq!(saved["refresh_token"], "refresh-a");

    // First refresh removes the variables and rotates the token. The second
    // changes both variables to B and omits rotation: both must still use A.
    for (changed_env, old_refresh, response) in [
        (
            false,
            "refresh-a",
            serde_json::json!({
                "access_token": "refreshed-a", "refresh_token": "rotated-a",
                "expires_in": 3600, "token_type": "Bearer"
            }),
        ),
        (
            true,
            "rotated-a",
            serde_json::json!({
                "access_token": "refreshed-a-again", "expires_in": 3600, "token_type": "Bearer"
            }),
        ),
    ] {
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(wiremock::matchers::body_string_contains(
                "grant_type=refresh_token",
            ))
            .and(wiremock::matchers::body_string_contains(
                "client_id=client-a",
            ))
            .and(wiremock::matchers::body_string_contains(format!(
                "refresh_token={old_refresh}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&issuer_a)
            .await;
        saved["expires_at"] = serde_json::json!(1);
        std::fs::write(&token_path, serde_json::to_vec(&saved).unwrap()).unwrap();
        let mut command = custom_oauth_command(dir.path(), &api.uri());
        if changed_env {
            command
                .env("CLICKHOUSECTL_AUTH_URL", issuer_b.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "client-b");
        }
        let output = command.args(["auth", "whoami"]).output().unwrap();
        assert_success(&output);
        saved = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
        assert_eq!(saved["auth_config"], config_a);
        assert_eq!(saved["refresh_token"], "rotated-a");
        assert!(saved["expires_at"].as_i64().unwrap() > chrono::Utc::now().timestamp());
        assert!(issuer_b.received_requests().await.unwrap().is_empty());
    }

    // Only a deliberate new login changes the saved issuer/token pairing.
    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", issuer_b.uri())
        .env("CLICKHOUSECTL_CLIENT_ID", "client-b")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_success(&output);
    saved = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
    assert_eq!(
        saved["auth_config"],
        serde_json::json!({
            "auth_url": issuer_b.uri(), "client_id": "client-b"
        })
    );
    assert_eq!(saved["access_token"], "access-b");
    assert_eq!(saved["refresh_token"], "refresh-b");
    assert_eq!(issuer_a.received_requests().await.unwrap().len(), 4);
    assert_eq!(issuer_b.received_requests().await.unwrap().len(), 2);
}

// A pipe with no reader before spawn makes failure deterministic, even if the
// scheduler never runs the reader until after the CLI has produced its output.
fn closed_stdout() -> std::io::PipeWriter {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    writer
}

#[tokio::test]
async fn closed_stdout_large_cloud_list_succeeds_in_human_and_json_modes() {
    let mock = MockServer::start().await;
    let services: Vec<_> = (0..512)
        .map(|id| serde_json::json!({"id": format!("00000000-0000-0000-0000-{id:012x}"), "name": "x".repeat(4096)}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": services})),
        )
        .mount(&mock)
        .await;
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    for json in [false, true] {
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("HOME", home.path())
            .env("DO_NOT_TRACK", "1")
            .current_dir(project.path())
            .args([
                "cloud",
                "--url",
                &mock.uri(),
                "--api-key",
                "test-key",
                "--api-secret",
                "test-secret",
                "service",
                "list",
                "--org-id",
                "org-1",
            ]);
        if json {
            command.arg("--json");
        }
        // Check that both forms really exceed ordinary pipe buffer sizes.
        let complete = command.output().unwrap();
        assert_success(&complete);
        assert!(complete.stdout.len() > 1024 * 1024);
        let output = command.stdout(closed_stdout()).output().unwrap();
        assert_success(&output);
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[tokio::test]
async fn closed_stdout_keeps_a_genuine_cloud_failure_and_error_envelope() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Denied"))
        .mount(&mock)
        .await;
    let home = tempfile::tempdir().unwrap();
    for json in [false, true] {
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("HOME", home.path())
            .env("DO_NOT_TRACK", "1")
            .current_dir(home.path())
            .args([
                "cloud",
                "--url",
                &mock.uri(),
                "--api-key",
                "test-key",
                "--api-secret",
                "test-secret",
                "service",
                "list",
                "--org-id",
                "org-1",
            ])
            .stdout(closed_stdout());
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        if json {
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"]["code"], "auth_required");
        } else {
            assert!(String::from_utf8_lossy(&output.stderr).contains("Error:"));
        }
    }
}

#[tokio::test]
async fn service_snapshot_reads_use_exact_routes_and_preserve_json() {
    let mock = MockServer::start().await;
    let snapshot = serde_json::json!({"id": "00000000-0000-0000-0000-000000000001", "status": "done", "startedAt": "2026-09-18T10:00:00Z", "bucket": {"region": "us-east-1"}});
    for (args, suffix, result) in [
        (
            vec!["list", "svc-1"],
            "snapshots",
            serde_json::json!([snapshot, {}]),
        ),
        (
            vec!["get", "svc-1", "--snapshot-id", "snap-1"],
            "snapshots/snap-1",
            snapshot,
        ),
        (
            vec!["config", "get", "svc-1"],
            "snapshotConfiguration",
            serde_json::json!({"enabled": true, "gap": 30.0, "timeFrame": 1440.0}),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/org-1/services/svc-1/{suffix}"
            )))
            .and(header(
                "authorization",
                "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let mut full = vec!["service", "snapshot"];
        full.extend(args);
        full.extend(["--org-id", "org-1"]);
        let json = invoke_cli_with_cloud_credentials(&mock, &full);
        assert_success(&json);
        assert_eq!(
            serde_json::from_slice::<Value>(&json.stdout).unwrap(),
            result
        );
    }
}

#[tokio::test]
async fn service_snapshot_config_patch_sends_the_builder_body() {
    // Omission and the --gap/--time-frame pairing are builder rules; see
    // `snapshot_config_builder_preserves_partial_and_complete_updates`.
    let body = serde_json::json!({"enabled": true, "gap": 30.0, "timeFrame": 1440.0});
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
        ))
        .and(body_json(&body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": body})))
        .expect(1)
        .mount(&mock)
        .await;
    let mut args = vec![
        "service", "snapshot", "config", "update", "svc-1", "--org-id", "org-1",
    ];
    args.extend(["--enabled", "true", "--gap", "30", "--time-frame", "1440"]);
    let output = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        body
    );
}

fn kinesis_protobuf_args<'a>(
    operation: &'a str,
    format: &'a str,
    schema: Option<&'a str>,
) -> Vec<&'a str> {
    let mut args = kinesis_auth_args(operation, &[]);
    let format_index = args.iter().position(|arg| *arg == "--format").unwrap() + 1;
    args[format_index] = format;
    if let Some(schema) = schema {
        args.extend(["--protobuf-schema-file", schema]);
    }
    args
}

#[tokio::test]
async fn kinesis_protobuf_empty_stdin_is_a_usage_error_and_missing_file_stays_io() {
    // Create and schema discovery share the source builder, so one operation
    // covers both. Empty stdin is a usage error that keeps clap's text even in
    // JSON mode; a missing file is an I/O failure with the JSON `io` code.
    let mock = MockServer::start().await;
    let args = kinesis_protobuf_args("create", "Protobuf", Some("-"));
    for json in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", project.path())
            .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
            .current_dir(project.path())
            .args(["cloud", "--url", &mock.uri()])
            .args(&args)
            .stdin(Stdio::null());
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(2), "json={json}: {output:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Protobuf") && stderr.contains("stdin"),
            "{stderr}"
        );
        assert!(serde_json::from_slice::<Value>(&output.stderr).is_err());
    }
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &kinesis_protobuf_args("create", "Protobuf", Some("/missing/schema.proto")),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(cloud_runtime_error(&output)["code"], "io");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn kinesis_protobuf_schema_can_be_read_from_stdin() {
    let raw = "syntax = \"proto3\"; message Event {}";
    let mock = start_mock_clickpipes_api().await;
    let args = [
        "clickpipe",
        "create",
        "kinesis",
        "svc-id",
        "--org-id",
        "org",
        "--stream-name",
        "events",
        "--region",
        "us-east-1",
        "--format",
        "Protobuf",
        "--protobuf-schema-file",
        "-",
        "--name",
        "pipe",
        "--database",
        "default",
        "--table",
        "events",
    ];
    let body = invoke_cli_capture_body_with_stdin(&mock, &args, raw.as_bytes()).await;
    assert_eq!(
        body["source"]["kinesis"]["protobufSchema"],
        "c3ludGF4ID0gInByb3RvMyI7IG1lc3NhZ2UgRXZlbnQge30="
    );
}

// API-key pagination: explicit pages, all-page listing, and exhaustive name lookup.
const API_KEYS_PATH: &str = "/v1/organizations/org-1/keys";
const KEY_PAGE_CURSOR: &str = "opaque +/&?=雪";
const PAGED_KEY_ID: &str = "11111111-2222-3333-4444-555555555555";
const OTHER_PAGED_KEY_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

async fn mount_key_page(mock: &MockServer, cursor: Option<&str>, status: u16, body: Value) {
    let matcher = Mock::given(method("GET")).and(path(API_KEYS_PATH));
    let matcher = match cursor {
        Some(cursor) => matcher.and(query_param("cursor", cursor)),
        None => matcher.and(wiremock::matchers::query_param_is_missing("cursor")),
    };
    matcher
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .expect(1)
        .mount(mock)
        .await;
}

async fn assert_key_page_requests(mock: &MockServer, cursors: &[Option<&str>]) {
    let requests = mock.received_requests().await.unwrap();
    let pages: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path() == API_KEYS_PATH)
        .collect();
    assert_eq!(pages.len(), cursors.len());
    for (request, cursor) in pages.iter().zip(cursors) {
        assert_eq!(request.method, "GET");
        assert!(request.body.is_empty());
        let expected: Vec<_> = cursor
            .iter()
            .map(|cursor| ("cursor".to_owned(), (*cursor).to_owned()))
            .collect();
        assert_eq!(
            request.url.query_pairs().into_owned().collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            request.headers["authorization"].to_str().unwrap(),
            "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw=="
        );
    }
}

#[tokio::test]
async fn key_pagination_list_collects_all_pages_in_json_and_human_output() {
    for human in [false, true] {
        let mock = MockServer::start().await;
        let first = serde_json::json!({"id": OTHER_PAGED_KEY_ID, "name": "first-key"});
        let last = serde_json::json!({"id": PAGED_KEY_ID, "name": "last-key"});
        mount_key_page(
            &mock,
            None,
            200,
            serde_json::json!({
                "result": [first], "nextCursor": KEY_PAGE_CURSOR, "limit": 1, "totalCount": 2
            }),
        )
        .await;
        // An empty page is not a terminator when the server supplies another cursor.
        mount_key_page(
            &mock,
            Some(KEY_PAGE_CURSOR),
            200,
            serde_json::json!({
                "result": [], "nextCursor": "last-page"
            }),
        )
        .await;
        mount_key_page(
            &mock,
            Some("last-page"),
            200,
            serde_json::json!({
                "result": [last], "nextCursor": null
            }),
        )
        .await;
        let args = ["key", "list", "--all", "--org-id", "org-1"];
        let output = if human {
            invoke_cli_with_cloud_credentials_human(&mock, &args)
        } else {
            invoke_cli_with_cloud_credentials(&mock, &args)
        };
        assert_success(&output);
        if human {
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(stdout.contains("| first-key"), "{stdout}");
            assert!(stdout.contains("| last-key"), "{stdout}");
        } else {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                serde_json::json!([first, last])
            );
        }
        assert_key_page_requests(&mock, &[None, Some(KEY_PAGE_CURSOR), Some("last-page")]).await;
        assert_eq!(mock.received_requests().await.unwrap().len(), 3);
    }
}

#[tokio::test]
async fn key_pagination_omits_a_null_cursor_from_the_page() {
    // An absent cursor is library serde; a null one must not be printed.
    let mock = MockServer::start().await;
    let result = serde_json::json!([{ "id": PAGED_KEY_ID, "name": "only-key" }]);
    mount_key_page(
        &mock,
        None,
        200,
        serde_json::json!({"result": result, "nextCursor": null}),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials(&mock, &["key", "list", "--org-id", "org-1"]);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"result": result})
    );
    assert_key_page_requests(&mock, &[None]).await;
}

#[tokio::test]
async fn key_pagination_name_lookup_finds_later_match_and_rejects_cross_page_ambiguity() {
    // Each command resolves the name across pages; a cross-page duplicate
    // stops one of them before any write.
    for (command, http_method, ambiguous) in [
        ("get", "GET", false),
        ("get", "GET", true),
        ("update", "PATCH", false),
        ("delete", "DELETE", false),
    ] {
        let mock = MockServer::start().await;
        mount_key_page(&mock, None, 200, serde_json::json!({
        "result": [{"id": OTHER_PAGED_KEY_ID, "name": if ambiguous { "target" } else { "other" }}],
        "nextCursor": KEY_PAGE_CURSOR
        })).await;
        let target = serde_json::json!({"id": PAGED_KEY_ID, "name": "target"});
        mount_key_page(
            &mock,
            Some(KEY_PAGE_CURSOR),
            200,
            serde_json::json!({"result": [target]}),
        )
        .await;
        Mock::given(method(http_method))
            .and(path(format!("{API_KEYS_PATH}/{PAGED_KEY_ID}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": target})),
            )
            .expect(if ambiguous { 0 } else { 1 })
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["key", command, "--name", "target", "--org-id", "org-1"],
        );
        if ambiguous {
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let error = cloud_runtime_error(&output);
            // Names the ambiguous name and the number of matches.
            let message = error["message"].as_str().unwrap();
            assert!(message.contains("'target'"), "{message}");
            assert_eq!(digits(message), "2", "{message}");
        } else {
            assert_success(&output);
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                if command == "delete" {
                    serde_json::json!({})
                } else {
                    target
                }
            );
        }
        assert_key_page_requests(&mock, &[None, Some(KEY_PAGE_CURSOR)]).await;
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            if ambiguous { 2 } else { 3 }
        );
    }
}
#[tokio::test]
async fn key_pagination_later_page_errors_prevent_partial_output_and_name_resolution() {
    // The listing and the name lookup each stop at a failed later page; the
    // status → exit code mapping is the `dispatched_cloud_*` tests'.
    for (lookup, status, exit) in [(false, 400, 1), (true, 401, 4)] {
        let mock = MockServer::start().await;
        mount_key_page(
            &mock,
            None,
            200,
            serde_json::json!({
                "result": [{"id": PAGED_KEY_ID, "name": "target"}], "nextCursor": KEY_PAGE_CURSOR
            }),
        )
        .await;
        mount_key_page(
            &mock,
            Some(KEY_PAGE_CURSOR),
            status,
            serde_json::json!({"error": "later page denied"}),
        )
        .await;
        let args = if lookup {
            vec!["key", "get", "--name", "target", "--org-id", "org-1"]
        } else {
            vec!["key", "list", "--all", "--org-id", "org-1"]
        };
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_eq!(output.status.code(), Some(exit), "{output:?}");
        assert!(output.stdout.is_empty());
        let error = cloud_runtime_error(&output);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("later page denied")
        );
        assert_key_page_requests(&mock, &[None, Some(KEY_PAGE_CURSOR)]).await;
        assert_eq!(mock.received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn key_pagination_default_is_one_page_with_metadata() {
    let mock = MockServer::start().await;
    let body = serde_json::json!({
        "result": [{"id": PAGED_KEY_ID, "name": "first"}],
        "limit": 250, "totalCount": 501, "nextCursor": KEY_PAGE_CURSOR
    });
    mount_key_page(&mock, None, 200, body.clone()).await;
    let output = invoke_cli_with_cloud_credentials(&mock, &["key", "list", "--org-id", "org-1"]);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        body
    );
    assert_key_page_requests(&mock, &[None]).await;
}

#[tokio::test]
async fn key_pagination_explicit_page_forwards_limit_and_opaque_cursor() {
    // An empty cursor is passed through the same way (`collect_cursor_pages`
    // unit tests cover it as a page token).
    let cursor = "-opaque +/=&?'雪";
    let mock = MockServer::start().await;
    mount_key_page(
        &mock,
        Some(cursor),
        200,
        serde_json::json!({
            "result": [], "limit": 25, "totalCount": 0
        }),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "key", "list", "--org-id", "org-1", "--limit", "25", "--cursor", cursor,
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "result": [], "limit": 25, "totalCount": 0
        })
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.query_pairs().collect::<Vec<_>>(),
        vec![
            ("limit".into(), "25".into()),
            ("cursor".into(), cursor.into())
        ]
    );
}

#[tokio::test]
async fn key_pagination_all_sends_limit_on_every_page() {
    let mock = MockServer::start().await;
    mount_key_page(
        &mock,
        None,
        200,
        serde_json::json!({"result": [], "nextCursor": ""}),
    )
    .await;
    mount_key_page(&mock, Some(""), 200, serde_json::json!({"result": []})).await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["key", "list", "--org-id", "org-1", "--all", "--limit", "1"],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!([])
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert!(
            request
                .url
                .query_pairs()
                .any(|(key, value)| key == "limit" && value == "1")
        );
    }
}

#[tokio::test]
async fn key_pagination_human_continuation_preserves_shell_tokens_and_page_context() {
    // Quoting each kind of token is
    // `api_keys::quote_shell_argument_for_display_round_trips_or_refuses_control_characters`;
    // here one quoted and one refused cursor reach the printed hint.
    for cursor in ["-opaque +/=&?'雪", "\n\u{1b}[31m"] {
        let mock = MockServer::start().await;
        mount_key_page(
            &mock,
            None,
            200,
            serde_json::json!({"result": [], "nextCursor": cursor}),
        )
        .await;
        let output = invoke_cli_with_cloud_credentials_human(
            &mock,
            &["key", "list", "--org-id", "org-1", "--limit", "25"],
        );
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        if cursor.chars().any(char::is_control) {
            assert!(!stdout.contains(cursor), "{stdout}");
            assert!(stdout.contains("--json"), "{stdout}");
            assert!(!stdout.contains('\u{1b}'), "{stdout}");
        } else {
            let (_, flags) = stdout.split_once("with --org-id=").unwrap();
            let (flags, _) = flags.split_once(" to continue").unwrap();
            let roundtrip = Command::new("sh")
                .args(["-c", &format!("printf '%s\\n' --org-id={flags}")])
                .output()
                .unwrap();
            assert!(roundtrip.status.success());
            assert_eq!(
                String::from_utf8(roundtrip.stdout).unwrap(),
                format!("--org-id=org-1\n--limit=25\n--cursor={cursor}\n")
            );
        }
        assert_eq!(mock.received_requests().await.unwrap().len(), 1);
    }
}

// Postgres uses the OAuth-only non-OpenAPI Query API, without native clients.
const POSTGRES_QUERY_ID: &str = "11111111-2222-3333-4444-555555555555";
const POSTGRES_QUERY_ROWS: &str = "[\"answer\",\"answer\",\"nullable\"]\n[\"Int32\",\"String\",\"String\"]\n[42,\"hello\\tworld\\nnext\",null]\n";

fn postgres_query_command(root: &Path, control: &MockServer, query_host: &str) -> Command {
    let cloud_dir = root.join("home/.clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &control.uri());
    let mut command = Command::new(clickhousectl_binary());
    command
        .isolated_env()
        .env("HOME", root.join("home"))
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host)
        .current_dir(root)
        .args(["cloud", "--url", &control.uri(), "postgres", "query"])
        .stdin(Stdio::null());
    command
}

async fn mount_postgres_query(query: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(format!("/service/{POSTGRES_QUERY_ID}/runPostgres")))
        .and(query_param("orgId", "org-1"))
        .and(query_param("format", "JSONCompactEachRowWithNamesAndTypes"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .and(header("x-service-type", "postgres"))
        .and(header("content-type", "application/json"))
        .respond_with(response)
        .expect(1)
        .mount(query)
        .await;
}

#[tokio::test]
async fn postgres_query_streams_compact_json_in_explicit_and_agent_modes() {
    for mode in ["json", "agent"] {
        let control = MockServer::start().await;
        let query = MockServer::start().await;
        mount_postgres_query(
            &query,
            ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
        )
        .await;
        let root = tempfile::tempdir().unwrap();
        let mut command = postgres_query_command(root.path(), &control, &query.uri());
        command.args([
            POSTGRES_QUERY_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
        ]);
        if mode == "json" {
            command.arg("--json");
        } else {
            command.env("CLAUDECODE", "1");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        assert_eq!(output.stdout, POSTGRES_QUERY_ROWS.as_bytes());
        let requests = query.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
            serde_json::json!({"sql": "SELECT 42"})
        );
        assert!(!requests[0].headers.contains_key("auth-provider"));
        assert!(!requests[0].headers.contains_key("wake-service"));
        assert!(control.received_requests().await.unwrap().is_empty());
        assert!(
            !root.path().join(".clickhouse").exists(),
            "no client downloads or service query keys"
        );
    }
}

#[tokio::test]
async fn postgres_query_human_output_preserves_duplicate_columns_and_escapes_cells() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    mount_postgres_query(
        &query,
        ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            POSTGRES_QUERY_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert_eq!(lines[0], "answer\tanswer\tnullable");
    assert_eq!(
        lines[1].split('\t').count(),
        3,
        "escaped data cannot introduce columns"
    );
    assert!(lines[1].contains("hello\\tworld\\nnext"));
}

#[tokio::test]
async fn postgres_query_resolves_name_and_sends_database_and_script_unchanged() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"result": [{"id": POSTGRES_QUERY_ID, "name": "reporting"}]}),
        ))
        .expect(1)
        .mount(&control)
        .await;
    mount_postgres_query(
        &query,
        ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let sql = "SELECT 1;\nSELECT current_database();\n";
    std::fs::write(root.path().join("queries.sql"), sql).unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            "--name",
            "reporting",
            "--org-id",
            "org-1",
            "--database",
            "analytics",
            "--queries-file",
            "queries.sql",
            "--json",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    let requests = query.received_requests().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
        serde_json::json!({"sql": sql, "database": "analytics"})
    );
    assert_eq!(control.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn postgres_query_accepts_default_and_explicit_stdin_and_ignores_it_with_inline_sql() {
    for source in ["implicit", "file-dash", "inline"] {
        let control = MockServer::start().await;
        let query = MockServer::start().await;
        mount_postgres_query(
            &query,
            ResponseTemplate::new(200).set_body_string(POSTGRES_QUERY_ROWS),
        )
        .await;
        let root = tempfile::tempdir().unwrap();
        let mut command = postgres_query_command(root.path(), &control, &query.uri());
        command.args([POSTGRES_QUERY_ID, "--org-id", "org-1", "--json"]);
        match source {
            "file-dash" => {
                command.args(["--queries-file", "-"]);
            }
            "inline" => {
                command.args(["--query", "SELECT 2"]);
            }
            _ => {}
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"SELECT 1\n").unwrap();
        // Keep stdin open with --query: the binary must not wait for EOF.
        if source != "inline" {
            drop(stdin);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                panic!("Postgres query waited for unused stdin or failed to complete");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_success(&output);
        let requests = query.received_requests().await.unwrap();
        let expected = if source == "inline" {
            "SELECT 2"
        } else {
            "SELECT 1\n"
        };
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap()["sql"],
            expected
        );
    }
}

#[tokio::test]
async fn postgres_query_rejects_api_keys_even_with_saved_oauth_before_input_or_http() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret")
        .args([
            "--name",
            "reporting",
            "--org-id",
            "org-1",
            "--queries-file",
            "does-not-exist.sql",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    let error = cloud_runtime_error(&output);
    assert_eq!(error["code"], "auth_required");
    assert!(error["message"].as_str().unwrap().contains("OAuth"));
    assert!(query.received_requests().await.unwrap().is_empty());
    assert!(control.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_query_preserves_structured_failures_without_retries_or_provisioning() {
    for (status, body, exit, code) in [
        (
            400,
            r#"{"error":{"code":"POSTGRES_ERROR","details":"permission denied for relation private_table"}}"#,
            1,
            "sql_error",
        ),
        (401, r#"{"error":"Unauthorized"}"#, 4, "auth_required"),
        (403, r#"{"error":"Forbidden"}"#, 1, "http_4xx"),
        (404, "route missing", 1, "http_4xx"),
        (429, "rate limited", 1, "rate_limited"),
        (500, r#"{"error":"Timeout error."}"#, 1, "timeout"),
        (502, "upstream unavailable", 1, "http_5xx"),
    ] {
        let control = MockServer::start().await;
        let query = MockServer::start().await;
        mount_postgres_query(&query, ResponseTemplate::new(status).set_body_string(body)).await;
        let root = tempfile::tempdir().unwrap();
        let output = postgres_query_command(root.path(), &control, &query.uri())
            .args([
                POSTGRES_QUERY_ID,
                "--org-id",
                "org-1",
                "--query",
                "SELECT 1",
                "--json",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit),
            "status {status}: {output:?}"
        );
        assert!(output.stdout.is_empty());
        let error = cloud_runtime_error(&output);
        assert_eq!(error["code"], code, "status {status}: {error}");
        assert_eq!(query.received_requests().await.unwrap().len(), 1);
        assert!(control.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn postgres_query_decodes_gzip_with_the_cli_http_client() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    // The upstream Postgres runner always gzips its compact JSON stream.
    let compressed: Vec<u8> = vec![
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 139, 86, 74, 204, 43, 46, 79, 45, 82, 138, 229, 138, 86,
        242, 204, 43, 49, 54, 2, 177, 76, 140, 98, 185, 0, 244, 250, 162, 112, 26, 0, 0, 0,
    ];
    mount_postgres_query(
        &query,
        ResponseTemplate::new(200)
            .insert_header("content-encoding", "gzip")
            .set_body_bytes(compressed),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            POSTGRES_QUERY_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
            "--json",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(output.stdout, b"[\"answer\"]\n[\"Int32\"]\n[42]\n");
}

#[tokio::test]
async fn postgres_query_reports_truncated_stream_as_failure() {
    use std::io::{BufRead, Read};
    let control = MockServer::start().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 200\r\nConnection: close\r\n\r\n[\"answer\"]\n").unwrap();
    });
    let root = tempfile::tempdir().unwrap();
    let output = postgres_query_command(root.path(), &control, &url)
        .args([
            POSTGRES_QUERY_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
            "--json",
        ])
        .output()
        .unwrap();
    // Assert before joining: a child that never reached the query host leaves
    // the fake server blocked in accept(), so joining first would hang.
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(cloud_runtime_error(&output)["code"], "transport");
    assert!(control.received_requests().await.unwrap().is_empty());
    // A transport failure that never reached the host (a refused connection,
    // say) passes the asserts above; the child has exited, so wait boundedly.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !server.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "the child never reached the query host"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    server.join().unwrap();
}

#[tokio::test]
async fn postgres_query_closed_stdout_keeps_successful_query_successful() {
    let control = MockServer::start().await;
    let query = MockServer::start().await;
    let body = format!("[\"answer\"]\n[\"Int32\"]\n{}", "[42]\n".repeat(100_000));
    mount_postgres_query(&query, ResponseTemplate::new(200).set_body_string(body)).await;
    let root = tempfile::tempdir().unwrap();
    let mut child = postgres_query_command(root.path(), &control, &query.uri())
        .args([
            POSTGRES_QUERY_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 42",
            "--json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert_success(&output);
}

// ── Saved queries (#1029) ───────────────────────────────────────────────────

const SAVED_QUERIES_PATH: &str = "/v1/organizations/org-1/services/svc-1/saved-queries";
const SAVED_QUERY_ID: &str = "22222222-3333-4444-8555-666666666666";
const SAVED_QUERY_OTHER_ID: &str = "33333333-4444-4555-8666-777777777777";

fn saved_query_body() -> Value {
    serde_json::json!({
        "name": "orders",
        "sql": "SELECT * FROM orders WHERE id = {id:String}",
        "database": "analytics",
        "parameters": {"id": "default-id", "region": "eu=west"}
    })
}

fn saved_query_response() -> Value {
    let mut response = saved_query_body();
    response["id"] = serde_json::json!(SAVED_QUERY_ID);
    response
}

fn saved_query_create_args(extra: &[&'static str]) -> Vec<&'static str> {
    let mut args = vec![
        "create",
        "svc-1",
        "--name",
        "orders",
        "--sql",
        "SELECT * FROM orders WHERE id = {id:String}",
        "--database",
        "analytics",
        "--param",
        "id=default-id",
        "--param",
        "region=eu=west",
        "--org-id",
        "org-1",
    ];
    args.extend_from_slice(extra);
    args
}

fn invoke_saved_query(
    server: &MockServer,
    project: &Path,
    oauth: bool,
    json: bool,
    args: &[&str],
    stdin: Option<&str>,
) -> std::process::Output {
    let home = project.join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    if oauth {
        write_oauth_tokens(&cloud_dir, &server.uri());
    }
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project)
        .args(["cloud", "--url", &server.uri()]);
    if !oauth {
        command.args([
            "--api-key",
            "saved-query-key",
            "--api-secret",
            "saved-query-secret",
        ]);
    }
    if json {
        command.arg("--json");
    }
    command.arg("saved-query").args(args);
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
        command.stdin(Stdio::null()).output().unwrap()
    }
}

fn saved_query_envelope(status: u16, result: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(serde_json::json!({
        "status": status,
        "requestId": "saved-query-request",
        "result": result
    }))
}

fn saved_query_list_envelope(
    result: Value,
    total: i64,
    next_cursor: Option<&str>,
) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "saved-query-list",
        "result": result,
        "limit": 100,
        "totalCount": total,
        "nextCursor": next_cursor
    }))
}

#[tokio::test]
async fn saved_query_all_verbs_use_exact_routes_auth_and_bodies() {
    let server = MockServer::start().await;
    let item_route = format!("{SAVED_QUERIES_PATH}/{SAVED_QUERY_ID}");
    Mock::given(method("POST"))
        .and(path(SAVED_QUERIES_PATH))
        .and(wiremock::matchers::basic_auth(
            "saved-query-key",
            "saved-query-secret",
        ))
        .and(body_json(saved_query_body()))
        .respond_with(saved_query_envelope(201, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    let update_body = serde_json::json!({
        "name": "orders v2",
        "sql": "SELECT count() FROM orders\n",
        "database": "default"
    });
    Mock::given(method("PUT"))
        .and(path(item_route.clone()))
        .and(body_json(update_body.clone()))
        .respond_with(saved_query_envelope(200, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(item_route.clone()))
        .respond_with(saved_query_envelope(200, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(item_route.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "saved-query-delete"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let list_item = serde_json::json!([
        {"id": SAVED_QUERY_ID, "name": "orders", "database": "analytics"}
    ]);
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .and(query_param("cursor", "page /+?"))
        .and(query_param("limit", "2"))
        .respond_with(saved_query_list_envelope(
            list_item.clone(),
            3,
            Some("next /+?"),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let project = tempfile::tempdir().unwrap();
    let create = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &saved_query_create_args(&[]),
        None,
    );
    assert_success(&create);
    assert_eq!(
        serde_json::from_slice::<Value>(&create.stdout).unwrap(),
        saved_query_response()
    );

    let update = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &[
            "update",
            "svc-1",
            SAVED_QUERY_ID,
            "--new-name",
            "orders v2",
            "--sql-file",
            "-",
            "--database",
            "default",
            "--org-id",
            "org-1",
        ],
        Some("SELECT count() FROM orders\n"),
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        saved_query_response()
    );

    let list = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &[
            "list", "svc-1", "--cursor", "page /+?", "--limit", "2", "--org-id", "org-1",
        ],
        None,
    );
    assert_success(&list);
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        serde_json::json!({
            "result": list_item,
            "limit": 100,
            "totalCount": 3,
            "nextCursor": "next /+?"
        })
    );

    let get = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["get", "svc-1", SAVED_QUERY_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        saved_query_response()
    );

    let delete = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["delete", "svc-1", SAVED_QUERY_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&delete);
    assert_eq!(
        serde_json::from_slice::<Value>(&delete.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "saved-query-delete"})
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        5,
        "an ID target must not list or prefetch"
    );
}

#[tokio::test]
async fn saved_query_sql_file_path_is_sent_verbatim() {
    let server = MockServer::start().await;
    let sql = "SELECT *\nFROM orders -- é\n";
    Mock::given(method("POST"))
        .and(path(SAVED_QUERIES_PATH))
        .and(body_json(serde_json::json!({
            "name": "orders",
            "sql": sql,
            "database": "default"
        })))
        .respond_with(saved_query_envelope(201, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let file = project.path().join("orders.sql");
    std::fs::write(&file, sql).unwrap();
    let output = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &[
            "create",
            "svc-1",
            "--name",
            "orders",
            "--sql-file",
            file.to_str().unwrap(),
            "--database",
            "default",
            "--org-id",
            "org-1",
        ],
        None,
    );
    assert_success(&output);
}

#[tokio::test]
async fn saved_query_name_selector_pages_through_list_before_mutating() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .and(query_param("limit", "100"))
        .and(query_param("cursor", "page-2"))
        .respond_with(saved_query_list_envelope(
            serde_json::json!([{"id": SAVED_QUERY_ID, "name": "orders", "database": "default"}]),
            2,
            None,
        ))
        .with_priority(1)
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .and(query_param("limit", "100"))
        .respond_with(saved_query_list_envelope(
            serde_json::json!([{"id": SAVED_QUERY_OTHER_ID, "name": "other", "database": "default"}]),
            2,
            Some("page-2"),
        ))
        .with_priority(10)
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{SAVED_QUERIES_PATH}/{SAVED_QUERY_ID}")))
        .respond_with(saved_query_envelope(200, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{SAVED_QUERIES_PATH}/{SAVED_QUERY_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "saved-query-delete"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let get = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["get", "svc-1", "--name", "orders", "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        saved_query_response()
    );
    let delete = invoke_saved_query(
        &server,
        project.path(),
        false,
        false,
        &["delete", "svc-1", "--name", "orders", "--org-id", "org-1"],
        None,
    );
    assert_success(&delete);
    let stdout = String::from_utf8_lossy(&delete.stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stdout.contains(SAVED_QUERY_ID), "{stdout}");

    // An unknown name fails before any mutation.
    let missing = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["delete", "svc-1", "--name", "absent", "--org-id", "org-1"],
        None,
    );
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("absent"),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );
    let deletes = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|request| request.method == wiremock::http::Method::DELETE)
        .count();
    assert_eq!(deletes, 1);
}

#[tokio::test]
async fn saved_query_human_list_renders_table_and_next_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .respond_with(saved_query_list_envelope(
            serde_json::json!([
                {},
                {"id": SAVED_QUERY_ID, "name": "orders", "database": "analytics"}
            ]),
            5,
            Some("next-page"),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let output = invoke_saved_query(
        &server,
        project.path(),
        false,
        false,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        SAVED_QUERY_ID,
        "orders",
        "analytics",
        "Next cursor: next-page",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    let request = &server.received_requests().await.unwrap()[0];
    assert!(request.url.query().is_none(), "{:?}", request.url.query());
}

const POSTGRES_BACKUPS_PATH: &str = "/v1/organizations/org-1/postgres/pg-1/backups";
const WHOAMI_USER_ID: &str = "44444444-5555-4666-8777-888888888888";
const WHOAMI_ORG_ID: &str = "55555555-6666-4777-8888-999999999999";

/// Run a cloud command with API-key flags, or with saved OAuth tokens only.
fn invoke_cloud_as(
    server: &MockServer,
    project: &Path,
    oauth: bool,
    json: bool,
    args: &[&str],
) -> std::process::Output {
    let home = project.join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    if oauth {
        write_oauth_tokens(&cloud_dir, &server.uri());
    }
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project)
        .args(["cloud", "--url", &server.uri()]);
    if !oauth {
        command.args(["--api-key", "drift-key", "--api-secret", "drift-secret"]);
    }
    if json {
        command.arg("--json");
    }
    command.args(args).stdin(Stdio::null()).output().unwrap()
}

fn postgres_backup_page(result: Value, next_cursor: Option<&str>) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "postgres-backup-list",
        "result": result,
        "limit": 2,
        "totalCount": 3,
        "nextCursor": next_cursor
    }))
}

#[tokio::test]
async fn postgres_backup_list_sends_pagination_and_renders_json_and_table() {
    let server = MockServer::start().await;
    let backups = serde_json::json!([
        {
            "key": "basebackups_005/000000010000000000000002_backup_stop_sentinel.json",
            "lastModified": "2026-03-31T18:17:37Z"
        },
        {}
    ]);
    Mock::given(method("GET"))
        .and(path(POSTGRES_BACKUPS_PATH))
        .and(wiremock::matchers::basic_auth("drift-key", "drift-secret"))
        .and(query_param("cursor", "page /+?"))
        .and(query_param("limit", "2"))
        .respond_with(postgres_backup_page(backups.clone(), Some("next /+?")))
        .with_priority(1)
        .expect(2)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let args = [
        "postgres", "backup", "list", "pg-1", "--cursor", "page /+?", "--limit", "2", "--org-id",
        "org-1",
    ];

    let json = invoke_cloud_as(&server, project.path(), false, true, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!({
            "result": backups,
            "limit": 2,
            "totalCount": 3,
            "nextCursor": "next /+?"
        })
    );

    let human = invoke_cloud_as(&server, project.path(), false, false, &args);
    assert_success(&human);
    let stdout = String::from_utf8_lossy(&human.stdout);
    for expected in [
        "Key",
        "Last modified",
        "basebackups_005/000000010000000000000002_backup_stop_sentinel.json",
        "2026-03-31T18:17:37Z",
        "Next cursor: next /+?",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
}

fn whoami_api_key_identity() -> Value {
    serde_json::json!({
        "actorType": "apiKey",
        "keyId": "key-123",
        "name": "ci deploy",
        "organizationId": WHOAMI_ORG_ID
    })
}

fn whoami_envelope(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "whoami",
        "result": result
    }))
}

#[tokio::test]
async fn auth_whoami_renders_both_identities_without_an_organization() {
    let user = serde_json::json!({
        "actorType": "user",
        "userId": WHOAMI_USER_ID,
        "email": "ada@example.com",
        "name": "Ada Lovelace",
        "organizations": [
            {"organizationId": WHOAMI_ORG_ID, "organizationName": "Analytical Engines"}
        ]
    });
    let api_key = serde_json::json!({
        "actorType": "apiKey",
        "keyId": "key-123",
        "name": "ci deploy",
        "organizationId": WHOAMI_ORG_ID
    });
    let future = serde_json::json!({"actorType": "robot", "robotId": "r2"});

    // (identity, oauth, output in JSON, human lines). JSON and human output
    // alternate: both print the identity unchanged.
    for (identity, oauth, json, human_expected) in [
        (
            user.clone(),
            true,
            true,
            vec![
                "actorType: user",
                WHOAMI_USER_ID,
                "ada@example.com",
                "Ada Lovelace",
                WHOAMI_ORG_ID,
                "Analytical Engines",
            ],
        ),
        (
            api_key.clone(),
            false,
            false,
            vec!["actorType: apiKey", "key-123", "ci deploy", WHOAMI_ORG_ID],
        ),
        (
            future.clone(),
            false,
            true,
            vec!["actorType: robot", "robotId: r2"],
        ),
    ] {
        let server = MockServer::start().await;
        let auth = if oauth {
            header("authorization", "Bearer test-bearer-token")
        } else {
            header(
                "authorization",
                "Basic ZHJpZnQta2V5OmRyaWZ0LXNlY3JldA==", // drift-key:drift-secret
            )
        };
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .and(auth)
            .respond_with(whoami_envelope(identity.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let project = tempfile::tempdir().unwrap();

        let output = invoke_cloud_as(&server, project.path(), oauth, json, &["auth", "whoami"]);
        assert_success(&output);
        if json {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                identity
            );
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for expected in human_expected {
                assert!(stdout.contains(expected), "missing {expected}: {stdout}");
            }
        }

        // No organization lookup or other call: exactly the one whoami request.
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(
            requests
                .iter()
                .all(|request| request.url.path() == "/v1/whoami" && request.url.query().is_none())
        );
    }
}

#[tokio::test]
async fn auth_whoami_auth_failures_exit_4() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "status": 401,
            "requestId": "whoami-401",
            "error": "Invalid credentials"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    // Human rendering of a 401 is `dispatched_cloud_401_exits_with_auth_required`'s.
    let output = invoke_cloud_as(&server, project.path(), false, true, &["auth", "whoami"]);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid credentials"), "{stderr}");
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error"]["code"], "auth_required");

    // With no credentials at all, whoami fails as auth-required before any request.
    let empty = MockServer::start().await;
    let output = invoke_cli_without_cloud_credentials(&empty, &["auth".into(), "whoami".into()]);
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(empty.received_requests().await.unwrap().is_empty());
}

// ── auth login / status verify the identity via whoami (#1044) ────────────

fn whoami_rejected() -> ResponseTemplate {
    ResponseTemplate::new(401).set_body_json(serde_json::json!({
        "status": 401,
        "requestId": "whoami-401",
        "error": "Invalid credentials"
    }))
}

/// A URL nothing listens on, so requests fail before any HTTP response.
fn unreachable_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

fn invoke_auth(project: &Path, url: &str, json: bool, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.join("home"))
        .current_dir(project)
        .args(["cloud", "--url", url, "auth"])
        .args(args);
    if json {
        command.arg("--json");
    }
    command.stdin(Stdio::null()).output().unwrap()
}

const LOGIN_ARGS: &[&str] = &[
    "login",
    "--api-key",
    "new-key",
    "--api-secret",
    "new-secret",
];

#[tokio::test]
async fn api_key_login_verifies_and_prints_the_identity_before_saving() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .and(header(
            "authorization",
            "Basic bmV3LWtleTpuZXctc2VjcmV0", // new-key:new-secret
        ))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .expect(2)
        .mount(&server)
        .await;
    for json in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let output = invoke_auth(project.path(), &server.uri(), json, LOGIN_ARGS);
        assert_success(&output);
        assert!(output.stderr.is_empty(), "json={json}");
        let saved = project.path().join(".clickhouse/credentials.json");
        assert!(saved.exists());
        let stdout = String::from_utf8_lossy(&output.stdout);
        for secret in ["new-key", "new-secret"] {
            assert!(!stdout.contains(secret), "credential exposed: {stdout}");
        }
        if json {
            let report: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(report["verification"], "verified");
            assert_eq!(report["identity"], whoami_api_key_identity());
            assert!(
                report["saved"]
                    .as_str()
                    .unwrap()
                    .ends_with("credentials.json")
            );
            assert!(report.get("warning").is_none());
        } else {
            for expected in [
                ".clickhouse/credentials.json",
                "key-123",
                "ci deploy",
                WHOAMI_ORG_ID,
            ] {
                assert!(stdout.contains(expected), "missing {expected}: {stdout}");
            }
        }
    }
}

#[tokio::test]
async fn api_key_login_rejected_by_whoami_exits_4_and_saves_nothing() {
    // whoami refusing the key with a 403 blocks the save as a 401 does, even
    // though a 403 elsewhere is a request refusal, not auth_required (#1225).
    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .respond_with(
                ResponseTemplate::new(status).set_body_json(serde_json::json!({
                    "status": status,
                    "requestId": "whoami-rejected",
                    "error": "Invalid credentials"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let project = tempfile::tempdir().unwrap();
        let output = invoke_auth(project.path(), &server.uri(), true, LOGIN_ARGS);
        assert_eq!(output.status.code(), Some(4), "status {status}");
        assert!(output.stdout.is_empty());
        assert!(!project.path().join(".clickhouse/credentials.json").exists());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Invalid credentials"), "{stderr}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error"]["code"], "auth_required");
    }
}

#[tokio::test]
async fn api_key_login_saves_with_a_warning_when_whoami_is_unreachable() {
    let url = unreachable_url();
    for json in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let output = invoke_auth(project.path(), &url, json, LOGIN_ARGS);
        assert_success(&output);
        let saved: Value = serde_json::from_slice(
            &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(saved["api_key"], "new-key");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if json {
            assert!(stderr.is_empty(), "{stderr}");
            let report: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(report["verification"], "unverified");
            assert!(report["identity"].is_null());
            assert!(report["warning"].is_string(), "{report}");
        } else {
            assert!(stdout.contains(".clickhouse/credentials.json"), "{stdout}");
            assert!(stderr.starts_with("warning: "), "{stderr}");
        }
    }
}

#[tokio::test]
async fn auth_status_without_credentials_skips_the_identity_check() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let output = invoke_auth(project.path(), &server.uri(), true, &["status"]);
    assert_success(&output);
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["verification"], "skipped");
    assert!(status["identity"].is_null());
    assert_eq!(status["sources"].as_array().unwrap().len(), 4);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_reports_the_identity_and_never_fails_on_it() {
    let user = serde_json::json!({
        "actorType": "user",
        "userId": WHOAMI_USER_ID,
        "email": "ada@example.com",
        "name": "Ada Lovelace",
        "organizations": [
            {"organizationId": WHOAMI_ORG_ID, "organizationName": "Analytical Engines"}
        ]
    });
    // (response, oauth, JSON output, expected verification, expected human line).
    // Each outcome once; the output format alternates.
    for (response, oauth, json, verification, human) in [
        (
            Some(whoami_envelope(whoami_api_key_identity())),
            false,
            true,
            "verified",
            "key-123",
        ),
        (
            Some(whoami_envelope(user.clone())),
            true,
            false,
            "verified",
            "ada@example.com",
        ),
        (
            Some(whoami_rejected()),
            false,
            true,
            "rejected",
            "Identity: rejected (",
        ),
        (None, false, false, "unavailable", "Identity: unavailable ("),
    ] {
        let server = MockServer::start().await;
        let url = match response {
            Some(response) => {
                Mock::given(method("GET"))
                    .and(path("/v1/whoami"))
                    .respond_with(response)
                    .expect(1)
                    .mount(&server)
                    .await;
                server.uri()
            }
            None => unreachable_url(),
        };
        let project = tempfile::tempdir().unwrap();
        if oauth {
            let cloud_dir = project.path().join("home/.clickhouse");
            std::fs::create_dir_all(&cloud_dir).unwrap();
            write_oauth_tokens(&cloud_dir, &url);
        } else {
            write_project_api_credentials(project.path(), "file-key", "file-secret", &url);
        }
        let output = invoke_auth(project.path(), &url, json, &["status"]);
        assert_success(&output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if json {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], verification, "{stdout}");
            assert_eq!(status["identity"].is_null(), verification != "verified");
            assert_eq!(status["warning"].is_string(), verification != "verified");
            let active = status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["active"] == "yes")
                .count();
            assert_eq!(active, 1);
        } else {
            assert!(stdout.contains(human), "missing {human}: {stdout}");
        }
    }
}

// ── UDF directories, native archives and waking (#1030) ─────────────────────

fn udf_dir(parent: &Path, name: &str, runtime: &str) -> std::path::PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("udf.json"),
        serde_json::json!({
            "functionName": name, "type": "executable", "runtime": runtime,
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"
        })
        .to_string(),
    )
    .unwrap();
    if runtime == "native" {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\n").unwrap();
        for arch in ["amd64", "arm64"] {
            std::fs::create_dir_all(dir.join(arch)).unwrap();
            std::fs::write(dir.join(arch).join("main"), "binary\n").unwrap();
        }
        std::fs::write(dir.join("amd64/model.bin"), "weights\n").unwrap();
    } else {
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("main.py"), "import sys\n").unwrap();
        std::fs::write(dir.join("lib/helper.py"), "x = 1\n").unwrap();
        std::fs::write(dir.join(".env"), "SECRET=1\n").unwrap();
    }
    dir
}

fn project_udf(project: &Path, name: &str, runtime: &str) -> std::path::PathBuf {
    udf_dir(&project.join("clickhouse/udfs"), name, runtime)
}

async fn mount_udf_upload_session(server: &MockServer, storage: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
                "uploadId": "fresh-upload",
                "uploadUrl": format!("{}/artifact?signature=upload-secret", storage.uri())
            }})),
        )
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/artifact"))
        .and(header("content-type", "application/zip"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(storage)
        .await;
}

fn zip_entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
    (0..archive.len())
        .map(|index| archive.by_index(index).unwrap().name().unwrap().to_string())
        .collect()
}

fn udf_result(name: &str, version: i64, status: &str) -> ResponseTemplate {
    ResponseTemplate::new(201).set_body_json(serde_json::json!({"result": {
        "functionName": name, "version": version, "status": status
    }}))
}

#[tokio::test]
async fn udf_create_name_packages_the_directory_and_reads_its_udf_json() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    // `--dir` names another parent; `version create` below covers the default.
    let shared = tempfile::tempdir().unwrap();
    udf_dir(&shared.path().join("udfs"), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(body_json(serde_json::json!({
            "functionName": "my_udf", "type": "executable", "runtime": "python3.11",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64",
            "uploadId": "fresh-upload"
        })))
        .respond_with(udf_result("my_udf", 1, "building"))
        .expect(1)
        .mount(&server)
        .await;

    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &[
            "create",
            "my_udf",
            "--dir",
            shared.path().join("udfs").to_str().unwrap(),
        ],
    )
    .output()
    .unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
        "building"
    );
    let upload = storage.received_requests().await.unwrap().pop().unwrap();
    assert!(upload.body.starts_with(b"PK\x03\x04"));
    assert_eq!(
        zip_entry_names(&upload.body),
        ["lib/", "lib/helper.py", "main.py"]
    );
    assert!(!upload.headers.contains_key("authorization"));
}

#[tokio::test]
async fn udf_version_create_name_archives_native_layout_and_drops_the_name() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "native");
    mount_udf_upload_session(&server, &storage).await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(body_json(serde_json::json!({
            "type": "executable", "runtime": "native",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64",
            "uploadId": "fresh-upload"
        })))
        .respond_with(udf_result("my_udf", 2, "building"))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["version", "create", "my_udf"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    assert_eq!(
        zip_entry_names(
            &storage
                .received_requests()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .body
        ),
        [
            "amd64/",
            "amd64/main",
            "amd64/model.bin",
            "arm64/",
            "arm64/main"
        ]
    );
}

// The UDF directory failures that render JSON carry the codes `local udf
// deploy` reports for the same problems, before any request is made.
#[tokio::test]
async fn udf_directory_problems_report_local_udf_codes() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let symlinked = project_udf(project.path(), "symlinked", "python3.11");
    std::os::unix::fs::symlink(symlinked.join("lib/helper.py"), symlinked.join("link.py")).unwrap();
    let not_json = project_udf(project.path(), "not_json", "python3.11");
    std::fs::write(not_json.join("udf.json"), "{ not json").unwrap();
    let no_binary = project_udf(project.path(), "no_binary", "native");
    std::fs::remove_file(no_binary.join("amd64/main")).unwrap();

    // Which problem gets which code is `udf_json_failures_carry_the_local_udf_codes`
    // and `package_source_dir_rejects_*`; one run per command reaches JSON.
    for (args, code) in [
        (vec!["create", "symlinked"], "udf_source_invalid"),
        (
            vec!["version", "create", "not_json"],
            "udf_definition_invalid",
        ),
        (
            vec!["deploy", "no_binary", "--service", "svc-1"],
            "udf_source_invalid",
        ),
    ] {
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let error: Value = serde_json::from_slice(&output.stderr)
            .unwrap_or_else(|_| panic!("{args:?}: {}", String::from_utf8_lossy(&output.stderr)));
        assert_eq!(error["error"]["code"], code, "{args:?}: {error}");
    }

    // The functionName check runs in each handler (version create: its own
    // test); a mismatch is a usage error before anything is uploaded.
    let renamed = project_udf(project.path(), "renamed", "python3.11");
    std::fs::write(
        renamed.join("udf.json"),
        serde_json::json!({
            "functionName": "my_udf", "type": "executable", "runtime": "python3.11",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"
        })
        .to_string(),
    )
    .unwrap();
    for args in [
        vec!["create", "renamed"],
        vec!["deploy", "renamed", "--service", "svc-1"],
    ] {
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn udf_attach_reports_idle_services_and_wakes_them_with_wake() {
    let idle_424 = ResponseTemplate::new(424).set_body_json(serde_json::json!({
        "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
        "canWake": true, "status": 424
    }));

    // Without --wake: the typed reason and the remedy are in the message, and
    // no wake request is sent.
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(idle_424.clone())
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("SERVICE_IDLE"), "{message}");
    assert!(message.contains("--wake"), "{message}");
    assert_eq!(error["error"]["code"], "service_idle", "{error}");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl cloud service wake svc-1"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // With --wake: 424 → PATCH awake → service running → attach accepted.
    // The command returns the accepted attachment without polling it.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(idle_424)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .and(body_json(serde_json::json!({"command": "awake"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "awaking"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "running"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/attachments/svc-1"))
        .and(body_json(serde_json::json!({"version": 2})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "my_udf", "serviceId": "svc-1", "version": 2, "status": "provisioning"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1", "--version", "2", "--wake"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    let attachment: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(attachment["status"], "provisioning");
    assert_eq!(attachment["version"], 2);
    let calls: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect();
    assert_eq!(
        calls,
        [
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            "PATCH /v1/organizations/org-1/services/svc-1/state",
            "GET /v1/organizations/org-1/services/svc-1",
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ]
    );
}

// A stopped or starting service gets its own JSON code, derived from the
// typed 424 payload, and exit 1; so does a --wake that finds the service
// stopped while waiting for it.
#[tokio::test]
async fn udf_attach_reports_stopped_and_not_running_services_with_their_codes() {
    for (body, code) in [
        (
            serde_json::json!({
                "error": "service is stopped", "code": "SERVICE_STOPPED",
                "serviceState": "stopped", "canWake": false, "status": 424
            }),
            "service_stopped",
        ),
        (
            serde_json::json!({
                "error": "service is not running", "code": "SERVICE_NOT_RUNNING",
                "serviceState": "starting", "canWake": false, "status": 424
            }),
            "service_not_running",
        ),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method("PUT"))
            .and(path(
                "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            ))
            .respond_with(ResponseTemplate::new(424).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            true,
            &["attach", "my_udf", "svc-1"],
        )
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1), "{code}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code, "{error}");
    }

    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
            "canWake": true, "status": 424
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "awaking"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "stopped"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1", "--wake"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "service_stopped", "{error}");
}

// A failed wake PATCH is not fatal while the service is on its way up
// (here `provisioning`): the CLI waits for `running` and retries the attach.
#[tokio::test]
async fn udf_attach_wake_waits_out_a_failed_wake_while_the_service_comes_up() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
            "canWake": true, "status": 424
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "status": 409, "error": "service state is changing"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "provisioning"}})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "running"}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "my_udf", "serviceId": "svc-1", "version": 1, "status": "provisioning"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1", "--wake"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    let calls: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect();
    assert_eq!(
        calls,
        [
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            "PATCH /v1/organizations/org-1/services/svc-1/state",
            "GET /v1/organizations/org-1/services/svc-1",
            "GET /v1/organizations/org-1/services/svc-1",
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ]
    );
}

/// The service `udf deploy` checks before it uploads anything.
async fn mount_deploy_service(server: &MockServer, service: &str, state: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/organizations/org-1/services/{service}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": state}})),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn udf_deploy_creates_or_versions_then_attaches_the_built_version() {
    for existing in [false, true] {
        let server = MockServer::start().await;
        let storage = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        let source = project_udf(project.path(), "my_udf", "python3.11");
        if existing {
            // Cloud no longer accepts sandboxVersion: deploy warns once and
            // omits it (the publish mocks below match the body exactly).
            let file = source.join("udf.json");
            let mut definition: Value =
                serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            definition["sandboxVersion"] = serde_json::json!("v3");
            std::fs::write(&file, definition.to_string()).unwrap();
        }
        mount_udf_upload_session(&server, &storage).await;
        mount_deploy_service(&server, "svc-1", "running").await;
        let new_version = if existing { 2 } else { 1 };
        let udf_body = |version: i64, status: &str| {
            serde_json::json!({"result": {
                "functionName": "my_udf", "version": version, "status": status, "runtime": "python3.11"
            }})
        };

        // The first GET decides between create and version create; later
        // GETs follow the build of the new version.
        let first_get = if existing {
            ResponseTemplate::new(200).set_body_json(udf_body(1, "ready"))
        } else {
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"}))
        };
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/udfs/my_udf"))
            .respond_with(first_get)
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/udfs/my_udf"))
            .respond_with(ResponseTemplate::new(200).set_body_json(udf_body(
                if existing {
                    new_version + 1
                } else {
                    new_version
                },
                "ready",
            )))
            .mount(&server)
            .await;
        if existing {
            // A concurrent publisher made GET point at version 3. The wait
            // must find the created version 2 on a subsequent list page.
            for (cursor, result, next) in [
                (
                    None,
                    serde_json::json!([{"functionName":"my_udf","version":3,"status":"ready"}]),
                    Some("page /+?"),
                ),
                (
                    Some("page /+?"),
                    serde_json::json!([{"functionName":"my_udf","version":2,"status":"ready"}]),
                    None,
                ),
            ] {
                let mock = Mock::given(method("GET"))
                    .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
                    .and(query_param("limit", "100"));
                let mock = if let Some(cursor) = cursor {
                    mock.and(query_param("cursor", cursor))
                } else {
                    mock.and(wiremock::matchers::query_param_is_missing("cursor"))
                };
                mock.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result":result,"limit":100,"totalCount":null,"nextCursor":next
                })))
                .expect(1)
                .mount(&server)
                .await;
            }
        }
        let definition = serde_json::json!({
            "type": "executable", "runtime": "python3.11",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64",
            "uploadId": "fresh-upload"
        });
        if existing {
            Mock::given(method("POST"))
                .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
                .and(body_json(definition))
                .respond_with(ResponseTemplate::new(201).set_body_json(udf_body(2, "building")))
                .expect(1)
                .mount(&server)
                .await;
        } else {
            let mut create = definition;
            create["functionName"] = serde_json::json!("my_udf");
            Mock::given(method("POST"))
                .and(path("/v1/organizations/org-1/udfs"))
                .and(body_json(create))
                .respond_with(ResponseTemplate::new(201).set_body_json(udf_body(1, "building")))
                .expect(1)
                .mount(&server)
                .await;
        }
        Mock::given(method("PUT"))
            .and(path("/v1/organizations/org-1/udfs/my_udf/attachments/svc-1"))
            .and(body_json(serde_json::json!({"version": new_version})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
                "functionName": "my_udf", "serviceId": "svc-1", "version": new_version, "status": "provisioning"
            }})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/udfs/my_udf/attachments/svc-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
                "functionName": "my_udf", "serviceId": "svc-1", "version": new_version, "status": "deployed"
            }})))
            .expect(1)
            .mount(&server)
            .await;

        let json = !existing;
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            json,
            &["deploy", "my_udf", "--service", "svc-1", "--timeout", "30"],
        )
        .output()
        .unwrap();
        assert_success(&output);
        let warnings = String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| line.starts_with("Warning:") && line.contains("sandboxVersion"))
            .count();
        assert_eq!(warnings, usize::from(existing));
        let stdout = String::from_utf8_lossy(&output.stdout);
        if json {
            let result: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(result["action"], "created");
            assert_eq!(result["udf"]["version"], 1);
            assert_eq!(result["udf"]["status"], "ready");
            assert_eq!(result["attachment"]["status"], "deployed");
            assert_eq!(result["attachment"]["version"], 1);
        } else {
            // One result line naming the UDF, version and service.
            assert_eq!(stdout.lines().count(), 1, "{stdout}");
            for identifier in ["my_udf", "2", "svc-1"] {
                assert!(stdout.contains(identifier), "{identifier}: {stdout}");
            }
            // Two waits: the build (carrying `--timeout 30`, not the shipped
            // default), then the deployment to the service.
            let stderr = String::from_utf8_lossy(&output.stderr);
            let waits: Vec<&str> = stderr
                .lines()
                .filter(|line| !line.starts_with("Warning: "))
                .collect();
            assert_eq!(waits.len(), 2, "{stderr}");
            assert!(
                waits[0].contains("my_udf") && waits[0].contains("30s"),
                "{stderr}"
            );
            assert!(!waits[0].contains("svc-1"), "{stderr}");
            assert!(waits[1].contains("svc-1"), "{stderr}");
        }
        let calls: Vec<String> = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| format!("{} {}", request.method, request.url.path()))
            .collect();
        let publish = if existing {
            "POST /v1/organizations/org-1/udfs/my_udf/versions"
        } else {
            "POST /v1/organizations/org-1/udfs"
        };
        let mut expected_calls = vec![
            "GET /v1/organizations/org-1/services/svc-1",
            "GET /v1/organizations/org-1/udfs/my_udf",
            "POST /v1/organizations/org-1/udfUploads/url",
            publish,
            "GET /v1/organizations/org-1/udfs/my_udf",
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            "GET /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ];
        if existing {
            expected_calls.splice(
                5..5,
                [
                    "GET /v1/organizations/org-1/udfs/my_udf/versions",
                    "GET /v1/organizations/org-1/udfs/my_udf/versions",
                ],
            );
        }
        assert_eq!(calls, expected_calls, "existing={existing}");
        assert_eq!(
            zip_entry_names(
                &storage
                    .received_requests()
                    .await
                    .unwrap()
                    .pop()
                    .unwrap()
                    .body
            ),
            ["lib/", "lib/helper.py", "main.py"]
        );
    }
}

#[tokio::test]
async fn udf_deploy_native_archives_only_the_architecture_directories() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "native_udf", "native");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    let ready = serde_json::json!({"result": {"functionName": "native_udf", "version": 1, "status": "ready"}});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/native_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/native_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ready.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(ready))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/udfs/native_udf/attachments/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "native_udf", "serviceId": "svc-1", "version": 1, "status": "deployed"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/native_udf/attachments/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "native_udf", "serviceId": "svc-1", "version": 1, "status": "deployed"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "native_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    assert_eq!(
        zip_entry_names(
            &storage
                .received_requests()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .body
        ),
        [
            "amd64/",
            "amd64/main",
            "amd64/model.bin",
            "arm64/",
            "arm64/main"
        ]
    );
}

#[tokio::test]
async fn udf_deploy_failures_after_publish_name_the_built_version() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let ready =
        serde_json::json!({"result": {"functionName": "my_udf", "version": 1, "status": "ready"}});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ready.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(ready))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is stopped", "code": "SERVICE_STOPPED", "serviceState": "stopped",
            "canWake": false, "status": 424
        })))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("SERVICE_STOPPED"), "{message}");
    assert!(message.contains("cloud service start svc-1"), "{message}");
    assert!(
        message.contains("`clickhousectl cloud udf attach my_udf svc-1 --version 1`"),
        "{message}"
    );
}

/// A fresh `my_udf` that `udf deploy` creates as version 1, its build
/// reporting `status` on every poll.
async fn mount_deploy_new_udf(server: &MockServer, status: &str) {
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(server)
        .await;
    let built = serde_json::json!({"result": {
        "functionName": "my_udf", "version": 1, "status": status
    }});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(built.clone()))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(built))
        .expect(1)
        .mount(server)
        .await;
}

// The service idled during the build and --no-wake kept the attach from
// waking it: deploy has no --wake to rerun with, and rerunning would build
// again, so the way out is the attach of the built version, with --wake.
#[tokio::test]
async fn udf_deploy_idle_at_attach_resumes_with_a_waking_attach() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    mount_deploy_new_udf(&server, "ready").await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
            "canWake": true, "status": 424
        })))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1", "--no-wake"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "service_idle", "{error}");
    assert_eq!(
        error["error"]["command"], "clickhousectl cloud udf attach my_udf svc-1 --version 1 --wake",
        "{error}"
    );
    let message = error["error"]["message"].as_str().unwrap();
    assert!(!message.contains("cloud service wake"), "{message}");
    assert!(
        message.contains("`clickhousectl cloud udf attach my_udf svc-1 --version 1 --wake`"),
        "{message}"
    );
    assert!(
        !deploy_calls(&server)
            .await
            .iter()
            .any(|call| call.starts_with("PATCH ")),
        "--no-wake must not wake the service"
    );
}

// A build still running when the wait gives up can finish later; the error
// names the attach that resumes the deploy once it does.
// A build that reported an error has nothing to resume.
#[tokio::test]
async fn udf_deploy_failed_build_offers_no_resume() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    mount_deploy_new_udf(&server, "error").await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("failed"), "{message}");
    assert!(!message.contains("cloud udf attach"), "{message}");
    assert!(
        !deploy_calls(&server)
            .await
            .iter()
            .any(|call| call.starts_with("PUT ")),
        "nothing is attached before the build is ready"
    );
}

/// Paths `udf deploy` requested, in order.
async fn deploy_calls(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect()
}

// The service is checked before anything is uploaded: a bad ID must not
// leave an orphan UDF version behind.
#[tokio::test]
async fn udf_deploy_rejects_an_unknown_service_before_publishing() {
    let org = "00000000-0000-4000-8000-000000000001";
    let service = "11111111-2222-3333-4444-555555555555";
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    Mock::given(method("GET"))
        .and(path(format!("/v1/organizations/{org}/services/{service}")))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400, "error": "not a valid UUID"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let home = project.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut cmd = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut cmd);
    let output = cmd
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project.path())
        .args(["cloud", "--url", &server.uri()])
        .args([
            "--api-key",
            "udf-key",
            "--api-secret",
            "udf-secret",
            "--json",
        ])
        .args([
            "udf",
            "--org-id",
            org,
            "deploy",
            "my_udf",
            "--service",
            service,
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "resource_not_found", "{error}");
    assert_eq!(
        deploy_calls(&server).await,
        [format!("GET /v1/organizations/{org}/services/{service}")]
    );
}

// `--no-wake` on an idle service, or a service in a state a wake cannot
// fix, fails before the upload with the command that unblocks it.
#[tokio::test]
async fn udf_deploy_rejects_an_unusable_service_before_publishing() {
    for (state, no_wake, code, command) in [
        (
            "idle",
            true,
            "service_idle",
            Some("clickhousectl cloud service wake svc-1"),
        ),
        (
            "stopped",
            false,
            "service_stopped",
            Some("clickhousectl cloud service start svc-1"),
        ),
        (
            "stopped",
            true,
            "service_stopped",
            Some("clickhousectl cloud service start svc-1"),
        ),
        ("terminated", false, "service_not_running", None),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        project_udf(project.path(), "my_udf", "python3.11");
        mount_deploy_service(&server, "svc-1", state).await;
        let mut args = vec!["deploy", "my_udf", "--service", "svc-1"];
        if no_wake {
            args.push("--no-wake");
        }
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{state}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code, "{error}");
        assert_eq!(error["error"]["command"].as_str(), command, "{error}");
        assert_eq!(
            deploy_calls(&server).await,
            ["GET /v1/organizations/org-1/services/svc-1"],
            "{state}"
        );
    }
}

// Without --no-wake an idle service is not woken up front: the build takes
// minutes and the service could idle again, so the attach wakes it.
#[tokio::test]
async fn udf_deploy_proceeds_on_an_idle_service_without_waking_it_early() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "idle").await;
    let ready =
        serde_json::json!({"result": {"functionName": "my_udf", "version": 1, "status": "ready"}});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ready.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(ready))
        .expect(1)
        .mount(&server)
        .await;
    let attached = serde_json::json!({"result": {
        "functionName": "my_udf", "serviceId": "svc-1", "version": 1, "status": "deployed"
    }});
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(attached.clone()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(attached))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    let calls = deploy_calls(&server).await;
    assert_eq!(calls[0], "GET /v1/organizations/org-1/services/svc-1");
    assert!(
        !calls.iter().any(|call| call.starts_with("PATCH ")),
        "{calls:?}"
    );
}

const REPLICA_AWARE_ROUTING_PATH: &str =
    "/v1/organizations/org-1/services/svc-1/replicaAwareRouting";

#[tokio::test]
async fn replica_aware_routing_get_supports_oauth_and_human_output() {
    let mock = MockServer::start().await;
    let routing = serde_json::json!({"httpEnabled": true, "nativeEnabled": false});
    Mock::given(method("GET"))
        .and(path(REPLICA_AWARE_ROUTING_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": routing})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(REPLICA_AWARE_ROUTING_PATH))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"nativeEnabled": true}})),
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
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "service",
            "replica-aware-routing",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        routing
    );

    // A sparse response renders without inventing the missing protocol.
    let human = invoke_cli_with_cloud_credentials_human(
        &mock,
        &[
            "service",
            "replica-aware-routing",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(stdout.contains("nativeEnabled: true"), "{stdout}");
    assert!(!stdout.contains("httpEnabled: true"), "{stdout}");
}

#[tokio::test]
async fn replica_aware_routing_update_sends_each_flag_to_its_field_with_basic_auth() {
    // Omitting an unsupplied field is the builder's rule; see
    // `build_replica_aware_routing_request_supports_minimal_and_maximal_values`.
    // Opposite values here catch the two flags being swapped on the way in.
    let body = serde_json::json!({"httpEnabled": false, "nativeEnabled": true});
    let mock = MockServer::start().await;
    let result = serde_json::json!({"httpEnabled": false, "nativeEnabled": true});
    Mock::given(method("PATCH"))
        .and(path(REPLICA_AWARE_ROUTING_PATH))
        .and(wiremock::matchers::basic_auth(
            "fake-key-for-tests",
            "fake-secret-for-tests",
        ))
        .and(body_json(&body))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let mut args = vec![
        "service",
        "replica-aware-routing",
        "update",
        "svc-1",
        "--org-id",
        "org-1",
    ];
    args.extend(["--http-enabled", "false", "--native-enabled", "true"]);
    let output = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        result
    );
}

const PAGED_ORG_FIRST: &str = "00000000-0000-0000-0000-0000000000a1";
const PAGED_ORG_SECOND: &str = "00000000-0000-0000-0000-0000000000a2";

async fn mount_cursor_page(
    mock: &MockServer,
    route: &str,
    cursor: Option<&str>,
    body: Value,
    expected: u64,
) {
    let matcher = Mock::given(method("GET")).and(path(route));
    let matcher = match cursor {
        Some(cursor) => matcher.and(query_param("cursor", cursor)),
        None => matcher.and(wiremock::matchers::query_param_is_missing("cursor")),
    };
    matcher
        .and(wiremock::matchers::query_param_is_missing("limit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(expected)
        .mount(mock)
        .await;
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
async fn ambiguous_org_name_points_to_org_id_with_every_match() {
    // Organizations can share a name and have no positional ID, so the
    // runtime error (exit 1) points to --org-id and lists both IDs (#1200).
    let mock = MockServer::start().await;
    mount_cursor_page(
        &mock,
        "/v1/organizations",
        None,
        serde_json::json!({"result": [
            {"id": PAGED_ORG_FIRST, "name": "shared"},
            {"id": PAGED_ORG_SECOND, "name": "shared"}
        ]}),
        1,
    )
    .await;

    let output =
        invoke_cli_with_cloud_credentials(&mock, &["service", "list", "--org-name", "shared"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let message = cloud_runtime_error(&output)["message"]
        .as_str()
        .unwrap()
        .to_owned();
    for token in ["--org-id", PAGED_ORG_FIRST, PAGED_ORG_SECOND] {
        assert!(message.contains(token), "missing {token}: {message}");
    }
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
}

#[tokio::test]
async fn snapshot_list_reads_every_page_in_json_and_human_output() {
    const SNAPSHOTS_PATH: &str = "/v1/organizations/org-1/services/svc-1/snapshots";
    let mock = MockServer::start().await;
    let first = serde_json::json!({"id": "00000000-0000-0000-0000-0000000000b1", "status": "done"});
    let second =
        serde_json::json!({"id": "00000000-0000-0000-0000-0000000000b2", "status": "done"});
    mount_cursor_page(
        &mock,
        SNAPSHOTS_PATH,
        None,
        serde_json::json!({"result": [first], "nextCursor": "snap-page-2", "limit": 1}),
        2,
    )
    .await;
    mount_cursor_page(
        &mock,
        SNAPSHOTS_PATH,
        Some("snap-page-2"),
        serde_json::json!({"result": [second]}),
        2,
    )
    .await;
    let args = ["service", "snapshot", "list", "svc-1", "--org-id", "org-1"];
    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!([first, second])
    );
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(
        stdout.contains("00000000-0000-0000-0000-0000000000b1"),
        "{stdout}"
    );
    assert!(
        stdout.contains("00000000-0000-0000-0000-0000000000b2"),
        "{stdout}"
    );
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

// ── Cloud service .env output (issue #1164) ────────────────────────────────

const DOTENV_SERVICE_ID: &str = "22222222-3333-4444-5555-666666666666";

fn dotenv_service_json() -> Value {
    serde_json::json!({
        "id": DOTENV_SERVICE_ID,
        "name": "analytics",
        "endpoints": [
            {"protocol": "https", "host": "abc.us-east-1.aws.clickhouse.cloud", "port": 8443},
            {"protocol": "nativesecure", "host": "abc.us-east-1.aws.clickhouse.cloud", "port": 9440}
        ]
    })
}

fn invoke_cli_in_dir(
    mock: &MockServer,
    dir: &Path,
    json: bool,
    cli_args: &[&str],
) -> std::process::Output {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url];
    if json {
        args.push("--json");
    }
    args.extend(cli_args);
    Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

async fn mount_dotenv_service_get(mock: &MockServer, service: Value) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{DOTENV_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": service, "status": 200, "requestId": "stub-dotenv-get"
        })))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn service_dotenv_edits_env_in_place_without_a_password() {
    let mock = MockServer::start().await;
    mount_dotenv_service_get(&mock, dotenv_service_json()).await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "APP=1\nCLICKHOUSE_PASSWORD=kept\nCLICKHOUSE_HOST=old\n",
    )
    .unwrap();

    let output = invoke_cli_in_dir(
        &mock,
        dir.path(),
        true,
        &["service", "dotenv", DOTENV_SERVICE_ID, "--org-id", "org-1"],
    );
    assert_success(&output);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".env")).unwrap(),
        "APP=1\nCLICKHOUSE_PASSWORD=kept\nCLICKHOUSE_HOST=abc.us-east-1.aws.clickhouse.cloud\n\
         CLICKHOUSE_PORT=9440\nCLICKHOUSE_HTTP_PORT=8443\nCLICKHOUSE_USER=default\n\
         CLICKHOUSE_SECURE=true\n"
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["file"], ".env");
    assert_eq!(result["service"], "analytics");
    let keys: Vec<&str> = result["vars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|var| var["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "CLICKHOUSE_HOST",
            "CLICKHOUSE_PORT",
            "CLICKHOUSE_HTTP_PORT",
            "CLICKHOUSE_USER",
            "CLICKHOUSE_SECURE"
        ]
    );

    let local = invoke_cli_in_dir(
        &mock,
        dir.path(),
        false,
        &[
            "service",
            "dotenv",
            DOTENV_SERVICE_ID,
            "--org-id",
            "org-1",
            "--local",
        ],
    );
    assert_success(&local);
    let written = std::fs::read_to_string(dir.path().join(".env.local")).unwrap();
    assert!(written.starts_with("CLICKHOUSE_HOST=abc.us-east-1.aws.clickhouse.cloud\n"));
    assert!(!written.contains("CLICKHOUSE_PASSWORD"));
}

#[tokio::test]
async fn service_dotenv_without_endpoints_fails_and_writes_nothing() {
    let mock = MockServer::start().await;
    mount_dotenv_service_get(
        &mock,
        serde_json::json!({"id": DOTENV_SERVICE_ID, "name": "analytics"}),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let output = invoke_cli_in_dir(
        &mock,
        dir.path(),
        true,
        &["service", "dotenv", DOTENV_SERVICE_ID, "--org-id", "org-1"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!dir.path().join(".env").exists());
}

async fn mount_dotenv_service_create(mock: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"service": dotenv_service_json(), "password": "one-time-secret"},
            "status": 200,
            "requestId": "stub-dotenv-create"
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn service_create_dotenv_writes_the_password_and_keeps_it_off_stdout() {
    for json in [true, false] {
        let mock = MockServer::start().await;
        mount_dotenv_service_create(&mock).await;
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("app")).unwrap();
        let target = dir.path().join("app").join(".env");

        let output = invoke_cli_in_dir(
            &mock,
            dir.path(),
            json,
            &[
                "service",
                "create",
                "--name",
                "analytics",
                "--org-id",
                "org-1",
                "--dotenv",
                target.to_str().unwrap(),
            ],
        );
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains("one-time-secret"), "json={json}: {stdout}");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("one-time-secret"));
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "CLICKHOUSE_HOST=abc.us-east-1.aws.clickhouse.cloud\nCLICKHOUSE_PORT=9440\n\
             CLICKHOUSE_HTTP_PORT=8443\nCLICKHOUSE_USER=default\nCLICKHOUSE_SECURE=true\n\
             CLICKHOUSE_PASSWORD=one-time-secret\n"
        );
        if json {
            let result: Value = serde_json::from_str(&stdout).unwrap();
            assert!(result.get("password").is_none());
            assert_eq!(result["service"]["id"], DOTENV_SERVICE_ID);
            assert_eq!(result["dotenv"]["file"], target.to_str().unwrap());
            assert_eq!(result["dotenv"]["keys"][5], "CLICKHOUSE_PASSWORD");
        }
    }
}

#[tokio::test]
async fn service_create_without_dotenv_still_prints_the_password() {
    let mock = MockServer::start().await;
    mount_dotenv_service_create(&mock).await;
    let dir = tempfile::tempdir().unwrap();
    let output = invoke_cli_in_dir(
        &mock,
        dir.path(),
        true,
        &[
            "service",
            "create",
            "--name",
            "analytics",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["password"], "one-time-secret");
    assert!(result.get("dotenv").is_none());
    assert!(!dir.path().join(".env").exists());
}

#[tokio::test]
async fn service_create_dotenv_rejects_an_unwritable_target_before_creating() {
    let mock = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let missing_dir = dir.path().join("missing").join(".env");
    for target in [dir.path().to_str().unwrap(), missing_dir.to_str().unwrap()] {
        let output = invoke_cli_in_dir(
            &mock,
            dir.path(),
            true,
            &[
                "service",
                "create",
                "--name",
                "analytics",
                "--org-id",
                "org-1",
                "--dotenv",
                target,
            ],
        );
        assert_eq!(output.status.code(), Some(2), "{target}");
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}
