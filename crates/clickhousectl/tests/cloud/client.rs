//! Behaviour every Cloud command shares through the client: credential precedence, organization
//! resolution, error conversion and the JSON error envelope, request headers, closed pipes
//! (src/cloud/client.rs, src/cloud/credentials.rs, src/cloud/output.rs).

use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

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

/// #1290: saved API keys live with the project, so a command run from a
/// subdirectory sends the enclosing project's key.
#[tokio::test]
async fn saved_key_is_found_from_a_project_subdirectory() {
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
    let sub = dir.path().join("src/app");
    std::fs::create_dir_all(&sub).unwrap();

    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .current_dir(&sub)
        .args(["cloud", "--json", "org", "list"])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    let auth = requests[0].headers.get("Authorization").unwrap();
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "file-key:file-secret",
        )
    );
    assert_eq!(auth.to_str().unwrap(), expected);
    assert!(!sub.join(".clickhouse").exists());
}

/// A corrupt credentials file fails Cloud commands instead of falling through
/// to the usable env credentials below it, and `auth status` reports it
/// rather than showing the API key tier as absent (#1327).
#[tokio::test]
async fn corrupt_credentials_file_fails_closed_and_is_reported_by_status() {
    let mock = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    std::fs::write(credentials_dir.join("credentials.json"), b"{corrupt\n").unwrap();

    let output = cloud_runtime_error_command(dir.path(), &mock.uri(), "explicit")
        .args(["service", "list", "--org-id", "org-1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "other");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.contains(".clickhouse/credentials.json"),
        "{message}"
    );

    let output = cloud_runtime_error_command(dir.path(), &mock.uri(), "explicit")
        .args(["auth", "status"])
        .output()
        .unwrap();
    assert_success(&output);
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    let warning = status["credentials_warning"].as_str().unwrap();
    assert!(
        warning.contains(".clickhouse/credentials.json"),
        "{warning}"
    );
    let sources = status["sources"].as_array().unwrap();
    assert!(sources.iter().all(|row| row["active"] == "-"), "{status}");
    assert_eq!(status["verification"], "skipped");

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
    // One notice naming both ignored variables and the flag that explains why.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    for identifier in [
        "CLICKHOUSE_CLOUD_API_KEY",
        "CLICKHOUSE_CLOUD_API_SECRET",
        "--debug",
    ] {
        assert!(stderr.contains(identifier), "{identifier}: {stderr}");
    }

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
    command.isolated_env();
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
    // because the loader reads `cwd/.env` first (the project root's only when
    // `cwd` has none). The child's environment is cleared and its HOME is
    // empty, so the .env is the only source of credentials — otherwise the
    // test could silently pass for the wrong reason.
    let dir = tempfile::tempdir().unwrap();
    let mut env_file = std::fs::File::create(dir.path().join(".env")).unwrap();
    env_file
        .write_all(
            b"CLICKHOUSE_CLOUD_API_KEY=dotenv-key\nCLICKHOUSE_CLOUD_API_SECRET=dotenv-secret\n",
        )
        .unwrap();
    drop(env_file);

    let url = mock.uri();
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .current_dir(dir.path())
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
    let cwd = tempfile::tempdir().unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", cwd.path())
        .current_dir(cwd.path())
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
        // A service's singleton settings, and a Postgres service's
        // observability, name the missing parent (#1315).
        (
            vec!["service", "upgrade-window", "get", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["service", "replica-aware-routing", "get", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["service", "scaling-schedule", "get", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["service", "private-endpoint", "get-config", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["service", "snapshot", "config", "get", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["backup", "bucket", "get", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec!["service", "backup-config", "get", SERVICE],
            &[],
            service_list.clone(),
        ),
        (
            vec![
                "postgres",
                "metrics",
                SERVICE,
                "--from-date",
                "2026-04-16T12:00:00Z",
                "--to-date",
                "2026-04-16T13:00:00Z",
            ],
            &[],
            format!("clickhousectl cloud postgres list --org-id {org}"),
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

/// A 404 from a service singleton or a Postgres subresource whose parent
/// exists is the subresource's own answer, so it is reported unchanged
/// after the confirming read of the parent (#1315).
#[tokio::test]
async fn subresource_404_under_an_existing_parent_keeps_the_original_error() {
    const SERVICE: &str = "11111111-2222-4333-8444-555555555555";
    let org = LOOKUP_ORG_ID;
    let service_path = format!("/v1/organizations/{org}/services/{SERVICE}");
    let postgres_path = format!("/v1/organizations/{org}/postgres/{SERVICE}");
    for (args, parent_path, leaf_path) in [
        (
            vec!["service", "upgrade-window", "get", SERVICE],
            service_path.clone(),
            format!("{service_path}/upgradeWindow"),
        ),
        (
            vec![
                "postgres",
                "metrics",
                SERVICE,
                "--from-date",
                "2026-04-16T12:00:00Z",
                "--to-date",
                "2026-04-16T13:00:00Z",
            ],
            postgres_path.clone(),
            format!("{postgres_path}/metrics"),
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(parent_path.clone()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"result": {"id": SERVICE, "name": "existing"}}),
                ),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let mut cli_args = args.clone();
        cli_args.extend(["--org-id", org]);

        let output = invoke_cli_with_cloud_credentials(&mock, &cli_args);

        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let error = cloud_runtime_error(&output);
        assert_eq!(error["code"], "http_4xx", "{args:?}: {error}");
        let requests = received_request_shape(&mock).await;
        assert_eq!(requests.len(), 2, "{args:?}: {requests:?}");
        assert_eq!(requests[0].0, "GET");
        assert!(requests[0].1.starts_with(&leaf_path), "{requests:?}");
        assert_eq!(requests[1], ("GET".to_string(), parent_path), "{args:?}");
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

// ── All Cloud runtime errors use the JSON envelope (issue #825) ────────────

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
        assert_eq!(cloud_runtime_error(&missing_auth)["code"], "auth_required");

        let validation = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["auth", "signup", "--url", "invalid-url"])
            .output()
            .unwrap();
        assert_eq!(validation.status.code(), Some(1));
        assert!(validation.stdout.is_empty());
        assert_eq!(cloud_runtime_error(&validation)["code"], "other");

        let transport = cloud_runtime_error_command(dir.path(), &closed_url, mode)
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(transport.status.code(), Some(1));
        assert!(transport.stdout.is_empty());
        assert_eq!(cloud_runtime_error(&transport)["code"], "transport");

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
    assert_eq!(cloud_runtime_error(&output)["code"], "sql_error");
    let message = cloud_runtime_error(&output)["message"]
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
