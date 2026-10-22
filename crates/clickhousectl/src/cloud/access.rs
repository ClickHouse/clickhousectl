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
use crate::cloud::permissions::Declaration as Permission;

// Access negotiation runs on the user's OAuth (Bearer) identity, not an API
// key, and reaches lanes the typed client does not model as metered
// operations: `access roles` reads the verified-CLI assignable-role lane and
// `access request` runs the PKCE handshake. Declared as authorization, not API
// operations, and unscoped because org selection is resolved from the user's
// own memberships rather than the `--org-id`/organization-list API path.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::non_api(
        "access roles",
        "Requires OAuth user identity (`cloud auth login`); reads the verified-CLI assignable-role lane.",
    )
    .unscoped(),
    Permission::non_api(
        "access request",
        "Requires OAuth user identity (`cloud auth login`); runs the browser PKCE approval handshake.",
    )
    .unscoped(),
];
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::{AssignableRole, RBACPolicyAllowdeny, RBACRoleType};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tabled::{Table, Tabled, settings::Style};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// How long to wait for a human to approve the request before giving up.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// How often to poll the request's status as a fallback to the loopback redirect.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_EPOCH_MILLIS: i64 = 8_640_000_000_000_000;

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
  Requires OAuth from `cloud auth login`, even with a saved API key; roles are per organization: pass --org-id.
  Each command's `--help` declares `API key permission:` needs; take conditionals only if their condition holds.
  `Authorization:` needs have no granting role.
  `cloud access roles --json` lists roles (empty: stop); policies `ALLOW`/`DENY` permissions on `*` or named resources.
  Choose roles covering every need with `ALLOW`, none denied by chosen roles, and the fewest extras.
  Propose with `--perms` and `--purpose`; a human approves the stderr URL within 15 minutes; check `grantedRoleIds`.
  The approved key becomes the active credential; changing permissions means a new flow (API keys cannot read roles).")]
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
    /// Print the approval URL without opening a browser
    #[arg(long)]
    pub no_browser: bool,

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
    error: Option<serde_json::Value>,
    status: Option<u16>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccessRequestResponse {
    requested_role_ids: Option<Vec<String>>,
    request_id: String,
    status: Option<String>,
    approval_url: String,
    /// Pending approval deadline only, in epoch milliseconds.
    expires_at: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessRequestStatus {
    requested_role_ids: Option<Vec<String>>,
    request_id: Option<String>,
    expires_at: Option<i64>,
    status: Option<String>,
    claim_code: Option<String>,
    claim_code_expires_at: Option<i64>,
    issued_key_id: Option<String>,
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
    granted_role_ids: Option<Vec<String>>,
    key_id: String,
    key: String,
    secret: String,
}

// Server arrays are authoritative: reject duplicate identities rather than silently
// deduplicating them as we do for user proposal input. Never echo invalid values.
fn canonical_role_ids(ids: Option<&[String]>, proposal: bool) -> CloudResult<Vec<String>> {
    let field = if proposal {
        "requestedRoleIds"
    } else {
        "grantedRoleIds"
    };
    let invalid = || {
        CloudError::new(format!(
            "access request: missing or invalid {field}; server role reporting support is required"
        ))
    };
    let ids = ids.ok_or_else(invalid)?;
    if (proposal && ids.len() > 100) || (!proposal && ids.is_empty()) {
        return Err(invalid());
    }
    let mut seen = HashSet::new();
    ids.iter()
        .map(|id| {
            let parsed = uuid::Uuid::parse_str(id).map_err(|_| invalid())?;
            let canonical = parsed.hyphenated().to_string();
            if !id.eq_ignore_ascii_case(&canonical) || !seen.insert(parsed) {
                return Err(invalid());
            }
            Ok(canonical)
        })
        .collect()
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

/// The client for filing, polling, and claiming: like `oauth_client()` in
/// `auth.rs`, it never follows redirects. A 307/308 from the claim endpoint
/// would replay the JSON body — `claimCode` and the secret PKCE
/// `code_verifier` — to another origin, and dropping the `Authorization`
/// header on cross-origin redirects does not protect those body fields.
/// Reject redirects instead of forwarding secrets or replaying the claim POST.
fn handshake_client() -> CloudResult<reqwest::Client> {
    crate::http::client_builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| CloudError::new(format!("failed to build HTTP client: {error}")))
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

    let http = handshake_client()?;

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
    let created: CreateAccessRequestResponse = operation(
        OPERATION_TIMEOUT,
        send_json(
            http.post(&create_url),
            &auth,
            &create_body,
            "file access request",
            true,
        ),
    )
    .await
    .map_err(|error| {
        error.into_cloud("file access request timed out; the request may have been filed")
    })?;
    let pending = PendingApproval::new(&created)?;
    if pending.requested_role_ids.as_slice() != requested_role_ids.as_deref().unwrap_or_default() {
        return Err(CloudError::new(
            "file access request: saved proposal differs from submitted proposal",
        ));
    }
    let saved_requested_role_ids = pending.requested_role_ids.clone();

    approval_handoff(&created.approval_url, args.no_browser, |url| {
        open::that(url)
    });

    let status_url = format!(
        "{base_url}/organizations/{org_id}/accessRequests/{}",
        created.request_id
    );

    // A callback only wakes the single status/claim owner; GET supplies authority.
    let permit = wait_for_approval(&http, &status_url, &auth, pending, listener, &state).await?;
    let claim_url = format!("{status_url}/claim");
    let claimed = claim_access_key(&http, &claim_url, &auth, &permit, &pkce.verifier).await?;

    if claimed.key_id.is_empty() || claimed.key.is_empty() || claimed.secret.is_empty() {
        return Err(CloudError::new("claim access key: incomplete response"));
    }

    let granted_role_ids = canonical_role_ids(claimed.granted_role_ids.as_deref(), false)?;
    let credentials_path = credentials::credentials_path().display().to_string();
    // Prepare the complete output before the only credential write. Actual roles
    // come only from the successful claim, never from the proposal or status GET.
    let json_result = if json {
        Some(serde_json::to_string_pretty(&serde_json::json!({
            "status": "approved",
            "keyId": claimed.key_id,
            "key": claimed.key,
            "credentialsPath": credentials_path,
            "requestId": created.request_id,
            "organizationId": org_id,
            "requestedRoleIds": saved_requested_role_ids,
            "grantedRoleIds": granted_role_ids,
        }))?)
    } else {
        None
    };

    // Never print the secret; store it with the CLI's existing credential file.
    credentials::set_api_credentials(claimed.key.clone(), claimed.secret)?;

    if let Some(out) = json_result {
        println!("{out}");
    } else {
        println!("Access approved. Scoped API key stored.");
        println!("  Organization: {org_id}");
        println!("  Request ID: {}", created.request_id);
        if saved_requested_role_ids.is_empty() {
            println!("  Requested roles: none (manual selection)");
        } else {
            println!("  Requested roles: {}", saved_requested_role_ids.join(", "));
        }
        println!("  Granted roles: {}", granted_role_ids.join(", "));
        println!("  Key ID: {}", claimed.key_id);
        println!("  Saved to: {credentials_path}");
    }

    Ok(())
}

fn approval_handoff(
    approval_url: &str,
    no_browser: bool,
    open_browser: impl FnOnce(&str) -> std::io::Result<()>,
) {
    eprint_line(format!(
        "Approve this access request in your browser:\n  {approval_url}"
    ));
    if !no_browser && open_browser(approval_url).is_err() {
        eprint_line("Could not open a browser automatically. Open the URL above to continue.");
    }
    eprint_line("Waiting for approval (up to 15 minutes)...");
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
    if envelope.error.is_some() || envelope.status.is_some_and(|status| status >= 400) {
        return Err(CloudError::new(format!(
            "{action}: invalid response envelope"
        )));
    }
    Ok(envelope.result)
}

// ── loopback redirect server ────────────────────────────────────────────────

/// Accept loopback connections until one hits `/callback` with a `code` and the
/// matching `state`, then return the code. A `state` mismatch is a CSRF failure
/// and aborts; unrelated paths (e.g. `/favicon.ico`) are answered and ignored.
async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> CloudResult<String> {
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
                if !valid_claim_code(code) {
                    write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
                    return Err(CloudError::new("invalid access request callback code"));
                }
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
}

/// The largest request line (terminator included) accepted from the loopback
/// callback before the connection is treated as malformed and ignored.
const MAX_REQUEST_LINE: usize = 8 * 1024;

/// Read the HTTP request line and return its request target (the second token).
///
/// A TCP read may split the request line anywhere — including midway through
/// the `state` value — so bytes are buffered through the request-line
/// terminator and only then parsed; parsing whatever a single read happened to
/// deliver would treat a partial `state` as a CSRF mismatch and abort approval.
/// An oversized line without a terminator is reported as no target.
async fn read_request_target(stream: &mut tokio::net::TcpStream) -> Option<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(line_end) = buffer.iter().position(|byte| *byte == b'\n') {
            let text = String::from_utf8_lossy(&buffer[..line_end]);
            let line = text.strip_suffix('\r').unwrap_or(&text);
            return line.split_whitespace().nth(1).map(str::to_string);
        }
        if buffer.len() > MAX_REQUEST_LINE {
            return None;
        }
    }
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

