//! `cloud service query` and `cloud service client`: auth modes, first-use key binding, waking and
//! timeouts (src/cloud/services/query.rs, query_key.rs, native_client.rs).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Service query auth modes (issue #247) ──────────────────────────────────
//
// `cloud service query` has two auth paths:
//   - API key auth: the active API key is tried directly, and bound to the
//     endpoint only if the endpoint rejects it (#1043).
//   - OAuth: the user's own bearer token is sent directly to the query host
//     — no key lookup and, crucially, NO binding calls (the endpoint upsert
//     needs write access an OAuth token doesn't have).
// Both tests run the binary against two mocks: one impersonating the
// control plane (service lookup), one impersonating the query host (wired
// up via CLICKHOUSE_CLOUD_QUERY_HOST, which overrides host derivation).

async fn start_mock_query_host() -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        TEST_SERVICE_ID.to_string(),
        "--org-id".to_string(),
        "org-1".to_string(),
        "--query".to_string(),
        "SELECT 1".to_string(),
    ];
    args.extend(extra_args.iter().map(|arg| (*arg).to_string()));

    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        TEST_SERVICE_ID.to_string(),
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
    command.isolated_env();
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
    // The same refusal as data present at startup: the flag and the way out.
    assert!(stderr.contains("--query"), "{stderr}");
    assert!(stderr.contains("--queries-file -"), "{stderr}");
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
        "clickhousectl cloud service client <service-id> --query",
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
        vec!["--id", TEST_SERVICE_ID],
        vec![TEST_SERVICE_ID],
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
        command.isolated_env();
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
        command.isolated_env();
        command
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path())
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
                TEST_SERVICE_ID,
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
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            TEST_SERVICE_ID,
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
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path())
        .args([
            "cloud",
            "--url",
            &control.uri(),
            "service",
            "query",
            "--id",
            TEST_SERVICE_ID,
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

/// A per-service key stored by an earlier version is ignored (#1325): the
/// query authenticates with the management key, the record is dropped while
/// another service's record is kept, and stderr names the key that is left.
#[tokio::test]
async fn a_stored_query_key_record_is_ignored_dropped_and_reported() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;

    let dir = tempfile::tempdir().unwrap();
    let preserved = write_preserved_query_credentials(dir.path());
    let mut creds = preserved.clone();
    creds["service_query_keys"][TEST_SERVICE_ID] = serde_json::json!({
        "organization_id": "org-1",
        "api_key_id": "stored-api-key-uuid",
        "pending_cleanup_api_key_ids": ["retired-api-key-uuid"],
        "key_id": "stored-key-id",
        "key_secret": "stored-key-secret",
        "endpoint_id": "ep-1",
        "service_name": "demo",
        "created_at": "2026-05-11T12:00:00Z",
    });
    let creds_path = dir.path().join(".clickhouse/credentials.json");
    std::fs::write(&creds_path, serde_json::to_vec(&creds).unwrap()).unwrap();
    std::fs::create_dir(dir.path().join("home")).unwrap();

    let output = service_query_process(dir.path(), &control, &query_host)
        .arg("--json")
        .output()
        .await
        .expect("failed to spawn clickhousectl");
    assert_success(&output);
    assert_eq!(output.stdout, b"1\n");

    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    assert_eq!(
        query_requests[0].headers.get("authorization").unwrap(),
        query_test_basic_auth("fake-key-for-tests:fake-secret-for-tests").as_str()
    );
    assert!(
        control
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| request.method == wiremock::http::Method::GET),
        "an ignored record must not trigger any control-plane write"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    for id in ["stored-api-key-uuid", "retired-api-key-uuid"] {
        assert!(
            stderr.contains(&format!(
                "clickhousectl cloud key delete {id} --org-id org-1"
            )),
            "{stderr}"
        );
    }
    assert!(!stderr.contains("stored-key-secret"), "{stderr}");
    let stored: Value = serde_json::from_slice(&std::fs::read(&creds_path).unwrap()).unwrap();
    assert_eq!(stored, preserved);
}

// ── Key propagation (issue #658) ────────────────────────────────────────────
//
// The endpoint upsert can reject a key with
// `400 OpenAPI key <id> does not belong to the organization` while
// `GET /keys/{id}` already returns it, so the first-use bind of the caller's
// key waits that out: the upsert alone is retried, with the same body, inside
// a bounded window; the notice below is printed once; a success ends the
// wait. The condition is structural (a typed `Error::Api` with status 400),
// never the message text.

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

#[tokio::test]
async fn first_use_bind_waits_for_the_endpoint_to_accept_the_caller_key() {
    // A transient 400 from the first-use upsert is waited out.
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

/// A service with a stored record of its own in the credentials file, which
/// a query of another service leaves untouched.
const PRESERVED_QUERY_SERVICE_ID: &str = "preserved-service";

/// Shortens the 120 s readiness window for tests that wait all of it out.
const QUERY_READINESS_TIMEOUT_ENV: &str = "CHCTL_QUERY_READINESS_TIMEOUT_MS";
const SHORT_QUERY_READINESS_MS: &str = "2000";

async fn control_plane_requests_to(control: &MockServer, path: &str) -> usize {
    control
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == path)
        .count()
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

const OTHER_ORG_ID: &str = "5fae43a3-1043-4000-8000-0000000000f2";
const CALLER_KEY_UUID: &str = "cccccccc-1043-4000-8000-00000000ca11";
const BOUND_KEY_A: &str = "aaaaaaaa-1043-4000-8000-00000000000a";
const BOUND_KEY_B: &str = "bbbbbbbb-1043-4000-8000-00000000000b";
const DELETED_BOUND_KEY: &str = "dddddddd-1043-4000-8000-00000000000d";
const UNVERIFIABLE_BOUND_KEY: &str = "eeeeeeee-1043-4000-8000-00000000000e";

fn first_use_endpoint_path() -> String {
    format!("/v1/organizations/{FIRST_USE_ORG_ID}/services/{TEST_SERVICE_ID}/serviceQueryEndpoint")
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
        .and(header("authorization", caller_auth.as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
        .up_to_n_times(3)
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
            .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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

/// The JSON envelope after the bind notice that precedes it on stderr.
fn structured_error(output: &std::process::Output) -> Value {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let object = &stderr[stderr.find('{').unwrap_or(0)..];
    serde_json::from_str(object.trim())
        .unwrap_or_else(|e| panic!("stderr does not end in one JSON object ({e}): {stderr}"))
}

/// A key without write access cannot bind itself (#1325): the `403` keeps
/// exit 1 (it is not an auth failure), nothing else is written, and the JSON
/// error names the key and the endpoint to inspect.
#[tokio::test]
async fn a_forbidden_first_use_bind_exits_1_with_its_own_code() {
    let control = start_mock_control_plane_with_service().await;
    mount_whoami(&control, caller_api_key_identity(FIRST_USE_ORG_ID)).await;
    mount_first_use_endpoint_get(&control, Some(serde_json::json!([]))).await;
    mount_first_use_endpoint_upsert(
        &control,
        ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": "FORBIDDEN",
            "status": 403,
            "requestId": "stub-endpoint-upsert",
        })),
    )
    .await;
    let query_host = start_query_host_rejecting_the_first_query().await;
    let (project, output) = run_first_use_query(&control, &query_host, &["--json"]).await;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = &structured_error(&output)["error"];
    assert_eq!(error["code"], "query_key_bind_forbidden");
    assert_eq!(error["api_key_id"], CALLER_KEY_UUID);
    assert_eq!(
        error["command"],
        format!(
            "clickhousectl cloud service query-endpoint get {TEST_SERVICE_ID} --org-id \
             {FIRST_USE_ORG_ID}"
        )
    );
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
    let query_path = format!("/service/{TEST_SERVICE_ID}/run");
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
    let query_path = format!("/service/{TEST_SERVICE_ID}/run");
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
            "`clickhousectl cloud service start {TEST_SERVICE_ID} --org-id {FIRST_USE_ORG_ID}`"
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

async fn invoke_oauth_service_query_error(body: &str) -> std::process::Output {
    let control = start_mock_control_plane_with_service().await;
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
            TEST_SERVICE_ID,
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
        .and(header("wake-service", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            TEST_SERVICE_ID,
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
    // told about the wake on stderr, by service name.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("demo"), "{stderr}");
}

#[tokio::test]
async fn service_query_fails_with_start_hint_when_service_is_stopped() {
    let control = start_mock_control_plane_with_service().await;

    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
            TEST_SERVICE_ID,
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
            "`clickhousectl cloud service start {TEST_SERVICE_ID} --org-id org-1`"
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
            "id": TEST_SERVICE_ID,
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
                "/v1/organizations/{org_id}/services/{TEST_SERVICE_ID}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(stub_service.clone()))
            .mount(&mock)
            .await;
    }
    mock
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
    // The rerun names the same service and org; it needs no password.
    assert!(
        stderr.contains(&format!(
            "clickhousectl cloud service client {TEST_SERVICE_ID} --org-id org-1 \
             --query '<your SQL>'"
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
            "clickhousectl cloud service client {TEST_SERVICE_ID} --org-id org-1 \
             --query '<your SQL>'"
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

// ── cloud service client (#1154) ───────────────────────────────────────────

/// `cloud service client` looks the service's native endpoint up with the
/// Cloud credentials, then execs the installed `clickhouse client` with that
/// host and port, `--secure --login`, the queries, and the passthrough last.
/// The fake client prints its argv, so the handoff is observed exactly; no
/// credential reaches it on the command line.
#[tokio::test]
async fn service_client_execs_the_installed_client_against_the_native_endpoint() {
    let control = start_mock_control_plane_with_native_endpoint().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let client = home.join(".clickhouse/versions/25.12.1.649/clickhouse");
    std::fs::create_dir_all(client.parent().unwrap()).unwrap();
    std::fs::write(&client, "#!/bin/sh\nprintf '%s\\0' \"$@\"\n").unwrap();
    std::fs::set_permissions(&client, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();

    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args(["cloud", "--url", &control.uri(), "--org-id", "org-1"])
        .args(["service", "client", TEST_SERVICE_ID, "-q", "SELECT 1"])
        .args(["--", "--format", "CSV"])
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);
    let argv: Vec<&str> = std::str::from_utf8(&output.stdout)
        .unwrap()
        .strip_suffix('\0')
        .expect("NUL-terminated argv")
        .split('\0')
        .collect();
    assert_eq!(
        argv,
        [
            "client",
            "--host",
            QUERY_TEST_NATIVE_HOST,
            "--port",
            "9440",
            "--secure",
            "--login",
            "--query",
            "SELECT 1",
            "--format",
            "CSV",
        ]
    );
    for secret in ["fake-key-for-tests", "fake-secret-for-tests"] {
        assert!(!argv.contains(&secret));
    }

    let help = permission_help(&control, &["service", "client"], "--help");
    assert_observed_permissions_advertised(&control, &help).await;
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
            format!("clickhousectl cloud service get {TEST_SERVICE_ID} --org-id {org_id}")
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
            "`clickhousectl cloud service get {TEST_SERVICE_ID} --org-id {org_id}`"
        )),
        "{message}"
    );
    assert!(!message.contains("clickhouse client"), "{message}");
    assert!(!message.contains("system.processes"), "{message}");
    for sensitive in ["SELECT 826", "INSERT INTO", "test-bearer-token"] {
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
        for auth in ["management", "oauth"] {
            for json in [false, true] {
                let control = start_mock_control_plane_with_query_state(state).await;
                let query_host = MockServer::start().await;
                Mock::given(method("POST"))
                    .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
    for auth in ["management", "oauth"] {
        for json in [false, true] {
            // The newer pre-execution idle response overrides this earlier
            // running snapshot for diagnosis, without another service GET.
            let control = start_mock_control_plane_with_query_state(Some("running")).await;
            let query_host = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(401).set_body_string("not authorized"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{TEST_SERVICE_ID}/run")))
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
                == format!("/v1/organizations/{FIRST_USE_ORG_ID}/services/{TEST_SERVICE_ID}")
        })
        .count();
    assert_eq!(service_reads, 1);
}

#[tokio::test]
async fn query_on_idle_service_can_succeed_without_a_wake_confirmation() {
    for auth in ["management", "oauth"] {
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

    assert_query_http_error(&output, 500, r#"{"error":"Internal error."}"#);
    let stderr = String::from_utf8_lossy(&output.stderr);
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
