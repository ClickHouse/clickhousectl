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
use wiremock::matchers::{body_json, header, method, path, path_regex};
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
async fn postgres_invalid_inputs_exit_as_usage_errors_before_auth_file_or_network() {
    let mock = MockServer::start().await;
    let missing_ca = "/missing/postgres-ca.pem";
    let base = || {
        let mut args = postgres_args_minimal();
        args.extend(["--ca-certificate".into(), missing_ca.into()]);
        args
    };
    let replace_value = |args: &mut Vec<String>, flag: &str, value: &str| {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .unwrap_or_else(|| panic!("missing test flag {flag}"));
        args[index + 1] = value.into();
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
    let mut args = postgres_args_minimal();
    args.push("--auth".into());
    args.push("IAM_ROLE".into());
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
async fn postgres_unknown_authority_error_preserves_api_detail_and_adds_ca_hint() {
    let mock = MockServer::start().await;
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

    let args = postgres_args_minimal();
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

#[tokio::test]
async fn stored_query_key_401_or_403_explains_repair_without_provisioning() {
    for status in [401, 403] {
        let control = start_mock_control_plane_with_service().await;
        let query_host = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
            .respond_with(ResponseTemplate::new(status).set_body_string("stale stored key"))
            .expect(1)
            .mount(&query_host)
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
        let output = service_query_process(project.path(), &control, &query_host)
            .output()
            .await
            .expect("failed to spawn clickhousectl");

        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("rejected with HTTP {status}")),
            "{stderr}"
        );
        assert!(stderr.contains("no replacement was created"), "{stderr}");
        assert!(
            stderr.contains(&format!(
                "clickhousectl cloud service repair-query-key {QUERY_TEST_SERVICE_ID} --org-id org-1"
            )),
            "{stderr}"
        );
        assert!(
            control
                .received_requests()
                .await
                .unwrap()
                .iter()
                .all(|request| request.method == wiremock::http::Method::GET),
            "a rejected stored key must not trigger provisioning"
        );
        let stored: Value = serde_json::from_slice(
            &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(stored, original);
    }
}

#[tokio::test]
async fn service_query_key_repair_replaces_exact_binding_and_preserves_credentials() {
    let control = MockServer::start().await;
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
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("replacement query key"), "{stderr}");
    assert!(stderr.contains("is active"), "{stderr}");
    assert!(stderr.contains("rerun"), "{stderr}");

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
async fn repair_cleanup_retry_deletes_only_pending_key_without_reprovisioning() {
    let control = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/keys/{OLD_QUERY_TEST_KEY_UUID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-pending-cleanup"
        })))
        .expect(1)
        .mount(&control)
        .await;

    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("home")).unwrap();
    let original = write_repair_query_credentials(
        project.path(),
        Some("org-1"),
        Some(QUERY_TEST_KEY_UUID),
        Some("ep-1"),
        &[OLD_QUERY_TEST_KEY_UUID],
    );
    let output = service_query_key_repair_process(project.path(), &control)
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "cleanup_completed");
    assert_eq!(result["apiKeyId"], QUERY_TEST_KEY_UUID);

    let requests = control.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, wiremock::http::Method::DELETE);
    let stored: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["api_key"], original["api_key"]);
    assert_eq!(
        stored["service_query_keys"][PRESERVED_QUERY_SERVICE_ID],
        original["service_query_keys"][PRESERVED_QUERY_SERVICE_ID]
    );
    assert!(
        stored["service_query_keys"][QUERY_TEST_SERVICE_ID]
            .get("pending_cleanup_api_key_ids")
            .is_none()
    );
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
        serde_json::json!({ "postgres": { "host": "db.example.com" } }),
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
async fn clickpipe_settings_update_preserves_or_defaults_kafka_read_committed() {
    for (current_settings, expected) in [
        (serde_json::json!({ "kafka_read_committed": true }), true),
        (serde_json::json!({ "kafka_read_committed": false }), false),
        (serde_json::json!({}), false),
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
            "10.0.0.0/8",
        ],
    );

    assert_success(&output);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
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
