//! Telemetry end-to-end tests (issue #283).
//!
//! Each test invokes the real `clickhousectl` binary as a subprocess with
//! `HOME` pointed at a temp dir (sandboxing `~/.clickhouse/telemetry.json`)
//! and `CHCTL_TELEMETRY_URL` pointed at a local `wiremock` server, then
//! asserts on the consent flow (notice/marker/silence) and on the recorded
//! payload shape — in particular that flag and positional *values* never
//! appear on the wire, while positional presence does (#480).
//!
//! The send happens in a detached child process, so tests that expect an
//! event poll the mock briefly; tests that expect *no* event give the
//! (nonexistent) child a moment before asserting zero requests.
//!
//!     cargo test -p clickhousectl --test telemetry_test

#![cfg(feature = "telemetry")]

use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// A sandboxed home directory plus the telemetry ingest mock.
struct Sandbox {
    home: tempfile::TempDir,
    /// The default working directory, so project discovery never reaches a
    /// `.clickhouse/` above the checkout. Callers may override it.
    cwd: tempfile::TempDir,
    mock: MockServer,
    endpoint_path: String,
}

impl Sandbox {
    async fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let endpoint_path = format!(
            "/v1/telemetry/{}",
            home.path().file_name().unwrap().to_string_lossy()
        );
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(endpoint_path.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&mock)
            .await;
        Sandbox {
            home,
            cwd: tempfile::tempdir().unwrap(),
            mock,
            endpoint_path,
        }
    }

    fn telemetry_url(&self) -> String {
        format!("{}{}", self.mock.uri(), self.endpoint_path)
    }

    fn state_path(&self) -> PathBuf {
        self.home.path().join(".clickhouse").join("telemetry.json")
    }

    fn write_state(&self, disabled: bool) {
        let dir = self.state_path();
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::write(&dir, format!(r#"{{"disabled":{disabled}}}"#)).unwrap();
    }

    /// Run the binary sandboxed: `HOME` at the temp dir, telemetry pointed at
    /// the mock, and inherited settings cleared for determinism (the harness
    /// itself may run under CI or a coding agent). Preserve executable lookup.
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(clickhousectl_binary());
        cmd.args(args)
            .current_dir(self.cwd.path())
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", self.home.path())
            .env("CHCTL_TELEMETRY_URL", self.telemetry_url())
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0");
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("failed to spawn binary")
    }

    /// Run the binary with its stderr attached to a pipe whose read end is
    /// already closed, so every stderr write in the child fails. A panic on
    /// such a write would surface as exit code 101.
    fn run_with_closed_stderr(&self, args: &[&str]) -> Output {
        let (reader, writer) = std::io::pipe().expect("failed to create pipe");
        drop(reader);
        self.command(args)
            .stderr(writer)
            .output()
            .expect("failed to spawn binary")
    }

    /// Poll until the mock has seen `n` requests; panics after ~5s. The send
    /// child is detached, so arrival is asynchronous.
    async fn wait_for_requests(&self, n: usize) -> Vec<Value> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let requests = self.received_requests().await;
            if requests.len() >= n {
                return requests
                    .iter()
                    .map(|r| serde_json::from_slice(&r.body).expect("payload must be JSON"))
                    .collect();
            }
            assert!(
                Instant::now() < deadline,
                "telemetry event did not arrive within 5s (saw {})",
                requests.len()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Assert the mock saw no requests, giving a hypothetical stray child a
    /// moment to fire first.
    async fn assert_no_requests(&self) {
        assert_no_requests_in(&[self]).await;
    }

    async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.mock
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|request| request.url.path() == self.endpoint_path.as_str())
            .collect()
    }
}

/// How long a hypothetical stray send child is given to fire before a test
/// concludes it never will. The child is detached, so only waiting can show
/// it stayed silent.
const STRAY_EVENT_GRACE: Duration = Duration::from_millis(750);

/// [`Sandbox::assert_no_requests`] for several sandboxes behind one wait, so
/// a test that runs many commands sleeps once, not once per command.
async fn assert_no_requests_in(sandboxes: &[&Sandbox]) {
    tokio::time::sleep(STRAY_EVENT_GRACE).await;
    for sandbox in sandboxes {
        let requests = sandbox.received_requests().await;
        assert!(
            requests.is_empty(),
            "expected no telemetry, saw: {:?}",
            requests.iter().map(|r| &r.body).collect::<Vec<_>>()
        );
    }
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Whether stderr carries the first-run notice, recognised by the opt-out
/// command it names rather than by its wording.
fn has_first_run_notice(output: &Output) -> bool {
    stderr_of(output).contains("`clickhousectl telemetry disable`")
}

/// Run independent invocations at once, so a table test costs about one
/// invocation's latency rather than one per row.
fn outputs_in_parallel(commands: Vec<Command>) -> Vec<Output> {
    std::thread::scope(|scope| {
        let runs: Vec<_> = commands
            .into_iter()
            .map(|mut command| scope.spawn(move || command.output().expect("spawn binary")))
            .collect();
        runs.into_iter().map(|run| run.join().unwrap()).collect()
    })
}

/// One enabled sandbox per table row, each row's command run in parallel
/// from `cwd`.
async fn run_rows(rows: &[&[&str]], cwd: &std::path::Path) -> Vec<(Sandbox, Output)> {
    let mut sandboxes = Vec::new();
    for _ in rows {
        let sandbox = Sandbox::new().await;
        sandbox.write_state(false);
        sandboxes.push(sandbox);
    }
    let outputs = outputs_in_parallel(
        rows.iter()
            .zip(&sandboxes)
            .map(|(args, sandbox)| {
                let mut command = sandbox.command(args);
                command.current_dir(cwd);
                command
            })
            .collect(),
    );
    sandboxes.into_iter().zip(outputs).collect()
}

// ---------------------------------------------------------------------------

#[tokio::test]
async fn sandbox_ignores_requests_for_another_endpoint_path() {
    let sandbox = Sandbox::new().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/telemetry/stale", sandbox.mock.uri()))
        .body("{}")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    assert_eq!(sandbox.mock.received_requests().await.unwrap().len(), 1);
    assert!(sandbox.received_requests().await.is_empty());
}

#[tokio::test]
async fn first_run_sends_nothing_then_second_run_sends_event() {
    let sandbox = Sandbox::new().await;

    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());

    assert!(
        has_first_run_notice(&output) && stderr_of(&output).contains("DO_NOT_TRACK=1"),
        "first run must print the notice, got stderr: {}",
        stderr_of(&output)
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":false}"#
    );
    sandbox.assert_no_requests().await;

    // Second run: the notice appears exactly once, ever.
    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());
    assert!(!has_first_run_notice(&output));
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(payloads[0]["command"], "local list");
}

#[tokio::test]
async fn enabled_run_sends_payload_with_expected_shape() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local list");
    assert!(event["flags"].as_array().unwrap().is_empty());
    assert!(event["positionals"].as_array().unwrap().is_empty());
    assert_eq!(event["exit_code"], 0);
    // Whether an agent is detected depends on the harness environment; pin
    // that the two fields exist and agree (one detection feeds both).
    assert!(event["is_agent"].is_boolean());
    assert_eq!(
        event["is_agent"].as_bool().unwrap(),
        event["agent"].is_string(),
        "is_agent and agent must come from the same detection: {event}"
    );
    assert_eq!(event["ci"], false);
    assert_eq!(event["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(event["os"], std::env::consts::OS);
    assert_eq!(event["arch"], std::env::consts::ARCH);
    assert!(
        event.get("failure_kind").is_none(),
        "a successful run carries no failure kind: {event}"
    );

    // The send deliberately bypasses http::client_builder() (see
    // no_agent_correlation_headers_on_the_wire) but keeps the same canonical
    // User-Agent as every other outbound request. The ingest worker relies on
    // the `clickhousectl/<version>` prefix to reject non-CLI traffic (an
    // ` (agent=...)` comment may follow).
    let requests = sandbox.received_requests().await;
    let ua = requests[0]
        .headers
        .get("user-agent")
        .expect("telemetry POST must carry a User-Agent")
        .to_str()
        .unwrap();
    let prefix = format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"));
    assert!(
        ua == prefix || ua.starts_with(&format!("{prefix} (")),
        "unexpected User-Agent: {ua}"
    );
}

#[tokio::test]
async fn no_agent_correlation_headers_on_the_wire() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    // Simulate running under Claude Code with a session id and an active
    // trace: this is exactly the environment in which the shared
    // http::client_builder() would attach `agent-session-id` and
    // `traceparent`. Telemetry must not — a stable session identifier on an
    // anonymous event would be fingerprinting. The agent facts travel in the
    // payload (`is_agent`/`agent`) instead, by design.
    let output = sandbox
        .command(&["local", "list"])
        .env_remove("AGENT")
        .env("CLAUDECODE", "1")
        .env("CLAUDE_CODE_SESSION_ID", "sess-should-never-hit-the-wire")
        .env(
            "TRACEPARENT",
            "00-11111111111111111111111111111111-2222222222222222-01",
        )
        .output()
        .unwrap();
    assert!(output.status.success());

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    // Detection was positive: the payload says so...
    assert_eq!(event["is_agent"], true);
    assert_eq!(event["agent"], "claude-code");

    // ...but no correlation headers accompany it.
    let requests = sandbox.received_requests().await;
    let headers = &requests[0].headers;
    assert!(
        headers.get("agent-session-id").is_none(),
        "telemetry POST must not carry agent-session-id: {headers:?}"
    );
    assert!(
        headers.get("traceparent").is_none(),
        "telemetry POST must not carry traceparent: {headers:?}"
    );
}

