//! `clickhousectl feedback` end-to-end tests (#1211).
//!
//! Each test runs the real binary with `HOME` at a temp dir (sandboxing
//! `~/.clickhouse/telemetry.json`) and `CHCTL_FEEDBACK_URL` at a local
//! `wiremock` server, then asserts on the request the binary sent, its exit
//! code and its streams. Assertions use codes, JSON fields and identifiers,
//! never wording. One case per distinct runtime path: message validation,
//! the status-to-reason mapping and the payload's field values are unit
//! tests in `src/feedback.rs`.
//!
//!     cargo test -p clickhousectl --test feedback_test

#![cfg(feature = "feedback")]

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FEEDBACK_PATH: &str = "/v1/feedback";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// A sandboxed home directory plus the feedback ingest mock, which answers
/// `status` at [`FEEDBACK_PATH`].
struct Sandbox {
    home: tempfile::TempDir,
    mock: MockServer,
}

impl Sandbox {
    async fn new() -> Self {
        Self::responding(200).await
    }

    async fn responding(status: u16) -> Self {
        Self::with_response(
            ResponseTemplate::new(status).set_body_json(serde_json::json!({"ok": true})),
        )
        .await
    }

    async fn with_response(response: ResponseTemplate) -> Self {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(FEEDBACK_PATH))
            .respond_with(response)
            .mount(&mock)
            .await;
        Sandbox {
            home: tempfile::tempdir().unwrap(),
            mock,
        }
    }

    fn write_telemetry_state(&self, disabled: bool) {
        let dir = self.home.path().join(".clickhouse");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("telemetry.json"),
            format!(r#"{{"disabled":{disabled}}}"#),
        )
        .unwrap();
    }

    /// The binary, sandboxed: inherited settings cleared for determinism (the
    /// harness itself may run under CI or a coding agent), telemetry silenced
    /// with `DO_NOT_TRACK` unless a test removes it, and any telemetry send
    /// pointed at the mock rather than production.
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(clickhousectl_binary());
        cmd.arg("feedback")
            .args(args)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", self.home.path())
            .env("DO_NOT_TRACK", "1")
            .env(
                "CHCTL_FEEDBACK_URL",
                format!("{}{FEEDBACK_PATH}", self.mock.uri()),
            )
            .env(
                "CHCTL_TELEMETRY_URL",
                format!("{}/v1/telemetry", self.mock.uri()),
            )
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0");
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("failed to spawn binary")
    }

    async fn requests(&self) -> Vec<wiremock::Request> {
        self.mock.received_requests().await.unwrap_or_default()
    }

    /// The single request the binary sent, which must be a feedback POST.
    async fn the_request(&self) -> wiremock::Request {
        let mut requests = self.requests().await;
        assert_eq!(requests.len(), 1, "expected exactly one request");
        let request = requests.remove(0);
        assert_eq!(request.method.as_str(), "POST");
        assert_eq!(request.url.path(), FEEDBACK_PATH);
        request
    }
}

fn body_of(request: &wiremock::Request) -> Value {
    serde_json::from_slice(&request.body).expect("feedback body must be JSON")
}

fn header<'a>(request: &'a wiremock::Request, name: &str) -> Option<&'a str> {
    request.headers.get(name).map(|v| v.to_str().unwrap())
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not one JSON value ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

/// The telemetry debug event the binary printed to stderr.
fn telemetry_event(output: &Output) -> Value {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("command").is_some())
        .unwrap_or_else(|| panic!("no telemetry debug payload on stderr:\n{stderr}"))
}

#[tokio::test]
async fn an_agent_session_sends_one_json_object_with_its_headers() {
    let sandbox = Sandbox::new().await;
    let output = sandbox
        .command(&["  service create", "needs", "a wait flag  "])
        .env("CLAUDECODE", "1")
        .env("CLAUDE_CODE_SESSION_ID", "sess-feedback-1")
        .env(
            "TRACEPARENT",
            "00-11111111111111111111111111111111-2222222222222222-01",
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    // A detected agent gets JSON without `--json`.
    let message = "service create needs a wait flag";
    assert_eq!(
        stdout_json(&output),
        serde_json::json!({"sent": true, "length": message.len()})
    );

    let request = sandbox.the_request().await;
    assert_eq!(header(&request, "content-type"), Some("application/json"));
    assert!(
        header(&request, "user-agent")
            .unwrap()
            .starts_with(&format!("clickhousectl/{}", env!("CARGO_PKG_VERSION")))
    );
    assert_eq!(
        header(&request, "agent-session-id"),
        Some("sess-feedback-1")
    );
    assert_eq!(
        header(&request, "traceparent"),
        Some("00-11111111111111111111111111111111-2222222222222222-01")
    );

    let body = body_of(&request);
    let mut keys: Vec<&str> = body
        .as_object()
        .expect("the body is one JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "agent", "arch", "ci", "is_agent", "message", "os", "version"
        ]
    );
    assert_eq!(body["message"], message);
}

