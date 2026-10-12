//! `cloud access request`: obtain a scoped API key through the control-plane
//! access-request PKCE handshake.
//!
//! The user authenticates with their existing CLI identity (OAuth Bearer from
//! `cloud auth login`, or an API key), files an access request carrying a PKCE
//! challenge and a CSRF `state`, approves it in a browser, and the CLI claims
//! the issued key over a loopback redirect — falling back to polling the
//! request's status. The claimed key id and secret are stored with the CLI's
//! existing credential storage; the secret is never printed or placed in a URL.
//!
//! These endpoints are not modelled by the typed `clickhouse-cloud-api` client,
//! so the handshake calls them directly with a plain `reqwest` client carrying
//! the credentials resolved by [`CloudClient`] ([`CloudClient::request_auth`]).

use crate::cloud::client::{CloudClient, CloudError, RequestAuth, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::eprint_line;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clap::{Args, Subcommand};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Default control-plane base URL. Points at the local dev control plane so the
/// MVP is testable without production credentials; override with `--url`.
const DEFAULT_ACCESS_API_BASE: &str = "http://localhost:2023";

/// How long to wait for a human to approve the request before giving up.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// How often to poll the request's status as a fallback to the loopback redirect.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Subcommand)]
pub enum AccessCommands {
    /// Request a scoped API key via browser approval
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Authenticates as the current user (OAuth login or API key) to file the request.
  Opens a browser for a human to approve; waits up to 15 minutes.
  On approval the issued key id + secret are saved to the credentials file; the secret is never printed.
  Control-plane base URL defaults to http://localhost:2023; override with --url.")]
    Request(AccessRequestArgs),
}

impl AccessCommands {
    /// The access-request flow authenticates as the user to *request* access;
    /// OAuth (read-only) identity is the intended credential, so it is not a
    /// write command and must not be blocked by the OAuth write gate.
    pub fn is_write(&self) -> bool {
        match self {
            AccessCommands::Request(_) => false,
        }
    }
}

#[derive(Args)]
pub struct AccessRequestArgs {
    /// Name to give the issued API key
    #[arg(long)]
    pub key_name: Option<String>,

    /// Agent purpose recorded with the request
    #[arg(long)]
    pub purpose: Option<String>,

    /// Agent tool name recorded with the request
    #[arg(long)]
    pub tool: Option<String>,

    /// Agent tool version recorded with the request
    #[arg(long)]
    pub version: Option<String>,

    /// Agent session id recorded with the request
    #[arg(long)]
    pub session_id: Option<String>,
}

// ── request/response bodies ────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentContext<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccessRequestBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    key_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<AgentContext<'a>>,
    code_challenge: &'a str,
    code_challenge_method: &'a str,
    state: &'a str,
    redirect_uri: &'a str,
}

/// Every `/v1` response from the openapi app wraps its payload as
/// `{ "result": <payload>, "requestId": <string> }`; the handshake reads
/// `result` and ignores the correlation id.
#[derive(serde::Deserialize)]
struct Envelope<T> {
    result: T,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccessRequestResponse {
    request_id: String,
    #[allow(dead_code)]
    status: Option<String>,
    approval_url: String,
    /// Epoch milliseconds.
    #[allow(dead_code)]
    expires_at: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessRequestStatus {
    status: Option<String>,
    claim_code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimBody<'a> {
    claim_code: &'a str,
    code_verifier: &'a str,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimResponse {
    key_id: String,
    key: String,
    secret: String,
}

// ── PKCE ───────────────────────────────────────────────────────────────────

struct Pkce {
    verifier: String,
    challenge: String,
}

/// PKCE pair (RFC 7636): a random 32-byte verifier, base64url-encoded, and its
/// S256 challenge `base64url(SHA256(verifier))`.
fn generate_pkce() -> Pkce {
    let verifier = URL_SAFE_NO_PAD.encode(random_bytes::<32>());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    Pkce {
        verifier,
        challenge,
    }
}

/// A random, URL-safe CSRF nonce tying the filed request to its redirect.
fn generate_state() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes::<16>())
}