/// The `error.code` of a local `--json` error object on stderr.
fn json_error_code(output: &Output) -> String {
    let stderr = stderr_of(output);
    let value: Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|e| panic!("stderr is not one JSON error object ({e}):\n{stderr}"));
    value["error"]["code"]
        .as_str()
        .unwrap_or_else(|| panic!("no error.code in {value}"))
        .to_string()
}

/// A local failure's event carries the code of its `--json` error object as
/// `failure_kind`, whichever output mode the command ran in (#1063).
#[tokio::test]
async fn local_failure_kind_is_the_json_error_code() {
    let project = tempfile::tempdir().unwrap();
    let stop: &[&str] = &["local", "server", "stop", "no-such-server"];
    let remove: &[&str] = &["local", "remove", "99.99.99.99"];
    let stop_json = [stop, &["--json"]].concat();
    let remove_json = [remove, &["--json"]].concat();
    let rows = [stop_json.as_slice(), stop, remove_json.as_slice(), remove];
    let runs = run_rows(&rows, project.path()).await;

    // Rows come in (`--json`, human) pairs; the `--json` row's error object
    // names the code both events must carry.
    for (args, pair) in rows.chunks(2).zip(runs.chunks(2)) {
        let expected = json_error_code(&pair[0].1);
        for (argv, (sandbox, output)) in args.iter().zip(pair) {
            assert_eq!(output.status.code(), Some(1), "{argv:?}");
            let event = the_event(sandbox).await;
            assert_eq!(event["outcome"], "error", "{argv:?}");
            assert_eq!(
                event["failure_kind"],
                expected.as_str(),
                "{argv:?}: {event}"
            );
            assert!(event.get("failure_stage").is_none(), "{event}");
        }
    }
}

/// Cloud failures keep the Cloud vocabulary: a local code never lands on a
/// Cloud event.
#[tokio::test]
async fn cloud_failure_carries_no_local_error_code() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["cloud", "org", "list", "--json"]);
    assert!(!output.status.success());

    let event = &sandbox.wait_for_requests(1).await[0];
    assert_eq!(event["command"], "cloud org list");
    assert!(event.get("failure_kind").is_none(), "{event}");
}

