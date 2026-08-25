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
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
            && stderr.contains(".clickhouse/credentials.json")
    );
    assert_eq!(std::fs::read(credentials_path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn api_key_login_preserves_unreadable_credentials() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    let credentials_path = credentials_dir.join("credentials.json");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    let original = br#"{"api_key":"old-key","api_secret":"old-secret"}"#;
    std::fs::write(&credentials_path, original).unwrap();
    std::fs::set_permissions(&credentials_path, std::fs::Permissions::from_mode(0o000)).unwrap();

    let output = invoke_api_key_login(dir.path());

    std::fs::set_permissions(&credentials_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Error: failed to read ")
            && stderr.contains(".clickhouse/credentials.json")
    );
    assert_eq!(std::fs::read(credentials_path).unwrap(), original);
}

#[test]
fn api_key_login_initializes_missing_credentials() {
    let dir = tempfile::tempdir().unwrap();

    let output = invoke_api_key_login(dir.path());

    assert_success(&output);
    let credentials: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(credentials["api_key"], "new-key");
    assert_eq!(credentials["api_secret"], "new-secret");
}

#[test]
fn logout_does_not_report_success_when_credentials_removal_fails() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(&credentials_dir).unwrap();
    std::fs::create_dir(credentials_dir.join("credentials.json")).unwrap();

    let output = Command::new(clickhousectl_binary())
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

#[tokio::test]
async fn query_endpoint_create_sends_typed_roles() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "endpoint-1",
                "roles": ["sql_console_read_only", "sql_console_admin"]
            },
            "status": 200,
            "requestId": "stub-query-endpoint-create"
        })))
        .mount(&mock)
        .await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "service",
            "query-endpoint",
            "create",
            "svc-1",
            "--role",
            "sql_console_read_only",
            "--role",
            "sql_console_admin",
            "--open-api-key",
            "key-1",
            "--org-id",
            "org-1",
        ],
    )
    .await;

    assert_eq!(
        body,
        serde_json::json!({
            "roles": ["sql_console_read_only", "sql_console_admin"],
            "openApiKeys": ["key-1"],
            "allowedOrigins": "*"
        })
    );
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

const DELETE_TEST_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";
const DELETE_TEST_API_KEY_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

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