fn random_bytes<const N: usize>() -> [u8; N] {
    use rand::Rng;
    let mut bytes = [0u8; N];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

// ── entry point ─────────────────────────────────────────────────────────────

pub async fn run(
    command: AccessCommands,
    api_key: Option<&str>,
    api_secret: Option<&str>,
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    json: bool,
) -> CloudResult<()> {
    match command {
        AccessCommands::Request(args) => {
            request(
                args,
                api_key,
                api_secret,
                url_override,
                org_id,
                org_name,
                json,
            )
            .await
        }
    }
}

/// The CLI base URL (with `/v1` suffix) for the access-request endpoints.
fn access_api_base(url_override: Option<&str>) -> String {
    crate::cloud::auth::normalize_api_url(url_override.unwrap_or(DEFAULT_ACCESS_API_BASE))
}

async fn request(
    args: AccessRequestArgs,
    api_key: Option<&str>,
    api_secret: Option<&str>,
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    json: bool,
) -> CloudResult<()> {
    // Resolve credentials, base URL, and organization through the shared
    // client, pointed at the control plane the handshake talks to.
    let api_base = access_api_base(url_override);
    let client = CloudClient::new(api_key, api_secret, Some(&api_base))?
        .with_organization_id(org_id)
        .with_organization_name(org_name);
    let org_id = client.resolve_organization_id().await?;
    let auth = client.request_auth();
    let base_url = client.base_url().to_string();

    let http = crate::http::client_builder()
        .build()
        .map_err(|error| CloudError::new(format!("failed to build HTTP client: {error}")))?;

    // Start the loopback redirect target before filing, so the port is known.
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|error| CloudError::new(format!("failed to start loopback server: {error}")))?;
    let port = listener
        .local_addr()
        .map_err(|error| CloudError::new(format!("failed to read loopback address: {error}")))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let pkce = generate_pkce();
    let state = generate_state();

    let agent = agent_context(&args);
    let create_url = format!("{base_url}/organizations/{org_id}/accessRequests");
    let create_body = CreateAccessRequestBody {
        key_name: args.key_name.as_deref(),
        agent,
        code_challenge: &pkce.challenge,
        code_challenge_method: "S256",
        state: &state,
        redirect_uri: &redirect_uri,
    };
    let created: CreateAccessRequestResponse = send_json(
        http.post(&create_url),
        &auth,
        &create_body,
        "file access request",
    )
    .await?;

    if !json {
        eprint_line(format!(
            "Approve this access request in your browser:\n  {}",
            created.approval_url
        ));
    }
    if open::that(&created.approval_url).is_err() && !json {
        eprint_line("Could not open a browser automatically. Open the URL above to continue.");
    }
    if !json {
        eprint_line("Waiting for approval (up to 15 minutes)...");
    }

    let status_url = format!(
        "{base_url}/organizations/{org_id}/accessRequests/{}",
        created.request_id
    );

    // The loopback redirect is the primary path; polling the request status is
    // the fallback when the browser cannot reach the loopback server. Whichever
    // yields the claim code first wins.
    let claim_code = tokio::select! {
        result = wait_for_callback(listener, &state) => result?,
        result = poll_until_approved(&http, &status_url, &auth) => result?,
    };

    let claim_url = format!("{status_url}/claim");
    let claim_body = ClaimBody {
        claim_code: &claim_code,
        code_verifier: &pkce.verifier,
    };
    let claimed: ClaimResponse = send_json(
        http.post(&claim_url),
        &auth,
        &claim_body,
        "claim access key",
    )
    .await?;

    // Never print the secret; store it with the CLI's existing credential file.
    credentials::set_api_credentials(claimed.key.clone(), claimed.secret)?;

    if json {
        let out = serde_json::json!({
            "status": "approved",
            "keyId": claimed.key_id,
            "key": claimed.key,
            "credentialsPath": credentials::credentials_path().display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Access approved. Scoped API key stored.");
        println!("  Key ID: {}", claimed.key_id);
        println!("  Saved to: {}", credentials::credentials_path().display());
    }

    Ok(())
}

fn agent_context(args: &AccessRequestArgs) -> Option<AgentContext<'_>> {
    if args.purpose.is_none()
        && args.tool.is_none()
        && args.version.is_none()
        && args.session_id.is_none()
    {
        return None;
    }
    Some(AgentContext {
        purpose: args.purpose.as_deref(),
        tool: args.tool.as_deref(),
        version: args.version.as_deref(),
        session_id: args.session_id.as_deref(),
    })
}

fn authorized(builder: reqwest::RequestBuilder, auth: &RequestAuth) -> reqwest::RequestBuilder {
    match auth {
        RequestAuth::Basic { key, secret } => builder.basic_auth(key, Some(secret)),
        RequestAuth::Bearer { token } => builder.bearer_auth(token),
    }
}

async fn send_json<T: DeserializeOwned>(
    builder: reqwest::RequestBuilder,
    auth: &RequestAuth,
    body: &impl Serialize,
    action: &str,
) -> CloudResult<T> {
    let response = authorized(builder, auth)
        .json(body)
        .send()
        .await
        .map_err(|error| CloudError::new(format!("{action}: {error}")))?;
    parse_response(response, action).await
}

async fn parse_response<T: DeserializeOwned>(
    response: reqwest::Response,
    action: &str,
) -> CloudResult<T> {
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| CloudError::new(format!("{action}: {error}")))?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        let message = format!("{action} failed ({status}): {}", body.trim());
        return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
            CloudError::auth(message)
        } else {
            CloudError::new(message)
        });
    }
    let envelope: Envelope<T> = serde_json::from_slice(&bytes)
        .map_err(|error| CloudError::new(format!("{action}: invalid response: {error}")))?;
    Ok(envelope.result)
}