#[tokio::test]
async fn managed_client_failure_details_never_reach_telemetry() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project-private-token");
    let server_name = "server-private-token";
    let version = "99.99.1-version-private-token";
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).unwrap();
    std::fs::write(
        servers.join(format!("{server_name}.json")),
        serde_json::to_vec(&serde_json::json!({
            "name": server_name,
            "pid": std::process::id(),
            "version": version,
            "http_port": 8123,
            "tcp_port": 9000,
            "started_at": "test",
            "cwd": project,
            "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .unwrap();

    let output = sandbox
        .command(&["local", "client", "--name", server_name])
        .current_dir(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let raw_message = stderr_of(&output);
    assert!(raw_message.contains(server_name));
    assert!(raw_message.contains(version));

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local client");
    assert_eq!(event["flags"], serde_json::json!(["name"]));
    assert_eq!(event["positionals"], serde_json::json!([]));
    assert_eq!(event["exit_code"], 1);
    let raw_payload = serde_json::to_string(event).unwrap();
    for sensitive in [
        server_name,
        version,
        "project-private-token",
        raw_message.as_str(),
    ] {
        assert!(
            !raw_payload.contains(sensitive),
            "managed client detail leaked into telemetry: {raw_payload}"
        );
    }
}

#[tokio::test]
async fn server_scope_failure_paths_never_reach_telemetry() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("server-project-private-token");
    let server_name = "server-name-private-token";
    std::fs::create_dir(&project).unwrap();

    let output = sandbox
        .command(&["local", "server", "stop", server_name])
        .current_dir(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let raw_message = stderr_of(&output);
    assert!(raw_message.contains(server_name));
    assert!(raw_message.contains("server-project-private-token"));

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local clickhouse stop");
    assert_eq!(event["positionals"], serde_json::json!(["name"]));
    assert_eq!(event["exit_code"], 1);
    let raw_payload = serde_json::to_string(event).unwrap();
    for sensitive in [
        server_name,
        "server-project-private-token",
        raw_message.as_str(),
    ] {
        assert!(
            !raw_payload.contains(sensitive),
            "server scope detail leaked into telemetry: {raw_payload}"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn child_exit_code_reaches_the_telemetry_tail_unchanged() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();

    let binary = sandbox
        .home
        .path()
        .join(".clickhouse/versions/25.12.9.61/clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    // 3 is clickhousectl's own cancellation code, so this also verifies that
    // a child status cannot be mistaken for a CLI cancellation.
    std::fs::write(&binary, "#!/bin/sh\nexit 3\n").unwrap();
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&binary, permissions).unwrap();

    let cache = sandbox.home.path().join(".clickhouse/last_update_check");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(cache, format!("{now}\n999.0.0")).unwrap();

    let output = sandbox
        .command(&[
            "local",
            "server",
            "start",
            "--version",
            "25.12.9.61",
            "--foreground",
        ])
        .env_clear()
        .env("HOME", sandbox.home.path())
        .env("CHCTL_TELEMETRY_URL", sandbox.telemetry_url())
        .current_dir(project.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        !stderr_of(&output).contains("Error: "),
        "child stderr should not gain a wrapper error: {}",
        stderr_of(&output)
    );
    assert!(
        stderr_of(&output).contains("`clickhousectl update`"),
        "child failure should still reach the update-notice tail: {}",
        stderr_of(&output)
    );
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(payloads[0]["command"], "local clickhouse start");
    assert_eq!(payloads[0]["exit_code"], 3);
    assert_eq!(payloads[0]["outcome"], "error");
}

// ---------------------------------------------------------------------------
// `exec()` handoffs (#471). `local client` replaces the process image, so its
// event is emitted by the pre-exec hook and is *censored*: `exec_attempt`
// proves the handoff was reached, never that the launch or the native client
// succeeded. These tests pin both halves of that contract — the deterministic
// launch failures are ordinary `error` events, the residual race is censored,
// and either way exactly one event is emitted and the shell keeps the real
// status.
// ---------------------------------------------------------------------------

const FAKE_VERSION: &str = "25.12.9.61";

/// Install a fake native client at `~/.clickhouse/versions/<version>/clickhouse`
/// in the sandboxed home, with the given contents and mode.
#[cfg(unix)]
fn install_fake_clickhouse(sandbox: &Sandbox, version: &str, contents: &str, mode: u32) -> PathBuf {
    let binary = sandbox
        .home
        .path()
        .join(".clickhouse/versions")
        .join(version)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::write(&binary, contents).unwrap();
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(mode);
    std::fs::set_permissions(&binary, permissions).unwrap();
    binary
}

/// `local client` on the direct-connect path (no server metadata needed),
/// selecting the fake binary by exact version.
///
/// The environment is cleared down to `HOME` and the ingest URL — as in
/// `child_exit_code_reaches_the_telemetry_tail_unchanged` — so agent detection
/// cannot flip the assertions from human text to the JSON envelope depending on
/// where the suite runs.
#[cfg(unix)]
fn run_local_client(sandbox: &Sandbox, project: &std::path::Path) -> Output {
    sandbox
        .command(&[
            "local",
            "client",
            "--version",
            FAKE_VERSION,
            "--host",
            "127.0.0.1",
            "--port",
            "9000",
        ])
        .env_clear()
        .env("HOME", sandbox.home.path())
        .env("CHCTL_TELEMETRY_URL", sandbox.telemetry_url())
        .current_dir(project)
        .output()
        .expect("run clickhousectl")
}

/// Wait for the event, then give a hypothetical second one time to arrive and
/// fail if it does: exactly one event per invocation is the contract, and the
/// handoff hook shares its guard with `main`'s tail. That guard is shared by
/// every handoff path, so one test waits for a second event: a failed
/// `exec()`, the one path where both the hook and the tail run. The others
/// take [`the_event`].
async fn exactly_one_event(sandbox: &Sandbox) -> Value {
    let payloads = sandbox.wait_for_requests(1).await;
    tokio::time::sleep(STRAY_EVENT_GRACE).await;
    let requests = sandbox.received_requests().await;
    assert_eq!(
        requests.len(),
        1,
        "expected exactly one telemetry event, saw {:?}",
        requests
            .iter()
            .map(|r| String::from_utf8_lossy(&r.body).into_owned())
            .collect::<Vec<_>>()
    );
    payloads.into_iter().next().unwrap()
}

/// The invocation's event, as soon as it arrives, and the only one so far.
async fn the_event(sandbox: &Sandbox) -> Value {
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(
        payloads.len(),
        1,
        "expected one telemetry event: {payloads:?}"
    );
    payloads.into_iter().next().unwrap()
}

/// What sits where `local client` expects the native binary.
#[cfg(unix)]
enum FakeClient {
    Script(&'static str, u32),
    Directory,
    /// Listed by version resolution, then gone before the pre-flight.
    Removed,
}

/// How the shell sees the `local client` process end.
#[cfg(unix)]
#[derive(Debug, PartialEq)]
enum Ended {
    Code(i32),
    Signal(i32),
    /// Any failure status; the shell, not clickhousectl, chooses it.
    Failure,
}

/// Every `local client` launch path, run in parallel, one sandbox each. A
/// handoff that was reached is a censored `exec_attempt` with a fixed 0 (it
/// proves the handoff, never that the native client succeeded), and the shell
/// keeps the child's own status. A launch the pre-flight refuses is an
/// ordinary `error` event with the process's exit code.
#[cfg(unix)]
#[tokio::test]
async fn local_client_launches_are_censored_handoffs_or_error_events() {
    use std::os::unix::process::ExitStatusExt;

    let cases = [
        (
            "success",
            FakeClient::Script("#!/bin/sh\nprintf 'child ran\\n'\nexit 0\n", 0o755),
            Ended::Code(0),
            "exec_attempt",
        ),
        (
            "child status",
            FakeClient::Script("#!/bin/sh\nprintf 'child ran\\n'\nexit 23\n", 0o755),
            Ended::Code(23),
            "exec_attempt",
        ),
        // Killed by a signal: no exit code at all, not a wrapper's
        // translation of it.
        (
            "signal",
            FakeClient::Script("#!/bin/sh\nkill -TERM $$\n", 0o755),
            Ended::Signal(15),
            "exec_attempt",
        ),
        // Executable, but not an executable format: `execvp` gets ENOEXEC
        // and re-execs it through `/bin/sh`, so the image is replaced and
        // the shell picks the failure status.
        (
            "bad format",
            FakeClient::Script("\u{0}\u{1}not a binary\n", 0o755),
            Ended::Failure,
            "exec_attempt",
        ),
        (
            "not executable",
            FakeClient::Script("#!/bin/sh\nexit 0\n", 0o644),
            Ended::Code(1),
            "error",
        ),
        // A directory satisfies the `exists()` resolution checks, so this
        // reaches the launch pre-flight.
        ("directory", FakeClient::Directory, Ended::Code(1), "error"),
        ("removed", FakeClient::Removed, Ended::Code(1), "error"),
    ];

    let mut sandboxes = Vec::new();
    for (_, fake, _, _) in &cases {
        let sandbox = Sandbox::new().await;
        sandbox.write_state(false);
        match fake {
            FakeClient::Script(contents, mode) => {
                install_fake_clickhouse(&sandbox, FAKE_VERSION, contents, *mode);
            }
            FakeClient::Directory => {
                std::fs::create_dir_all(
                    sandbox
                        .home
                        .path()
                        .join(".clickhouse/versions")
                        .join(FAKE_VERSION)
                        .join("clickhouse"),
                )
                .unwrap();
            }
            FakeClient::Removed => {
                let binary =
                    install_fake_clickhouse(&sandbox, FAKE_VERSION, "#!/bin/sh\nexit 0\n", 0o755);
                std::fs::remove_file(&binary).unwrap();
                std::fs::write(binary.parent().unwrap().join("clickhouse.bak"), "stub").unwrap();
            }
        }
        sandboxes.push(sandbox);
    }
    let project = tempfile::tempdir().unwrap();
    let outputs: Vec<Output> = std::thread::scope(|scope| {
        let runs: Vec<_> = sandboxes
            .iter()
            .map(|sandbox| scope.spawn(|| run_local_client(sandbox, project.path())))
            .collect();
        runs.into_iter().map(|run| run.join().unwrap()).collect()
    });

    for (((label, _, ended, outcome), sandbox), output) in
        cases.iter().zip(&sandboxes).zip(&outputs)
    {
        let status = output.status;
        let actual = match (status.code(), status.signal()) {
            (Some(code), _) if *ended == Ended::Failure && code != 0 => Ended::Failure,
            (Some(code), _) => Ended::Code(code),
            (None, Some(signal)) => Ended::Signal(signal),
            (None, None) => panic!("{label}: no status"),
        };
        assert_eq!(actual, *ended, "{label}: {}", stderr_of(output));
        if matches!(ended, Ended::Code(0) | Ended::Code(23)) {
            // The child inherited stdout: the image really was replaced.
            assert!(
                stdout_of(output).contains("child ran"),
                "{label}: {output:?}"
            );
        }
        let event = the_event(sandbox).await;
        assert_eq!(event["command"], "local client", "{label}");
        assert_eq!(event["outcome"], *outcome, "{label}");
        let expected_code = match ended {
            Ended::Code(code) if *outcome == "error" => *code,
            _ => 0,
        };
        assert_eq!(event["exit_code"], expected_code, "{label}");
    }
    // The refused non-executable launch says how to repair the install.
    let not_executable = &outputs[4];
    assert!(
        stderr_of(not_executable).contains("clickhousectl local install"),
        "{}",
        stderr_of(not_executable)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn the_native_client_keeps_this_process_and_its_stdio() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    // `$$` is the exec'd shell's PID. It equalling the PID the harness spawned
    // is the whole reason the handoff stays an `exec()` and its telemetry stays
    // censored: same process means same process group, same session and the
    // same controlling TTY, so job control, Ctrl-C and window resizes reach the
    // native client exactly as if the shell had launched it. Reading a line
    // back off stdin pins that stdin/stdout are inherited, not rewired.
    install_fake_clickhouse(
        &sandbox,
        FAKE_VERSION,
        "#!/bin/sh\nread line\nprintf 'pid:%s\\nstdin:%s\\n' \"$$\" \"$line\"\nexit 0\n",
        0o755,
    );

    let mut child = sandbox
        .command(&[
            "local",
            "client",
            "--version",
            FAKE_VERSION,
            "--host",
            "127.0.0.1",
            "--port",
            "9000",
        ])
        .env_clear()
        .env("HOME", sandbox.home.path())
        .env("CHCTL_TELEMETRY_URL", sandbox.telemetry_url())
        .current_dir(project.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn clickhousectl");
    let pid = child.id();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().expect("piped stdin");
        stdin
            .write_all(b"hello\n")
            .expect("write to inherited stdin");
    }
    let output = child.wait_with_output().expect("wait for clickhousectl");

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);
    assert!(
        stdout.contains(&format!("pid:{pid}")),
        "the native client must run as this very process: {stdout}"
    );
    assert!(stdout.contains("stdin:hello"), "{stdout}");

    let event = the_event(&sandbox).await;
    assert_eq!(event["outcome"], "exec_attempt");
    assert_eq!(event["exit_code"], 0);
}

#[cfg(unix)]
#[tokio::test]
async fn launch_failure_after_the_pre_flight_is_censored_never_successful() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    // A regular file with an execute bit — the pre-flight passes — whose
    // shebang interpreter does not exist, so `execve` fails with the very
    // ENOENT a binary unlinked between the pre-flight and the launch would
    // produce. That race is not closable, so this is the deterministic stand-in
    // for it: the residue lands on the censored outcome, never on `ok`.
    install_fake_clickhouse(
        &sandbox,
        FAKE_VERSION,
        "#!/no/such/interpreter\nexit 0\n",
        0o755,
    );

    let output = run_local_client(&sandbox, project.path());

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    let os_error = std::io::Error::from_raw_os_error(libc::ENOENT).to_string();
    assert!(
        stderr.contains(&os_error),
        "the OS launch failure must still reach the shell: {stderr}"
    );

    // The hook already sent `exec_attempt`, then the error returned to
    // `main`'s tail: the shared guard is what keeps that to one event.
    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "exec_attempt");
    assert_eq!(event["exit_code"], 0);
}

#[cfg(unix)]
#[tokio::test]
async fn missing_psql_is_an_error_event_not_a_handoff() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    let empty_path = sandbox.home.path().join("empty-path");
    std::fs::create_dir_all(&empty_path).unwrap();

    let output = sandbox
        .command(&["local", "postgres", "client", "--host", "127.0.0.1"])
        .env_clear()
        .env("HOME", sandbox.home.path())
        .env("CHCTL_TELEMETRY_URL", sandbox.telemetry_url())
        .env("PATH", &empty_path)
        .current_dir(project.path())
        .output()
        .expect("run clickhousectl");

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("psql"), "{stderr}");

    // The other `exec()` handoff: a program that cannot be launched is an
    // error, not a censored attempt.
    let event = the_event(&sandbox).await;
    assert_eq!(event["command"], "local postgres client");
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["exit_code"], 1);
}

/// Dispatched commands record flag *names* and positional *slots*, never a
/// value (#480). The bare, positional and `--name` forms of one command stay
/// distinguishable, and arguments forwarded after `--` are not this CLI's
/// shape at all. Capture is unit-tested in `telemetry.rs`; this pins the wire.
#[tokio::test]
async fn dispatched_events_record_names_and_slots_but_never_values() {
    const HOSTILE: &str = "postgres://user:pa55w0rd@db.internal:5432/prod";
    let project = tempfile::tempdir().unwrap();
    let json = |value: &[&str]| serde_json::json!(value);

    // (argv, command, flags, positionals, exit code, values that must not leak)
    type Row<'a> = (&'a [&'a str], &'a str, Value, Value, i32, &'a [&'a str]);
    let cases: &[Row] = &[
        (
            &["local", "--json", "list"],
            "local list",
            json(&["json"]),
            json(&[]),
            0,
            &[],
        ),
        (
            &["local", "server", "stop"],
            "local clickhouse stop",
            json(&[]),
            json(&[]),
            0,
            &[],
        ),
        (
            &["local", "server", "stop", "SECRET-SERVER-NAME"],
            "local clickhouse stop",
            json(&[]),
            json(&["name"]),
            1,
            &["SECRET"],
        ),
        (
            &["local", "server", "stop", "--name", "SECRET-SERVER-NAME"],
            "local clickhouse stop",
            json(&["name"]),
            json(&[]),
            1,
            &["SECRET"],
        ),
        // A server name is validated in clap, so the hostile value goes
        // through an opaque positional that reaches its handler. `local remove`
        // rejects path separators in clap, so its value carries none.
        (
            &["local", "remove", "user:pa55w0rd@db.internal"],
            "local remove",
            json(&[]),
            json(&["version"]),
            1,
            &["postgres://", "pa55w0rd", "db.internal"],
        ),
        (
            &["local", "client", "--", HOSTILE],
            "local client",
            json(&[]),
            json(&[]),
            1,
            &["postgres://", "pa55w0rd", "db.internal"],
        ),
        // A failed handler records the slot and derives `error` from the
        // exit code it exited with.
        (
            &["local", "remove", "no-such-version-xyz"],
            "local remove",
            json(&[]),
            json(&["version"]),
            1,
            &["no-such-version-xyz"],
        ),
    ];

    let rows: Vec<&[&str]> = cases.iter().map(|case| case.0).collect();
    let runs = run_rows(&rows, project.path()).await;
    for ((args, command, flags, positionals, exit_code, secrets), (sandbox, output)) in
        cases.iter().zip(&runs)
    {
        assert_eq!(
            output.status.code(),
            Some(*exit_code),
            "{args:?}: {}",
            stderr_of(output)
        );
        let event = &the_event(sandbox).await;
        assert_eq!(event["command"], *command, "{args:?}");
        assert_eq!(event["flags"], *flags, "{args:?}");
        assert_eq!(event["positionals"], *positionals, "{args:?}");
        assert_eq!(event["exit_code"], *exit_code, "{args:?}");
        assert_eq!(
            event["outcome"],
            if *exit_code == 0 { "ok" } else { "error" },
            "{args:?}"
        );
        let raw = serde_json::to_string(event).unwrap();
        for secret in *secrets {
            assert!(!raw.contains(secret), "{secret} leaked for {args:?}: {raw}");
        }
    }
}

// -- positional presence (#480) ---------------------------------------------

// -- any invocation counts (#320): bare, help, version, parse errors --------

#[tokio::test]
async fn first_run_of_help_prints_notice_and_writes_marker() {
    let sandbox = Sandbox::new().await;

    // A user whose first-ever touch is `--help` still starts their consent
    // clock: notice shown, marker written, nothing sent.
    let output = sandbox.run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).contains("Usage:"));
    assert!(
        has_first_run_notice(&output),
        "first --help must print the notice, got stderr: {}",
        stderr_of(&output)
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":false}"#
    );
    sandbox.assert_no_requests().await;

    // Second help run: no repeated notice, and (consent granted) an event
    // with the help outcome and the flag recorded under its long name.
    let output = sandbox.run(&["cloud", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(!has_first_run_notice(&output));
    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "cloud");
    assert_eq!(event["flags"], serde_json::json!(["help"]));
    assert_eq!(event["outcome"], "help");
    assert_eq!(event["exit_code"], 0);
}

