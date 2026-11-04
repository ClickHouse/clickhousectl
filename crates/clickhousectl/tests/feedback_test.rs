//! `clickhousectl feedback` end-to-end tests (#1211).
//!
//! Each test runs the real binary with `HOME` and the working directory at
//! temp dirs (sandboxing `~/.clickhouse/telemetry.json`, saved OAuth tokens
//! and the project's `.clickhouse/credentials.json`) and `CHCTL_FEEDBACK_URL`
//! at a local `wiremock` server. Saved Cloud credentials name the same server
//! as their API, so the best-effort whoami lookup lands there too. Tests
//! assert on the requests the binary sent, its exit code and its streams,
//! using codes, JSON fields and identifiers, never wording. One case per
//! distinct runtime path: message validation, the status-to-reason mapping,
//! the whoami-to-org mapping and the payload's field values are unit tests in
//! `src/feedback.rs`.
//!
//!     cargo test -p clickhousectl --test feedback_test

#![cfg(feature = "feedback")]

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FEEDBACK_PATH: &str = "/v1/feedback";
const WHOAMI_PATH: &str = "/v1/whoami";

const ORG_A: &str = "0b1c2d3e-1211-4000-8000-00000000000a";
const ORG_B: &str = "4f5a6b7c-1211-4000-8000-00000000000b";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// A sandboxed home and project directory plus one mock server: the
/// feedback ingest answering at [`FEEDBACK_PATH`], and the Cloud API for any
/// credentials a test saves.
struct Sandbox {
    home: tempfile::TempDir,
    project: tempfile::TempDir,
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
            project: tempfile::tempdir().unwrap(),
            mock,
        }
    }
    /// The Cloud API base URL saved credentials point at.
    fn api_url(&self) -> String {
        format!("{}/v1", self.mock.uri())
    }

    /// Answer `GET /v1/whoami` with `status`, wrapping a 200 `identity` in
    /// the API's response envelope.
    async fn mount_whoami(&self, status: u16, identity: Value) {
        self.mount_whoami_response(whoami_response(status, identity))
            .await;
    }

    async fn mount_whoami_response(&self, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path(WHOAMI_PATH))
            .respond_with(response)
            .mount(&self.mock)
            .await;
    }

    /// Save an OAuth login for the mock API in `~/.clickhouse/tokens.json`,
    /// expiring at `expires_at` (Unix seconds).
    fn write_oauth_tokens(&self, expires_at: u64) -> PathBuf {
        self.write_oauth_tokens_in(self.home.path(), expires_at)
    }

    /// Save an OAuth login in `<root>/.clickhouse/tokens.json`.
    fn write_oauth_tokens_in(&self, root: &std::path::Path, expires_at: u64) -> PathBuf {
        let dir = root.join(".clickhouse");
        std::fs::create_dir_all(&dir).unwrap();
        let tokens = serde_json::json!({
            "access_token": "test-bearer-token",
            "refresh_token": "test-refresh-token",
            "expires_at": expires_at,
            "api_url": self.api_url(),
            "auth_config": {"auth_url": self.mock.uri(), "client_id": "test-client"},
        });
        let path = dir.join("tokens.json");
        std::fs::write(&path, serde_json::to_vec(&tokens).unwrap()).unwrap();
        path
    }

    /// Save a project API key pair for the mock API, the way
    /// `cloud auth login --api-key ... --url <mock>` does.
    fn write_saved_api_key(&self) {
        let dir = self.project.path().join(".clickhouse");
        std::fs::create_dir_all(&dir).unwrap();
        let credentials = serde_json::json!({
            "api_key": "test-key",
            "api_secret": "test-secret",
            "api_url": self.api_url(),
        });
        std::fs::write(
            dir.join("credentials.json"),
            serde_json::to_vec(&credentials).unwrap(),
        )
        .unwrap();
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
            .current_dir(self.project.path())
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

    /// The whoami lookups the binary made.
    async fn whoami_requests(&self) -> Vec<wiremock::Request> {
        let mut requests = self.requests().await;
        requests.retain(|request| request.url.path() == WHOAMI_PATH);
        requests
    }

    /// The single feedback POST, alongside at most one whoami lookup.
    async fn the_feedback_request(&self) -> wiremock::Request {
        let mut requests = self.requests().await;
        let whoami = requests
            .iter()
            .filter(|request| request.url.path() == WHOAMI_PATH)
            .count();
        assert!(whoami <= 1, "whoami is never retried");
        requests.retain(|request| request.url.path() != WHOAMI_PATH);
        assert_eq!(requests.len(), 1, "expected exactly one feedback request");
        let request = requests.remove(0);
        assert_eq!(request.method.as_str(), "POST");
        assert_eq!(request.url.path(), FEEDBACK_PATH);
        request
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

/// A whoami response: a 200 wraps `identity` in the API's envelope.
fn whoami_response(status: u16, identity: Value) -> ResponseTemplate {
    if status == 200 {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "whoami",
            "result": identity,
        }))
    } else {
        ResponseTemplate::new(status)
            .set_body_json(serde_json::json!({"status": status, "error": "refused"}))
    }
}

/// Every file under `root` with its contents, to show a run wrote or removed
/// nothing.
fn snapshot(root: &std::path::Path) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let contents = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
                files.push((path, contents));
            }
        }
    }
    files.sort();
    files
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
            "agent",
            "arch",
            "ci",
            "cloud_auth",
            "is_agent",
            "message",
            "org_ids",
            "os",
            "version"
        ]
    );
    assert_eq!(body["message"], message);
    // No Cloud credentials: no whoami (the feedback POST was the only
    // request) and no credential kind.
    assert_eq!(body["cloud_auth"], "none");
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

