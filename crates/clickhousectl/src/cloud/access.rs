//! `cloud access`: discover assignable roles and obtain scoped API keys.
//! Role discovery uses the typed API client; requests use the PKCE handshake.
//!
//! The user authenticates with their existing CLI identity (OAuth Bearer from
//! `cloud auth login`), files an access request carrying a PKCE
//! challenge and a CSRF `state`, approves it in a browser, and the CLI claims
//! the issued key over a loopback redirect — falling back to polling the
//! request's status. The claimed key id and secret are stored with the CLI's
//! existing credential storage; the secret is never printed or placed in a URL.
//!
//! The handshake endpoints are not modelled by the typed `clickhouse-cloud-api` client,
//! so the handshake calls them directly with a plain `reqwest` client carrying
//! the credentials resolved by [`CloudClient`] ([`CloudClient::request_auth`]).

use crate::cloud::client::{CloudClient, CloudError, RequestAuth, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::eprint_line;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::{AssignableRole, RBACPolicyAllowdeny, RBACRoleType};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tabled::{Table, Tabled, settings::Style};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// How long to wait for a human to approve the request before giving up.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// How often to poll the request's status as a fallback to the loopback redirect.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Subcommand)]
pub enum AccessCommands {
    /// List caller-assignable roles with policy scopes
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns current assignable roles, not a guarantee of later approval.
  JSON output is the complete role array, including all policies and tags.")]
    Roles,
    /// Request a scoped API key via browser approval
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Requires user OAuth identity from `cloud auth login`, even when an API key is saved.
  Opens a browser for a human to approve; waits up to 15 minutes.
  On approval the issued key id + secret are saved to the credentials file; the secret is never printed.
  Discover proposal IDs with `cloud access roles`; proposals do not grant access.
  Uses the login environment unless --url is provided.
  Select --org/--org-id or exact --org-name when your user belongs to multiple organizations.")]
    Request(AccessRequestArgs),
}

impl AccessCommands {
    /// The access-request flow authenticates as the user to *request* access;
    /// OAuth (read-only) identity is the intended credential, so it is not a
    /// write command and must not be blocked by the OAuth write gate.
    pub fn is_write(&self) -> bool {
        match self {
            AccessCommands::Request(_) | AccessCommands::Roles => false,
        }
    }
}

#[derive(Args)]
pub struct AccessRequestArgs {
    /// Proposed role:<UUID> tokens, comma-separated or repeated (max 100)
    #[arg(long, value_name = "role:UUID", value_delimiter = ',', value_parser = parse_requested_role)]
    pub perms: Vec<uuid::Uuid>,

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

// Keep the token grammar narrower than Uuid::parse_str (which also accepts
// compact, braced and URN representations). UUID version is unrestricted.
fn parse_requested_role(token: &str) -> Result<uuid::Uuid, &'static str> {
    let id = token.strip_prefix("role:").ok_or("expected role:<UUID>")?;
    let parsed = uuid::Uuid::parse_str(id).map_err(|_| "expected role:<UUID>")?;
    if !id.eq_ignore_ascii_case(&parsed.hyphenated().to_string()) {
        return Err("expected role:<hyphenated UUID> without whitespace");
    }
    Ok(parsed)
}

impl AccessRequestArgs {
    pub fn requested_role_ids(&self) -> Result<Option<Vec<String>>, &'static str> {
        if self.perms.is_empty() {
            return Ok(None);
        }
        let mut seen = HashSet::new();
        let ids: Vec<_> = self
            .perms
            .iter()
            .filter(|id| seen.insert(**id))
            .map(|id| id.hyphenated().to_string())
            .collect();
        if ids.len() > 100 {
            return Err("--perms accepts at most 100 distinct roles");
        }
        Ok(Some(ids))
    }
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
    requested_role_ids: Option<&'a [String]>,
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

impl CloudClient {
    async fn list_assignable_roles(&self, org_id: &str) -> CloudResult<Vec<AssignableRole>> {
        let response = self
            .api()
            .organization_assignable_roles_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        if let Some(error) = &response.error {
            return Err(CloudError::new(format!("Role discovery failed: {error}")));
        }
        if let Some(status) = response.status.filter(|status| *status >= 400) {
            return Err(CloudError::new(format!(
                "Role discovery failed with envelope status {status}"
            )));
        }
        let roles = Self::unwrap_response(response)?;
        validate_assignable_roles(&roles)?;
        Ok(roles)
    }

    /// Read all user memberships before auto-selection or exact-name selection.
    async fn resolve_access_organization_id(
        &self,
        org_id: Option<String>,
        org_name: Option<String>,
    ) -> CloudResult<String> {
        if org_id.is_some() && org_name.is_some() {
            return Err(CloudError::usage(
                "Organization ID conflicts with --org-name",
            ));
        }
        if let Some(id) = org_id {
            return Ok(id);
        }
        let mut organizations = Vec::new();
        let mut cursor = None;
        let mut seen_cursors = std::collections::HashSet::new();
        let mut seen_ids = std::collections::HashSet::new();
        loop {
            let response = self
                .api()
                .organization_get_list(None, cursor.as_deref())
                .await
                .map_err(|error| self.convert_error(error))?;
            let next_cursor = response.next_cursor.clone();
            let page = Self::unwrap_response(response)?;
            let had_rows = !page.is_empty();
            let previous_count = organizations.len();
            for organization in page {
                let id = organization.id.ok_or_else(|| CloudError::new(
                    "Organization response is missing its id; cannot safely select an organization",
                ))?;
                if seen_ids.insert(id) {
                    organizations.push((id, organization));
                }
            }
            match next_cursor {
                None => break,
                Some(next) => {
                    if (had_rows && previous_count == organizations.len())
                        || !seen_cursors.insert(next.clone())
                    {
                        return Err(CloudError::new(
                            "Organization pagination made no progress; cannot safely select an organization",
                        ));
                    }
                    cursor = Some(next);
                }
            }
        }
        if let Some(name) = org_name {
            return crate::cloud::shared::select_named_id(
                "organization",
                &name,
                organizations
                    .iter()
                    .map(|(id, org)| (org.name.as_deref(), Some(id))),
            );
        }
        match organizations.as_slice() {
            [] => Err(CloudError::new(
                "Your OAuth user has no organizations. Join or create an organization, then run `clickhousectl cloud org list --json`.",
            )),
            [(id, _)] => Ok(id.to_string()),
            _ => Err(CloudError::new(
                "Your OAuth user belongs to multiple organizations. Run `clickhousectl cloud org list --json` and select an ID with --org (or --org-id).",
            )),
        }
    }
}

pub async fn run(
    command: AccessCommands,
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    json: bool,
) -> CloudResult<()> {
    match command {
        AccessCommands::Request(args) => request(args, url_override, org_id, org_name, json).await,
        AccessCommands::Roles => roles(url_override, org_id, org_name, json).await,
    }
}

/// Validate the entire menu before either output mode can claim success.
fn validate_assignable_roles(roles: &[AssignableRole]) -> CloudResult<()> {
    fn required<T>(value: &Option<T>, field: &str) -> CloudResult<()> {
        if value.is_none() {
            return Err(CloudError::new(format!(
                "Incomplete role discovery response: missing {field}"
            )));
        }
        Ok(())
    }
    for role in roles {
        required(&role.id, "role.id")?;
        if let Some(id) = &role.id {
            uuid::Uuid::parse_str(id).map_err(|_| {
                CloudError::new("Invalid role discovery response: role.id is not a UUID")
            })?;
        }
        required(&role.name, "role.name")?;
        required(&role.r#type, "role.type")?;
        required(&role.policies, "role.policies")?;
        if let Some(policies) = &role.policies {
            for policy in policies {
                required(&policy.allow_deny, "policy.allowDeny")?;
                required(&policy.permissions, "policy.permissions")?;
                required(&policy.resources, "policy.resources")?;
            }
        }
    }
    Ok(())
}

async fn roles(
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    json: bool,
) -> CloudResult<()> {
    let client = CloudClient::for_access(url_override).await?;
    let org_id = client
        .resolve_access_organization_id(org_id, org_name)
        .await?;
    let roles = client.list_assignable_roles(&org_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&roles)?);
    } else {
        print_assignable_roles(&org_id, &roles);
    }
    Ok(())
}

