//! `cloud service`, except `query` and `client` (src/cloud/services/).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{
    body_json, body_partial_json, header, method, path, path_regex, query_param,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

const SERVICE_PROFILES_PATH: &str = "/v1/organizations/org-1/serviceProfiles";
const SCALING_SCHEDULE_PATH: &str = "/v1/organizations/org-1/services/svc-1/scalingSchedule";

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
    command.isolated_env();
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
    assert_eq!(
        service["next"]["command"],
        "clickhousectl cloud service get svc-1 --org-id org-1 --json"
    );
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
async fn service_start_and_stop_preserve_json_responses_and_name_the_poll() {
    for (command, returned_state, until) in [
        ("start", "running", serde_json::json!(["running"])),
        ("stop", "stopped", serde_json::json!(["stopped", "idle"])),
    ] {
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
        let (response, next) = take_next(&output.stdout);
        assert_eq!(response, result);
        let next = next.expect("an accepted state change names its poll");
        assert_eq!(
            next["command"],
            "clickhousectl cloud service get svc-1 --org-id org-1 --json"
        );
        assert_eq!(next["field"], "state");
        assert_eq!(next["until"], until, "{command}");
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
    let error = cloud_runtime_error(&output);
    assert_eq!(error["code"], "http_4xx");
    // Both ways out, by flag and by command, then the API's own response.
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("--force"), "{message}");
    assert!(
        message.contains("`clickhousectl cloud service stop svc-1`"),
        "{message}"
    );
    assert!(message.ends_with("opaque conflict response"), "{message}");
}

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
            "service_query_keys": { TEST_SERVICE_ID: key },
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
    spawn_service_delete_with_env(mock, project_dir, force, stdout, stderr, &[])
}

fn spawn_service_delete_with_env(
    mock: &MockServer,
    project_dir: &Path,
    force: bool,
    stdout: Stdio,
    stderr: Stdio,
    env: &[(&str, &str)],
) -> std::process::Child {
    let url = mock.uri();
    let mut args = vec![
        "cloud",
        "--url",
        &url,
        "--json",
        "service",
        "delete",
        TEST_SERVICE_ID,
        "--org-id",
        "org-1",
    ];
    if force {
        args.push("--force");
    }
    Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        // A forced delete of a running service polls the stop; keep the
        // test from waiting out the shipped 5 s interval.
        .env("CHCTL_SERVICE_STOP_POLL_INTERVAL_MS", "100")
        .envs(env.iter().copied())
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

/// A stored record is never used to delete a key (#1325): the service is
/// deleted, its record is dropped, and stderr names the key that is left
/// with the command that deletes it.
#[tokio::test]
async fn service_delete_drops_a_stored_query_key_record_and_names_the_key_it_leaves() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_service_query_key(dir.path(), Some("org-1"), Some(DELETE_TEST_API_KEY_ID));
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_success(&output);
    serde_json::from_slice::<Value>(&output.stdout).expect("stdout stays one JSON value");
    assert_eq!(
        received_request_shape(&mock).await,
        [(
            "DELETE".to_string(),
            format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
        )]
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "clickhousectl cloud key delete {DELETE_TEST_API_KEY_ID} --org-id org-1"
        )),
        "{stderr}"
    );
    assert!(!stderr.contains("query-key-secret"), "{stderr}");

    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert!(stored.get("service_query_keys").is_none(), "{stored}");
}

#[tokio::test]
async fn service_delete_without_a_stored_query_key_only_deletes_the_service() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), false);
    assert_success(&output);
    let (_, next) = take_next(&output.stdout);
    let next = next.expect("an accepted delete names its poll");
    assert_eq!(
        next["command"],
        format!("clickhousectl cloud service get {TEST_SERVICE_ID} --org-id org-1 --json")
    );
    assert_eq!(next["until_error_code"], "resource_not_found");

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, wiremock::http::Method::DELETE);
    assert_eq!(
        requests[0].url.path(),
        format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
    );
    assert!(requests[0].body.is_empty());
}

#[tokio::test]
async fn forced_service_delete_surfaces_not_found_for_an_absent_service() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
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
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
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
                format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
            ),
            (
                "DELETE".to_string(),
                format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
            ),
        ]
    );

    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][TEST_SERVICE_ID]["api_key_id"],
        DELETE_TEST_API_KEY_ID
    );
}

/// Mock a `--force` delete of a service whose `GET`s answer `states` in
/// order: the first is the pre-stop check, the rest come from the poll loop.
async fn mount_forced_delete_stop_sequence(mock: &MockServer, states: &'static [&'static str]) {
    mount_forced_stop_sequence(mock, states).await;
    mount_forced_delete_call(mock, 1).await;
}