/// Every parse failure still reaches the telemetry tail through
/// `capture_lossy`, keeps clap's exit 2, and keeps the unmatched or invalid
/// token off the wire. Each outcome's capture rules are unit-tested as the
/// `lossy_*` tests in `telemetry.rs`; this is the end-to-end path, once per
/// outcome.
#[tokio::test]
async fn parse_failures_reach_the_wire_as_outcomes_without_their_tokens() {
    let project = tempfile::tempdir().unwrap();

    // (argv, command, outcome, flags, positionals, tokens that must not leak)
    type Row<'a> = (&'a [&'a str], &'a str, &'a str, Value, Value, &'a [&'a str]);
    let cases: &[Row] = &[
        (
            &[],
            "",
            "missing_subcommand",
            serde_json::json!([]),
            serde_json::json!([]),
            &[],
        ),
        (
            &["hallucinated-subcommand-xyz"],
            "",
            "invalid_subcommand",
            serde_json::json!([]),
            serde_json::json!([]),
            &["hallucinated-subcommand-xyz"],
        ),
        (
            &["local", "use"],
            "local use",
            "missing_required",
            serde_json::json!([]),
            serde_json::json!([]),
            &[],
        ),
        // Flags after a positional are still captured, by name only.
        (
            &[
                "cloud",
                "postgres",
                "logs",
                "SECRET-POSTGRES-ID",
                "--org-id",
                "SECRET-ORG-ID",
                "--from-date",
                "SECRET-FROM-DATE",
                "--to-date",
                "SECRET-TO-DATE",
            ],
            "cloud postgres logs",
            "invalid_value",
            serde_json::json!(["from-date", "org-id", "to-date"]),
            serde_json::json!(["resource_id"]),
            &["SECRET"],
        ),
        (
            &["local", "install", "not.a.version"],
            "local install",
            "invalid_value",
            serde_json::json!([]),
            serde_json::json!(["version"]),
            &["not.a.version"],
        ),
    ];

    let rows: Vec<&[&str]> = cases.iter().map(|case| case.0).collect();
    let runs = run_rows(&rows, project.path()).await;
    for ((args, command, outcome, flags, positionals, tokens), (sandbox, output)) in
        cases.iter().zip(&runs)
    {
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stderr_of(output)
        );
        let event = &the_event(sandbox).await;
        assert_eq!(event["command"], *command, "{args:?}");
        assert_eq!(event["outcome"], *outcome, "{args:?}");
        assert_eq!(event["exit_code"], 2, "{args:?}");
        assert_eq!(event["flags"], *flags, "{args:?}");
        assert_eq!(event["positionals"], *positionals, "{args:?}");
        assert!(
            event.get("failure_kind").is_none(),
            "a parse error carries no failure kind: {event}"
        );
        let raw = serde_json::to_string(event).unwrap();
        for token in *tokens {
            assert!(!raw.contains(token), "{token} leaked for {args:?}: {raw}");
        }
    }
    assert!(!project.path().join(".clickhouse").exists());
    let install_home = runs[4].0.home.path();
    assert!(!install_home.join(".clickhouse/versions").exists());

    // Clap's did-you-mean names the *defined* subcommand, so it is safe to
    // record; the typo'd token stays off the wire.
    let sandbox = &runs[0].0;
    let output = sandbox.run(&["cloud", "servce", "list"]);
    assert_eq!(output.status.code(), Some(2));
    let event = &sandbox.wait_for_requests(2).await[1];
    assert_eq!(event["command"], "cloud");
    assert_eq!(event["outcome"], "invalid_subcommand");
    assert_eq!(event["suggestion"], "service");
    assert!(!serde_json::to_string(event).unwrap().contains("servce"));
}

/// `DO_NOT_TRACK` opts out before anything else: no notice, no marker file,
/// no event. A set value that is not valid UTF-8 must opt out too; if the
/// lookup treated it as "unset" the opt-out would fail open (see
/// `real_env_lookup`, which uses `var_os` + lossy conversion for exactly this
/// reason).
#[tokio::test]
async fn do_not_track_is_fully_silent() {
    let mut values = vec![std::ffi::OsString::from("1")];
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        values.push(std::ffi::OsString::from_vec(vec![0xff, 0xfe]));
    }
    let mut sandboxes = Vec::new();
    for value in values {
        let sandbox = Sandbox::new().await;
        let output = sandbox
            .command(&["local", "list"])
            .env("DO_NOT_TRACK", &value)
            .output()
            .unwrap();
        assert!(output.status.success(), "{value:?}");
        assert!(!has_first_run_notice(&output), "{value:?}");
        assert!(
            !sandbox.state_path().exists(),
            "DO_NOT_TRACK={value:?} must not write the marker file"
        );
        sandboxes.push(sandbox);
    }
    assert_no_requests_in(&sandboxes.iter().collect::<Vec<_>>()).await;
}