/// An immutable wall deadline with a monotonic ceiling: clock rollback cannot
/// extend it, while a forward adjustment may shorten it.
struct Deadline {
    epoch_millis: i64,
    monotonic: Instant,
}

fn epoch_millis() -> CloudResult<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .ok_or_else(|| CloudError::new("invalid system clock for access request"))
}

impl Deadline {
    fn new(value: Option<i64>, now: i64, instant: Instant) -> CloudResult<Self> {
        let epoch_millis = value.filter(|value| (1..=MAX_EPOCH_MILLIS).contains(value))
            .ok_or_else(|| CloudError::new("access request has a missing or invalid deadline; server deadline support is required"))?;
        let remaining = Duration::from_millis(epoch_millis.saturating_sub(now).max(0) as u64);
        let monotonic = instant
            .checked_add(remaining)
            .ok_or_else(|| CloudError::new("access request deadline is not representable"))?;
        Ok(Self {
            epoch_millis,
            monotonic,
        })
    }

    fn remaining_at(&self, now: i64, instant: Instant) -> Duration {
        Duration::from_millis(self.epoch_millis.saturating_sub(now).max(0) as u64)
            .min(self.monotonic.saturating_duration_since(instant))
    }

    fn remaining(&self) -> CloudResult<Duration> {
        Ok(self.remaining_at(epoch_millis()?, Instant::now()))
    }
}