fn invoke_service_delete(
    mock: &MockServer,
    project_dir: &Path,
    force: bool,
) -> std::process::Output {
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
        .output()
        .expect("failed to spawn clickhousectl")
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
    credentials_lock.unlock().unwrap();
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
async fn service_query_key_removal_preserves_credentials_corrupted_while_waiting_for_lock() {
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

    let corrupted = b"{corrupted while locked\n";
    std::fs::write(&credentials_path, corrupted).unwrap();
    credentials_lock.unlock().unwrap();
    drop(credentials_lock);

    let output = tokio::task::spawn_blocking(move || child.wait_with_output().unwrap())
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Error: failed to parse ")
            && stderr.contains(".clickhouse/credentials.json")
    );
    assert_eq!(std::fs::read(credentials_path).unwrap(), corrupted);
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
async fn forced_service_delete_treats_an_absent_key_and_service_as_success() {
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
            "/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "NOT_FOUND",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {},
            "status": 200,
            "requestId": "stub-org-get",
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
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "null\n");

    let requests = mock.received_requests().await.unwrap();
    let request_shape = requests
        .iter()
        .map(|request| {
            (
                request.method.as_str().to_string(),
                request.url.path().to_string(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        request_shape,
        vec![
            (
                "GET".to_string(),
                format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
            ),
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
            ),
            ("GET".to_string(), "/v1/organizations/org-1".to_string()),
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/keys/{DELETE_TEST_API_KEY_ID}")
            ),
        ]
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
             {DELETE_TEST_SERVICE_ID}: cleanup failed\n"
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
        .respond_with(not_found.clone())
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1"))
        .respond_with(not_found)
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: NOT_FOUND: request scoped to organization org-1\n"
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{DELETE_TEST_SERVICE_ID}")
    );
    assert_eq!(requests[1].url.path(), "/v1/organizations/org-1");
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

fn invoke_org_usage(
    mock: &MockServer,
    json: bool,
    ambient_env: &[(&str, &str)],
) -> std::process::Output {
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
    command.envs(ambient_env.iter().copied());
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
    let output = invoke_org_usage(&mock, false, &[]);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("production"));
    assert!(stdout.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee (unknown)"));
}

#[tokio::test]
async fn org_usage_human_output_ignores_ambient_agent_markers() {
    let mock = start_mock_usage_entities_api().await;
    let expected = invoke_org_usage(&mock, false, &[]);
    let output = invoke_org_usage(
        &mock,
        false,
        &[
            ("CLAUDECODE", "1"),
            ("CLAUDE_CODE_ENTRYPOINT", "cli"),
            ("CODEX_THREAD_ID", "test-thread"),
            ("GEMINI_CLI", "1"),
        ],
    );
    assert_success(&expected);
    assert_success(&output);

    assert_eq!(output.stdout, expected.stdout);
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee (unknown)")
    );
}

#[tokio::test]
async fn org_usage_json_keeps_uuid_only_entities_faithful_to_the_api() {
    let mock = start_mock_usage_entities_api().await;
    let output = invoke_org_usage(&mock, true, &[]);
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
    assert_eq!(
        dest["database"], "default",
        "database should default to 'default' for postgres CDC, got {dest}"
    );
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into destination body — Al's Bug 2 regression: {dest}",
        );
    }
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
    assert_eq!(dest["database"], "default");
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into MySQL destination body: {dest}",
        );
    }
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
    assert_eq!(dest["database"], "default");
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into Mongo destination body: {dest}",
        );
    }
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
    for field in ["iamRole", "tlsHost", "caCertificate"] {
        assert!(
            pg.get(field).is_none(),
            "{field} leaked into postgres source body: {pg}",
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
}

// ── Kafka ──────────────────────────────────────────────────────────────────
//
// Kafka has the largest optional-flag surface; cover the high-traffic ones
// plus all 4 SASL credential shapes (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512,
// MUTUAL_TLS, IAM_ROLE).

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
        "--auth",
        "PLAIN",
        "--username",
        "u",
        "--password",
        "p",
        "--org-id",
        "org",
    ]
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
// BigQuery has fewer optional flags than other sources, but still falls into
// the "database pipe" bucket — destination MUST omit table/columns/etc.

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
    assert_eq!(dest["database"], "default");
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into BigQuery destination body: {dest}",
        );
    }
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
    let mut args = postgres_args_minimal();
    args.push("--iam-role".into());
    args.push("arn:aws:iam::123:role/x".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["iamRole"], "arn:aws:iam::123:role/x",
        "iamRole should round-trip the user-provided value"
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

#[tokio::test]
async fn service_query_requires_exactly_one_selector_before_network_access() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;
    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    std::fs::create_dir_all(&home_dir).unwrap();
    let url = control.uri();

    let invoke = |selector: &[&str], authenticated: bool| {
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
        args.extend_from_slice(selector);

        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home_dir)
            .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
            .current_dir(dir.path())
            .args(args);
        if authenticated {
            command
                .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
                .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests");
        } else {
            command
                .env_remove("CLICKHOUSE_CLOUD_API_KEY")
                .env_remove("CLICKHOUSE_CLOUD_API_SECRET");
        }
        command.output().expect("failed to spawn clickhousectl")
    };

    let missing = invoke(&[], false);
    assert_eq!(missing.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("--name <NAME>"), "{stderr}");
    assert!(stderr.contains("--id <ID>"), "{stderr}");

    let both = invoke(&["--name", "demo", "--id", QUERY_TEST_SERVICE_ID], false);
    assert_eq!(both.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&both.stderr);
    assert!(stderr.contains("--name <NAME>"), "{stderr}");
    assert!(stderr.contains("--id <ID>"), "{stderr}");

    assert!(control.received_requests().await.unwrap().is_empty());
    assert!(query_host.received_requests().await.unwrap().is_empty());

    for selector in [["--name", "demo"], ["--id", QUERY_TEST_SERVICE_ID]] {
        let output = invoke(&selector, true);
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
async fn service_query_rejects_conflicting_sql_sources_before_network_access() {
    let control = MockServer::start().await;
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &control.uri(),
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--query",
            "SELECT 1",
            "--queries-file",
            "/definitely/not/present.sql",
        ])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--query"), "{stderr}");
    assert!(stderr.contains("--queries-file"), "{stderr}");
    assert!(control.received_requests().await.unwrap().is_empty());
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
        .output()
        .expect("failed to spawn clickhousectl");

    (output, query_host)
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
        let dir = tempfile::tempdir().unwrap();
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        let output = command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .env("CLICKHOUSE_CLOUD_API_KEY", "unused-key")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "unused-secret")
            .current_dir(dir.path())
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

