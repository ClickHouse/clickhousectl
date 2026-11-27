//! `cloud init` subprocess + wiremock tests (src/cloud/init.rs).
//!
//! These tests exercise the full binary against a mock server and assert:
//! - correct HTTP wiring (no Authorization header on POST /v1/claimable)
//! - exit codes
//! - stdout / stderr content (org_id present, key_secret absent)
//! - credentials file written correctly
//! - guard behavior when credentials already exist

#![cfg(feature = "experimental-claimable")]

use std::path::Path;
use std::process::Command;

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// A fixed claimable response body. The key_secret is `very-secret-value`
/// and must NEVER appear in stdout or stderr.
fn claimable_response() -> Value {
    serde_json::json!({
        "result": {
            "organizationId": "org-claimable-1",
            "apiKey": {
                "keyId":     "returned-key-id",
                "keySecret": "very-secret-value"
            },
            "expiresAt": "2099-12-31T23:59:59Z",
            "limits": {
                "creditUsd": 100,
                "tier":      "trial"
            }
        },
        "status":    200,
        "requestId": "stub-claimable"
    })
}

/// Spawn `cloud --url <mock> init [extra_args...]` with no credentials and no
/// ambient API key env vars. Returns (output, project_dir) so the caller can
/// inspect the credentials file. The project_dir is kept alive for the caller.
fn invoke_init(
    mock: &MockServer,
    extra_args: &[&str],
) -> (std::process::Output, tempfile::TempDir) {
    invoke_init_with_env(mock, extra_args, &[])
}

fn invoke_init_with_env(
    mock: &MockServer,
    extra_args: &[&str],
    env: &[(&str, &str)],
) -> (std::process::Output, tempfile::TempDir) {
    let project_dir = tempfile::tempdir().unwrap();
    let home = project_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url, "init"];
    args.extend(extra_args);

    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSECTL_PROJECT_DIR", project_dir.path())
        .envs(env.iter().copied())
        .current_dir(project_dir.path())
        .args(&args)
        .output()
        .expect("failed to spawn clickhousectl");

    (output, project_dir)
}

/// Read the credentials file written inside `project_dir/.clickhouse/`.
fn read_credentials(project_dir: &Path) -> Value {
    let path = project_dir.join(".clickhouse/credentials.json");
    let data = std::fs::read(&path)
        .unwrap_or_else(|err| panic!("credentials.json missing at {}: {err}", path.display()));
    serde_json::from_slice(&data).expect("credentials.json is not valid JSON")
}

/// Mount a single POST /v1/claimable mock returning `status`/`body`.
async fn mount_claimable(mock: &MockServer, status: u16, body: Value) {
    Mock::given(method("POST"))
        .and(path("/v1/claimable"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(mock)
        .await;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn cloud_init_happy_path() {
    let mock = MockServer::start().await;
    mount_claimable(&mock, 200, claimable_response()).await;

    let (output, project_dir) = invoke_init(&mock, &[]);

    // Exit 0.
    assert_success(&output);

    // org_id on stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("org-claimable-1"),
        "org_id missing from stdout: {stdout}"
    );

    // key_secret must NOT appear anywhere.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("very-secret-value"),
        "key_secret leaked to stdout: {stdout}"
    );
    assert!(
        !stderr.contains("very-secret-value"),
        "key_secret leaked to stderr: {stderr}"
    );

    // Credentials file has organization_id + api_key + api_secret + expires_at.
    let creds = read_credentials(project_dir.path());
    assert_eq!(creds["organization_id"], "org-claimable-1");
    assert_eq!(creds["api_key"], "returned-key-id");
    assert_eq!(creds["api_secret"], "very-secret-value");
    assert!(
        creds["expires_at"].is_string(),
        "expires_at missing or not a string: {creds}"
    );

    // The POST carried no Authorization header.
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected exactly 1 POST");
    let req = &requests[0];
    assert!(
        !req.headers.contains_key("authorization"),
        "POST should not carry an Authorization header, but got: {:?}",
        req.headers
    );
}

#[tokio::test]
async fn cloud_init_json_output_shape() {
    let mock = MockServer::start().await;
    mount_claimable(&mock, 200, claimable_response()).await;

    let (output, _project_dir) = invoke_init(&mock, &["--json"]);

    assert_success(&output);

    // The JSON output is on stdout.
    let json: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "stdout is not JSON ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    // Required keys present.
    assert!(
        json["organization_id"].is_string(),
        "organization_id missing: {json}"
    );
    assert!(json["expires_at"].is_string(), "expires_at missing: {json}");
    assert_eq!(json["organization_id"], "org-claimable-1");

    // key_secret must not appear in the JSON output.
    let raw = serde_json::to_string(&json).unwrap();
    assert!(
        !raw.contains("very-secret-value"),
        "key_secret leaked to JSON output: {raw}"
    );
    // key_secret also absent from stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("very-secret-value"),
        "key_secret leaked to stderr: {stderr}"
    );
}