struct PendingApproval<'a> {
    requested_role_ids: Vec<String>,
    request_id: &'a str,
    deadline: Deadline,
}

impl<'a> PendingApproval<'a> {
    fn new(created: &'a CreateAccessRequestResponse) -> CloudResult<Self> {
        if created.request_id.trim().is_empty()
            || created.approval_url.trim().is_empty()
            || created.status.as_deref() != Some("pending")
        {
            return Err(CloudError::new("file access request: incomplete response"));
        }
        let now = Instant::now();
        let mut deadline = Deadline::new(created.expires_at, epoch_millis()?, now)?;
        deadline.monotonic = deadline.monotonic.min(now + APPROVAL_TIMEOUT);
        Ok(Self {
            requested_role_ids: canonical_role_ids(created.requested_role_ids.as_deref(), true)?,
            request_id: &created.request_id,
            deadline,
        })
    }
}

// Contains a secret. Do not derive Debug or include it in diagnostics.
struct ClaimPermit {
    code: String,
    deadline: Deadline,
}

fn valid_claim_code(code: &str) -> bool {
    (1..=512).contains(&code.len())
        && code
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn interpret_status(
    status: AccessRequestStatus,
    pending: &PendingApproval<'_>,
    callback_code: Option<&str>,
) -> CloudResult<Option<ClaimPermit>> {
    if status.request_id.as_deref() != Some(pending.request_id)
        || status.expires_at != Some(pending.deadline.epoch_millis)
    {
        return Err(CloudError::new(
            "access request status has inconsistent identity or pending deadline",
        ));
    }
    if canonical_role_ids(status.requested_role_ids.as_deref(), true)? != pending.requested_role_ids
    {
        return Err(CloudError::new(
            "access request status has inconsistent saved proposal",
        ));
    }
    match status.status.as_deref() {
        Some("pending") => Ok(None),
        Some("denied") => Err(CloudError::new("access request denied")),
        Some("expired") => Err(CloudError::new("access request expired")),
        Some("approved") => {
            let deadline = Deadline::new(
                status.claim_code_expires_at,
                epoch_millis()?,
                Instant::now(),
            )?;
            if let Some(key_id) = status.issued_key_id {
                return Err(CloudError::new(if key_id.trim().is_empty() {
                    "access request has invalid issued key identity"
                } else {
                    "access request has already been claimed"
                }));
            }
            if deadline.remaining()?.is_zero() {
                return Err(CloudError::new("access request claim window expired"));
            }
            let code = status
                .claim_code
                .filter(|code| valid_claim_code(code))
                .ok_or_else(|| {
                    CloudError::new("approved access request has no usable claim code")
                })?;
            if callback_code.is_some_and(|callback| callback != code) {
                return Err(CloudError::new(
                    "access request callback does not match authoritative status",
                ));
            }
            Ok(Some(ClaimPermit { code, deadline }))
        }
        _ => Err(CloudError::new("invalid access request status")),
    }
}

enum OperationError {
    Timeout,
    Failed(CloudError),
}

impl OperationError {
    fn into_cloud(self, timeout_message: &str) -> CloudError {
        match self {
            Self::Timeout => CloudError::new(timeout_message),
            Self::Failed(error) => error,
        }
    }
}

/// Covers send, body and decoding. The elapsed check also catches synchronous
/// decoding that finishes after the timeout without yielding to Tokio.
async fn operation<T>(
    budget: Duration,
    future: impl std::future::Future<Output = CloudResult<T>>,
) -> Result<T, OperationError> {
    if budget.is_zero() {
        return Err(OperationError::Timeout);
    }
    let start = Instant::now();
    match tokio::time::timeout(budget, future).await {
        Ok(result) if start.elapsed() < budget => result.map_err(OperationError::Failed),
        _ => Err(OperationError::Timeout),
    }
}

async fn fetch_status(
    http: &reqwest::Client,
    status_url: &str,
    auth: &RequestAuth,
) -> CloudResult<Option<AccessRequestStatus>> {
    let response = authorized(http.get(status_url), auth)
        .send()
        .await
        .map_err(|error| {
            CloudError::new(format!("poll access request: {}", error.without_url()))
        })?;
    if response.status().is_server_error()
        || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return Ok(None);
    }
    parse_response(response, "poll access request", false)
        .await
        .map(Some)
}

