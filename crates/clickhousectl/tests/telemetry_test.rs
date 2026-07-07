//! Telemetry end-to-end tests (issue #283).
//!
//! Each test invokes the real `clickhousectl` binary as a subprocess with
//! `HOME` pointed at a temp dir (sandboxing `~/.clickhouse/telemetry.json`)
//! and `CHCTL_TELEMETRY_URL` pointed at a local `wiremock` server, then
//! asserts on the consent flow (notice/marker/silence) and on the recorded
//! payload shape — in particular that flag values and positional arguments
//! never appear on the wire.
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
}

impl Sandbox {
    async fn new() -> Self {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/telemetry"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})),
            )
            .mount(&mock)
            .await;
        Sandbox {
            home: tempfile::tempdir().unwrap(),
            mock,
        }
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
            .env(
                "CHCTL_TELEMETRY_URL",
                format!("{}/v1/telemetry", self.mock.uri()),
            )
            .env_remove("DO_NOT_TRACK")
            .env_remove("CHCTL_TELEMETRY_DEBUG")
            .env_remove("CHCTL_TELEMETRY_PAYLOAD")
            .env_remove("CI");
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("failed to spawn binary")
    }

    /// Poll until the mock has seen `n` requests; panics after ~5s. The send
    /// child is detached, so arrival is asynchronous.
    async fn wait_for_requests(&self, n: usize) -> Vec<Value> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let requests = self.mock.received_requests().await.unwrap_or_default();
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
        let requests = self.mock.received_requests().await.unwrap_or_default();
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

// ---------------------------------------------------------------------------

#[tokio::test]
async fn first_run_writes_marker_prints_notice_sends_nothing() {
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
    assert!(!stderr_of(&output).contains("anonymous usage data"));
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

    // The send goes through the canonical http::client_builder(), so it
    // carries the same User-Agent as every other outbound request. The
    // ingest worker relies on the `clickhousectl/<version>` prefix to
    // reject non-CLI traffic (an ` (agent=...)` comment may follow).
    let requests = sandbox.mock.received_requests().await.unwrap();
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
async fn failure_reported_and_positional_value_never_leaks() {
    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let output = sandbox.run(&["local", "remove", "no-such-version-xyz"]);
    assert!(!output.status.success());

    let payloads = sandbox.wait_for_requests(1).await;
    let event = &payloads[0];
    assert_eq!(event["command"], "local remove");
    // The event carries the gh-style exit code the process exited with.
    assert_eq!(event["exit_code"], 1);
    assert_eq!(event["exit_code"], output.status.code().unwrap());
    let raw = serde_json::to_string(event).unwrap();
    assert!(
        !raw.contains("no-such-version-xyz"),
        "positional argument leaked into the payload: {raw}"
    );
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

    // Consent is evaluated after the command ran, so the enable run itself
    // is the first event.
    let payloads = sandbox.wait_for_requests(1).await;
    assert_eq!(payloads[0]["command"], "telemetry enable");
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
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/telemetry"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
        .mount(&mock)
        .await;

    let sandbox = Sandbox::new().await;
    sandbox.write_state(false);

    let started = Instant::now();
    let output = sandbox
        .command(&["local", "list"])
        .env("CHCTL_TELEMETRY_URL", format!("{}/v1/telemetry", mock.uri()))
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
    assert!(entries.contains(&"telemetry.json".to_string()), "{entries:?}");
}