async fn provision_while_project_auth_changes(auth_args: &[&str]) -> tempfile::TempDir {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": { "id": QUERY_TEST_KEY_UUID },
                "keyId": "provisioned-key-id",
                "keySecret": "provisioned-key-secret",
            },
            "status": 200,
            "requestId": "stub-key-create",
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
            "requestId": "stub-endpoint-get",
        })))
        .expect(1)
        .mount(&control)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    std::fs::create_dir_all(&home_dir).unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join(".gitignore"), "*\n").unwrap();
    std::fs::write(
        credentials_dir.join("credentials.json"),
        serde_json::to_vec(&serde_json::json!({
            "api_key": "fake-key-for-tests",
            "api_secret": "fake-secret-for-tests",
            "service_query_keys": {
                "existing-service": {
                    "key_id": "existing-key-id",
                    "key_secret": "existing-key-secret",
                    "service_name": "existing",
                    "created_at": "2026-05-11T12:00:00Z",
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let auth_args: Vec<String> = auth_args.iter().map(|arg| (*arg).to_string()).collect();
    let auth_project_dir = dir.path().to_path_buf();
    let auth_home_dir = home_dir.clone();
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |_: &wiremock::Request| {
            let mut command = Command::new(clickhousectl_binary());
            clear_inherited_env(&mut command);
            let output = command
                .env("DO_NOT_TRACK", "1")
                .env("HOME", &auth_home_dir)
                .env_remove("CLICKHOUSE_CLOUD_API_KEY")
                .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
                .current_dir(&auth_project_dir)
                .args(&auth_args)
                .output()
                .expect("failed to spawn concurrent auth command");
            assert_success(&output);
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": "ep-1" },
                "status": 200,
                "requestId": "stub-endpoint-upsert",
            }))
        })
        .expect(1)
        .mount(&control)
        .await;

    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home_dir)
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .current_dir(dir.path())
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
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(output.stdout, b"1\n");
    dir
}

#[tokio::test]
async fn provisioning_merges_a_concurrent_api_key_login() {
    let dir = provision_while_project_auth_changes(&[
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
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["api_key"], "concurrent-key");
    assert_eq!(stored["api_secret"], "concurrent-secret");
    assert_eq!(
        stored["service_query_keys"]["existing-service"]["key_id"],
        "existing-key-id"
    );
    assert_eq!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]["key_id"],
        "provisioned-key-id"
    );
}