// ── loopback redirect server ────────────────────────────────────────────────

/// Accept loopback connections until one hits `/callback` with a `code` and the
/// matching `state`, then return the code. A `state` mismatch is a CSRF failure
/// and aborts; unrelated paths (e.g. `/favicon.ico`) are answered and ignored.
async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> CloudResult<String> {
    let accept = async {
        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .map_err(|error| CloudError::new(format!("loopback accept failed: {error}")))?;

            let target = match read_request_target(&mut stream).await {
                Some(target) => target,
                None => {
                    write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
                    continue;
                }
            };

            let (path, query) = split_target(&target);
            if path != "/callback" {
                write_response(&mut stream, "404 Not Found", NOT_FOUND_HTML).await;
                continue;
            }

            let params = parse_query(query);
            match (params.get("code"), params.get("state")) {
                (Some(code), Some(got_state)) if got_state == expected_state => {
                    write_response(&mut stream, "200 OK", SUCCESS_HTML).await;
                    return Ok(code.clone());
                }
                (_, Some(_)) => {
                    write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
                    return Err(CloudError::auth(
                        "access request callback failed CSRF check (state mismatch)",
                    ));
                }
                _ => {
                    write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
                    continue;
                }
            }
        }
    };

    match tokio::time::timeout(APPROVAL_TIMEOUT, accept).await {
        Ok(result) => result,
        Err(_) => Err(CloudError::new(
            "timed out waiting for access request approval",
        )),
    }
}

/// Read the HTTP request line and return its request target (the second token).
async fn read_request_target(stream: &mut tokio::net::TcpStream) -> Option<String> {
    let mut buffer = [0u8; 2048];
    let read = stream.read(&mut buffer).await.ok()?;
    if read == 0 {
        return None;
    }
    let text = String::from_utf8_lossy(&buffer[..read]);
    let first_line = text.lines().next()?;
    first_line.split_whitespace().nth(1).map(str::to_string)
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            let value = urlencoding::decode(value).ok()?.into_owned();
            Some((key.to_string(), value))
        })
        .collect()
}