#[tokio::test]
async fn disable_persists_and_silences() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["telemetry", "disable"]);
    assert!(output.status.success());
    assert_eq!(
        stdout_of(&output).lines().count(),
        1,
        "{}",
        stdout_of(&output)
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":true}"#
    );

    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());
    assert!(!has_first_run_notice(&output));

    // Status names the file that disabled it.
    let output = sandbox.run(&["telemetry", "status"]);
    assert!(
        stdout_of(&output).contains(&sandbox.state_path().display().to_string()),
        "{}",
        stdout_of(&output)
    );

    // Neither the disable itself, nor anything after it, sent an event.
    sandbox.assert_no_requests().await;
}

/// `telemetry status` only reads: whatever the saved preference and output
/// mode, it writes no marker, prints no first-run notice, touches no update
/// cache and sends nothing.
#[tokio::test]
async fn status_is_read_only_in_every_state_and_output_mode() {
    let mut sandboxes = Vec::new();
    for (disabled, preference, enabled, reason) in [
        (None, "unconfigured", false, "unconfigured"),
        (Some(false), "enabled", true, "preference_enabled"),
        (Some(true), "disabled", false, "preference_disabled"),
    ] {
        for mode in ["human", "json", "agent"] {
            let sandbox = Sandbox::new().await;
            if let Some(disabled) = disabled {
                sandbox.write_state(disabled);
            }
            let original = std::fs::read(sandbox.state_path()).ok();
            let mut command = sandbox.command(&["telemetry", "status"]);
            // Allow the background update check, so the absent cache below
            // shows that `status` skips it rather than that the timeout did.
            command.env_remove("CHCTL_UPDATE_CHECK_TIMEOUT_MS");
            match mode {
                "json" => {
                    command.arg("--json");
                }
                "agent" => {
                    command.env("CLAUDECODE", "1");
                }
                _ => {}
            }
            let output = command.output().unwrap();
            let label = format!("{preference}/{mode}");
            assert!(output.status.success(), "{label}: {}", stderr_of(&output));
            assert!(output.stderr.is_empty(), "{label}: {}", stderr_of(&output));
            if mode != "human" {
                let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(value["action"], "status", "{label}");
                assert_eq!(value["preference"], preference, "{label}");
                assert_eq!(value["enabled"], enabled, "{label}");
                assert_eq!(value["reason"], reason, "{label}");
                assert_eq!(
                    value["config_path"],
                    sandbox.state_path().to_str().unwrap(),
                    "{label}"
                );
            } else {
                // The human line names the saved state, as `preference` does.
                let stdout = stdout_of(&output);
                for state in ["enabled", "disabled"] {
                    assert_eq!(
                        stdout.contains(state),
                        preference == state,
                        "{label}: {stdout}"
                    );
                }
            }
            assert_eq!(
                std::fs::read(sandbox.state_path()).ok(),
                original,
                "{label}"
            );
            assert!(
                !sandbox
                    .home
                    .path()
                    .join(".clickhouse/last_update_check")
                    .exists(),
                "{label}"
            );
            sandboxes.push(sandbox);
        }
    }
    assert_no_requests_in(&sandboxes.iter().collect::<Vec<_>>()).await;
}

#[tokio::test]
async fn json_telemetry_changes_report_saved_preference_and_effective_consent() {
    let mut sandboxes = Vec::new();
    for agent_mode in [false, true] {
        let sandbox = Sandbox::new().await;
        for (action, preference, disabled) in
            [("enable", "enabled", false), ("disable", "disabled", true)]
        {
            let mut command = sandbox.command(&["telemetry", action]);
            command.env("DO_NOT_TRACK", "1");
            if agent_mode {
                command.env("CLAUDECODE", "1");
            } else {
                command.arg("--json");
            }
            let output = command.output().unwrap();
            assert!(output.status.success(), "{}", stderr_of(&output));
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["action"], action);
            assert_eq!(value["preference"], preference);
            assert_eq!(value["enabled"], false);
            assert_eq!(value["reason"], "do_not_track");
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(sandbox.state_path()).unwrap())
                    .unwrap()["disabled"],
                disabled
            );

            let output = sandbox.run(&["telemetry", "status", "--json"]);
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["enabled"], !disabled);
            assert_eq!(value["preference"], preference);
        }
        sandboxes.push(sandbox);
    }
    assert_no_requests_in(&sandboxes.iter().collect::<Vec<_>>()).await;
}

#[tokio::test]
async fn enable_sends_an_event_for_itself() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(true);

    let output = sandbox.run(&["telemetry", "enable"]);
    assert!(output.status.success());
    assert_eq!(
        stdout_of(&output).lines().count(),
        1,
        "{}",
        stdout_of(&output)
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":false}"#
    );
    // Without DO_NOT_TRACK there is nothing to warn about.
    assert!(!stderr_of(&output).contains("DO_NOT_TRACK"));

    // Consent is evaluated after the command ran, so the enable run itself
    // is the first event.
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(payloads[0]["command"], "telemetry enable");
}

/// `telemetry enable` under `DO_NOT_TRACK` still records the preference (so
/// it takes effect once DNT is lifted) but must tell the user that nothing
/// will actually be sent — otherwise "Telemetry enabled." would be silently
/// untrue.
#[tokio::test]
async fn enable_under_do_not_track_warns_that_telemetry_stays_silent() {
    let sandbox = Sandbox::new().await;

    let output = sandbox
        .command(&["telemetry", "enable"])
        .env("DO_NOT_TRACK", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.status.code(), Some(0));

    // stdout stays the clean confirmation; the note, naming the variable,
    // goes to stderr.
    assert_eq!(
        stdout_of(&output).lines().count(),
        1,
        "{}",
        stdout_of(&output)
    );
    assert!(!stdout_of(&output).contains("DO_NOT_TRACK"));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("DO_NOT_TRACK"),
        "enable under DNT must warn on stderr, got: {stderr}"
    );

    // The preference is still written — writing under DNT is intentional.
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":false}"#
    );

    // And DNT still wins: the enable run itself sent nothing.
    sandbox.assert_no_requests().await;
}

#[tokio::test]
async fn debug_mode_prints_payload_without_sending() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox
        .command(&["local", "list"])
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stderr = stderr_of(&output);
    assert!(
        stderr.contains(r#""command":"local list""#),
        "debug mode must print the payload to stderr, got: {stderr}"
    );
    sandbox.assert_no_requests().await;
}

#[cfg(unix)]
#[tokio::test]
async fn unwritable_home_fails_open_to_silent() {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new().await;
    let perms = std::fs::Permissions::from_mode(0o555);
    std::fs::set_permissions(sandbox.home.path(), perms).unwrap();

    // Twice: silent every run, never a repeated notice, never an error, and
    // the same status as a home that was never configured.
    let fresh = Sandbox::new().await;
    let unconfigured = fresh.run(&["telemetry", "status"]);
    assert!(!fresh.state_path().exists());
    for _ in 0..2 {
        let output = sandbox.run(&["telemetry", "status"]);
        assert!(output.status.success());
        assert!(!has_first_run_notice(&output));
        assert_eq!(stdout_of(&output), stdout_of(&unconfigured));
    }
    assert!(!sandbox.state_path().exists());
    sandbox.assert_no_requests().await;

    // Restore so TempDir cleanup can delete the directory.
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(sandbox.home.path(), perms).unwrap();
}