async fn wait_for_approval(
    http: &reqwest::Client,
    status_url: &str,
    auth: &RequestAuth,
    pending: PendingApproval<'_>,
    listener: TcpListener,
    state: &str,
) -> CloudResult<ClaimPermit> {
    let callback = wait_for_callback(listener, state);
    tokio::pin!(callback);
    let mut callback_code = None;
    loop {
        let remaining = pending.deadline.remaining()?;
        if !remaining.is_zero() {
            tokio::select! {
                result = &mut callback, if callback_code.is_none() => callback_code = Some(result?),
                _ = tokio::time::sleep(POLL_INTERVAL.min(remaining)) => {},
            }
        }
        let remaining = pending.deadline.remaining()?;
        let final_observation = remaining.is_zero();
        let budget = if final_observation {
            OPERATION_TIMEOUT
        } else {
            OPERATION_TIMEOUT.min(remaining)
        };
        let fetch = operation(budget, fetch_status(http, status_url, auth));
        tokio::pin!(fetch);
        let fetched = tokio::select! {
            biased;
            result = &mut callback, if callback_code.is_none() => {
                callback_code = Some(result?);
                fetch.await
            }
            result = &mut fetch => result,
        };
        let status = match fetched {
            Ok(status) => status,
            Err(OperationError::Timeout)
                if !final_observation && pending.deadline.remaining()?.is_zero() =>
            {
                continue;
            }
            Err(error) => return Err(error.into_cloud("poll access request timed out")),
        };
        if let Some(status) = status
            && let Some(permit) = interpret_status(status, &pending, callback_code.as_deref())?
        {
            return Ok(permit);
        }
        if final_observation {
            return Err(CloudError::new(
                "access request approval wait ended without a usable approval",
            ));
        }
    }
}

