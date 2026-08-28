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
    /// the mock, and every env var that would alter the consent flow or the
    /// payload cleared for determinism (the harness itself may run under CI
    /// or a coding agent).
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(clickhousectl_binary());
        cmd.args(args)
            .env("HOME", self.home.path())
            .env("CHCTL_TELEMETRY_URL", self.telemetry_url())
            .env_remove("DO_NOT_TRACK")
            .env_remove("CHCTL_TELEMETRY_DEBUG")
            .env_remove("CHCTL_TELEMETRY_PAYLOAD")
            .env_remove("CI");
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
        tokio::time::sleep(Duration::from_millis(750)).await;
        let requests = self.received_requests().await;
        assert!(
            requests.is_empty(),
            "expected no telemetry, saw: {:?}",
            requests.iter().map(|r| &r.body).collect::<Vec<_>>()
        );
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

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
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

    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("anonymous usage data") && stderr.contains("telemetry disable"),
        "first run must print the notice, got stderr: {stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":false}"#
    );
    sandbox.assert_no_requests().await;

    // Second run: the notice appears exactly once, ever.
    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());
    assert!(!stderr_of(&output).contains("anonymous usage data"));
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

#[tokio::test]
async fn failure_reported_and_positional_value_never_leaks() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["local", "remove", "no-such-version-xyz"]);
    assert!(!output.status.success());

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local remove");
    // The slot the version went into is recorded; the version is not (#480).
    assert_eq!(event["positionals"], serde_json::json!(["version"]));
    // The event carries the exit code the process exited with, and
    // the outcome derived from it — a failed handler is "error", not "ok".
    assert_eq!(event["exit_code"], 1);
    assert_eq!(event["exit_code"], output.status.code().unwrap());
    assert_eq!(event["outcome"], "error");
    let raw = serde_json::to_string(event).unwrap();
    assert!(
        !raw.contains("no-such-version-xyz"),
        "positional argument leaked into the payload: {raw}"
    );
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
    assert_eq!(event["command"], "local server stop");
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
        !stderr_of(&output).contains("Error: child process exited"),
        "child stderr should not gain a wrapper error: {}",
        stderr_of(&output)
    );
    assert!(
        stderr_of(&output).contains("There is a new version of clickhousectl"),
        "child failure should still reach the update-notice tail: {}",
        stderr_of(&output)
    );
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(payloads[0]["command"], "local server start");
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
/// handoff hook shares its guard with `main`'s tail.
async fn exactly_one_event(sandbox: &Sandbox) -> Value {
    let payloads = sandbox.wait_for_requests(1).await;
    tokio::time::sleep(Duration::from_millis(750)).await;
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

#[cfg(unix)]
#[tokio::test]
async fn successful_handoff_is_one_censored_exec_attempt() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    install_fake_clickhouse(
        &sandbox,
        FAKE_VERSION,
        "#!/bin/sh\nprintf 'child ran\\n'\nexit 0\n",
        0o755,
    );

    let output = run_local_client(&sandbox, project.path());

    // The child inherited stdout and its status is the process status: the
    // image really was replaced.
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(stdout_of(&output).contains("child ran"), "{output:?}");

    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "exec_attempt");
    // Fixed 0: censored, not "the native client succeeded".
    assert_eq!(event["exit_code"], 0);
}

#[cfg(unix)]
#[tokio::test]
async fn child_exit_status_is_preserved_and_the_event_stays_censored() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    install_fake_clickhouse(
        &sandbox,
        FAKE_VERSION,
        "#!/bin/sh\nprintf 'child ran\\n'\nexit 23\n",
        0o755,
    );

    let output = run_local_client(&sandbox, project.path());

    // The shell sees the native client's own status, unmodified.
    assert_eq!(output.status.code(), Some(23), "{}", stderr_of(&output));
    assert!(stdout_of(&output).contains("child ran"), "{output:?}");

    // And the event does not claim to know it: `exec_attempt` with a fixed 0
    // is the documented censored reading, never `ok` and never the child's 23.
    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "exec_attempt");
    assert_eq!(event["exit_code"], 0);
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

    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["outcome"], "exec_attempt");
    assert_eq!(event["exit_code"], 0);
}