async fn write_response(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

const SUCCESS_HTML: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Access approved</title></head><body style=\"font-family:sans-serif;text-align:center;padding-top:4rem\"><h1>Access approved</h1><p>You can return to your terminal.</p></body></html>";
const NOT_FOUND_HTML: &str = "<!doctype html><html><body>Not found</body></html>";
const BAD_REQUEST_HTML: &str = "<!doctype html><html><body>Invalid request. You can close this tab and return to your terminal.</body></html>";

// ── status polling fallback ──────────────────────────────────────────────────

async fn poll_until_approved(
    http: &reqwest::Client,
    status_url: &str,
    auth: &RequestAuth,
) -> CloudResult<String> {
    let deadline = Instant::now() + APPROVAL_TIMEOUT;
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        if Instant::now() > deadline {
            return Err(CloudError::new(
                "timed out waiting for access request approval",
            ));
        }
        let response = authorized(http.get(status_url), auth)
            .send()
            .await
            .map_err(|error| CloudError::new(format!("poll access request: {error}")))?;
        if !response.status().is_success() {
            // Transient control-plane errors should not abort the wait; keep
            // polling until the deadline. Auth failures surface on the claim.
            continue;
        }
        let status: AccessRequestStatus = match response.json().await {
            Ok(status) => status,
            Err(_) => continue,
        };
        if status.status.as_deref() == Some("approved")
            && let Some(code) = status.claim_code
        {
            return Ok(code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn parse(args: &[&str]) -> AccessRequestArgs {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Access { command } = cloud.command else {
            panic!("expected access command");
        };
        let AccessCommands::Request(args) = command;
        args
    }

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let pkce = generate_pkce();
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(pkce.verifier.as_bytes()));
        assert_eq!(pkce.challenge, expected);
        // 32 random bytes base64url-encode (no pad) to 43 chars.
        assert_eq!(pkce.verifier.len(), 43);
        assert!(!pkce.verifier.contains('='));
        assert!(!pkce.verifier.contains('+'));
        assert!(!pkce.verifier.contains('/'));
    }

    #[test]
    fn pkce_and_state_are_unique_per_call() {
        assert_ne!(generate_pkce().verifier, generate_pkce().verifier);
        assert_ne!(generate_state(), generate_state());
    }

    #[test]
    fn access_api_base_defaults_to_local_control_plane_with_v1() {
        assert_eq!(access_api_base(None), "http://localhost:2023/v1");
        assert_eq!(
            access_api_base(Some("https://api.control-plane.example.com")),
            "https://api.control-plane.example.com/v1"
        );
        // An explicit /v1 suffix is not doubled.
        assert_eq!(
            access_api_base(Some("http://localhost:9000/v1")),
            "http://localhost:9000/v1"
        );
    }

    #[test]
    fn split_target_separates_path_and_query() {
        assert_eq!(
            split_target("/callback?code=a&state=b"),
            ("/callback", "code=a&state=b")
        );
        assert_eq!(split_target("/callback"), ("/callback", ""));
    }

    #[test]
    fn parse_query_url_decodes_values() {
        let params = parse_query("code=ab%20cd&state=xyz");
        assert_eq!(params.get("code").map(String::as_str), Some("ab cd"));
        assert_eq!(params.get("state").map(String::as_str), Some("xyz"));
        assert!(parse_query("").is_empty());
    }

    #[test]
    fn agent_context_is_absent_when_no_fields_set() {
        let args = parse(["chctl", "cloud", "access", "request"].as_slice());
        assert!(agent_context(&args).is_none());
    }

    #[test]
    fn create_body_serializes_camel_case_with_agent() {
        let args = parse(
            [
                "chctl",
                "cloud",
                "access",
                "request",
                "--key-name",
                "agent-key",
                "--purpose",
                "ci",
                "--tool",
                "claude",
                "--version",
                "1.2",
                "--session-id",
                "sess-1",
            ]
            .as_slice(),
        );
        let agent = agent_context(&args).expect("agent present");
        let body = CreateAccessRequestBody {
            key_name: args.key_name.as_deref(),
            agent: Some(agent),
            code_challenge: "challenge",
            code_challenge_method: "S256",
            state: "state",
            redirect_uri: "http://127.0.0.1:1234/callback",
        };
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value["keyName"], "agent-key");
        assert_eq!(value["codeChallenge"], "challenge");
        assert_eq!(value["codeChallengeMethod"], "S256");
        assert_eq!(value["state"], "state");
        assert_eq!(value["redirectUri"], "http://127.0.0.1:1234/callback");
        assert_eq!(value["agent"]["purpose"], "ci");
        assert_eq!(value["agent"]["tool"], "claude");
        assert_eq!(value["agent"]["version"], "1.2");
        assert_eq!(value["agent"]["sessionId"], "sess-1");
    }

    #[test]
    fn request_is_classified_read_only() {
        // OAuth (read-only) identity is the intended credential for filing.
        assert!(
            !AccessCommands::Request(parse(["chctl", "cloud", "access", "request"].as_slice()))
                .is_write()
        );
    }

    fn unwrap_result<T: DeserializeOwned>(body: &str) -> T {
        serde_json::from_str::<Envelope<T>>(body)
            .expect("deserialize envelope")
            .result
    }

    #[test]
    fn file_response_reads_payload_from_result_envelope() {
        // `/v1` responses wrap the payload: { "result": <payload>, "requestId": ... }.
        let created: CreateAccessRequestResponse = unwrap_result(
            r#"{
                "result": {
                    "requestId": "req-1",
                    "status": "pending",
                    "approvalUrl": "https://console.example.com/approve/req-1",
                    "expiresAt": 1739000000000
                },
                "requestId": "corr-1"
            }"#,
        );
        assert_eq!(created.request_id, "req-1");
        assert_eq!(created.status.as_deref(), Some("pending"));
        assert_eq!(
            created.approval_url,
            "https://console.example.com/approve/req-1"
        );
        // expiresAt is epoch millis (a number), not a string.
        assert_eq!(created.expires_at, Some(1_739_000_000_000));
    }

    #[test]
    fn poll_response_reads_claim_code_from_result_envelope() {
        let approved: AccessRequestStatus = unwrap_result(
            r#"{
                "result": {
                    "requestId": "req-1",
                    "status": "approved",
                    "grantedRoleIds": ["role-a", "role-b"],
                    "issuedKeyId": "key-1",
                    "claimCode": "claim-xyz",
                    "expiresAt": 1739000000000
                },
                "requestId": "corr-2"
            }"#,
        );
        assert_eq!(approved.status.as_deref(), Some("approved"));
        assert_eq!(approved.claim_code.as_deref(), Some("claim-xyz"));
    }

    #[test]
    fn pending_poll_response_has_no_claim_code() {
        let pending: AccessRequestStatus = unwrap_result(
            r#"{"result":{"requestId":"req-1","status":"pending","expiresAt":1739000000000},"requestId":"corr-3"}"#,
        );
        assert_eq!(pending.status.as_deref(), Some("pending"));
        assert!(pending.claim_code.is_none());
    }

    #[test]
    fn claim_response_reads_key_and_secret_from_result_envelope() {
        let claimed: ClaimResponse = unwrap_result(
            r#"{
                "result": { "keyId": "mgmt-1", "key": "PUBLICKEY", "secret": "SH_secret" },
                "requestId": "corr-4"
            }"#,
        );
        assert_eq!(claimed.key_id, "mgmt-1");
        assert_eq!(claimed.key, "PUBLICKEY");
        assert_eq!(claimed.secret, "SH_secret");
    }
}