async fn claim_access_key(
    http: &reqwest::Client,
    claim_url: &str,
    auth: &RequestAuth,
    permit: &ClaimPermit,
    verifier: &str,
) -> CloudResult<ClaimResponse> {
    let remaining = permit.deadline.remaining()?;
    if remaining.is_zero() {
        return Err(CloudError::new("access request claim window expired"));
    }
    operation(OPERATION_TIMEOUT.min(remaining), send_json(
        http.post(claim_url), auth,
        &ClaimBody { claim_code: &permit.code, code_verifier: verifier },
        "claim access key", false,
    )).await.map_err(|error| error.into_cloud("claim access key timed out; the claim may have completed on the server; no credentials were installed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn created(expires_at: i64) -> CreateAccessRequestResponse {
        CreateAccessRequestResponse {
            requested_role_ids: Some(vec![]),
            request_id: "request-1".into(),
            status: Some("pending".into()),
            approval_url: "https://example.invalid/approve".into(),
            expires_at: Some(expires_at),
        }
    }

    fn status_body(status: &str, expires_at: i64) -> serde_json::Value {
        serde_json::json!({"result": {"requestedRoleIds":[], "requestId":"request-1", "expiresAt":expires_at, "status":status, "claimCode":"test-code", "claimCodeExpiresAt":4102444800000_i64}})
    }

    async fn poll_until_approved(
        http: &reqwest::Client,
        url: &str,
        auth: &RequestAuth,
    ) -> CloudResult<String> {
        let created = created(4102444800000);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        wait_for_approval(
            http,
            url,
            auth,
            PendingApproval::new(&created)?,
            listener,
            "state",
        )
        .await
        .map(|permit| permit.code)
    }

    #[test]
    fn canonical_role_ids_validate_server_arrays_without_inferring_grants() {
        let system = "21111111-1111-1111-9999-000000000007".to_string();
        let upper = "AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA".to_string();
        assert_eq!(
            canonical_role_ids(Some(&[system.clone(), upper.clone()]), true).unwrap(),
            vec![system, upper.to_lowercase()]
        );
        assert_eq!(
            canonical_role_ids(Some(&[]), true).unwrap(),
            Vec::<String>::new()
        );
        assert!(canonical_role_ids(Some(&[]), false).is_err());
        for proposal in [true, false] {
            assert!(canonical_role_ids(None, proposal).is_err());
            assert!(
                canonical_role_ids(Some(&[upper.clone(), upper.to_lowercase()]), proposal).is_err()
            );
            for invalid in [
                "",
                "role-name",
                "*",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "{aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}",
                "urn:uuid:aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                " aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            ] {
                let error = canonical_role_ids(Some(&[invalid.into()]), proposal).unwrap_err();
                assert!(!error.message.contains("aaaaaaaa"));
            }
        }
        let many: Vec<_> = (1..=101)
            .map(|id| format!("00000000-0000-0000-0000-{id:012x}"))
            .collect();
        assert!(canonical_role_ids(Some(&many[..100]), true).is_ok());
        assert!(canonical_role_ids(Some(&many), true).is_err());
        assert_eq!(canonical_role_ids(Some(&many), false).unwrap().len(), 101);
    }

    #[test]
    fn role_response_fields_reject_malformed_types_and_status_proposal_changes() {
        for value in [
            serde_json::json!(42),
            serde_json::json!("secret-value"),
            serde_json::json!([null]),
            serde_json::json!([1]),
            serde_json::json!({}),
        ] {
            let create = serde_json::json!({"requestId":"request-1", "status":"pending", "approvalUrl":"https://example.invalid", "expiresAt":4102444800000_i64, "requestedRoleIds":value});
            assert!(serde_json::from_value::<CreateAccessRequestResponse>(create).is_err());
            assert!(
                serde_json::from_value::<AccessRequestStatus>(
                    serde_json::json!({"requestedRoleIds":value})
                )
                .is_err()
            );
            let claim = serde_json::json!({"keyId":"key-id", "key":"key", "secret":"secret", "grantedRoleIds":value});
            assert!(serde_json::from_value::<ClaimResponse>(claim).is_err());
        }
        let a = "21111111-1111-1111-9999-000000000007".to_string();
        let b = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string();
        let mut created = created(4102444800000);
        created.requested_role_ids = Some(vec![a.clone(), b.to_uppercase()]);
        let pending = PendingApproval::new(&created).unwrap();
        let mut status = status_body("approved", 4102444800000);
        for ids in [
            serde_json::json!(null),
            serde_json::json!([]),
            serde_json::json!([b, a]),
            serde_json::json!([b, b.to_uppercase()]),
        ] {
            status["result"]["requestedRoleIds"] = ids;
            assert!(
                interpret_status(
                    serde_json::from_value(status["result"].clone()).unwrap(),
                    &pending,
                    None
                )
                .is_err()
            );
        }
        status["result"]["requestedRoleIds"] = serde_json::json!([a, b]);
        assert!(
            interpret_status(
                serde_json::from_value(status["result"].clone()).unwrap(),
                &pending,
                None
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn deadline_boundaries_clock_changes_and_epoch_domain() {
        let instant = Instant::now();
        for (epoch, remaining) in [(999, 0), (1000, 0), (1001, 1)] {
            let deadline = Deadline::new(Some(epoch), 1000, instant).unwrap();
            assert_eq!(
                deadline.remaining_at(1000, instant),
                Duration::from_millis(remaining)
            );
        }
        let deadline = Deadline::new(Some(2000), 1000, instant).unwrap();
        assert_eq!(
            deadline.remaining_at(500, instant + Duration::from_millis(600)),
            Duration::from_millis(400)
        );
        assert_eq!(
            deadline.remaining_at(1900, instant),
            Duration::from_millis(100)
        );
        for value in [
            None,
            Some(-1),
            Some(0),
            Some(MAX_EPOCH_MILLIS + 1),
            Some(i64::MAX),
        ] {
            assert!(Deadline::new(value, 1000, instant).is_err());
        }
        // Platforms may reject a huge Instant, but must never panic or clamp.
        if let Ok(deadline) = Deadline::new(Some(MAX_EPOCH_MILLIS), 1000, instant) {
            assert_eq!(deadline.epoch_millis, MAX_EPOCH_MILLIS);
        }
        for value in [
            serde_json::json!(null),
            serde_json::json!(1.5),
            serde_json::json!("secret"),
            serde_json::json!(18446744073709551615_u64),
        ] {
            let wire = serde_json::json!({"requestId":"request-1", "status":"pending", "approvalUrl":"https://example.invalid", "expiresAt":value});
            if let Ok(created) = serde_json::from_value::<CreateAccessRequestResponse>(wire) {
                assert!(PendingApproval::new(&created).is_err());
            }
        }
        let future = created(4102444800000);
        assert!(
            PendingApproval::new(&future)
                .unwrap()
                .deadline
                .remaining()
                .unwrap()
                <= APPROVAL_TIMEOUT
        );
    }

    #[test]
    fn interpret_status_requires_independent_claim_deadline_and_valid_code() {
        let created = created(1); // pending expiry cannot cut off an approved claim
        let pending = PendingApproval::new(&created).unwrap();
        let body = status_body("approved", 1);
        let parse = |body: serde_json::Value| {
            serde_json::from_value::<AccessRequestStatus>(body["result"].clone()).unwrap()
        };
        assert!(
            interpret_status(parse(body.clone()), &pending, Some("test-code"))
                .unwrap()
                .is_some()
        );
        for (field, value) in [
            ("claimCodeExpiresAt", serde_json::json!(null)),
            ("claimCodeExpiresAt", serde_json::json!(1)),
            ("claimCode", serde_json::json!(null)),
            ("claimCode", serde_json::json!(" ")),
            ("claimCode", serde_json::json!("a".repeat(513))),
            ("issuedKeyId", serde_json::json!("issued")),
            ("issuedKeyId", serde_json::json!("")),
            ("requestId", serde_json::json!("other")),
            ("expiresAt", serde_json::json!(2)),
        ] {
            let mut invalid = body.clone();
            invalid["result"][field] = value;
            assert!(
                interpret_status(parse(invalid), &pending, None).is_err(),
                "{field}"
            );
        }
        assert!(interpret_status(parse(body), &pending, Some("different-secret")).is_err());
        for code in ["a", "_-09AZaz"] {
            assert!(valid_claim_code(code));
        }
        for code in ["", "a=", " a", "a\n", "é"] {
            assert!(!valid_claim_code(code));
        }
        assert!(valid_claim_code(&"a".repeat(512)));
    }

    #[tokio::test]
    async fn wait_for_approval_final_observation_is_once_and_can_claim_after_pending_expiry() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for (http_status, status, success) in [
            (200, "approved", true),
            (200, "pending", false),
            (429, "pending", false),
            (503, "pending", false),
        ] {
            let server = MockServer::start().await;
            Mock::given(wiremock::matchers::method("GET"))
                .respond_with(
                    ResponseTemplate::new(http_status).set_body_json(status_body(status, 1)),
                )
                .expect(1)
                .mount(&server)
                .await;
            let created = created(1);
            let result = wait_for_approval(
                &reqwest::Client::new(),
                &server.uri(),
                &RequestAuth::Bearer {
                    token: "fake".into(),
                },
                PendingApproval::new(&created).unwrap(),
                TcpListener::bind("127.0.0.1:0").await.unwrap(),
                "state",
            )
            .await;
            assert_eq!(result.is_ok(), success);
        }
    }

    #[tokio::test]
    async fn operation_timeout_covers_body_and_claim_is_not_retried() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(1)))
            .expect(1)
            .mount(&server)
            .await;
        let permit = ClaimPermit {
            code: "secret-code".into(),
            deadline: Deadline::new(
                Some(epoch_millis().unwrap() + 50),
                epoch_millis().unwrap(),
                Instant::now(),
            )
            .unwrap(),
        };
        let error = claim_access_key(
            &reqwest::Client::new(),
            &server.uri(),
            &RequestAuth::Bearer {
                token: "fake".into(),
            },
            &permit,
            "secret-verifier",
        )
        .await
        .err()
        .unwrap();
        assert!(error.message.contains("may have completed"));
        assert!(!error.message.contains("secret"));
        let result = operation::<()>(Duration::from_millis(5), async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(())
        })
        .await;
        assert!(matches!(result, Err(OperationError::Timeout)));
    }

    #[tokio::test]
    async fn wait_for_approval_callback_is_only_a_signal_and_idle_socket_is_bounded() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for callback in [true, false] {
            let server = MockServer::start().await;
            let attempts = Arc::new(AtomicUsize::new(0));
            let count = attempts.clone();
            Mock::given(wiremock::matchers::method("GET"))
                .respond_with(move |_: &wiremock::Request| {
                    let status = if callback && count.fetch_add(1, Ordering::SeqCst) == 0 {
                        "pending"
                    } else {
                        "approved"
                    };
                    ResponseTemplate::new(200).set_body_json(status_body(status, 4102444800000))
                })
                .expect(if callback { 2 } else { 1 })
                .mount(&server)
                .await;
            let created = created(4102444800000);
            let mut pending = PendingApproval::new(&created).unwrap();
            // Exercise the local wait cap without waiting fifteen minutes.
            pending.deadline.monotonic = Instant::now() + Duration::from_millis(100);
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let client = reqwest::Client::new();
            let auth = RequestAuth::Bearer {
                token: "fake".into(),
            };
            let url = server.uri();
            let wait = wait_for_approval(&client, &url, &auth, pending, listener, "state");
            let notify = async {
                let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
                if callback {
                    stream
                        .write_all(b"GET /callback?code=test-code&state=state HTTP/1.1\r\n\r\n")
                        .await
                        .unwrap();
                    let mut response = Vec::new();
                    stream.read_to_end(&mut response).await.unwrap();
                    assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 200"));
                } else {
                    // No request line: the outer pending clock must cancel this read.
                    let mut response = Vec::new();
                    stream.read_to_end(&mut response).await.unwrap();
                }
            };
            let (permit, ()) =
                tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                    .await
                    .unwrap();
            assert_eq!(permit.unwrap().code, "test-code");
        }
    }

    #[tokio::test]
    async fn wait_for_callback_reassembles_a_request_line_split_across_writes() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(status_body("approved", 4102444800000)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let created = created(4102444800000);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = reqwest::Client::new();
        let auth = RequestAuth::Bearer {
            token: "fake".into(),
        };
        let url = server.uri();
        let wait = wait_for_approval(
            &client,
            &url,
            &auth,
            PendingApproval::new(&created).unwrap(),
            listener,
            "state",
        );
        let notify = async {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            stream
                .write_all(b"GET /callback?code=test-code&sta")
                .await
                .unwrap();
            // Keep the halves in separate reads; a read need not cover the
            // whole line, and this split lands midway through the state value.
            tokio::time::sleep(Duration::from_millis(50)).await;
            stream
                .write_all(b"te=state HTTP/1.1\r\n\r\n")
                .await
                .unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).await.unwrap();
            assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 200"));
        };
        let (permit, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                .await
                .unwrap();
        assert_eq!(permit.unwrap().code, "test-code");
    }

    #[tokio::test]
    async fn read_request_target_skips_an_oversized_line_without_a_terminator() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let oversized = "GET /".to_string() + &"a".repeat(MAX_REQUEST_LINE + 1);
        let writer = tokio::spawn(async move {
            // The reader may stop before the tail lands; delivery is best-effort.
            let _ = client.write_all(oversized.as_bytes()).await;
        });
        let target = tokio::time::timeout(Duration::from_secs(2), read_request_target(&mut server))
            .await
            .unwrap();
        assert_eq!(target, None);
        writer.await.unwrap();
    }

    #[tokio::test]
    async fn handshake_client_never_follows_redirects() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for status in [301, 302, 303, 307, 308] {
            let server = MockServer::start().await;
            let destination = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/organizations/org/accessRequests/request-1/claim"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .insert_header("Location", format!("{}/stolen", destination.uri())),
                )
                .expect(1)
                .mount(&server)
                .await;
            let claimed = claim_access_key(
                &handshake_client().unwrap(),
                &format!(
                    "{}/v1/organizations/org/accessRequests/request-1/claim",
                    server.uri()
                ),
                &RequestAuth::Bearer {
                    token: "fake".into(),
                },
                &ClaimPermit {
                    code: "claim-code-secret".into(),
                    deadline: Deadline::new(
                        Some(epoch_millis().unwrap() + 50),
                        epoch_millis().unwrap(),
                        Instant::now(),
                    )
                    .unwrap(),
                },
                "secret-verifier",
            )
            .await;
            let error = claimed.err().unwrap();
            assert!(
                !error.message.contains("secret"),
                "redirect handling must not leak the verifier: {error}"
            );
            assert!(destination.received_requests().await.unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn wait_for_approval_pending_transport_crosses_boundary_once() {
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
                let response = ResponseTemplate::new(200)
                    .set_body_json(status_body("approved", 4102444800000));
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    response.set_delay(Duration::from_secs(1))
                } else {
                    response
                }
            })
            .expect(2)
            .mount(&server)
            .await;
        let created = created(4102444800000);
        let mut pending = PendingApproval::new(&created).unwrap();
        pending.deadline.monotonic = Instant::now() + Duration::from_millis(100);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = reqwest::Client::new();
        let auth = RequestAuth::Bearer {
            token: "fake".into(),
        };
        let url = server.uri();
        let wait = wait_for_approval(&client, &url, &auth, pending, listener, "state");
        let notify = async {
            client
                .get(format!(
                    "http://{address}/callback?code=test-code&state=state"
                ))
                .send()
                .await
                .unwrap();
        };
        let (permit, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                .await
                .unwrap();
        assert_eq!(permit.unwrap().code, "test-code");
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn operation_bounds_response_body_read_and_sanitizes_envelope() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n{")
                .await
                .unwrap();
            std::future::pending::<()>().await;
        });
        let client = reqwest::Client::new();
        let result = operation(Duration::from_millis(50), async {
            let response = client
                .get(format!("http://{address}"))
                .send()
                .await
                .unwrap();
            parse_response::<AccessRequestStatus>(response, "poll access request", false).await
        })
        .await;
        server.abort();
        assert!(matches!(result, Err(OperationError::Timeout)));

        use wiremock::{Mock, MockServer, ResponseTemplate};
        for field in ["error", "status"] {
            let server = MockServer::start().await;
            let mut body = status_body("approved", 1);
            body[field] = if field == "error" {
                serde_json::json!("secret-echo")
            } else {
                serde_json::json!(500)
            };
            Mock::given(wiremock::matchers::method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&server)
                .await;
            let error = fetch_status(
                &client,
                &server.uri(),
                &RequestAuth::Bearer {
                    token: "fake".into(),
                },
            )
            .await
            .err()
            .unwrap();
            assert!(!error.message.contains("secret-echo"));
            assert!(error.message.contains("envelope"));
        }
    }

    #[test]
    fn approval_handoff_no_browser_skips_launcher_and_default_launches_once() {
        approval_handoff("https://example.invalid/approve", true, |_| {
            panic!("must not launch")
        });
        let mut calls = 0;
        approval_handoff("https://example.invalid/approve", false, |url| {
            assert_eq!(url, "https://example.invalid/approve");
            calls += 1;
            Ok(())
        });
        assert_eq!(calls, 1);
        // Launcher failure still permits manual approval and polling.
        approval_handoff("https://example.invalid/approve", false, |_| {
            Err(std::io::Error::other("fake launcher failure"))
        });
        assert!(!parse(&["chctl", "cloud", "access", "request"]).no_browser);
        assert!(parse(&["chctl", "cloud", "access", "request", "--no-browser"]).no_browser);
        assert!(
            Cli::try_parse_from(["chctl", "cloud", "access", "roles", "--no-browser"]).is_err()
        );
    }

    #[tokio::test]
    async fn poll_until_approved_propagates_terminal_and_invalid_responses() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let mut tasks = Vec::new();
        for (http_status, mut body, auth_error) in [
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
            if let Some(result) = body
                .get_mut("result")
                .and_then(serde_json::Value::as_object_mut)
            {
                result.insert("requestedRoleIds".into(), serde_json::json!([]));
                result.insert("requestId".into(), serde_json::json!("request-1"));
                result.insert("expiresAt".into(), serde_json::json!(4102444800000_i64));
                result.insert(
                    "claimCodeExpiresAt".into(),
                    serde_json::json!(4102444800000_i64),
                );
            }
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
                        .set_body_json(status_body("pending", 4102444800000)),
                    _ => ResponseTemplate::new(200)
                        .set_body_json(status_body("approved", 4102444800000)),
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