#[tokio::test]
async fn provisioning_does_not_restore_credentials_cleared_by_concurrent_logout() {
    let dir =
        provision_while_project_auth_changes(&["cloud", "auth", "logout", "--api-keys"]).await;

    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert!(stored.get("api_key").is_none());
    assert!(stored.get("api_secret").is_none());
    assert!(
        stored["service_query_keys"]
            .get("existing-service")
            .is_none()
    );
    assert_eq!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]["key_id"],
        "provisioned-key-id"
    );
    assert_eq!(stored["service_query_keys"].as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn service_query_auto_provisioning_is_single_flight_across_processes() {
    const PROCESS_COUNT: usize = 6;

    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host_for_provisioning().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": { "id": QUERY_TEST_KEY_UUID },
                "keyId": "provisioned-key-id",
                "keySecret": "provisioned-key-secret",
            },
            "status": 200,
            "requestId": "stub-key-create",
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
            "requestId": "stub-endpoint-get",
        })))
        .expect(1)
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "ep-1" },
            "status": 200,
            "requestId": "stub-endpoint-upsert",
        })))
        .expect(1)
        .mount(&control)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    std::fs::create_dir_all(&home_dir).unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join(".gitignore"), "*\n").unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let initial_credentials = serde_json::json!({
        "api_key": "fake-key-for-tests",
        "api_secret": "fake-secret-for-tests",
        "service_query_keys": {
            "existing-service": {
                "key_id": "existing-key-id",
                "key_secret": "existing-key-secret",
                "endpoint_id": "existing-endpoint",
                "service_name": "existing",
                "created_at": "2026-05-11T12:00:00Z",
            }
        }
    });
    std::fs::write(
        &credentials_path,
        serde_json::to_vec(&initial_credentials).unwrap(),
    )
    .unwrap();
    let initial_metadata = std::fs::metadata(&credentials_path).unwrap();

    // Hold the same project lock as the CLI until every process has reached
    // provisioning. This makes the contention deterministic rather than
    // relying on process startup timing.
    let provision_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(credentials_dir.join("query-provision.lock"))
        .unwrap();
    provision_lock.lock().unwrap();

    let control_url = control.uri();
    let query_host_url = query_host.uri();
    let mut children = Vec::with_capacity(PROCESS_COUNT);
    for _ in 0..PROCESS_COUNT {
        let mut command = Command::new(clickhousectl_binary());
        clear_inherited_env(&mut command);
        children.push(
            command
                .env("DO_NOT_TRACK", "1")
                .env("HOME", &home_dir)
                .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
                .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
                .env("CLICKHOUSE_CLOUD_QUERY_HOST", &query_host_url)
                .current_dir(dir.path())
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
                    "--query",
                    "SELECT 1",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn concurrent clickhousectl"),
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

    provision_lock.unlock().unwrap();
    drop(provision_lock);
    let outputs = futures_util::future::join_all(
        children
            .into_iter()
            .map(|child| tokio::task::spawn_blocking(move || child.wait_with_output().unwrap())),
    )
    .await;
    for output in outputs {
        let output = output.unwrap();
        assert_success(&output);
        assert_eq!(output.stdout, b"1\n");
    }

    let control_requests = control.received_requests().await.unwrap();
    assert_eq!(
        control_requests
            .iter()
            .filter(|request| {
                request.method == wiremock::http::Method::POST
                    && request.url.path() == "/v1/organizations/org-1/keys"
            })
            .count(),
        1,
        "only the lock winner may create a key",
    );
    let endpoint_upserts: Vec<_> = control_requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
        })
        .collect();
    assert_eq!(endpoint_upserts.len(), 1, "the key must be bound once");
    let upsert_body: Value = serde_json::from_slice(&endpoint_upserts[0].body).unwrap();
    assert_eq!(
        upsert_body["openApiKeys"],
        serde_json::json!([QUERY_TEST_KEY_UUID])
    );
    assert!(
        control_requests
            .iter()
            .all(|request| request.method != wiremock::http::Method::DELETE),
        "a successfully shared key must not be deleted",
    );

    let stored: Value = serde_json::from_slice(&std::fs::read(&credentials_path).unwrap()).unwrap();
    assert_eq!(stored["api_key"], "fake-key-for-tests");
    assert_eq!(stored["api_secret"], "fake-secret-for-tests");
    assert_eq!(
        stored["service_query_keys"]["existing-service"]["key_id"], "existing-key-id",
        "the lock winner must merge the credentials it re-read",
    );
    let new_key = &stored["service_query_keys"][QUERY_TEST_SERVICE_ID];
    assert_eq!(new_key["api_key_id"], QUERY_TEST_KEY_UUID);
    assert_eq!(new_key["key_id"], "provisioned-key-id");
    assert_eq!(new_key["key_secret"], "provisioned-key-secret");
    assert_eq!(stored["service_query_keys"].as_object().unwrap().len(), 2);

    let final_metadata = std::fs::metadata(&credentials_path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        assert_ne!(
            initial_metadata.ino(),
            final_metadata.ino(),
            "credentials must be replaced atomically, not truncated in place",
        );
        assert_eq!(final_metadata.permissions().mode() & 0o777, 0o600);
    }
    let mut entries: Vec<_> = std::fs::read_dir(&credentials_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        [
            ".gitignore",
            "credentials.json",
            "credentials.lock",
            "query-provision.lock",
        ],
        "atomic persistence must not leave a temporary file",
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
        .filter(|r| {
            r.method == wiremock::http::Method::DELETE
                && r.url.path().starts_with("/v1/organizations/org-1/keys/")
        })
        .map(|r| r.url.path().to_string())
        .collect()
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

fn invoke_service_query_provisioning_in(
    control: &MockServer,
    query_host: &MockServer,
    project_dir: &Path,
) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    clear_inherited_env(&mut command);
    command
        .env("DO_NOT_TRACK", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .current_dir(project_dir)
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
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn service_query_removes_a_new_endpoint_before_deleting_the_key_on_persistence_failure() {
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

    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join(".gitignore"), "*\n").unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    let get_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if get_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                ResponseTemplate::new(404).set_body_json(serde_json::json!({
                    "error": "not found",
                    "status": 404,
                    "requestId": "stub-endpoint-get",
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": {
                        "id": "ep-1",
                        "allowedOrigins": "*",
                        "openApiKeys": [QUERY_TEST_KEY_UUID],
                        "roles": ["sql_console_admin"],
                    },
                    "status": 200,
                    "requestId": "stub-endpoint-get-after-bind",
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            // The child has already re-read credentials by the time it binds
            // the endpoint. A directory at the destination makes the atomic
            // commit fail after key creation and binding have succeeded.
            std::fs::create_dir(&credentials_path).unwrap();
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": "ep-1" },
                "status": 200,
                "requestId": "stub-endpoint-upsert",
            }))
        })
        .expect(1)
        .mount(&control)
        .await;
    Mock::given(method("DELETE"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-endpoint-delete",
        })))
        .expect(1)
        .mount(&control)
        .await;

    let output = invoke_service_query_provisioning_in(&control, &query_host, dir.path());

    assert!(!output.status.success());
    assert_eq!(
        recorded_key_deletes(&control).await,
        vec![format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )],
        "persistence failure must delete the exact key it created",
    );
    let requests = control.received_requests().await.unwrap();
    let endpoint_delete = requests
        .iter()
        .position(|request| {
            request.method == wiremock::http::Method::DELETE && request.url.path() == endpoint_path
        })
        .expect("the newly created endpoint must be removed");
    let key_delete = requests
        .iter()
        .position(|request| {
            request.method == wiremock::http::Method::DELETE
                && request.url.path()
                    == format!("/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}")
        })
        .expect("the newly created key must be deleted");
    assert!(
        endpoint_delete < key_delete,
        "the key must remain valid until its endpoint binding is removed",
    );
    assert!(credentials_dir.join("credentials.json").is_dir());
}