fn print_assignable_roles(org_id: &str, roles: &[AssignableRole]) {
    use crate::cloud::output::or_absent;
    #[derive(Tabled)]
    struct RoleRow {
        id: String,
        name: String,
        r#type: String,
        policies: String,
    }
    let rows = roles
        .iter()
        .map(|role| {
            let role_type = match &role.r#type {
                Some(RBACRoleType::Unknown(value)) => format!("Unknown ({value})"),
                other => or_absent(other.as_ref()).to_string(),
            };
            let mut summaries = Vec::new();
            if let Some(policies) = &role.policies {
                if policies.is_empty() {
                    summaries.push("No policies".into());
                }
                for policy in policies {
                    let effect = match &policy.allow_deny {
                        Some(RBACPolicyAllowdeny::Unknown(value)) => {
                            format!("Unknown effect ({value})")
                        }
                        other => or_absent(other.as_ref()).to_string(),
                    };
                    if let (Some(permissions), Some(resources)) =
                        (&policy.permissions, &policy.resources)
                    {
                        summaries.push(format!(
                            "{effect}: permissions [{}]; resources [{}]",
                            permissions.join(", "),
                            resources.join(", ")
                        ));
                    }
                }
            }
            RoleRow {
                id: or_absent(role.id.as_ref()).to_string(),
                name: or_absent(role.name.as_ref()).to_string(),
                r#type: role_type,
                policies: summaries.join("\n"),
            }
        })
        .collect::<Vec<_>>();
    println!("Organization: {org_id}");
    if rows.is_empty() {
        println!("No assignable roles for your user in this organization.");
    } else {
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
}