#[tokio::test]
async fn an_invalid_message_is_a_usage_error_and_sends_nothing() {
    let sandbox = Sandbox::new().await;
    let output = sandbox.run(&["--json", "colour \u{1b}[31mred"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(!output.stderr.is_empty());
    assert!(sandbox.requests().await.is_empty());
}

#[tokio::test]
async fn a_rejected_message_exits_1_without_a_success_result() {
    let sandbox = Sandbox::responding(429).await;
    let output = sandbox.run(&["--json", "the gap"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(!output.stderr.is_empty());
    assert_eq!(sandbox.requests().await.len(), 1, "a send is never retried");
}

#[tokio::test]
async fn an_unreachable_endpoint_exits_1() {
    let sandbox = Sandbox::new().await;
    let output = sandbox
        .command(&["the gap"])
        // Port 1 is reserved and refuses connections.
        .env("CHCTL_FEEDBACK_URL", "http://127.0.0.1:1/v1/feedback")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert!(sandbox.requests().await.is_empty());
}

#[tokio::test]
async fn a_slow_endpoint_times_out_and_exits_1() {
    let sandbox = Sandbox::with_response(
        ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"ok": true}))
            .set_delay(std::time::Duration::from_secs(5)),
    )
    .await;
    let output = sandbox
        .command(&["the gap"])
        .env("CHCTL_FEEDBACK_SEND_TIMEOUT_MS", "100")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(!output.stderr.is_empty());
}

#[tokio::test]
async fn telemetry_settings_do_not_block_feedback() {
    // DO_NOT_TRACK=1 (the sandbox default) and a saved opt-out together.
    let sandbox = Sandbox::new().await;
    sandbox.write_telemetry_state(true);
    let output = sandbox.run(&["the gap"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!output.stdout.is_empty());
    assert_eq!(body_of(&sandbox.the_request().await)["message"], "the gap");
}

/// Telemetry never records the message: not after a send, not after a parse
/// failure, not for a help request. The slot takes every remaining word, so
/// telemetry classifies it as a passthrough slot and records nothing from it.
#[cfg(feature = "telemetry")]
#[tokio::test]
async fn the_telemetry_event_for_feedback_never_carries_the_message() {
    let token = "zebra-quartz-7Q2";
    for (args, exit_code) in [
        (&["the", token, "gap"][..], 0),
        // `--json` given twice is a usage error.
        (&["--json", "--json", "the", token, "gap"][..], 2),
        // Help before the message is still help.
        (&["-h", "the", token, "gap"][..], 0),
    ] {
        let sandbox = Sandbox::new().await;
        sandbox.write_telemetry_state(false);
        let output = sandbox
            .command(args)
            .env_remove("DO_NOT_TRACK")
            // Print the event instead of sending it.
            .env("CHCTL_TELEMETRY_DEBUG", "1")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit_code),
            "{args:?}: {output:?}"
        );

        let event = telemetry_event(&output);
        assert_eq!(event["command"], "feedback", "{args:?}");
        assert!(!event.to_string().contains(token), "{args:?}: {event}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains(token), "{args:?}: {stderr}");

        if exit_code == 0 && args[0] != "-h" {
            assert_eq!(event["positionals"], serde_json::json!([]));
            assert_eq!(event["outcome"], "ok");
            // The feedback itself was the only request, and it did carry the text.
            let request = sandbox.the_request().await;
            assert!(
                body_of(&request)["message"]
                    .as_str()
                    .unwrap()
                    .contains(token)
            );
        } else {
            assert!(sandbox.requests().await.is_empty(), "{args:?}");
        }
    }
}