#[tokio::test]
async fn parent_never_waits_for_a_slow_endpoint() {
    let sandbox = Sandbox::new().await;
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(sandbox.endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
        .mount(&mock)
        .await;

    sandbox.write_state(false);

    let started = Instant::now();
    let output = sandbox
        .command(&["local", "list"])
        .env(
            "CHCTL_TELEMETRY_URL",
            format!("{}{}", mock.uri(), sandbox.endpoint_path),
        )
        .output()
        .unwrap();
    let elapsed = started.elapsed();

    assert!(output.status.success());
    // The send child is detached; the parent must return well before the
    // mock's 10s delay (generous bound to absorb slow CI machines).
    assert!(
        elapsed < Duration::from_secs(5),
        "parent waited on the telemetry send: {elapsed:?}"
    );

    // Keep the mock alive until the detached child has connected. Otherwise
    // its port can be reused by another parallel test before the child sends.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if mock
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .any(|request| request.url.path() == sandbox.endpoint_path.as_str())
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "detached telemetry child did not connect within 5s"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Check the marker file used by `Sandbox::state_path` matches what the
/// binary actually writes — guards against the test suite silently diverging
/// from the real path.
#[tokio::test]
async fn marker_lives_in_dot_clickhouse_telemetry_json() {
    let sandbox = Sandbox::new().await;
    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());
    assert!(sandbox.state_path().exists());
    let entries: Vec<_> = std::fs::read_dir(sandbox.home.path().join(".clickhouse"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(
        entries.contains(&"telemetry.json".to_string()),
        "{entries:?}"
    );
}

// -- closed stderr must never turn an exit code into a panic (#320) ----------

#[tokio::test]
async fn closed_stderr_never_panics_or_bypasses_telemetry() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    // Cache a newer version so `--help` and the failing command both try to
    // write the update notice to the (closed) stderr.
    let cache = sandbox
        .home
        .path()
        .join(".clickhouse")
        .join("last_update_check");
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(&cache, format!("{now}\n999.0.0")).unwrap();

    // `--help`: the update notice write is swallowed, clap's exit code 0 is
    // preserved, and the telemetry tail still runs.
    let output = sandbox.run_with_closed_stderr(&["--help"]);
    assert_eq!(output.status.code(), Some(0), "help must not panic");
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(payloads[0]["outcome"], "help");

    // A failing command: the `Error: ...` line and the update notice both hit
    // the closed stderr; the handler's exit code 1 survives (not panic's 101)
    // and the failure event still goes out.
    let output = sandbox.run_with_closed_stderr(&["local", "remove", "no-such-version-xyz"]);
    assert_eq!(output.status.code(), Some(1), "failure must keep exit 1");
    let payloads = sandbox.wait_for_requests(2).await;
    assert_eq!(payloads[1]["exit_code"], 1);

    // `telemetry enable` under DO_NOT_TRACK: the stderr note about DNT is
    // swallowed and the command still succeeds.
    let output = {
        let (reader, writer) = std::io::pipe().expect("failed to create pipe");
        drop(reader);
        sandbox
            .command(&["telemetry", "enable"])
            .env("DO_NOT_TRACK", "1")
            .stderr(writer)
            .output()
            .expect("failed to spawn binary")
    };
    assert_eq!(output.status.code(), Some(0), "enable must not panic");
    assert_eq!(
        stdout_of(&output).lines().count(),
        1,
        "{}",
        stdout_of(&output)
    );
}

// -- query failure classification (#450) -------------------------------------
//
// `cloud service query` is the command whose exit-1 events were opaque, so it
// is the one whose stages are pinned end-to-end here: each test drives the
// real binary against a mock control plane and a mock query host, then reads
// the exact payload the process would have sent.
//
// `CHCTL_TELEMETRY_DEBUG=1` prints that payload to stderr instead of handing
// it to the detached send child, which keeps these tests synchronous — the
// consent state machine, including the debug branch, is covered above.

const QUERY_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";

/// Control plane that answers the service lookup `cloud service query`
/// performs before it talks to the query host.
async fn start_control_plane(service_status: u16) -> MockServer {
    let mock = MockServer::start().await;
    let body = serde_json::json!({
        "result": { "id": QUERY_SERVICE_ID, "name": "demo" },
        "status": 200,
        "requestId": "stub-service-get",
    });
    let response = if service_status == 200 {
        ResponseTemplate::new(200).set_body_json(body)
    } else {
        ResponseTemplate::new(service_status).set_body_string("NOT_FOUND")
    };
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{QUERY_SERVICE_ID}"
        )))
        .respond_with(response)
        .mount(&mock)
        .await;
    mock
}

async fn start_query_host(response: ResponseTemplate) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_SERVICE_ID}/run")))
        .respond_with(response)
        .mount(&mock)
        .await;
    mock
}

/// The OAuth token file that makes the CLI take the bearer query path.
fn write_oauth_tokens(sandbox: &Sandbox, control_uri: &str) {
    let ch_dir = sandbox.home.path().join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    std::fs::write(
        ch_dir.join("tokens.json"),
        serde_json::to_vec(&serde_json::json!({
            "access_token": "test-bearer-token",
            "refresh_token": "unused",
            "expires_at": 4102444800u64,
            "api_url": format!("{control_uri}/v1"),
            "auth_config": {"auth_url": control_uri, "client_id": "test-client"},
        }))
        .unwrap(),
    )
    .unwrap();
}

/// The payload the invocation would have sent, parsed out of the debug line
/// on stderr.
fn debug_payload(output: &Output) -> Value {
    let stderr = stderr_of(output);
    stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("command").is_some())
        .unwrap_or_else(|| panic!("no telemetry debug payload on stderr:\n{stderr}"))
}

/// Run `cloud service query` against the mocks, in a throwaway project
/// directory, with telemetry enabled in debug mode.
fn run_query(
    sandbox: &Sandbox,
    project: &std::path::Path,
    control: &MockServer,
    query_host: Option<&MockServer>,
    args: &[&str],
) -> Output {
    let url = control.uri();
    let mut argv = vec![
        "cloud",
        "--url",
        &url,
        "service",
        "query",
        "--id",
        QUERY_SERVICE_ID,
        "--org-id",
        "org-1",
    ];
    argv.extend_from_slice(args);
    let mut command = sandbox.command(&argv);
    command
        .current_dir(project)
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env_remove("CLICKHOUSE_CLOUD_QUERY_HOST");
    if let Some(query_host) = query_host {
        command.env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri());
    }
    command.output().expect("failed to spawn binary")
}

/// Everything a query test needs: an enabled telemetry sandbox, a bearer
/// credential, and an isolated project directory.
async fn query_sandbox(control: &MockServer) -> (Sandbox, tempfile::TempDir) {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    write_oauth_tokens(&sandbox, &control.uri());
    (sandbox, tempfile::tempdir().unwrap())
}

#[tokio::test]
async fn a_sql_error_is_classified_without_the_sql_reaching_the_payload() {
    let control = start_control_plane(200).await;
    let query_host = start_query_host(
        ResponseTemplate::new(400)
            .set_body_string(r#"{"error":{"code":"62","details":"Syntax error near FROM"}}"#),
    )
    .await;
    let (sandbox, project) = query_sandbox(&control).await;

    let output = run_query(
        &sandbox,
        project.path(),
        &control,
        Some(&query_host),
        &[
            "--query",
            "SELECT api_password FROM totally_secret_table -- SUPER-SECRET-VALUE",
        ],
    );
    assert_eq!(output.status.code(), Some(1));

    let event = debug_payload(&output);
    assert_eq!(event["command"], "cloud service query");
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["exit_code"], 1);
    // The service ran the request and rejected the statement: that is a
    // distinct fact from "the request never got there".
    assert_eq!(event["failure_stage"], "query_request");
    assert_eq!(event["failure_kind"], "sql_error");
    assert_eq!(event["http_status"], 400);
    assert_eq!(event["retry_bucket"], "0");
    assert_eq!(event["provisioning_state"], "bearer");
    assert!(
        event["duration_bucket"].is_string(),
        "a classified failure reports a duration bucket: {event}"
    );

    // The SQL, the identifiers in it and the server's own message stay on the
    // machine — the user still sees them on stderr.
    let raw = serde_json::to_string(&event).unwrap();
    for secret in [
        "SUPER-SECRET-VALUE",
        "api_password",
        "totally_secret_table",
        "SELECT",
        "Syntax error",
        QUERY_SERVICE_ID,
        "org-1",
    ] {
        assert!(!raw.contains(secret), "payload leaked {secret}: {raw}");
    }
    // The server's own code and details reach the user verbatim.
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("62") && stderr.contains("Syntax error near FROM"),
        "the user must still see the real error: {stderr}"
    );
}