#[tokio::test]
async fn cloud_init_already_initialized_exit_1() {
    // First run succeeds.
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/claimable"))
        .respond_with(ResponseTemplate::new(200).set_body_json(claimable_response()))
        .expect(1) // exactly one POST is ever made
        .mount(&mock)
        .await;

    let (first_output, project_dir) = invoke_init(&mock, &["--json"]);
    assert_success(&first_output);

    // Second run: same project directory already has credentials.
    let url = mock.uri();
    let home = project_dir.path().join("home");
    let second_output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .env("CLICKHOUSECTL_PROJECT_DIR", project_dir.path())
        .current_dir(project_dir.path())
        .args(["cloud", "--url", &url, "init", "--json"])
        .output()
        .expect("failed to spawn clickhousectl (second run)");

    // Exit 1 on second run.
    assert_eq!(
        second_output.status.code(),
        Some(1),
        "expected exit 1 on second run"
    );

    // JSON error code is "already_initialized".
    let error = cloud_runtime_error(&second_output);
    assert_eq!(
        error["code"], "already_initialized",
        "wrong error code: {error}"
    );

    // Wiremock verifies that exactly 1 POST was sent (enforced by `.expect(1)`
    // above; `MockServer::verify` runs on drop, but we can be explicit).
    mock.verify().await;
}

#[tokio::test]
async fn cloud_init_503_claimable_unavailable() {
    let mock = MockServer::start().await;
    mount_claimable(
        &mock,
        503,
        serde_json::json!({
            "status": 503,
            "error": "Service Unavailable",
            "requestId": "stub-503"
        }),
    )
    .await;

    let (output, _project_dir) = invoke_init(&mock, &["--json"]);

    assert_eq!(output.status.code(), Some(1), "expected exit 1 for 503");

    let error = cloud_runtime_error(&output);
    assert_eq!(
        error["code"], "claimable_unavailable",
        "wrong error code for 503: {error}"
    );
}

#[tokio::test]
async fn cloud_init_403_exit_1() {
    let mock = MockServer::start().await;
    mount_claimable(
        &mock,
        403,
        serde_json::json!({
            "status": 403,
            "error": "Forbidden",
            "requestId": "stub-403"
        }),
    )
    .await;

    let (output, project_dir) = invoke_init(&mock, &["--json"]);

    // Exit 1.
    assert_eq!(output.status.code(), Some(1), "expected exit 1 for 403");

    // No credentials file should have been written.
    let creds_path = project_dir.path().join(".clickhouse/credentials.json");
    assert!(
        !creds_path.exists(),
        "credentials file must not be written on 403"
    );
}

#[tokio::test]
async fn cloud_init_ignores_ambient_api_key_env() {
    let mock = MockServer::start().await;
    mount_claimable(&mock, 200, claimable_response()).await;

    let (output, project_dir) = invoke_init_with_env(
        &mock,
        &["--json"],
        &[
            ("CLICKHOUSE_CLOUD_API_KEY", "ambient-key"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "ambient-secret"),
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        !requests[0].headers.contains_key("authorization"),
        "ambient credentials must not be sent: {:?}",
        requests[0].headers
    );
    let creds = read_credentials(project_dir.path());
    assert_eq!(creds["api_key"], "returned-key-id");
}

#[tokio::test]
async fn cloud_init_pins_new_org_and_overwrites_stale_pin() {
    let mock = MockServer::start().await;
    mount_claimable(&mock, 200, claimable_response()).await;

    let (output, project_dir) = invoke_init(&mock, &[]);
    assert_success(&output);

    // context.json must exist and pin the new org.
    let context_path = project_dir.path().join(".clickhouse/context.json");
    assert!(
        context_path.exists(),
        "context.json not written after cloud init"
    );
    let context: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&context_path).unwrap()).unwrap();
    assert_eq!(
        context["org"]["id"], "org-claimable-1",
        "new org not pinned in context.json: {context}"
    );
}

#[tokio::test]
async fn cloud_init_overwrites_stale_pin_from_previous_cloud_use() {
    let mock = MockServer::start().await;
    mount_claimable(&mock, 200, claimable_response()).await;

    // Write a stale org pin as if `cloud use` had been run before.
    let project_dir = tempfile::tempdir().unwrap();
    let dot_dir = project_dir.path().join(".clickhouse");
    std::fs::create_dir_all(&dot_dir).unwrap();
    std::fs::write(
        dot_dir.join("context.json"),
        serde_json::to_string(&serde_json::json!({
            "version": 1,
            "api_url": format!("{}/v1", mock.uri()),
            "org": { "id": "stale-org-999", "name": "old org" }
        }))
        .unwrap(),
    )
    .unwrap();

    let home = project_dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .env("CLICKHOUSECTL_PROJECT_DIR", project_dir.path())
        .current_dir(project_dir.path())
        .args(["cloud", "--url", &url, "init"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let context: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dot_dir.join("context.json")).unwrap())
            .unwrap();
    assert_eq!(
        context["org"]["id"], "org-claimable-1",
        "stale pin not overwritten: {context}"
    );
}
