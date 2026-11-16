//! Shared helpers: the isolated CLI spawn, credential fixtures, mock mounting and output parsing.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Inspect the real help path without ambient credentials or a writable real home.
pub(crate) fn permission_help(mock: &MockServer, command: &[&str], help_flag: &str) -> String {
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

pub(crate) fn advertised_permission_ids(help: &str) -> std::collections::BTreeSet<&str> {
    help.split(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, ':' | '_' | '-'))
        .filter(|word| word.starts_with("control-plane:"))
        .collect()
}

pub(crate) fn permission_path_matches(template: &str, actual: &str) -> bool {
    let template: Vec<_> = template.split('/').collect();
    let actual: Vec<_> = actual.split('/').collect();
    template.len() == actual.len()
        && template.iter().zip(actual).all(|(expected, actual)| {
            *expected == actual
                || (expected.starts_with('{') && expected.ends_with('}') && !actual.is_empty())
        })
}

/// Check observed HTTP calls rather than duplicating the command's declared list.
pub(crate) async fn assert_observed_permissions_advertised(mock: &MockServer, help: &str) {
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

/// Locate the `clickhousectl` binary. cargo populates `CARGO_BIN_EXE_<name>`
/// for integration tests in the same package — so this is just the absolute
/// path to the build output, no `cargo build` shellout needed.
pub(crate) fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// Assert the binary exited zero, panicking with the captured stderr/stdout
/// so the failure cause is visible in the test output.
pub(crate) fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "clickhousectl exited {}\nstderr:\n{}\nstdout:\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

/// Split the `next` polling hint (#1162) off a command's JSON output, leaving
/// the API response the command printed around it.
pub(crate) fn take_next(stdout: &[u8]) -> (Value, Option<Value>) {
    let mut value: Value = serde_json::from_slice(stdout).expect("stdout should be JSON");
    let next = value
        .as_object_mut()
        .and_then(|object| object.remove("next"));
    (value, next)
}

/// Turns off the CLI's background update check, which would otherwise send
/// a real request to the release server and wait up to 400 ms for it.
pub(crate) const UPDATE_CHECK_TIMEOUT_ENV: &str = "CHCTL_UPDATE_CHECK_TIMEOUT_MS";

/// `env_clear`, keeping only what every spawned CLI needs: no background
/// update check (see [`UPDATE_CHECK_TIMEOUT_ENV`]). Every spawn in this binary
/// starts from it and sets `HOME` to a directory the test owns, so a
/// developer's credentials, proxies and coding-agent variables never reach the
/// child.
pub(crate) trait IsolatedEnv {
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

/// Run the clickhousectl binary against the mock, returning the JSON body
/// the binary POSTed. Panics with the captured stderr if the binary exits
/// non-zero — a failure here is almost always a clap-parsing error, which
/// is itself a bug worth surfacing.
pub(crate) async fn invoke_cli_capture_body(mock: &MockServer, cli_args: &[&str]) -> Value {
    let mut full_args: Vec<&str> = vec!["cloud", "--url"];
    let url = mock.uri();
    full_args.push(&url);
    full_args.push("--json");
    full_args.extend(cli_args);

    // DO_NOT_TRACK (here and on every other spawn in this binary) keeps the
    // binary's telemetry fully silent: no `~/.clickhouse/telemetry.json` write
    // in the developer's real home, no POST to the production endpoint.
    // An owned cwd keeps project discovery away from any `.clickhouse/` above
    // the checkout.
    let cwd = tempfile::tempdir().unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", cwd.path())
        .current_dir(cwd.path())
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

pub(crate) async fn invoke_cli_capture_body_with_stdin(
    mock: &MockServer,
    cli_args: &[&str],
    stdin: &[u8],
) -> Value {
    let mut full_args: Vec<&str> = vec!["cloud", "--url"];
    let url = mock.uri();
    full_args.push(&url);
    full_args.push("--json");
    full_args.extend(cli_args);

    let cwd = tempfile::tempdir().unwrap();
    let mut child = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", cwd.path())
        .current_dir(cwd.path())
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

pub(crate) fn invoke_cli_with_cloud_credentials(
    mock: &MockServer,
    cli_args: &[&str],
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url, "--json"];
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

pub(crate) fn invoke_cli_with_cloud_credentials_and_stdin(
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

pub(crate) fn invoke_cli_with_cloud_credentials_human(
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

pub(crate) const BASIC_TEST_AUTH: &str =
    "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==";

/// A saved key pair for `api_url`, the mock the test sends it to (#1202).
pub(crate) fn write_project_api_credentials(root: &Path, key: &str, secret: &str, api_url: &str) {
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

pub(crate) fn manual_query_endpoint_response(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result, "status": 200, "requestId": "stub-query-endpoint-get"
    }))
}

pub(crate) const TEST_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";

/// The HTTP methods and paths the mock received, in order.
pub(crate) async fn received_request_shape(mock: &MockServer) -> Vec<(String, String)> {
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

pub(crate) fn successful_delete_response(request_id: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": request_id,
    }))
}

pub(crate) fn postgres_args_minimal() -> Vec<String> {
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

pub(crate) async fn start_mock_control_plane_with_service() -> MockServer {
    let mock = MockServer::start().await;
    let stub_service = serde_json::json!({
        "result": { "id": TEST_SERVICE_ID, "name": "demo" },
        "status": 200,
        "requestId": "stub-service-get",
    });
    let stub_services = serde_json::json!({
        "result": [{ "id": TEST_SERVICE_ID, "name": "demo" }],
        "status": 200,
        "requestId": "stub-service-list",
    });
    // The first-use bind tests resolve a UUID organization, as `whoami`
    // reports one (#1043); everything else uses `org-1`.
    for org_id in ["org-1", FIRST_USE_ORG_ID] {
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/{org_id}/services/{TEST_SERVICE_ID}"
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

pub(crate) fn service_query_process(
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
) -> tokio::process::Command {
    service_query_process_with_sql(project_dir, control, query_host, "SELECT 1")
}

pub(crate) fn service_query_process_with_sql(
    project_dir: &Path,
    control: &MockServer,
    query_host: &MockServer,
    sql: &str,
) -> tokio::process::Command {
    service_query_process_in_org(project_dir, control, query_host, "org-1", sql)
}

pub(crate) fn service_query_process_in_org(
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
            TEST_SERVICE_ID,
            "--org-id",
            org_id,
            "--query",
            sql,
        ]);
    command
}

pub(crate) const FIRST_USE_ORG_ID: &str = "5fae43a3-1043-4000-8000-0000000000f1";

/// Write an OAuth tokens.json into `ch_dir` (the caller's `$HOME/.clickhouse`)
/// so the binary authenticates with a bearer token against the given control
/// plane. Callers must also set `HOME` to the parent of `ch_dir`.
pub(crate) fn write_oauth_tokens(ch_dir: &std::path::Path, control_uri: &str) {
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

pub(crate) const CDC_SCALING_PATH: &str =
    "/v1/organizations/org-1/services/svc-1/clickpipesCdcScaling";

pub(crate) fn cdc_scaling_envelope(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-cdc-scaling",
    }))
}

/// Run `cloud service query` with API-key auth against a query host that
/// fails with `status`/`body`. Returns the process output and the query host,
/// so a test can count how many times the statement was actually sent.
pub(crate) async fn invoke_service_query_against_failing_query_host(
    control: &MockServer,
    status: u16,
    body: &str,
    extra_args: &[&str],
) -> (std::process::Output, MockServer) {
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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

/// Run the binary with credentials but *without* `--json`, so human output can
/// be asserted. `invoke_cli_with_cloud_credentials` always adds `--json`.
pub(crate) fn invoke_cli_human(mock: &MockServer, cli_args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let url = mock.uri();
    let mut args = vec!["cloud", "--url", &url];
    args.extend(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    // Agent detection turns on `--json` on its own, so the inherited
    // environment has to go for human output to be observable at all.
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args);
    command.output().expect("failed to spawn clickhousectl")
}

/// A well-formed UUID that no resource has. All-zero is still a syntactically
/// valid UUID.
pub(crate) const UNKNOWN_UUID: &str = "00000000-0000-0000-0000-000000000000";
pub(crate) const LOOKUP_ORG_ID: &str = "00000000-0000-4000-8000-000000000001";

/// The 400 the API really answers for a well-formed id it cannot resolve.
pub(crate) fn invalid_id_400_response(thing: &str, id: &str) -> ResponseTemplate {
    ResponseTemplate::new(400).set_body_json(serde_json::json!({
        "status": 400,
        "error": format!("BAD_REQUEST: Invalid {thing} id string:\"{id}\""),
        "requestId": "stub-invalid-id",
    }))
}

pub(crate) async fn mount_invalid_id_400(
    mock: &MockServer,
    api_path: String,
    thing: &str,
    id: &str,
) {
    Mock::given(method("GET"))
        .and(path(api_path))
        .respond_with(invalid_id_400_response(thing, id))
        .mount(mock)
        .await;
}

pub(crate) fn unknown_postgres_path(suffix: &str) -> String {
    format!("/v1/organizations/{LOOKUP_ORG_ID}/postgres/{UNKNOWN_UUID}{suffix}")
}

/// The not-found message `postgres get` prints for [`UNKNOWN_UUID`]: it names
/// the ID and organization and ends in the rejected request's own server
/// message.
pub(crate) fn assert_postgres_not_found_message(message: &str, server_message: &str) {
    assert!(message.contains(UNKNOWN_UUID), "{message}");
    assert!(message.contains(LOOKUP_ORG_ID), "{message}");
    assert!(message.ends_with(server_message), "{message}");
}

pub(crate) fn assert_postgres_not_found_human(output: &std::process::Output, server_message: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr
        .strip_prefix("Error: ")
        .and_then(|rest| rest.strip_suffix('\n'))
        .unwrap_or_else(|| panic!("one error line: {stderr}"));
    assert_postgres_not_found_message(message, server_message);
}

/// The write-path response, plus for a restore the ID, tags and (blocked)
/// rules the CLI reads to decide whether the restored rules need a PATCH.
pub(crate) fn restored_service_response(endpoint: &str, result: &Value) -> Value {
    let mut result = result.clone();
    if endpoint.ends_with("/restoredService") {
        result["id"] = serde_json::json!("22222222-3333-4444-5555-666666666666");
        result["tags"] = serde_json::json!([]);
        result["ipAccessList"] = serde_json::json!([]);
    }
    result
}

/// The digits in `text`, in order: for messages whose only numbers are counts.
pub(crate) fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

pub(crate) fn query_api_endpoint_list_page(
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

pub(crate) fn cloud_runtime_error(output: &std::process::Output) -> Value {
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

/// A clean environment makes explicit, automatically detected, and human
/// output observable independently. Each invocation gets isolated credentials.
pub(crate) fn cloud_runtime_error_command(root: &Path, url: &str, mode: &str) -> Command {
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

pub(crate) fn custom_oauth_command(root: &Path, api_url: &str) -> Command {
    let mut command = cloud_runtime_error_command(root, api_url, "explicit");
    command
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET");
    command
}

pub(crate) const WHOAMI_ORG_ID: &str = "55555555-6666-4777-8888-999999999999";

/// Run a cloud command with API-key flags, or with saved OAuth tokens only.
pub(crate) fn invoke_cloud_as(
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
    command.isolated_env();
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

pub(crate) fn whoami_api_key_identity() -> Value {
    serde_json::json!({
        "actorType": "apiKey",
        "keyId": "key-123",
        "name": "ci deploy",
        "organizationId": WHOAMI_ORG_ID
    })
}

pub(crate) fn whoami_envelope(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "whoami",
        "result": result
    }))
}

pub(crate) const PAGED_ORG_FIRST: &str = "00000000-0000-0000-0000-0000000000a1";
pub(crate) const PAGED_ORG_SECOND: &str = "00000000-0000-0000-0000-0000000000a2";

pub(crate) async fn mount_cursor_page(
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