#[tokio::test]
async fn query_timeouts_retain_classification_with_or_without_wake_context() {
    for (state, confirm_wake) in [("running", false), ("idle", false), ("running", true)] {
        let control = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1/organizations/org-1/services/{QUERY_SERVICE_ID}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": { "id": QUERY_SERVICE_ID, "name": "demo", "state": state },
                "status": 200,
                "requestId": "stub-service-get"
            })))
            .expect(1)
            .mount(&control)
            .await;
        let query_host = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/service/{QUERY_SERVICE_ID}/run")))
            .respond_with(move |request: &wiremock::Request| {
                if confirm_wake && !request.headers.contains_key("wake-service") {
                    ResponseTemplate::new(206).set_body_string(r#"{"data":"Confirm wake service"}"#)
                } else {
                    ResponseTemplate::new(500).set_body_string(r#"{"error":"Timeout error."}"#)
                }
            })
            .mount(&query_host)
            .await;
        let (sandbox, project) = query_sandbox(&control).await;
        let output = run_query(
            &sandbox,
            project.path(),
            &control,
            Some(&query_host),
            &["--query", "SELECT 'secret-timeout-test'"],
        );
        assert_eq!(output.status.code(), Some(1));
        let event = debug_payload(&output);
        assert_eq!(event["failure_stage"], "query_request");
        assert_eq!(event["failure_kind"], "timeout");
        assert_eq!(event["http_status"], 500);
        assert_eq!(event["retry_bucket"], if confirm_wake { "1" } else { "0" });
        assert_eq!(event["provisioning_state"], "bearer");
        assert_eq!(
            query_host.received_requests().await.unwrap().len(),
            if confirm_wake { 2 } else { 1 }
        );
        let raw = event.to_string();
        for sensitive in [
            "secret-timeout-test",
            QUERY_SERVICE_ID,
            "test-bearer-token",
            "Timeout error.",
        ] {
            assert!(
                !raw.contains(sensitive),
                "telemetry leaked {sensitive}: {raw}"
            );
        }
    }
}

#[tokio::test]
async fn a_rate_limited_query_is_distinguishable_from_a_stopped_service() {
    let control = start_control_plane(200).await;
    let query_host = start_query_host(
        ResponseTemplate::new(429).set_body_string("Too many requests, please retry"),
    )
    .await;
    let (sandbox, project) = query_sandbox(&control).await;
    let output = run_query(
        &sandbox,
        project.path(),
        &control,
        Some(&query_host),
        &["--query", "SELECT 1"],
    );
    assert_eq!(output.status.code(), Some(1));
    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "query_request");
    assert_eq!(event["failure_kind"], "rate_limited");
    assert_eq!(event["http_status"], 429);

    // A stopped service answers 206 and can never be woken by a query, so it
    // is its own kind rather than another 4xx.
    let control = start_control_plane(200).await;
    let query_host = start_query_host(
        ResponseTemplate::new(206).set_body_string(r#"{"data":"Service is stopped"}"#),
    )
    .await;
    let (sandbox, project) = query_sandbox(&control).await;
    let output = run_query(
        &sandbox,
        project.path(),
        &control,
        Some(&query_host),
        &["--query", "SELECT 1"],
    );
    assert_eq!(output.status.code(), Some(1));
    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "query_request");
    assert_eq!(event["failure_kind"], "service_stopped");
    assert!(
        event.get("http_status").is_none(),
        "a state error carries no HTTP status: {event}"
    );
}

#[tokio::test]
async fn a_service_lookup_failure_names_its_own_stage() {
    let control = start_control_plane(404).await;
    let (sandbox, project) = query_sandbox(&control).await;

    let output = run_query(
        &sandbox,
        project.path(),
        &control,
        None,
        &["--query", "SELECT 1"],
    );
    assert_eq!(output.status.code(), Some(1));
    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "service_resolution");
    assert_eq!(event["failure_kind"], "http_4xx");
    assert_eq!(event["http_status"], 404);
    // Nothing was provisioned or attempted against a query host.
    assert!(event.get("provisioning_state").is_none(), "{event}");
}

#[tokio::test]
async fn empty_sql_is_a_sql_input_failure_before_any_request() {
    let control = start_control_plane(200).await;
    let (sandbox, project) = query_sandbox(&control).await;

    let output = run_query(
        &sandbox,
        project.path(),
        &control,
        None,
        &["--query", "   \n  "],
    );
    assert_eq!(output.status.code(), Some(2));
    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "sql_input");
    assert_eq!(event["failure_kind"], "other");
    assert!(event.get("http_status").is_none(), "{event}");
    assert!(event.get("provisioning_state").is_none(), "{event}");
    assert_eq!(event["retry_bucket"], "0");
    assert!(
        control.received_requests().await.unwrap().is_empty(),
        "an input failure must not touch the network"
    );
}

#[tokio::test]
async fn a_successful_query_carries_no_failure_classification() {
    let control = start_control_plane(200).await;
    let query_host = start_query_host(ResponseTemplate::new(200).set_body_string("1\n")).await;
    let (sandbox, project) = query_sandbox(&control).await;

    let output = run_query(
        &sandbox,
        project.path(),
        &control,
        Some(&query_host),
        &["--query", "SELECT 1"],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let event = debug_payload(&output);
    assert_eq!(event["outcome"], "ok");
    for key in [
        "failure_stage",
        "failure_kind",
        "http_status",
        "retry_bucket",
        "provisioning_state",
        "duration_bucket",
    ] {
        assert!(
            event.get(key).is_none(),
            "a successful run must carry no {key}: {event}"
        );
    }
}

/// The organization the first-use bind tests resolve: `whoami` reports a
/// UUID, and the bind refuses a key whose organization does not match it.
const BIND_ORG_ID: &str = "5fae43a3-1043-4000-8000-0000000000f1";
const CALLER_KEY_UUID: &str = "cccccccc-1043-4000-8000-00000000ca11";

fn envelope(result: Value, request_id: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": request_id,
    }))
}

/// A control plane in `org_id` that knows the service and identifies the
/// caller as an API key of `BIND_ORG_ID`.
async fn start_bind_control_plane(org_id: &str) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{org_id}/services/{QUERY_SERVICE_ID}"
        )))
        .respond_with(envelope(
            serde_json::json!({ "id": QUERY_SERVICE_ID, "name": "demo" }),
            "stub-service-get",
        ))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(envelope(
            serde_json::json!({
                "actorType": "apiKey",
                "keyId": CALLER_KEY_UUID,
                "organizationId": BIND_ORG_ID,
            }),
            "stub-whoami",
        ))
        .mount(&mock)
        .await;
    mock
}

/// Run `cloud service query` with API-key auth in `org_id`, in a fresh
/// project with no stored record, so a rejected key takes the bind path.
fn run_api_key_query(
    sandbox: &Sandbox,
    control: &MockServer,
    query_host: &MockServer,
    org_id: &str,
    sql: &str,
) -> Output {
    let project = tempfile::tempdir().unwrap();
    let url = control.uri();
    sandbox
        .command(&[
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_SERVICE_ID,
            "--org-id",
            org_id,
            "--query",
            sql,
        ])
        .current_dir(project.path())
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .output()
        .expect("failed to spawn binary")
}

fn assert_no_identifiers_leaked(event: &Value) {
    let raw = serde_json::to_string(event).unwrap();
    for secret in [
        "fake-key-for-tests",
        "fake-secret-for-tests",
        QUERY_SERVICE_ID,
        CALLER_KEY_UUID,
        BIND_ORG_ID,
    ] {
        assert!(!raw.contains(secret), "payload leaked {secret}: {raw}");
    }
}

#[tokio::test]
async fn a_failed_caller_identification_names_the_whoami_stage() {
    // The API-key path: the caller's key is refused by the query host, so the
    // CLI identifies it to bind it — and the control plane rate-limits the
    // `whoami`, the burst #450 could not see.
    let control = start_control_plane(200).await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(ResponseTemplate::new(429).set_body_string("TOO_MANY_REQUESTS"))
        .mount(&control)
        .await;
    let query_host =
        start_query_host(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
            .await;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let output = run_api_key_query(&sandbox, &control, &query_host, "org-1", "SELECT 1");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "whoami");
    assert_eq!(event["failure_kind"], "rate_limited");
    assert_eq!(event["http_status"], 429);
    assert_eq!(
        event["provisioning_state"], "provisioning",
        "the run died mid-bind, which is what separates it from a query failure: {event}"
    );
    assert_no_identifiers_leaked(&event);
}