#[tokio::test]
async fn service_query_restores_existing_endpoint_before_deleting_key_on_persistence_failure() {
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

    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join(".gitignore"), "*\n").unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    let existing_key = "99999999-8888-7777-6666-555555555555";
    let get_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            let result = if get_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                serde_json::json!({
                    "id": "ep-1",
                    "allowedOrigins": "https://before.example",
                    "openApiKeys": [existing_key],
                    "roles": ["sql_console_read_only"],
                })
            } else {
                serde_json::json!({
                    "id": "ep-1",
                    "allowedOrigins": "*",
                    "openApiKeys": [existing_key, QUERY_TEST_KEY_UUID],
                    "roles": ["sql_console_admin"],
                })
            };
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-endpoint-get",
            }))
        })
        .expect(2)
        .mount(&control)
        .await;
    let upsert_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if upsert_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                std::fs::create_dir(&credentials_path).unwrap();
            }
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": "ep-1" },
                "status": 200,
                "requestId": "stub-endpoint-upsert",
            }))
        })
        .expect(2)
        .mount(&control)
        .await;

    let output = invoke_service_query_provisioning_in(&control, &query_host, dir.path());
    assert!(!output.status.success());
    assert_eq!(
        recorded_key_deletes(&control).await,
        vec![format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )],
    );

    let requests = control.received_requests().await.unwrap();
    let endpoint_upserts: Vec<_> = requests
        .iter()
        .enumerate()
        .filter(|(_, request)| {
            request.method == wiremock::http::Method::POST && request.url.path() == endpoint_path
        })
        .collect();
    assert_eq!(endpoint_upserts.len(), 2);
    let bind_body: Value = serde_json::from_slice(&endpoint_upserts[0].1.body).unwrap();
    assert_eq!(
        bind_body["openApiKeys"],
        serde_json::json!([existing_key, QUERY_TEST_KEY_UUID])
    );
    let restore_body: Value = serde_json::from_slice(&endpoint_upserts[1].1.body).unwrap();
    assert_eq!(
        restore_body,
        serde_json::json!({
            "allowedOrigins": "https://before.example",
            "openApiKeys": [existing_key],
            "roles": ["sql_console_read_only"],
        }),
        "compensation must restore the snapshot when the endpoint still matches the bind",
    );
    let key_delete = requests
        .iter()
        .position(|request| {
            request.method == wiremock::http::Method::DELETE
                && request.url.path()
                    == format!("/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}")
        })
        .unwrap();
    assert!(
        endpoint_upserts[1].0 < key_delete,
        "the key must be deleted only after it is unbound",
    );
}

