//! `cloud use` (src/cloud/context.rs): the project's pinned organization.

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

const ACME: &str = "11111111-2222-4333-8444-555555555555";
const OTHER: &str = "66666666-7777-4888-8999-000000000000";

/// Pinning by name writes the project's gitignored pin, later commands use
/// it with a note on stderr while stdout stays JSON, and clearing it brings
/// back auto-detection, which fails with two organizations (#1509).
#[tokio::test]
async fn a_pinned_org_scopes_later_commands_until_cleared() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{"id": ACME, "name": "acme"}, {"id": OTHER, "name": "other"}]
        })))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/organizations/{ACME}/services")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": []})))
        .expect(1)
        .mount(&mock)
        .await;
    let project = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        cloud_runtime_error_command(project.path(), &mock.uri(), "explicit")
            .args(args)
            .output()
            .unwrap()
    };

    let pinned = run(&["use", "--org-name", "acme"]);
    assert_success(&pinned);
    let shown: Value = serde_json::from_slice(&pinned.stdout).unwrap();
    assert_eq!(shown["org"]["id"], ACME);
    let state = project.path().join(".clickhouse");
    assert!(state.join(".gitignore").exists());
    let file: Value =
        serde_json::from_slice(&std::fs::read(state.join("context.json")).unwrap()).unwrap();
    assert_eq!(file["org"]["id"], ACME);

    let listed = run(&["clickhouse", "list"]);
    assert_success(&listed);
    assert_eq!(
        serde_json::from_slice::<Value>(&listed.stdout).unwrap(),
        serde_json::json!([])
    );
    let stderr = String::from_utf8_lossy(&listed.stderr);
    assert!(stderr.contains(ACME), "{stderr}");
    assert!(stderr.contains("context.json"), "{stderr}");

    assert_success(&run(&["use", "--clear"]));
    assert!(!state.join("context.json").exists());
    let shown: Value = serde_json::from_slice(&run(&["use"]).stdout).unwrap();
    assert_eq!(shown["org"], Value::Null);
    assert_eq!(run(&["clickhouse", "list"]).status.code(), Some(2));
}

/// From the home directory, discovery selects home's own `.clickhouse/`,
/// the global one: pinning there is refused before any request and writes
/// nothing, while showing the pin still works.
#[tokio::test]
async fn pinning_from_the_home_directory_is_refused() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{"id": ACME, "name": "acme"}]
        })))
        .expect(0)
        .mount(&mock)
        .await;
    let home = tempfile::tempdir().unwrap();
    let state = home.path().join(".clickhouse");
    std::fs::create_dir_all(&state).unwrap();
    let run = |args: &[&str]| {
        cloud_runtime_error_command(home.path(), &mock.uri(), "human")
            .env("HOME", home.path())
            .current_dir(home.path())
            .args(args)
            .output()
            .unwrap()
    };

    let refused = run(&["use", "--org-name", "acme"]);
    assert_eq!(refused.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("home directory"), "{stderr}");
    assert!(stderr.contains("CLICKHOUSECTL_PROJECT_DIR"), "{stderr}");
    assert!(!state.join("context.json").exists());

    assert_success(&run(&["use"]));
}