async fn request(
    args: AccessRequestArgs,
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    json: bool,
) -> CloudResult<()> {
    let requested_role_ids = args.requested_role_ids().map_err(CloudError::usage)?;
    // Resolve the user identity and endpoint once for organization discovery
    // and every step of the existing PKCE handshake.
    let client = CloudClient::for_access(url_override).await?;
    let org_id = client
        .resolve_access_organization_id(org_id, org_name)
        .await?;
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
        requested_role_ids: requested_role_ids.as_deref(),
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
        true,
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
        false,
    )
    .await?;

    if claimed.key_id.is_empty() || claimed.key.is_empty() || claimed.secret.is_empty() {
        return Err(CloudError::new("claim access key: incomplete response"));
    }

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
    include_error_body: bool,
) -> CloudResult<T> {
    let response = authorized(builder, auth)
        .json(body)
        .send()
        .await
        .map_err(|error| CloudError::new(format!("{action}: {}", error.without_url())))?;
    parse_response(response, action, include_error_body).await
}

async fn parse_response<T: DeserializeOwned>(
    response: reqwest::Response,
    action: &str,
    include_error_body: bool,
) -> CloudResult<T> {
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| CloudError::new(format!("{action}: {}", error.without_url())))?;
    if !status.is_success() {
        let message = if include_error_body {
            format!(
                "{action} failed ({status}): {}",
                String::from_utf8_lossy(&bytes).trim()
            )
        } else {
            format!("{action} failed ({status})")
        };
        return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
            CloudError::auth(message)
        } else {
            CloudError::new(message)
        });
    }
    // Deserializer diagnostics can quote a secret-bearing unexpected value.
    let envelope: Envelope<T> = serde_json::from_slice(&bytes)
        .map_err(|_| CloudError::new(format!("{action}: invalid response")))?;
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
            .map_err(|error| {
                CloudError::new(format!("poll access request: {}", error.without_url()))
            })?;
        if response.status().is_server_error()
            || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            continue;
        }
        let status: AccessRequestStatus =
            parse_response(response, "poll access request", false).await?;
        match status.status.as_deref() {
            Some("pending") => continue,
            Some("approved") => {
                return status
                    .claim_code
                    .filter(|code| !code.is_empty())
                    .ok_or_else(|| CloudError::new("approved access request has no claim code"));
            }
            Some("denied") => return Err(CloudError::new("access request denied")),
            Some("expired") => return Err(CloudError::new("access request expired")),
            _ => return Err(CloudError::new("invalid access request status")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[tokio::test]
    async fn poll_until_approved_propagates_terminal_and_invalid_responses() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let mut tasks = Vec::new();
        for (http_status, body, auth_error) in [
            (401, serde_json::json!({"error":"echo-claim-code"}), true),
            (403, serde_json::json!({"error":"echo-claim-code"}), true),
            (400, serde_json::json!({"error":"echo-claim-code"}), false),
            (404, serde_json::json!({"error":"echo-claim-code"}), false),
            (
                200,
                serde_json::json!({"result":{"status":"denied"}}),
                false,
            ),
            (
                200,
                serde_json::json!({"result":{"status":"expired"}}),
                false,
            ),
            (
                200,
                serde_json::json!({"result":{"status":"future"}}),
                false,
            ),
            (200, serde_json::json!({"result":{}}), false),
            (
                200,
                serde_json::json!({"result":{"status":"approved"}}),
                false,
            ),
            (
                200,
                serde_json::json!({"result":{"status":"approved","claimCode":""}}),
                false,
            ),
            (
                200,
                serde_json::json!({"result":{"status":"approved","claimCode":{"echo-claim-code":1}}}),
                false,
            ),
        ] {
            tasks.push(tokio::spawn(async move {
                let server = MockServer::start().await;
                Mock::given(wiremock::matchers::method("GET"))
                    .respond_with(ResponseTemplate::new(http_status).set_body_json(body))
                    .expect(1)
                    .mount(&server)
                    .await;
                let result = tokio::time::timeout(
                    Duration::from_secs(8),
                    poll_until_approved(
                        &reqwest::Client::new(),
                        &server.uri(),
                        &RequestAuth::Bearer {
                            token: "fake-token".into(),
                        },
                    ),
                )
                .await
                .expect("terminal response must not wait for approval timeout")
                .unwrap_err();
                assert_eq!(
                    matches!(result.kind, crate::cloud::client::CloudErrorKind::Auth),
                    auth_error
                );
                assert!(!result.message.contains("echo-claim-code"));
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn poll_until_approved_retries_only_transient_http_and_pending() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        let count = Arc::new(AtomicUsize::new(0));
        let attempts = count.clone();
        Mock::given(wiremock::matchers::method("GET"))
            .respond_with(move |_: &wiremock::Request| {
                match attempts.fetch_add(1, Ordering::SeqCst) {
                    0 => ResponseTemplate::new(429),
                    1 => ResponseTemplate::new(503),
                    2 => ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({"result":{"status":"pending"}})),
                    _ => ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"result":{"status":"approved","claimCode":"test-code"}}),
                    ),
                }
            })
            .expect(4)
            .mount(&server)
            .await;
        let code = tokio::time::timeout(
            Duration::from_secs(18),
            poll_until_approved(
                &reqwest::Client::new(),
                &server.uri(),
                &RequestAuth::Bearer {
                    token: "fake-token".into(),
                },
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(code, "test-code");
        assert_eq!(count.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn claim_response_errors_do_not_echo_secret_values() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for status in [200, 400, 401, 500] {
            let server = MockServer::start().await;
            Mock::given(wiremock::matchers::method("POST"))
                .respond_with(
                    ResponseTemplate::new(status).set_body_json(serde_json::json!({
                        "result": "secret-verifier-code", "error": "secret-verifier-code"
                    })),
                )
                .mount(&server)
                .await;
            let error = send_json::<ClaimResponse>(
                reqwest::Client::new().post(server.uri()),
                &RequestAuth::Bearer {
                    token: "fake-token".into(),
                },
                &serde_json::json!({}),
                "claim access key",
                false,
            )
            .await
            .err()
            .unwrap();
            assert!(!error.message.contains("secret-verifier-code"));
        }
    }

    fn parse(args: &[&str]) -> AccessRequestArgs {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Access { command, .. } = cloud.command else {
            panic!("expected access command");
        };
        let AccessCommands::Request(args) = command else {
            panic!("expected request")
        };
        args
    }

    #[test]
    fn request_perms_parse_deduplicates_uuid_identity_and_preserves_order() {
        let first = "abcdefab-1234-1234-1234-123456789abc";
        let system = "21111111-1111-1111-9999-000000000007";
        let args = parse(&[
            "chctl",
            "cloud",
            "access",
            "request",
            "--perms",
            &format!("role:{first},role:{system}"),
            "--perms",
            &format!("role:{}", first.to_uppercase()),
        ]);
        assert_eq!(
            args.requested_role_ids().unwrap(),
            Some(vec![first.into(), system.into()])
        );
        assert_eq!(
            parse(&["chctl", "cloud", "access", "request"])
                .requested_role_ids()
                .unwrap(),
            None
        );
        let ids: Vec<_> = (0..100)
            .map(|i| format!("role:00000000-0000-0000-0000-{i:012x}"))
            .collect();
        let repeated = format!("{},{}", ids.join(","), ids.join(","));
        assert_eq!(
            parse(&["chctl", "cloud", "access", "request", "--perms", &repeated])
                .requested_role_ids()
                .unwrap()
                .unwrap()
                .len(),
            100
        );
        let oversized = format!(
            "{},role:00000000-0000-0000-0000-000000000100",
            ids.join(",")
        );
        assert!(
            parse(&["chctl", "cloud", "access", "request", "--perms", &oversized])
                .requested_role_ids()
                .is_err()
        );
    }

    #[test]
    fn request_perms_rejects_empty_malformed_and_unsupported_tokens() {
        let valid = "role:00000000-0000-0000-0000-000000000001";
        for value in [
            "".to_string(),
            ",".into(),
            format!("{valid},"),
            format!(",{valid}"),
            format!("{valid},,{valid}"),
            format!(" {valid}"),
            format!("{valid} "),
            format!("{valid}\n"),
            format!("{valid}, {valid}"),
            "role:00000000-0000-0000-0000-00000000000 1".into(),
            "role:00000000000000000000000000000001".into(),
            "role:{00000000-0000-0000-0000-000000000001}".into(),
            "role:urn:uuid:00000000-0000-0000-0000-000000000001".into(),
            "Role:00000000-0000-0000-0000-000000000001".into(),
            "00000000-0000-0000-0000-000000000001".into(),
            "role:Admin".into(),
            "control-plane:service:view".into(),
            "role:*".into(),
        ] {
            assert!(
                Cli::try_parse_from(["chctl", "cloud", "access", "request", "--perms", &value])
                    .is_err(),
                "{value:?}"
            );
        }
        assert!(
            Cli::try_parse_from(["chctl", "cloud", "access", "roles", "--perms", valid]).is_err()
        );
    }

    #[test]
    fn roles_selectors_and_help_preserve_command_grammar() {
        use clap::{CommandFactory, FromArgMatches};
        for (selectors, conflict) in [
            (vec!["--org-id=one", "access", "roles", "--org=one"], false),
            (vec!["--org-id=one", "access", "roles", "--org=two"], true),
            (
                vec!["--org-name=name", "access", "roles", "--org-name=name"],
                false,
            ),
            (vec!["--org-name=name", "access", "roles", "--org=id"], true),
            (vec!["access", "--org=one", "roles", "--org=two"], true),
            (
                vec![
                    "--json",
                    "access",
                    "roles",
                    "--url=http://localhost:1/v1",
                    "--",
                ],
                false,
            ),
        ] {
            let argv: Vec<std::ffi::OsString> = ["chctl", "cloud"]
                .into_iter()
                .chain(selectors)
                .map(Into::into)
                .collect();
            let mut cmd = Cli::command();
            let matches = cmd.try_get_matches_from_mut(&argv).unwrap();
            let cli = Cli::from_arg_matches(&matches).unwrap();
            let Commands::Cloud(args) = cli.command else {
                panic!("cloud expected")
            };
            let result = args.validate_access_selectors(&argv);
            if conflict {
                assert_eq!(
                    result.unwrap_err().kind(),
                    clap::error::ErrorKind::ArgumentConflict
                );
            } else {
                result.unwrap();
            }
            assert!(!args.command.is_write_command());
        }
        for args in [
            vec!["chctl", "cloud", "access", "--help"],
            vec!["chctl", "cloud", "access", "roles", "--help"],
        ] {
            assert_eq!(
                Cli::try_parse_from(args).err().unwrap().kind(),
                clap::error::ErrorKind::DisplayHelp
            );
        }
        assert!(Cli::try_parse_from(["chctl", "cloud", "org", "list", "--org=id"]).is_err());
        assert!(
            Cli::try_parse_from(["chctl", "cloud", "access", "roles", "--purpose=test"]).is_err()
        );
    }

    #[test]
    fn access_organization_selectors_validate_original_command_depths() {
        use clap::{CommandFactory, FromArgMatches};
        fn validate(args: &[&str]) -> Result<(), clap::Error> {
            let argv: Vec<std::ffi::OsString> = std::iter::once("chctl")
                .chain(args.iter().copied())
                .map(Into::into)
                .collect();
            let mut command = Cli::command();
            let matches = command.try_get_matches_from_mut(&argv)?;
            let cli = Cli::from_arg_matches(&matches)?;
            let Commands::Cloud(args) = cli.command else {
                panic!("cloud expected")
            };
            args.validate_access_selectors(&argv)
        }
        for args in [
            vec!["cloud", "access", "request", "--org", "id"],
            vec!["cloud", "access", "request", "--org=id", "--org-id=id"],
            vec!["cloud", "access", "--org=id", "request", "--org=id"],
            vec!["cloud", "access", "--org=id", "request"],
            vec!["cloud", "--org-id=id", "access", "request", "--org=id"],
            vec!["cloud", "--org-id=id", "access", "request", "--org-id=id"],
            vec![
                "cloud",
                "--org-name=name",
                "access",
                "request",
                "--org-name=name",
            ],
            vec![
                "cloud",
                "access",
                "request",
                "--purpose=--org=other",
                "--org=id",
            ],
            vec![
                "cloud",
                "access",
                "request",
                "--purpose",
                "text --org=other",
                "--org=id",
            ],
            vec!["cloud", "access", "request", "--org=id", "--"],
            vec![
                "cloud",
                "--json",
                "--url=http://localhost:1/v1",
                "--org-id=id",
                "access",
                "request",
                "--org=id",
            ],
            vec![
                "cloud",
                "access",
                "request",
                "--json",
                "--url=http://localhost:1/v1",
                "--org=id",
            ],
            vec!["cloud", "--org-id=first", "org", "list", "--org-id=second"],
        ] {
            validate(&args).unwrap_or_else(|error| panic!("{args:?}: {error}"));
        }
        for args in [
            vec![
                "cloud",
                "access",
                "request",
                "--org=first",
                "--org-id=second",
            ],
            vec!["cloud", "access", "request", "--org=first", "--org=first"],
            vec![
                "cloud",
                "--org-id=first",
                "access",
                "request",
                "--org-id=second",
            ],
            vec![
                "cloud", "access", "--org", "first", "request", "--org", "second",
            ],
            vec![
                "cloud",
                "--org-id=first",
                "access",
                "request",
                "--org=second",
            ],
            vec![
                "cloud",
                "access",
                "--org=first",
                "request",
                "--org-id=second",
            ],
            vec![
                "cloud",
                "--org-name=first",
                "access",
                "request",
                "--org-name=second",
            ],
            vec![
                "cloud",
                "--org-name=same",
                "access",
                "request",
                "--org=same",
            ],
            vec![
                "cloud",
                "--org-id=same",
                "access",
                "request",
                "--org-name=same",
            ],
            vec![
                "cloud",
                "access",
                "--org=same",
                "request",
                "--org-name=same",
            ],
        ] {
            let error = validate(&args).expect_err("conflict");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{args:?}: {error}"
            );
            assert_eq!(error.exit_code(), 2);
        }
        for args in [
            vec!["cloud", "access", "--help"],
            vec!["cloud", "access", "request", "--help"],
        ] {
            assert_eq!(
                validate(&args).unwrap_err().kind(),
                clap::error::ErrorKind::DisplayHelp
            );
        }
        assert!(validate(&["cloud", "access", "request", "--", "--org=id"]).is_err());
        assert!(validate(&["cloud", "org", "list", "--org=id"]).is_err());
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
            requested_role_ids: None,
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