async fn invoke_persistence_failure_with_current_endpoint(
    current_endpoint: Option<Value>,
) -> (MockServer, std::process::Output, String) {
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

    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join(".gitignore"), "*\n").unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    let existing_key = "99999999-8888-7777-6666-555555555555";
    let get_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if get_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": {
                        "id": "ep-1",
                        "allowedOrigins": "https://before.example",
                        "openApiKeys": [existing_key],
                        "roles": ["sql_console_read_only"],
                    },
                    "status": 200,
                    "requestId": "stub-endpoint-get-before-bind",
                }))
            } else if let Some(result) = &current_endpoint {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": result,
                    "status": 200,
                    "requestId": "stub-endpoint-get-before-compensation",
                }))
            } else {
                ResponseTemplate::new(404).set_body_json(serde_json::json!({
                    "error": "not found",
                    "status": 404,
                    "requestId": "stub-endpoint-get-before-compensation",
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;
    let upsert_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if upsert_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                std::fs::create_dir(&credentials_path).unwrap();
            }
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": "ep-1" },
                "status": 200,
                "requestId": "stub-endpoint-upsert",
            }))
        })
        .mount(&control)
        .await;

    let output = invoke_service_query_provisioning_in(&control, &query_host, dir.path());
    (control, output, endpoint_path)
}

#[tokio::test]
async fn service_query_does_not_restore_over_concurrently_changed_endpoint() {
    let concurrent_key = "11111111-2222-3333-4444-555555555555";
    let (control, output, endpoint_path) =
        invoke_persistence_failure_with_current_endpoint(Some(serde_json::json!({
            "id": "ep-1",
            "allowedOrigins": "https://new.example",
            "openApiKeys": [concurrent_key, QUERY_TEST_KEY_UUID],
            "roles": ["sql_console_read_only"],
        })))
        .await;

    assert!(!output.status.success());
    let requests = control.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| {
                request.method == wiremock::http::Method::POST
                    && request.url.path() == endpoint_path
            })
            .count(),
        1,
        "compensation must not overwrite the concurrently changed endpoint",
    );
    assert!(
        recorded_key_deletes(&control).await.is_empty(),
        "the possibly still-bound key must be retained for recovery",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("restoration was intentionally skipped")
            && stderr.contains("no longer match the state written by this operation")
            && stderr.contains(QUERY_TEST_KEY_UUID)
            && stderr.contains("inspect the current endpoint before deleting it"),
        "the error must explain the guarded skip and recovery action:\n{stderr}",
    );
}

#[tokio::test]
async fn service_query_does_not_recreate_concurrently_deleted_endpoint() {
    let (control, output, endpoint_path) =
        invoke_persistence_failure_with_current_endpoint(None).await;

    assert!(!output.status.success());
    let requests = control.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| {
                request.method == wiremock::http::Method::POST
                    && request.url.path() == endpoint_path
            })
            .count(),
        1,
        "compensation must not recreate a concurrently deleted endpoint",
    );
    assert_eq!(
        recorded_key_deletes(&control).await,
        vec![format!(
            "/v1/organizations/org-1/keys/{QUERY_TEST_KEY_UUID}"
        )],
        "the unbound API key should still be cleaned up",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cleanup was skipped because the endpoint was deleted concurrently")
            && stderr.contains("it was not recreated")
            && stderr.contains(QUERY_TEST_KEY_UUID)
            && stderr.contains("was deleted"),
        "the error must explain that deletion was preserved and cleanup completed:\n{stderr}",
    );
}