// ── Cloud identity ──────────────────────────────────────────────────────────

fn user_identity() -> Value {
    serde_json::json!({
        "actorType": "user",
        "userId": "5fae43a3-1211-4000-8000-0000000000aa",
        "email": "ada@example.com",
        "organizations": [
            {"organizationId": ORG_A, "organizationName": "First"},
            {"organizationId": ORG_B, "organizationName": "Second"},
        ],
    })
}

#[tokio::test]
async fn a_saved_api_key_sends_the_organization_that_owns_it() {
    let sandbox = Sandbox::new().await;
    sandbox.write_saved_api_key();
    sandbox
        .mount_whoami(
            200,
            serde_json::json!({
                "actorType": "apiKey",
                "keyId": "c0ffee00-1211-4000-8000-0000000000cc",
                "organizationId": ORG_A,
            }),
        )
        .await;
    let output = sandbox.run(&["the gap"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    // No stored-API notice or any other diagnostic.
    assert!(output.stderr.is_empty(), "{output:?}");

    let whoami = sandbox.whoami_requests().await;
    assert_eq!(whoami.len(), 1);
    assert!(
        header(&whoami[0], "authorization")
            .unwrap()
            .starts_with("Basic ")
    );

    let feedback = sandbox.the_feedback_request().await;
    // The feedback endpoint never receives the Cloud credentials.
    assert!(header(&feedback, "authorization").is_none());
    let body = body_of(&feedback);
    assert_eq!(body["cloud_auth"], "api_key");
    assert_eq!(body["org_ids"], serde_json::json!([ORG_A]));
}

#[tokio::test]
async fn an_oauth_login_sends_every_organization() {
    let sandbox = Sandbox::new().await;
    sandbox.write_oauth_tokens(4102444800); // 2100-01-01
    sandbox.mount_whoami(200, user_identity()).await;
    let output = sandbox.run(&["the gap"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");

    let whoami = sandbox.whoami_requests().await;
    assert_eq!(whoami.len(), 1);
    assert_eq!(
        header(&whoami[0], "authorization"),
        Some("Bearer test-bearer-token")
    );

    let feedback = sandbox.the_feedback_request().await;
    assert!(header(&feedback, "authorization").is_none());
    let body = body_of(&feedback);
    assert_eq!(body["cloud_auth"], "oauth");
    assert_eq!(body["org_ids"], serde_json::json!([ORG_A, ORG_B]));
}

/// Every whoami failure takes the same path, so one per credential kind: a
/// rejected API key, and an OAuth lookup that outlasts its timeout.
#[tokio::test]
async fn a_failed_whoami_keeps_the_credential_kind_and_still_sends() {
    let api_key = Sandbox::new().await;
    api_key.write_saved_api_key();
    api_key.mount_whoami(401, Value::Null).await;

    let oauth = Sandbox::new().await;
    oauth.write_oauth_tokens(4102444800);
    oauth
        .mount_whoami_response(
            whoami_response(200, user_identity()).set_delay(std::time::Duration::from_secs(2)),
        )
        .await;

    for (sandbox, kind, timeout_ms) in [(&api_key, "api_key", None), (&oauth, "oauth", Some("100"))]
    {
        let mut command = sandbox.command(&["the gap"]);
        if let Some(timeout_ms) = timeout_ms {
            command.env("CHCTL_FEEDBACK_WHOAMI_TIMEOUT_MS", timeout_ms);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{kind}: {output:?}");
        assert!(!output.stdout.is_empty(), "{kind}");
        assert!(output.stderr.is_empty(), "{kind}: {output:?}");
        assert_eq!(
            sandbox.whoami_requests().await.len(),
            1,
            "{kind}: whoami is never retried"
        );
        let body = body_of(&sandbox.the_feedback_request().await);
        assert_eq!(body["cloud_auth"], kind);
        assert_eq!(body["org_ids"], serde_json::json!([]), "{kind}");
    }
}

/// Agents run feedback unprompted in any directory, so a login saved by an
/// old version under `./.clickhouse/` is neither migrated to the global
/// store, removed, nor used.
#[tokio::test]
async fn a_legacy_project_login_is_left_alone_and_unused() {
    let sandbox = Sandbox::new().await;
    sandbox.write_oauth_tokens_in(sandbox.project.path(), 4102444800);
    sandbox.mount_whoami(200, user_identity()).await;
    let home = snapshot(sandbox.home.path());
    let project = snapshot(sandbox.project.path());
    let output = sandbox.run(&["the gap"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(snapshot(sandbox.home.path()), home);
    assert_eq!(snapshot(sandbox.project.path()), project);
    // The feedback POST was the only request: no whoami.
    let body = body_of(&sandbox.the_request().await);
    assert_eq!(body["cloud_auth"], "none");
    assert_eq!(body["org_ids"], serde_json::json!([]));
}

/// An expired login is not usable, so feedback treats it as no credentials:
/// it does not refresh it, rewrite it, or call whoami.
#[tokio::test]
async fn an_expired_oauth_login_is_neither_refreshed_nor_used() {
    let sandbox = Sandbox::new().await;
    let tokens = sandbox.write_oauth_tokens(1_000_000_000); // 2001
    let saved = std::fs::read(&tokens).unwrap();
    sandbox.mount_whoami(200, user_identity()).await;
    let output = sandbox.run(&["the gap"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    // The feedback POST was the only request: no whoami, no token refresh.
    let body = body_of(&sandbox.the_request().await);
    assert_eq!(body["cloud_auth"], "none");
    assert_eq!(std::fs::read(&tokens).unwrap(), saved);
}