#[cfg(unix)]
#[tokio::test]
async fn signal_terminated_child_reaches_the_shell_as_a_signal() {
    use std::os::unix::process::ExitStatusExt;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    install_fake_clickhouse(&sandbox, FAKE_VERSION, "#!/bin/sh\nkill -TERM $$\n", 0o755);

    let output = run_local_client(&sandbox, project.path());

    // Killed by a signal, so there is no exit code at all — the shell sees the
    // native client's death, not a wrapper's translation of it.
    assert_eq!(output.status.code(), None);
    assert_eq!(output.status.signal(), Some(15));

    // And telemetry says only that the handoff was reached.
    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "exec_attempt");
    assert_eq!(event["exit_code"], 0);
}

#[cfg(unix)]
#[tokio::test]
async fn non_executable_binary_is_an_error_event_not_a_handoff() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    install_fake_clickhouse(&sandbox, FAKE_VERSION, "#!/bin/sh\nexit 0\n", 0o644);

    let output = run_local_client(&sandbox, project.path());

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("not executable"), "{stderr}");
    assert!(
        stderr.contains("clickhousectl local install"),
        "the error should say how to repair the install: {stderr}"
    );

    // A launch that never happened is a failure, not an accepted handoff.
    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["exit_code"], 1);
    assert_eq!(event["exit_code"], output.status.code().unwrap());
}

#[cfg(unix)]
#[tokio::test]
async fn directory_in_place_of_the_binary_is_an_error_event() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    // A directory named `clickhouse` satisfies the `exists()` resolution
    // checks, so this reaches the launch pre-flight.
    let binary = sandbox
        .home
        .path()
        .join(".clickhouse/versions")
        .join(FAKE_VERSION)
        .join("clickhouse");
    std::fs::create_dir_all(&binary).unwrap();

    let output = run_local_client(&sandbox, project.path());

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("not a regular file"), "{stderr}");

    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["exit_code"], 1);
}

#[cfg(unix)]
#[tokio::test]
async fn binary_removed_before_the_pre_flight_is_an_error_event() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    let binary = install_fake_clickhouse(&sandbox, FAKE_VERSION, "#!/bin/sh\nexit 0\n", 0o755);
    // Version resolution lists the directory, but nothing is left to launch:
    // removal is detected by the pre-flight, so it is an ordinary error.
    std::fs::remove_file(&binary).unwrap();
    std::fs::write(binary.parent().unwrap().join("clickhouse.bak"), "stub").unwrap();

    let output = run_local_client(&sandbox, project.path());

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");

    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["exit_code"], 1);
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
    assert!(
        stderr.contains("Failed to execute ClickHouse"),
        "the OS launch failure must still reach the shell: {stderr}"
    );

    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local client");
    assert_eq!(event["outcome"], "exec_attempt");
    assert_eq!(event["exit_code"], 0);
}

#[cfg(unix)]
#[tokio::test]
async fn bad_executable_format_is_censored_never_successful() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    // Executable, but not an executable format. The pre-flight passes, and
    // `execvp` gets ENOEXEC — which POSIX has it answer by re-execing the file
    // through `/bin/sh`, so the image *is* replaced and the shell reports the
    // failure with its own non-zero status. Either way clickhousectl is gone by
    // then: the outcome the event can honestly carry is the censored attempt,
    // and the status the user sees is not clickhousectl's to choose.
    install_fake_clickhouse(&sandbox, FAKE_VERSION, "\u{0}\u{1}not a binary\n", 0o755);

    let output = run_local_client(&sandbox, project.path());

    let stderr = stderr_of(&output);
    assert!(
        !output.status.success(),
        "a file that is not an executable format must fail: {stderr}"
    );

    let event = exactly_one_event(&sandbox).await;
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
    assert!(stderr.contains("could not execute psql"), "{stderr}");

    // The other `exec()` handoff: a program that cannot be launched is an
    // error, not a censored attempt.
    let event = exactly_one_event(&sandbox).await;
    assert_eq!(event["command"], "local postgres client");
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["exit_code"], 1);
}