#[tokio::test]
async fn service_query_retains_bound_key_when_endpoint_compensation_fails() {
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

    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join(".gitignore"), "*\n").unwrap();
    let credentials_path = credentials_dir.join("credentials.json");
    let endpoint_path =
        format!("/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}/serviceQueryEndpoint");
    let get_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            let result = if get_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                serde_json::json!({
                    "id": "ep-1",
                    "allowedOrigins": "https://before.example",
                    "openApiKeys": [],
                    "roles": ["sql_console_read_only"],
                })
            } else {
                serde_json::json!({
                    "id": "ep-1",
                    "allowedOrigins": "*",
                    "openApiKeys": [QUERY_TEST_KEY_UUID],
                    "roles": ["sql_console_admin"],
                })
            };
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-endpoint-get",
            }))
        })
        .expect(2)
        .mount(&control)
        .await;
    let upsert_count = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(endpoint_path.clone()))
        .respond_with(move |_: &wiremock::Request| {
            if upsert_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                std::fs::create_dir(&credentials_path).unwrap();
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": "ep-1" },
                    "status": 200,
                    "requestId": "stub-endpoint-bind",
                }))
            } else {
                ResponseTemplate::new(500).set_body_json(serde_json::json!({
                    "error": "endpoint restore rejected",
                    "status": 500,
                    "requestId": "stub-endpoint-restore",
                }))
            }
        })
        .expect(2)
        .mount(&control)
        .await;

    let output = invoke_service_query_provisioning_in(&control, &query_host, dir.path());
    assert!(!output.status.success());
    assert!(
        recorded_key_deletes(&control).await.is_empty(),
        "a still-bound key must be retained",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("local credential persistence failed")
            && stderr.contains("query endpoint cleanup failed")
            && stderr.contains("endpoint restore rejected")
            && stderr.contains(QUERY_TEST_KEY_UUID)
            && stderr.contains("retained for recovery"),
        "the error must report both failures and the recoverable key ID:\n{stderr}",
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
            "result": {
                "id": "ep-1",
                "allowedOrigins": "https://existing.example",
                "openApiKeys": existing_keys,
                "roles": ["sql_console_read_only"],
            },
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

// ── ClickPipe settings updates preserve required values ─────────────────────

#[tokio::test]
async fn clickpipe_settings_update_preserves_or_defaults_kafka_read_committed() {
    let settings_path = "/v1/organizations/org/services/svc-id/clickpipes/pipe-id/settings";
    for (current_settings, expected) in [
        (serde_json::json!({ "kafka_read_committed": true }), true),
        (serde_json::json!({}), false),
    ] {
        let mock = MockServer::start().await;
        let current_settings = serde_json::json!({
            "result": current_settings,
            "status": 200,
            "requestId": "stub-settings-get",
        });
        let updated_settings = serde_json::json!({
            "result": {
                "streaming_max_insert_wait_ms": 1000,
                "kafka_read_committed": expected,
            },
            "status": 200,
            "requestId": "stub-settings-update",
        });

        Mock::given(method("GET"))
            .and(path(settings_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(current_settings))
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .and(path(settings_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(updated_settings))
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
                "--streaming-max-insert-wait-ms",
                "1000",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);

        let requests = mock.received_requests().await.unwrap();
        let request_shape = requests
            .iter()
            .map(|request| {
                (
                    request.method.as_str().to_string(),
                    request.url.path().to_string(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            request_shape,
            vec![
                ("GET".to_string(), settings_path.to_string()),
                ("PUT".to_string(), settings_path.to_string()),
            ]
        );

        let put = requests
            .iter()
            .find(|request| request.method == wiremock::http::Method::PUT)
            .expect("no settings PUT request recorded by mock");
        assert_eq!(
            serde_json::from_slice::<Value>(&put.body).unwrap(),
            serde_json::json!({
                "streaming_max_insert_wait_ms": 1000,
                "kafka_read_committed": expected,
            })
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