#[tokio::test]
async fn a_caller_of_another_organization_is_refused_at_the_whoami_stage() {
    let other_org = "5fae43a3-1043-4000-8000-0000000000f2";
    let control = start_bind_control_plane(other_org).await;
    let query_host =
        start_query_host(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
            .await;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let output = run_api_key_query(&sandbox, &control, &query_host, other_org, "SELECT 1");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "whoami");
    assert_eq!(event["failure_kind"], "other");
    assert!(event.get("http_status").is_none(), "{event}");
    assert_eq!(event["provisioning_state"], "provisioning");
    assert_no_identifiers_leaked(&event);
    assert!(!serde_json::to_string(&event).unwrap().contains(other_org));
}

#[tokio::test]
async fn a_query_that_fails_after_the_bind_reports_the_bound_caller_key_state() {
    // The bind succeeded and the re-run query is what fails: the state says
    // the caller's key was bound during this run, and no key was created.
    let control = start_bind_control_plane(BIND_ORG_ID).await;
    let endpoint_path =
        format!("/v1/organizations/{BIND_ORG_ID}/services/{QUERY_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(404).set_body_string("NOT_FOUND"))
        .mount(&control)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(envelope(
            serde_json::json!({ "id": "ep-1" }),
            "stub-endpoint-upsert",
        ))
        .expect(1)
        .mount(&control)
        .await;
    let query_host = MockServer::start().await;
    let attempts = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_SERVICE_ID}/run")))
        .respond_with(move |request: &wiremock::Request| {
            if attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                return ResponseTemplate::new(401).set_body_string("API key is not authorized");
            }
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            if body["sql"] == "SELECT 1" {
                ResponseTemplate::new(200).set_body_string("1\n")
            } else {
                ResponseTemplate::new(400)
                    .set_body_string(r#"{"error":{"code":"62","details":"Syntax error"}}"#)
            }
        })
        .mount(&query_host)
        .await;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let output = run_api_key_query(&sandbox, &control, &query_host, BIND_ORG_ID, "SELECT 2");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "query_request");
    assert_eq!(event["failure_kind"], "sql_error");
    assert_eq!(event["provisioning_state"], "bound_caller_key");
    assert_no_identifiers_leaked(&event);
}

fn assert_success_without_failure_details(event: &Value) {
    assert_eq!(event["exit_code"], 0, "{event}");
    assert_eq!(event["outcome"], "ok", "{event}");
    for key in [
        "failure_stage",
        "failure_kind",
        "http_status",
        "retry_bucket",
        "provisioning_state",
        "duration_bucket",
        "failure_count",
    ] {
        assert!(
            event.get(key).is_none(),
            "successful closed-pipe command carried {key}: {event}"
        );
    }
}

#[tokio::test]
async fn closed_stdout_query_stream_is_quiet_and_records_success() {
    let control = start_control_plane(200).await;
    let query_host =
        start_query_host(ResponseTemplate::new(200).set_body_string("1\n".repeat(1024 * 1024)))
            .await;
    for json in [false, true] {
        let (sandbox, project) = query_sandbox(&control).await;
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let mut command = sandbox.command(&[
            "cloud",
            "--url",
            &control.uri(),
            "service",
            "query",
            "--id",
            QUERY_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ]);
        command
            .current_dir(project.path())
            .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
            .stdout(writer);
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let events = sandbox.wait_for_requests(1).await;
        assert_success_without_failure_details(&events[0]);
    }
}

#[tokio::test]
async fn closed_stdout_large_list_records_success_in_human_and_json_modes() {
    let control = MockServer::start().await;
    let services: Vec<_> = (0..512)
        .map(|id| serde_json::json!({"id": format!("00000000-0000-0000-0000-{id:012x}"), "name": "x".repeat(4096)}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": services})),
        )
        .mount(&control)
        .await;
    for json in [false, true] {
        let sandbox = Sandbox::new().await;
        sandbox.write_state(false);
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let mut command = sandbox.command(&[
            "cloud",
            "--url",
            &control.uri(),
            "--api-key",
            "test-key",
            "--api-secret",
            "test-secret",
            "service",
            "list",
            "--org-id",
            "org-1",
        ]);
        command.current_dir(sandbox.home.path()).stdout(writer);
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let events = sandbox.wait_for_requests(1).await;
        assert_success_without_failure_details(&events[0]);
    }
}

#[tokio::test]
async fn closed_stdout_does_not_hide_a_later_response_stream_failure() {
    use std::io::{Read, Write};
    let control = start_control_plane(200).await;
    let (sandbox, project) = query_sandbox(&control).await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let query_url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "query request never arrived");
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept query request: {error}"),
            }
        };
        socket.set_nonblocking(false).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        // Consume the request body so closing the socket delivers a FIN,
        // rather than a reset that could discard the response already sent.
        let length = String::from_utf8_lossy(&request)
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        socket.read_exact(&mut vec![0; length]).unwrap();
        // Deliver much more than a pipe buffer, then truncate the body. The
        // stdout write fails first; the transport failure must still survive.
        let body = "1\n".repeat(1024 * 1024);
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len() + 100
        )
        .unwrap();
        socket.write_all(body.as_bytes()).unwrap();
        socket.shutdown(std::net::Shutdown::Write).unwrap();
    });
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let output = sandbox
        .command(&[
            "cloud",
            "--url",
            &control.uri(),
            "service",
            "query",
            "--id",
            QUERY_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
            "--json",
        ])
        .current_dir(project.path())
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_url)
        .stdout(writer)
        .output()
        .unwrap();
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "transport");
    let events = sandbox.wait_for_requests(1).await;
    assert_eq!(events[0]["outcome"], "error");
    assert_eq!(events[0]["failure_kind"], "transport");
    assert_eq!(events[0]["failure_stage"], "response_stream");
}

#[tokio::test]
async fn a_failed_endpoint_read_during_the_bind_names_the_endpoint_get_stage() {
    // The caller is identified and the endpoint read is what fails, so the
    // stage is the finer-grained one.
    let control = start_bind_control_plane(BIND_ORG_ID).await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/{BIND_ORG_ID}/services/{QUERY_SERVICE_ID}/serviceQueryEndpoint"
        )))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream unavailable"))
        .mount(&control)
        .await;
    let query_host =
        start_query_host(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
            .await;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let output = run_api_key_query(&sandbox, &control, &query_host, BIND_ORG_ID, "SELECT 1");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "endpoint_get");
    assert_eq!(event["failure_kind"], "http_5xx");
    assert_eq!(event["http_status"], 503);
    assert_eq!(event["provisioning_state"], "provisioning");
    assert_no_identifiers_leaked(&event);
}

#[tokio::test]
async fn a_bind_that_waited_on_key_propagation_counts_the_retry() {
    // The endpoint upsert refuses the caller's key once (#658), the CLI waits
    // and retries, and the retried upsert fails outright. The payload shows
    // one retry and the upsert stage, and no key ID or message text.
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let control = start_bind_control_plane(BIND_ORG_ID).await;
    let endpoint_path =
        format!("/v1/organizations/{BIND_ORG_ID}/services/{QUERY_SERVICE_ID}/serviceQueryEndpoint");
    Mock::given(method("GET"))
        .and(path(endpoint_path.clone()))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&control)
        .await;
    let attempts = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |_: &wiremock::Request| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(400).set_body_string(format!(
                    "OpenAPI key {CALLER_KEY_UUID} does not belong to the organization"
                ))
            } else {
                ResponseTemplate::new(503).set_body_string("upstream unavailable")
            }
        })
        .mount(&control)
        .await;
    let query_host =
        start_query_host(ResponseTemplate::new(401).set_body_string("API key is not authorized"))
            .await;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let output = run_api_key_query(&sandbox, &control, &query_host, BIND_ORG_ID, "SELECT 1");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "endpoint_upsert");
    assert_eq!(event["failure_kind"], "http_5xx");
    assert_eq!(event["retry_bucket"], "1");
    let raw = serde_json::to_string(&event).unwrap();
    assert!(!raw.contains("does not belong"), "{raw}");
    assert_no_identifiers_leaked(&event);
}

#[tokio::test]
async fn an_organization_lookup_failure_names_its_own_stage() {
    // No `--org-id`, so the run starts by resolving the organization; a
    // failure there is not a query failure.
    let control = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&control)
        .await;
    let (sandbox, project) = query_sandbox(&control).await;

    let url = control.uri();
    let output = sandbox
        .command(&[
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_SERVICE_ID,
            "--query",
            "SELECT 1",
        ])
        .current_dir(project.path())
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env_remove("CLICKHOUSE_CLOUD_QUERY_HOST")
        .output()
        .expect("failed to spawn binary");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let event = debug_payload(&output);
    assert_eq!(event["failure_stage"], "org_resolution");
    assert_eq!(event["failure_kind"], "http_5xx");
    assert_eq!(event["http_status"], 500);
    assert!(event.get("provisioning_state").is_none(), "{event}");
}