#[tokio::test]
async fn flag_names_sent_but_values_never_leak() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    // --json is a real flag; its name may appear but the CI-style value
    // asserts cover named flags with values via the unit tests. Here we pin
    // the end-to-end shape: flags is an array of known names only.
    let output = sandbox.run(&["local", "--json", "list"]);
    assert!(output.status.success());

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local list");
    assert_eq!(event["flags"], serde_json::json!(["json"]));
}

// -- positional presence (#480) ---------------------------------------------

/// The three shapes issue #480 could not tell apart, end to end: a bare
/// lifecycle command, the same command with a named server, and the
/// compatibility `--name` flag. `positionals` is the discriminator and never
/// carries the name itself.
#[tokio::test]
async fn positional_presence_distinguishes_bare_from_named_stop() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();

    let cases: &[(&[&str], Value)] = &[
        (&["local", "server", "stop"], serde_json::json!([])),
        (
            &["local", "server", "stop", "SECRET-SERVER-NAME"],
            serde_json::json!(["name"]),
        ),
        (
            &["local", "server", "stop", "--name", "SECRET-SERVER-NAME"],
            serde_json::json!([]),
        ),
    ];

    for (index, (args, expected)) in cases.iter().enumerate() {
        let output = sandbox
            .command(args)
            .current_dir(project.path())
            .output()
            .unwrap();
        // Whether the stop succeeds is irrelevant here; the event shape is not.
        let payloads = sandbox.wait_for_requests(index + 1).await;
        let event = &payloads[index];
        assert_eq!(event["command"], "local server stop", "for {args:?}");
        assert_eq!(event["positionals"], *expected, "for {args:?}");
        let raw = serde_json::to_string(event).unwrap();
        assert!(
            !raw.contains("SECRET"),
            "server name leaked for {args:?}: {raw} (stderr: {})",
            stderr_of(&output)
        );
    }

    // The flag form records the name as a *flag*, so the two naming styles
    // stay distinguishable without recording the value.
    let payloads = sandbox.wait_for_requests(3).await;
    assert_eq!(payloads[2]["flags"], serde_json::json!(["name"]));
}

/// A missing required positional (parse failure) and a supplied one (dispatch)
/// are now different events, which is what the August 2026 investigation
/// could not separate.
#[tokio::test]
async fn missing_required_positional_is_distinguishable_from_a_supplied_one() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();

    let output = sandbox
        .command(&["local", "use"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local use");
    assert_eq!(event["outcome"], "missing_required");
    assert_eq!(event["positionals"], serde_json::json!([]));

    // Supplied but not installed: the parse succeeded, so the slot is present
    // and the failure is a handler failure, not a usage error.
    let output = sandbox
        .command(&["local", "remove", "25.12.9.61"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    let payloads = sandbox.wait_for_requests(2).await;
    let event = &payloads[1];
    assert_eq!(event["command"], "local remove");
    assert_eq!(event["outcome"], "error");
    assert_eq!(event["positionals"], serde_json::json!(["version"]));
    let raw = serde_json::to_string(event).unwrap();
    assert!(!raw.contains("25.12.9.61"), "version leaked: {raw}");
}

/// Hostile, secret-shaped positionals: presence is recorded, the value is not,
/// and arguments forwarded to another program are not recorded at all.
#[tokio::test]
async fn hostile_positionals_never_appear_on_the_wire() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    const HOSTILE: &str = "postgres://user:pa55w0rd@db.internal:5432/prod";

    let cases: &[(&[&str], &str, Value)] = &[
        (
            &["local", "server", "stop", HOSTILE],
            "local server stop",
            serde_json::json!(["name"]),
        ),
        // Forwarded to clickhouse-client after `--`: not this CLI's shape.
        (
            &["local", "client", "--", HOSTILE],
            "local client",
            serde_json::json!([]),
        ),
    ];

    for (index, (args, command, expected)) in cases.iter().enumerate() {
        let output = sandbox
            .command(args)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{}", stdout_of(&output));
        let payloads = sandbox.wait_for_requests(index + 1).await;
        let event = &payloads[index];
        assert_eq!(event["command"], *command, "for {args:?}");
        assert_eq!(event["positionals"], *expected, "for {args:?}");
        let raw = serde_json::to_string(event).unwrap();
        for fragment in ["postgres://", "pa55w0rd", "db.internal", HOSTILE] {
            assert!(
                !raw.contains(fragment),
                "hostile positional leaked for {args:?}: {raw}"
            );
        }
    }
}

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
        stderr_of(&output).contains("anonymous usage data"),
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
    assert!(!stderr_of(&output).contains("anonymous usage data"));
    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "cloud");
    assert_eq!(event["flags"], serde_json::json!(["help"]));
    assert_eq!(event["outcome"], "help");
    assert_eq!(event["exit_code"], 0);
}