/// The `GET`s and the stop `PATCH` of [`mount_forced_delete_stop_sequence`].
async fn mount_forced_stop_sequence(mock: &MockServer, states: &'static [&'static str]) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let request_index = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
        )))
        .respond_with(move |_: &wiremock::Request| {
            let index = request_index.fetch_add(1, Ordering::SeqCst);
            let state = states[index.min(states.len() - 1)];
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "id": TEST_SERVICE_ID,
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
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}/state"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": TEST_SERVICE_ID,
                "name": "demo",
                "state": "stopping",
            },
            "status": 200,
            "requestId": "stub-service-stop",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

async fn mount_forced_delete_call(mock: &MockServer, expected: u64) {
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
        )))
        .respond_with(successful_delete_response("stub-service-delete"))
        .expect(expected)
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
    assert!(notice.contains(TEST_SERVICE_ID), "{stderr}");
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
            format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
        )),
        "unexpected request sequence: {shape:?}"
    );
}

/// A stop that ends somewhere other than `stopped` fails the forced delete
/// before it deletes anything (#1065).
#[tokio::test]
async fn forced_service_delete_fails_when_the_service_leaves_the_stop_path() {
    let mock = MockServer::start().await;
    mount_forced_stop_sequence(&mock, &["running", "stopping", "softdeleted"]).await;
    mount_forced_delete_call(&mock, 0).await;

    let dir = tempfile::tempdir().unwrap();
    let output = invoke_service_delete(&mock, dir.path(), true);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Progress lines precede the error object.
    let start = stderr
        .find("\n{")
        .expect("a JSON error after the progress lines");
    let error: Value = serde_json::from_str(&stderr[start..]).expect("JSON error");
    assert_eq!(error["error"]["code"], "other", "{stderr}");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("softdeleted"), "{message}");
}

/// A stop that outlasts the deadline fails with `timeout`, naming the last
/// state and the command to check, and deletes nothing (#1065).
#[tokio::test]
async fn forced_service_delete_times_out_waiting_for_the_stop() {
    let mock = MockServer::start().await;
    mount_forced_stop_sequence(&mock, &["running", "stopping"]).await;
    mount_forced_delete_call(&mock, 0).await;

    let dir = tempfile::tempdir().unwrap();
    let output = spawn_service_delete_with_env(
        &mock,
        dir.path(),
        true,
        Stdio::piped(),
        Stdio::piped(),
        &[("CHCTL_SERVICE_STOP_TIMEOUT_MS", "0")],
    )
    .wait_with_output()
    .expect("failed to run clickhousectl");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Progress lines precede the error object.
    let start = stderr
        .find("\n{")
        .expect("a JSON error after the progress lines");
    let error: Value = serde_json::from_str(&stderr[start..]).expect("JSON error");
    assert_eq!(error["error"]["code"], "timeout", "{stderr}");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("stopping"), "{message}");
    assert!(
        message.contains(&format!("cloud service get {TEST_SERVICE_ID}")),
        "{message}"
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
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
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
            format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
        )]
    );
}

#[tokio::test]
async fn service_delete_failure_preserves_the_query_key_without_cleanup() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
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
        format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
    );
    let stored: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["service_query_keys"][TEST_SERVICE_ID]["api_key_id"],
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
            "/v1/organizations/org-1/services/{TEST_SERVICE_ID}"
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
        format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}")
    );
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
    // One warning per unmatched removal, naming its flag and value; the
    // matched `env` tag gets none. The matching rules are the
    // `unmatched_removal_warnings_*` unit tests.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let warnings: Vec<&str> = stderr.lines().collect();
    assert_eq!(warnings.len(), 2, "{stderr}");
    assert!(
        warnings[0].contains("--remove-private-endpoint-id pe-missing"),
        "{stderr}"
    );
    assert!(warnings[1].contains("--remove-tag missing"), "{stderr}");

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

    assert_postgres_not_found_human(
        &output,
        &format!("BAD_REQUEST: Invalid service id string:\"{UNKNOWN_UUID}\""),
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

    assert_postgres_not_found_human(
        &output,
        &format!("BAD_REQUEST: Invalid service id string:\"{UNKNOWN_UUID}\""),
    );
    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("DELETE".to_string(), service_path.clone()),
            ("GET".to_string(), service_path),
        ]
    );
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

/// An unlisted `--region` goes to Cloud as-is (#1383), and when Cloud rejects
/// it the error still carries Cloud's message plus the closest known region.
#[tokio::test]
async fn service_create_sends_an_unlisted_region_and_hints_when_cloud_rejects_it() {
    let mock = MockServer::start().await;
    let api_error = "BAD_REQUEST: region is not available";
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .and(body_partial_json(
            serde_json::json!({"region": "us-esat-1"}),
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": api_error,
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
            "s",
            "--region",
            "us-esat-1",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(api_error), "{stderr}");
    assert!(stderr.contains("us-east-1"), "{stderr}");
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
    let cwd = tempfile::tempdir().unwrap();
    let mut child = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", cwd.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(cwd.path())
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
            assert_eq!(result["next"]["field"], "state");
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
    assert_eq!(
        result["next"]["command"],
        format!("clickhousectl cloud service get {DOTENV_SERVICE_ID} --org-id org-1 --json")
    );
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