#[tokio::test]
async fn bare_invocation_reports_missing_subcommand() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&[]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "clap usage errors keep exit 2"
    );

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "");
    assert_eq!(event["outcome"], "missing_subcommand");
    assert_eq!(event["exit_code"], 2);
}

#[tokio::test]
async fn invalid_subcommand_reports_kind_but_never_the_token() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    // An agent-hallucinated command: the event records that it happened and
    // where the valid prefix ended — never the unmatched token itself, which
    // is indistinguishable from a secret pasted into the wrong window.
    let output = sandbox.run(&["hallucinated-subcommand-xyz"]);
    assert_eq!(output.status.code(), Some(2));

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "");
    assert_eq!(event["outcome"], "invalid_subcommand");
    let raw = serde_json::to_string(event).unwrap();
    assert!(
        !raw.contains("hallucinated-subcommand-xyz"),
        "raw token leaked into the payload: {raw}"
    );
}

#[tokio::test]
async fn failed_parse_after_positional_captures_later_flags_without_values() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&[
        "cloud",
        "org",
        "usage",
        "SECRET-ORG-ID",
        "--from-date",
        "SECRET-FROM-DATE",
        "--to-date",
        "SECRET-TO-DATE",
    ]);
    assert_eq!(output.status.code(), Some(2));

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "cloud org usage");
    assert_eq!(event["flags"], serde_json::json!(["from-date", "to-date"]));
    // The deprecated positional org-id form is now visible as presence — the
    // exact signal #480 asked for, with the id still off the wire.
    assert_eq!(event["positionals"], serde_json::json!(["legacy_org_id"]));
    assert_eq!(event["exit_code"], 2);
    assert_eq!(event["outcome"], "invalid_value");
    let raw = serde_json::to_string(event).unwrap();
    assert!(!raw.contains("SECRET"), "argument value leaked: {raw}");
}

#[tokio::test]
async fn invalid_local_versions_report_invalid_value_without_dispatch_side_effects() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);
    let project = tempfile::tempdir().unwrap();
    let cases: &[(&[&str], &str, &str)] = &[
        (
            &["local", "install", "not.a.version"],
            "local install",
            "not.a.version",
        ),
        (&["local", "use", "25.12.9"], "local use", "25.12.9"),
        (
            &["local", "server", "start", "--version", "25.12.9.61.2"],
            "local server start",
            "25.12.9.61.2",
        ),
        (&["local", "use", "postgres@18"], "local use", "postgres@18"),
    ];

    for (index, (args, command, operand)) in cases.iter().enumerate() {
        let output = sandbox
            .command(args)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
        assert!(
            stderr_of(&output).contains("error: invalid value"),
            "{}",
            stderr_of(&output)
        );

        let payloads = sandbox.wait_for_requests(index + 1).await;
        let event = &payloads[index];
        assert_eq!(event["command"], *command);
        assert_eq!(event["exit_code"], 2);
        assert_eq!(event["outcome"], "invalid_value");
        let raw = serde_json::to_string(event).unwrap();
        assert!(!raw.contains(operand), "version operand leaked: {raw}");
    }

    let home_state = sandbox.home.path().join(".clickhouse");
    assert!(!home_state.join("versions").exists());
    assert!(!home_state.join("default").exists());
    assert!(!sandbox.home.path().join(".local").exists());
    assert!(!project.path().join(".clickhouse").exists());
}

#[tokio::test]
async fn typo_carries_definition_derived_suggestion() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["cloud", "servce", "list"]);
    assert_eq!(output.status.code(), Some(2));

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "cloud");
    assert_eq!(event["outcome"], "invalid_subcommand");
    // Clap's did-you-mean names the *defined* subcommand, so recording it is
    // safe; the typo'd token stays off the wire.
    assert_eq!(event["suggestion"], "service");
    let raw = serde_json::to_string(event).unwrap();
    assert!(!raw.contains("servce"), "typo leaked: {raw}");
}

#[tokio::test]
async fn do_not_track_is_fully_silent() {
    let sandbox = Sandbox::new().await;

    let output = sandbox
        .command(&["local", "list"])
        .env("DO_NOT_TRACK", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(!stderr_of(&output).contains("anonymous usage data"));
    assert!(
        !sandbox.state_path().exists(),
        "DO_NOT_TRACK must not write the marker file"
    );
    sandbox.assert_no_requests().await;
}

/// A `DO_NOT_TRACK` value that is set but not valid UTF-8 must still opt out.
/// `std::env::var` errors on such values; if the lookup treated that as
/// "unset" the opt-out would fail open (see `real_env_lookup`, which uses
/// `var_os` + lossy conversion for exactly this reason).
#[cfg(unix)]
#[tokio::test]
async fn non_utf8_do_not_track_is_fully_silent() {
    use std::os::unix::ffi::OsStringExt;

    let sandbox = Sandbox::new().await;

    let output = sandbox
        .command(&["local", "list"])
        .env(
            "DO_NOT_TRACK",
            std::ffi::OsString::from_vec(vec![0xff, 0xfe]),
        )
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(!stderr_of(&output).contains("anonymous usage data"));
    assert!(
        !sandbox.state_path().exists(),
        "non-UTF-8 DO_NOT_TRACK must not write the marker file"
    );
    sandbox.assert_no_requests().await;
}

#[tokio::test]
async fn disable_persists_and_silences() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["telemetry", "disable"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Telemetry disabled."));
    assert_eq!(
        std::fs::read_to_string(sandbox.state_path()).unwrap(),
        r#"{"disabled":true}"#
    );

    let output = sandbox.run(&["local", "list"]);
    assert!(output.status.success());
    assert!(!stderr_of(&output).contains("anonymous usage data"));

    let output = sandbox.run(&["telemetry", "status"]);
    assert!(stdout_of(&output).contains("disabled"));

    // Neither the disable itself, nor anything after it, sent an event.
    sandbox.assert_no_requests().await;
}

#[tokio::test]
async fn enable_sends_an_event_for_itself() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(true);

    let output = sandbox.run(&["telemetry", "enable"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Telemetry enabled."));
    // Without DO_NOT_TRACK there is nothing to warn about.
    assert!(!stderr_of(&output).contains("DO_NOT_TRACK is set"));

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

    // stdout stays the clean confirmation; the note goes to stderr.
    assert!(stdout_of(&output).contains("Telemetry enabled."));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("DO_NOT_TRACK") && stderr.contains("remain silent"),
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

    // Twice: silent every run, never a repeated notice, never an error.
    for _ in 0..2 {
        let output = sandbox.run(&["telemetry", "status"]);
        assert!(output.status.success());
        assert!(!stderr_of(&output).contains("anonymous usage data"));
        assert!(stdout_of(&output).contains("not yet configured"));
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
    assert!(stdout_of(&output).contains("Telemetry enabled."));
}
