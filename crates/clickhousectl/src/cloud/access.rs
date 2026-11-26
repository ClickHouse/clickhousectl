//! `cloud access`: discover assignable roles and obtain scoped API keys.
//! Role discovery uses the typed API client; requests use the PKCE handshake.
//!
//! The user authenticates with their existing CLI identity (OAuth Bearer from
//! `cloud auth login`) and runs the OAuth-style authorization code flow with
//! PKCE (RFC 6749, RFC 7636):
//! 1. file the request (`POST /accessRequests/authorizationCode`) with a PKCE
//!    challenge, a CSRF `state` and a loopback `redirectUri`;
//! 2. a human approves it in a browser, which redirects to the loopback
//!    listener with the one-time `code` and the `state` (or `error` on
//!    denial); the wait is bounded only by the pending deadline, and the
//!    request status is never read;
//! 3. exchange the code with the PKCE verifier and the same `redirectUri`
//!    (`POST /accessRequests/authorizationCode/token`) for the key.
//!
//! The issued key id and secret are stored with the CLI's existing credential
//! storage; the secret is never printed or placed in a URL.
//!
//! The handshake endpoints are not modelled by the typed `clickhouse-cloud-api` client,
//! so the handshake calls them directly with a plain `reqwest` client carrying
//! the credentials resolved by [`CloudClient`] ([`CloudClient::request_auth`]).

use crate::cloud::client::{CloudClient, CloudError, RequestAuth, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::grant::{AccessGrant, GrantPolicy, GrantedRole};
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
        "This command always uses OAuth credentials and ignores saved API keys. OAuth is required to list assignable roles.\n    \
         Reads the verified-CLI assignable-role lane. Not logged in: `cloud auth login`.",
    )
    .unscoped(),
    Permission::non_api(
        "access request",
        "This command always uses OAuth credentials and ignores saved API keys. OAuth is required to send access requests.\n    \
         Runs the browser PKCE approval handshake. Not logged in: `cloud auth login`.",
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
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tabled::{Table, Tabled, settings::Style};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// How long to wait for a human to approve the request before giving up.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(2 * 60);

/// The access-operation bound: every single access HTTP request — discovery
/// through the typed client, and each handshake step — must give up after
/// this long so a server holding a response open cannot stall the command.
pub(super) const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
/// The code exchange is a single attempt bounded like any access request.
#[cfg(not(test))]
const EXCHANGE_TIMEOUT: Duration = OPERATION_TIMEOUT;
/// Unit tests exercise the exchange timeout without waiting the full bound.
#[cfg(test)]
const EXCHANGE_TIMEOUT: Duration = Duration::from_millis(500);
const MAX_EPOCH_MILLIS: i64 = 8_640_000_000_000_000;

const MAX_PURPOSE_CHARS: usize = 200;
const MAX_KEY_NAME_CHARS: usize = 100;

#[derive(Subcommand)]
pub enum AccessCommands {
    /// List caller-assignable roles with policy scopes
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Requestable roles, fewest write permissions first: what each can change and on what.
  `--policies`: full policies. Listed does not mean approved.")]
    Roles(AccessRolesArgs),
    /// Request a scoped API key via browser approval
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  User approves or declines in their browser: no go-ahead needed first.
  --perms role:<UUID>[,role:<UUID>...] (from `cloud access roles`):
    the role or roles covering every write in the task, fewest extras.
  --purpose: required, shown to the approver.
  Blocks until approved (120s default; --timeout-seconds). Approver may change the roles: output names what was granted.
  Key is saved, never printed, and becomes the project's active credential until expiry. Wider role: new request.")]
    Request(AccessRequestArgs),
}

#[derive(Args)]
pub struct AccessRolesArgs {
    /// Print every role's complete policies instead of the summary
    #[arg(long)]
    pub policies: bool,
}

impl AccessCommands {
    /// The access-request flow authenticates as the user to *request* access;
    /// OAuth (read-only) identity is the intended credential, so it is not a
    /// write command and must not be blocked by the OAuth write gate.
    pub fn is_write(&self) -> bool {
        match self {
            AccessCommands::Request(_) | AccessCommands::Roles(_) => false,
        }
    }
}

#[derive(Args)]
pub struct AccessRequestArgs {
    /// Maximum approval wait in seconds (server expiry may be sooner)
    #[arg(
        long,
        env = "CLICKHOUSE_CLOUD_ACCESS_TIMEOUT_SECONDS",
        default_value_t = APPROVAL_TIMEOUT.as_secs() as u32,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub timeout_seconds: u32,

    /// Proposed role:<UUID> tokens, comma-separated or repeated (max 100)
    #[arg(long, value_name = "role:UUID", value_delimiter = ',', value_parser = parse_requested_role)]
    pub perms: Vec<uuid::Uuid>,

    /// Name to give the issued API key (1-100 characters)
    #[arg(long)]
    pub key_name: Option<String>,

    /// Purpose shown to the approver (1-200 characters)
    #[arg(long, required = true)]
    pub purpose: Option<String>,
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
    /// Input the server would reject, found before OAuth or any request.
    pub fn validate(&self) -> Result<(), &'static str> {
        let within = |value: &Option<String>, max: usize| {
            value
                .as_deref()
                .is_none_or(|value| (1..=max).contains(&value.chars().count()))
        };
        if !within(&self.purpose, MAX_PURPOSE_CHARS) {
            return Err("--purpose must be 1 to 200 characters");
        }
        if !within(&self.key_name, MAX_KEY_NAME_CHARS) {
            return Err("--key-name must be 1 to 100 characters");
        }
        Ok(())
    }

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

/// The filing body. The server rejects unknown fields, so this carries no
/// `flow` or `agent`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccessRequestBody<'a> {
    redirect_uri: &'a str,
    state: &'a str,
    code_challenge: &'a str,
    code_challenge_method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_role_ids: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose: Option<&'a str>,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenBody<'a> {
    code: &'a str,
    code_verifier: &'a str,
    redirect_uri: &'a str,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    granted_role_ids: Option<Vec<String>>,
    key_id: String,
    key: String,
    secret: String,
    /// When the key stops working, in epoch milliseconds.
    key_expires_at: Option<i64>,
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
    let challenge = s256_challenge(&verifier);
    Pkce {
        verifier,
        challenge,
    }
}

fn s256_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
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
                // Every page that promises more must add an organization: an
                // empty or all-duplicate page with a fresh cursor would
                // otherwise page forever.
                Some(next) => {
                    if previous_count == organizations.len() || !seen_cursors.insert(next.clone()) {
                        return Err(CloudError::new(
                            "Organization pagination made no progress; cannot safely select an organization",
                        ));
                    }
                    cursor = Some(next);
                }
            }
        }
        if let Some(name) = org_name {
            // Names may collide, and an organization has no positional ID:
            // only --org-id can choose one (#1200).
            return crate::cloud::shared::select_named_id_or(
                crate::cloud::client::ResourceKind::Organization,
                crate::cloud::client::Scope::Account,
                &name,
                organizations
                    .iter()
                    .map(|(id, org)| (org.name.as_deref(), Some(id))),
                |ids| format!("choose one with --org-id <ID>: {}", ids.join(", ")),
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
    debug: bool,
    json: bool,
) -> CloudResult<()> {
    match command {
        AccessCommands::Request(args) => {
            request(args, url_override, org_id, org_name, debug, json).await
        }
        AccessCommands::Roles(args) => {
            roles(args, url_override, org_id, org_name, debug, json).await
        }
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
    args: AccessRolesArgs,
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    debug: bool,
    json: bool,
) -> CloudResult<()> {
    let client = CloudClient::for_access(url_override).await?;
    if debug {
        super::print_client_diagnostics(&client);
    }
    let org_id = client
        .resolve_access_organization_id(org_id, org_name)
        .await?;
    let roles = client.list_assignable_roles(&org_id).await?;
    match (args.policies, json) {
        (true, true) => println!("{}", serde_json::to_string_pretty(&roles)?),
        (true, false) => print_assignable_roles(&org_id, &roles),
        (false, true) => println!("{}", summary_json(&summarize_roles(&roles))),
        (false, false) => print!("{}", summary_human(&org_id, &summarize_roles(&roles))),
    }
    Ok(())
}

/// Permissions of one role, grouped by resource family, in first-seen order.
type Families = Vec<(String, Vec<String>)>;

/// The compact `cloud access roles` entry of one role.
#[derive(Debug, Clone, PartialEq, Default)]
struct RoleSummary {
    id: String,
    name: String,
    write: Families,
    read: Families,
    on: Vec<String>,
    denied: Families,
}

impl RoleSummary {
    fn count(families: &Families) -> usize {
        families.iter().map(|(_, entries)| entries.len()).sum()
    }
}

/// A read is a permission whose last `:`-segment starts with `view`.
fn is_read_permission(permission: &str) -> bool {
    permission
        .rsplit(':')
        .next()
        .is_some_and(|last| last.starts_with("view"))
}

/// `(family, entry)` of a permission: `control-plane:` is dropped and the next
/// segment is the family; any other permission keeps its first segment as the
/// family, and its entry carries the policy's `roleV2` tag in parentheses.
fn permission_entry(permission: &str, role_v2: Option<&str>) -> (String, String) {
    if let Some(rest) = permission.strip_prefix("control-plane:") {
        return match rest.split_once(':') {
            Some((family, entry)) => (family.into(), entry.into()),
            None => (rest.into(), rest.into()),
        };
    }
    let (family, entry) = permission
        .split_once(':')
        .unwrap_or((permission, permission));
    let entry = match role_v2 {
        Some(tag) => format!("{entry} ({tag})"),
        None => entry.to_string(),
    };
    (family.into(), entry)
}

fn add_entry(families: &mut Families, family: String, entry: String) {
    match families.iter_mut().find(|(name, _)| *name == family) {
        Some((_, entries)) => {
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
        None => families.push((family, vec![entry])),
    }
}

fn summarize_role(role: &AssignableRole) -> RoleSummary {
    let mut summary = RoleSummary {
        id: role.id.clone().unwrap_or_default(),
        name: role.name.clone().unwrap_or_default(),
        ..RoleSummary::default()
    };
    for policy in role.policies.iter().flatten() {
        let role_v2 = policy
            .tags
            .as_ref()
            .and_then(|tags| tags.role_v2.as_ref())
            .map(ToString::to_string);
        let permissions = policy.permissions.iter().flatten();
        match &policy.allow_deny {
            Some(RBACPolicyAllowdeny::ALLOW) => {
                for permission in permissions {
                    let (family, entry) = permission_entry(permission, role_v2.as_deref());
                    let target = if is_read_permission(permission) {
                        &mut summary.read
                    } else {
                        &mut summary.write
                    };
                    add_entry(target, family, entry);
                }
                for resource in policy.resources.iter().flatten() {
                    if !summary.on.contains(resource) {
                        summary.on.push(resource.clone());
                    }
                }
            }
            Some(RBACPolicyAllowdeny::DENY) => {
                for permission in permissions {
                    let (family, entry) = permission_entry(permission, role_v2.as_deref());
                    add_entry(&mut summary.denied, family, entry);
                }
            }
            // An effect this CLI does not know neither grants nor denies here;
            // `--policies` shows it.
            _ => {}
        }
    }
    summary
}

/// One summary per role: fewest write permissions first, then fewest
/// permissions overall, then by name.
fn summarize_roles(roles: &[AssignableRole]) -> Vec<RoleSummary> {
    let mut summaries: Vec<_> = roles.iter().map(summarize_role).collect();
    summaries.sort_by(|a, b| {
        let writes = |s: &RoleSummary| RoleSummary::count(&s.write);
        let total = |s: &RoleSummary| RoleSummary::count(&s.write) + RoleSummary::count(&s.read);
        writes(a)
            .cmp(&writes(b))
            .then(total(a).cmp(&total(b)))
            .then_with(|| a.name.cmp(&b.name))
    });
    summaries
}

fn families_json(families: &Families) -> serde_json::Value {
    serde_json::Value::Object(
        families
            .iter()
            .map(|(family, entries)| (family.clone(), serde_json::json!(entries)))
            .collect(),
    )
}

/// A JSON array with one compact role object per line.
fn summary_json(summaries: &[RoleSummary]) -> String {
    if summaries.is_empty() {
        return "[]".into();
    }
    let lines: Vec<_> = summaries
        .iter()
        .map(|summary| {
            let mut object = serde_json::Map::new();
            object.insert("id".into(), summary.id.clone().into());
            object.insert("name".into(), summary.name.clone().into());
            object.insert("write".into(), families_json(&summary.write));
            object.insert("read".into(), families_json(&summary.read));
            object.insert("on".into(), serde_json::json!(summary.on));
            if !summary.denied.is_empty() {
                object.insert("denied".into(), families_json(&summary.denied));
            }
            serde_json::Value::Object(object).to_string()
        })
        .collect();
    format!("[\n{}\n]", lines.join(",\n"))
}

/// Human summary: per role a header, then `write`, `read`, `on` and `denied`
/// rows, one family per line.
fn summary_human(org_id: &str, summaries: &[RoleSummary]) -> String {
    const INDENT: &str = "         ";
    fn rows(out: &mut String, label: &str, families: &Families) {
        for (index, (family, entries)) in families.iter().enumerate() {
            let lead = if index == 0 {
                format!("  {label:<7}")
            } else {
                INDENT.to_string()
            };
            out.push_str(&format!("{lead}{family}: {}\n", entries.join(", ")));
        }
    }
    let mut out = format!("Organization: {org_id}\n");
    if summaries.is_empty() {
        out.push_str("No assignable roles for your user in this organization.\n");
        return out;
    }
    for summary in summaries {
        out.push_str(&format!("\n{}  {}\n", summary.name, summary.id));
        if summary.write.is_empty() {
            out.push_str("  write  none\n");
        } else {
            rows(&mut out, "write", &summary.write);
        }
        rows(&mut out, "read", &summary.read);
        if !summary.on.is_empty() {
            out.push_str(&format!("  {:<7}{}\n", "on", summary.on.join(", ")));
        }
        rows(&mut out, "denied", &summary.denied);
    }
    out
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

/// The client for filing and the code exchange: like
/// `oauth_client()` in `auth.rs`, it never follows redirects. A 307/308 from
/// the token endpoint would replay the JSON body — the one-time `code` and
/// the secret PKCE `codeVerifier` — to another origin, and dropping the
/// `Authorization` header on cross-origin redirects does not protect those
/// body fields. Reject redirects instead of forwarding secrets or replaying
/// the exchange POST.
///
/// Both steps go only to the Cloud API, so like every other Cloud call it
/// carries the agent session/trace headers; with redirects disabled they
/// cannot be forwarded to another host.
fn handshake_client() -> CloudResult<reqwest::Client> {
    crate::http::clickhouse_client_builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| CloudError::new(format!("failed to build HTTP client: {error}")))
}

async fn request(
    args: AccessRequestArgs,
    url_override: Option<&str>,
    org_id: Option<String>,
    org_name: Option<String>,
    debug: bool,
    json: bool,
) -> CloudResult<()> {
    args.validate().map_err(CloudError::usage)?;
    let requested_role_ids = args.requested_role_ids().map_err(CloudError::usage)?;
    // Resolve the user identity and endpoint once for organization discovery
    // and every step of the PKCE handshake.
    let client = CloudClient::for_access(url_override).await?;
    if debug {
        super::print_client_diagnostics(&client);
    }
    let org_id = client
        .resolve_access_organization_id(org_id, org_name)
        .await?;
    let auth = client.request_auth();
    let base_url = client.base_url().to_string();
    let file_url = access_request_url(&base_url, &org_id, &["authorizationCode"])?;
    let token_url = access_request_url(&base_url, &org_id, &["authorizationCode", "token"])?;

    let http = handshake_client()?;

    // The redirect URI is part of the filing, so the listener comes first.
    let Loopback {
        listener,
        redirect_uri,
        state,
    } = bind_loopback(TcpListener::bind).await?;

    let pkce = generate_pkce();

    let create_body = CreateAccessRequestBody {
        redirect_uri: &redirect_uri,
        state: &state,
        code_challenge: &pkce.challenge,
        code_challenge_method: "S256",
        requested_role_ids: requested_role_ids.as_deref(),
        key_name: args.key_name.as_deref(),
        purpose: args.purpose.as_deref(),
    };
    let created: CreateAccessRequestResponse = operation(
        OPERATION_TIMEOUT,
        send_json(
            http.post(&file_url),
            &auth,
            &create_body,
            "file access request",
            &[&state],
            AUTH_STATUSES,
        ),
    )
    .await
    .map_err(|error| {
        error.into_cloud("file access request timed out; the request may have been filed")
    })?;
    let approval_timeout = Duration::from_secs(u64::from(args.timeout_seconds));
    let pending = PendingApproval::with_timeout(&created, approval_timeout)?;
    if pending.requested_role_ids.as_slice() != requested_role_ids.as_deref().unwrap_or_default() {
        return Err(CloudError::new(
            "file access request: saved proposal differs from submitted proposal",
        ));
    }
    let saved_requested_role_ids = pending.requested_role_ids.clone();

    approval_handoff(&created.approval_url, |url| open::that(url));
    eprintln!(
        "Waiting for approval (up to {} seconds; server expiry may be sooner)...",
        args.timeout_seconds
    );

    let code = wait_for_redirect(pending, listener, &state).await?;
    // The approval wait can outlast the Bearer captured at filing, so refresh
    // it when near expiry. The code is single-use and already approved: any
    // refresh failure, or a login now for another API URL, keeps the filing
    // Bearer rather than dropping the code without trying the exchange.
    let fresh = CloudClient::for_access(url_override)
        .await
        .ok()
        .filter(|fresh| fresh.base_url() == base_url);
    let auth = fresh.as_ref().map_or(auth, CloudClient::request_auth);
    let issued = exchange_code(
        &http,
        &token_url,
        &auth,
        &code,
        &pkce.verifier,
        &redirect_uri,
    )
    .await?;

    if issued.key_id.is_empty() || issued.key.is_empty() || issued.secret.is_empty() {
        return Err(CloudError::new(
            "exchange authorization code: incomplete response",
        ));
    }

    let granted_role_ids = canonical_role_ids(issued.granted_role_ids.as_deref(), false)?;
    let key_expires = key_expiry(issued.key_expires_at)?;
    let credentials_path = credentials::credentials_path()?.display().to_string();
    let active_in = crate::init::project_root()?.dir.display().to_string();
    // Names and permissions of the granted roles, from the same OAuth listing
    // `cloud access roles` reads. Best effort: an approved key is never lost
    // because the lookup failed; the grant then carries ids only.
    let menu = client_for_lookup(fresh.as_ref(), &client)
        .list_assignable_roles(&org_id)
        .await
        .ok();
    let grant = AccessGrant {
        api_key: issued.key.clone(),
        key_id: issued.key_id.clone(),
        organization_id: org_id.clone(),
        request_id: created.request_id.clone(),
        expires_at: key_expires.clone(),
        roles: granted_role_ids
            .iter()
            .map(|id| granted_role(id, menu.as_deref()))
            .collect(),
    };
    // Prepare the complete output before the only credential write. Actual roles
    // come only from the successful exchange, never from the proposal or status GET.
    let json_result = if json {
        Some(serde_json::to_string_pretty(&serde_json::json!({
            "status": "approved",
            "keyId": issued.key_id,
            "key": issued.key,
            "keyExpiresAt": issued.key_expires_at,
            "credentialsPath": credentials_path,
            "requestId": created.request_id,
            "organizationId": org_id,
            "requestedRoleIds": saved_requested_role_ids,
            "grantedRoleIds": granted_role_ids,
            "grantedRoles": grant.roles.iter().map(|role| serde_json::json!({
                "id": role.id,
                "name": role.name,
            })).collect::<Vec<_>>(),
            "keyExpiresAtUtc": key_expires,
            "activeIn": active_in,
        }))?)
    } else {
        None
    };
    let human = approved_summary(
        &credentials_path,
        &active_in,
        &grant,
        &saved_requested_role_ids,
        menu.as_deref(),
        issued.key_expires_at,
        epoch_millis()?,
    );

    // Never print the secret; store it with the CLI's existing credential file.
    credentials::set_api_credentials_with_grant(
        issued.key.clone(),
        issued.secret,
        &base_url,
        grant,
    )?;

    match json_result {
        Some(out) => println!("{out}"),
        None => print!("{human}"),
    }

    Ok(())
}

fn client_for_lookup<'a>(
    fresh: Option<&'a CloudClient>,
    filing: &'a CloudClient,
) -> &'a CloudClient {
    fresh.unwrap_or(filing)
}

/// The listed role with this id (ids compare as UUIDs).
fn find_role<'a>(id: &str, menu: Option<&'a [AssignableRole]>) -> Option<&'a AssignableRole> {
    let wanted = uuid::Uuid::parse_str(id).ok();
    menu?.iter().find(|role| {
        role.id.as_deref().is_some_and(|candidate| {
            uuid::Uuid::parse_str(candidate).ok() == wanted || candidate == id
        })
    })
}

/// A granted role with the name and policies the listing gives it, or the id
/// alone when the listing failed or does not carry it.
fn granted_role(id: &str, menu: Option<&[AssignableRole]>) -> GrantedRole {
    let role = find_role(id, menu);
    GrantedRole {
        id: id.to_string(),
        name: role.and_then(|role| role.name.clone()),
        policies: role.map(|role| {
            role.policies
                .iter()
                .flatten()
                .map(|policy| GrantPolicy {
                    effect: policy
                        .allow_deny
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string),
                    permissions: policy.permissions.clone().unwrap_or_default(),
                    resources: policy.resources.clone().unwrap_or_default(),
                })
                .collect()
        }),
    }
}

/// The human output of an approved request.
fn approved_summary(
    credentials_path: &str,
    active_in: &str,
    grant: &AccessGrant,
    requested: &[String],
    menu: Option<&[AssignableRole]>,
    key_expires_at: Option<i64>,
    now_millis: i64,
) -> String {
    let granted = grant
        .roles
        .iter()
        .map(|role| match &role.name {
            Some(name) => format!("{name} ({})", role.id),
            None => role.id.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("Approved. Key saved: {credentials_path}\n  Granted: {granted}\n");
    let same =
        |a: &[String], b: Vec<&String>| a.len() == b.len() && a.iter().all(|id| b.contains(&id));
    if !same(requested, grant.roles.iter().map(|role| &role.id).collect()) {
        let names = if requested.is_empty() {
            "none".to_string()
        } else {
            requested
                .iter()
                .map(|id| {
                    find_role(id, menu)
                        .and_then(|role| role.name.clone())
                        .unwrap_or_else(|| id.clone())
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!(
            "  Requested: {names}; the approver changed the roles.\n"
        ));
    }
    if let (Some(expires), Some(millis)) = (&grant.expires_at, key_expires_at) {
        let minutes = (millis.saturating_sub(now_millis).max(0) + 30_000) / 60_000;
        out.push_str(&format!("  Expires: {expires} ({minutes} min)\n"));
    }
    out.push_str(&format!("  Active in {active_in} and below.\n"));
    out
}

/// The key's expiry as RFC 3339 UTC, for human output. A value outside the
/// epoch domain is an error, so it is found before the credential write.
fn key_expiry(value: Option<i64>) -> CloudResult<Option<String>> {
    value
        .map(|millis| {
            Some(millis)
                .filter(|millis| (1..=MAX_EPOCH_MILLIS).contains(millis))
                .and_then(|millis| jiff::Timestamp::from_second(millis / 1000).ok())
                .map(|time| time.to_string())
                .ok_or_else(|| CloudError::new("exchange authorization code: invalid keyExpiresAt"))
        })
        .transpose()
}

/// The loopback listener the browser redirects to, and the values filed with
/// it. `state` is a secret: never print it.
struct Loopback {
    listener: TcpListener,
    redirect_uri: String,
    state: String,
}

/// Bind an ephemeral loopback port: IPv4 first, then IPv6. The server accepts
/// only these IP literals in `redirectUri`, never `localhost`. There is no
/// other flow to fall back to, so failing both is an error.
async fn bind_loopback<F, Fut>(bind: F) -> CloudResult<Loopback>
where
    F: Fn(SocketAddr) -> Fut,
    Fut: std::future::Future<Output = std::io::Result<TcpListener>>,
{
    for ip in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ] {
        let Ok(listener) = bind(SocketAddr::new(ip, 0)).await else {
            continue;
        };
        let Ok(address) = listener.local_addr() else {
            continue;
        };
        // `SocketAddr` brackets IPv6: `http://[::1]:port/callback`.
        return Ok(Loopback {
            listener,
            redirect_uri: format!("http://{address}/callback"),
            state: generate_state(),
        });
    }
    Err(CloudError::new(
        "could not start a local redirect listener on 127.0.0.1 or [::1]",
    ))
}

fn approval_handoff(approval_url: &str, open_browser: impl FnOnce(&str) -> std::io::Result<()>) {
    eprintln!("Approve this access request in your browser:\n  {approval_url}");
    if open_browser(approval_url).is_err() {
        eprintln!(
            "Could not open a browser automatically. Open the URL above in a browser on this machine."
        );
    }
}

fn authorized(builder: reqwest::RequestBuilder, auth: &RequestAuth) -> reqwest::RequestBuilder {
    match auth {
        RequestAuth::Basic { key, secret } => builder.basic_auth(key, Some(secret)),
        RequestAuth::Bearer { token } => builder.bearer_auth(token),
    }
}

/// HTTP statuses that mean the user's identity was refused (exit 4). A 403 is
/// not one: a missing permission or RBAC prerequisite is a refusal that logging
/// in again cannot fix, so it is a generic failure (exit 1), as elsewhere.
const AUTH_STATUSES: &[u16] = &[401];

async fn send_json<T: DeserializeOwned>(
    builder: reqwest::RequestBuilder,
    auth: &RequestAuth,
    body: &impl Serialize,
    action: &str,
    secrets: &[&str],
    auth_statuses: &[u16],
) -> CloudResult<T> {
    let response = authorized(builder, auth)
        .json(body)
        .send()
        .await
        .map_err(|error| CloudError::new(format!("{action}: {}", error.without_url())))?;
    parse_response(response, action, secrets, auth_statuses).await
}

/// The longest server error message shown, in characters.
const MAX_SERVER_ERROR_CHARS: usize = 300;

/// The server's `error` string from a failure body, safe to show: values the
/// call carried in `secrets` are redacted (validator messages quote values),
/// control characters are dropped, and the length is capped. Anything other
/// than a JSON object with a string `error` yields nothing, so raw bodies are
/// never echoed.
fn server_error_message(bytes: &[u8], secrets: &[&str]) -> Option<String> {
    let body: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let mut message = body.get("error")?.as_str()?.to_string();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        message = message.replace(secret, "[redacted]");
    }
    let message: String = message
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_SERVER_ERROR_CHARS)
        .collect();
    let message = message.trim();
    (!message.is_empty()).then(|| message.to_string())
}

async fn parse_response<T: DeserializeOwned>(
    response: reqwest::Response,
    action: &str,
    secrets: &[&str],
    auth_statuses: &[u16],
) -> CloudResult<T> {
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| CloudError::new(format!("{action}: {}", error.without_url())))?;
    if !status.is_success() {
        let message = match server_error_message(&bytes, secrets) {
            Some(error) => format!("{action} failed ({status}): {error}"),
            None => format!("{action} failed ({status})"),
        };
        return Err(if auth_statuses.contains(&status.as_u16()) {
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

/// Connections the loopback listener serves at once. Browsers open
/// speculative connections they may never use, so one idle connection must not
/// hold up the callback; past this many, accepting pauses until one finishes.
const MAX_CALLBACK_CONNECTIONS: usize = 16;

/// How long one loopback connection may take to send its request line, and to
/// take the response, before it is dropped and its slot freed.
#[cfg(not(test))]
const CALLBACK_IO_TIMEOUT: Duration = Duration::from_secs(10);
/// Unit tests free a slot held by an idle connection without a long wait.
#[cfg(test)]
const CALLBACK_IO_TIMEOUT: Duration = Duration::from_millis(200);

/// Accept loopback connections until one hits `/callback` with the matching
/// `state`, then return its `code`. Connections are served concurrently (up to
/// [`MAX_CALLBACK_CONNECTIONS`], each bounded by [`CALLBACK_IO_TIMEOUT`]), so an
/// idle or slow connection cannot block the real redirect.
async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> CloudResult<String> {
    let expected_state: std::sync::Arc<str> = expected_state.into();
    // Dropping the set on return aborts the connections still being served.
    let mut connections = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept(), if connections.len() < MAX_CALLBACK_CONNECTIONS => {
                let (stream, _) = accepted
                    .map_err(|error| CloudError::new(format!("loopback accept failed: {error}")))?;
                let expected_state = expected_state.clone();
                connections.spawn(async move { serve_callback(stream, &expected_state).await });
            }
            Some(served) = connections.join_next() => {
                if let Ok(Some(outcome)) = served {
                    return outcome;
                }
            }
        }
    }
}

/// Serve one loopback connection; `None` means it was answered (or dropped)
/// and the wait goes on. A missing or mismatched `state` cannot be tied to this
/// request (a stray tab, or a forged redirect), so it is answered and ignored,
/// as are unrelated paths (e.g. `/favicon.ico`). With a matching `state`, an
/// `error` redirect (`access_denied` on denial) or a missing/unusable `code`
/// ends the wait.
async fn serve_callback(
    mut stream: tokio::net::TcpStream,
    expected_state: &str,
) -> Option<CloudResult<String>> {
    let target =
        match tokio::time::timeout(CALLBACK_IO_TIMEOUT, read_request_target(&mut stream)).await {
            Ok(Some(target)) => target,
            Ok(None) => {
                write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
                return None;
            }
            // Idle past the bound: drop it without a response.
            Err(_) => return None,
        };

    let (path, query) = split_target(&target);
    if path != "/callback" {
        write_response(&mut stream, "404 Not Found", NOT_FOUND_HTML).await;
        return None;
    }

    let params = parse_query(query);
    if params.get("state").map(String::as_str) != Some(expected_state) {
        write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
        return None;
    }
    // An error redirect (RFC 6749 §4.1.2.1) carries no code and is final.
    if let Some(error) = params.get("error") {
        return Some(Err(if error == "access_denied" {
            write_response(&mut stream, "200 OK", DENIED_HTML).await;
            CloudError::new("access request denied")
        } else {
            write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
            redirect_error(error)
        }));
    }
    Some(match params.get("code").filter(|code| valid_code(code)) {
        Some(code) => {
            write_response(&mut stream, "200 OK", APPROVED_HTML).await;
            Ok(code.clone())
        }
        None => {
            write_response(&mut stream, "400 Bad Request", BAD_REQUEST_HTML).await;
            Err(CloudError::new(
                "access request redirect carried no usable authorization code",
            ))
        }
    })
}

/// A non-denial error redirect. The RFC 6749 error code is echoed only when it
/// is a plain token, so the message never carries arbitrary redirect input.
fn redirect_error(error: &str) -> CloudError {
    let token = (1..=64).contains(&error.len())
        && error.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    if token {
        CloudError::new(format!(
            "access request redirect reported an error: {error}"
        ))
    } else {
        CloudError::new("access request redirect reported an error")
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
    // Best effort, and bounded: a client that never reads cannot hold the slot.
    let _ = tokio::time::timeout(CALLBACK_IO_TIMEOUT, async {
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await
    })
    .await;
}

/// Loopback page for an approved request; mirrors the console's confirmation
/// view and tries to close its tab after a short countdown.
const APPROVED_HTML: &str = include_str!("access_approved.html");
/// Loopback page for a denied request, styled like [`APPROVED_HTML`].
const DENIED_HTML: &str = include_str!("access_denied.html");
const NOT_FOUND_HTML: &str = "<!doctype html><html><body>Not found</body></html>";
const BAD_REQUEST_HTML: &str = "<!doctype html><html><body>Invalid request. You can close this tab and return to your terminal.</body></html>";

// ── waiting for approval ────────────────────────────────────────────────────

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

struct PendingApproval {
    requested_role_ids: Vec<String>,
    deadline: Deadline,
    /// Which deadline `deadline` holds, so its expiry is reported truthfully.
    limit: WaitLimit,
}

/// The deadline that bounds the wait: the server's `expiresAt`, or the
/// sooner `--timeout-seconds`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum WaitLimit {
    Server,
    Local(Duration),
}

impl WaitLimit {
    fn expired(self) -> CloudError {
        match self {
            Self::Server => CloudError::new("access request expired"),
            Self::Local(timeout) => CloudError::new(format!(
                "access request approval wait timed out after {} seconds",
                timeout.as_secs()
            )),
        }
    }
}

impl PendingApproval {
    #[cfg(test)]
    fn new(created: &CreateAccessRequestResponse) -> CloudResult<Self> {
        Self::with_timeout(created, APPROVAL_TIMEOUT)
    }

    fn with_timeout(created: &CreateAccessRequestResponse, timeout: Duration) -> CloudResult<Self> {
        if created.request_id.trim().is_empty()
            || created.approval_url.trim().is_empty()
            || created.status.as_deref() != Some("pending")
        {
            return Err(CloudError::new("file access request: incomplete response"));
        }
        let now = Instant::now();
        let mut deadline = Deadline::new(created.expires_at, epoch_millis()?, now)?;
        let local_deadline = now
            .checked_add(timeout)
            .ok_or_else(|| CloudError::usage("approval timeout is not representable"))?;
        let limit = if local_deadline < deadline.monotonic {
            deadline.monotonic = local_deadline;
            WaitLimit::Local(timeout)
        } else {
            WaitLimit::Server
        };
        Ok(Self {
            requested_role_ids: canonical_role_ids(created.requested_role_ids.as_deref(), true)?,
            deadline,
            limit,
        })
    }
}

/// An authorization code from the redirect: 1 to 512 bytes of printable,
/// non-space ASCII. The server's code format is otherwise opaque.
fn valid_code(code: &str) -> bool {
    (1..=512).contains(&code.len()) && code.bytes().all(|c| (0x21..=0x7E).contains(&c))
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

/// The authorization code arrives only on the loopback redirect, which also
/// carries a denial. The request status is never read: the redirect and the
/// pending deadline are the only signals, so no server status field can end
/// the wait before this machine's redirect arrives.
async fn wait_for_redirect(
    pending: PendingApproval,
    listener: TcpListener,
    state: &str,
) -> CloudResult<String> {
    let remaining = pending.deadline.remaining()?;
    tokio::select! {
        biased;
        code = wait_for_callback(listener, state) => code,
        _ = tokio::time::sleep(remaining) => Err(pending.limit.expired()),
    }
}

/// Build `{base}/organizations/{org}/accessRequests/{tail..}` with the
/// organization selector sent as one percent-encoded segment, as the typed
/// client does, so `/`, `?`, `#` or `%` in `--org` cannot re-target the
/// authenticated handshake. `.`, `..` and empty selectors would change the
/// route, so they are usage errors before any request.
fn access_request_url(base: &str, org_id: &str, tail: &[&str]) -> CloudResult<String> {
    if matches!(org_id, "" | "." | "..") {
        return Err(CloudError::usage(format!(
            "invalid organization ID '{org_id}'"
        )));
    }
    let mut url = url::Url::parse(base)
        .map_err(|error| CloudError::new(format!("invalid API URL '{base}': {error}")))?;
    url.path_segments_mut()
        .map_err(|()| CloudError::new(format!("invalid API URL '{base}'")))?
        .pop_if_empty()
        .push("organizations")
        .push(org_id)
        .push("accessRequests")
        .extend(tail);
    Ok(url.into())
}

/// Exchange the redirect's code for the key, once. A 401 here means the code
/// or the PKCE verifier was refused, not the user's identity, and a 403 is a
/// missing permission, so every failure is a plain one (exit 1).
async fn exchange_code(
    http: &reqwest::Client,
    token_url: &str,
    auth: &RequestAuth,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> CloudResult<TokenResponse> {
    operation(
        EXCHANGE_TIMEOUT,
        send_json(
            http.post(token_url),
            auth,
            &TokenBody {
                code,
                code_verifier: verifier,
                redirect_uri,
            },
            "exchange authorization code",
            &[code, verifier],
            &[],
        ),
    )
    .await
    .map_err(|error| {
        error.into_cloud(
            "exchange authorization code timed out; the exchange may have completed on the server; no credentials were installed",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn access_request_url_keeps_the_org_selector_in_one_segment() {
        let base = "https://api.example.test/v1";
        assert_eq!(
            access_request_url(base, "org-1", &["authorizationCode", "token"]).unwrap(),
            "https://api.example.test/v1/organizations/org-1/accessRequests/authorizationCode/token"
        );
        assert_eq!(
            access_request_url(base, "a/../b?x#y", &["authorizationCode"]).unwrap(),
            "https://api.example.test/v1/organizations/a%2F..%2Fb%3Fx%23y/accessRequests/authorizationCode"
        );
        for bad in ["", ".", ".."] {
            let error = access_request_url(base, bad, &["authorizationCode"]).unwrap_err();
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{bad:?}"
            );
        }
    }

    fn created(expires_at: i64) -> CreateAccessRequestResponse {
        CreateAccessRequestResponse {
            requested_role_ids: Some(vec![]),
            request_id: "request-1".into(),
            status: Some("pending".into()),
            approval_url: "https://example.invalid/approve".into(),
            expires_at: Some(expires_at),
        }
    }

    fn bearer() -> RequestAuth {
        RequestAuth::Bearer {
            token: "fake".into(),
        }
    }

    #[test]
    fn approved_and_denied_pages_are_distinct_and_autoclose() {
        assert_ne!(APPROVED_HTML, DENIED_HTML);
        assert!(APPROVED_HTML.contains("class=\"banner success\""));
        assert!(DENIED_HTML.contains("class=\"banner warning\""));
        for html in [APPROVED_HTML, DENIED_HTML] {
            assert!(!html.contains("{{"));
            assert!(html.contains("var seconds = 5;"));
            assert!(html.contains("window.close()"));
        }
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
    fn role_response_fields_reject_malformed_types() {
        for value in [
            serde_json::json!(42),
            serde_json::json!("secret-value"),
            serde_json::json!([null]),
            serde_json::json!([1]),
            serde_json::json!({}),
        ] {
            let create = serde_json::json!({"requestId":"request-1", "status":"pending", "approvalUrl":"https://example.invalid", "expiresAt":4102444800000_i64, "requestedRoleIds":value});
            assert!(serde_json::from_value::<CreateAccessRequestResponse>(create).is_err());
            let token = serde_json::json!({"keyId":"key-id", "key":"key", "secret":"secret", "grantedRoleIds":value});
            assert!(serde_json::from_value::<TokenResponse>(token).is_err());
        }
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
    fn valid_code_accepts_printable_non_space_ascii_up_to_512_bytes() {
        for code in ["a", "_-09AZaz", "a=b.c~d/e+f", "!~", &"a".repeat(512)] {
            assert!(valid_code(code), "{code}");
        }
        for code in [
            "",
            " a",
            "a b",
            "a\n",
            "a\t",
            "\u{7f}",
            "é",
            &"a".repeat(513),
        ] {
            assert!(!valid_code(code), "{code:?}");
        }
    }

    #[test]
    fn wait_limit_reports_the_deadline_that_won() {
        let created = created(4102444800000);
        let pending = PendingApproval::with_timeout(&created, Duration::from_secs(7)).unwrap();
        assert_eq!(pending.limit, WaitLimit::Local(Duration::from_secs(7)));
        assert_eq!(
            pending.limit.expired().message,
            "access request approval wait timed out after 7 seconds"
        );
        let soon = created_in(100);
        let pending = PendingApproval::with_timeout(&soon, Duration::from_secs(7)).unwrap();
        assert_eq!(pending.limit, WaitLimit::Server);
        assert_eq!(pending.limit.expired().message, "access request expired");
    }

    fn created_in(millis: i64) -> CreateAccessRequestResponse {
        created(epoch_millis().unwrap() + millis)
    }

    #[tokio::test]
    async fn ambiguous_org_name_points_to_org_id_with_every_match() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        const FIRST: &str = "00000000-0000-0000-0000-0000000000a1";
        const SECOND: &str = "00000000-0000-0000-0000-0000000000a2";
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/organizations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [
                    {"id": FIRST, "name": "shared"},
                    {"id": SECOND, "name": "shared"}
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = CloudClient::new(Some("key"), Some("secret"), Some(&server.uri())).unwrap();
        let error = client
            .resolve_access_organization_id(None, Some("shared".into()))
            .await
            .unwrap_err();
        // The advice must name a selector `cloud access` accepts.
        let selector = format!("--org-id={FIRST}");
        crate::cli::Cli::try_parse_from(["clickhousectl", "cloud", "access", "roles", &selector])
            .unwrap();
        for token in ["--org-id", FIRST, SECOND] {
            assert!(
                error.message.contains(token),
                "missing {token}: {}",
                error.message
            );
        }
    }

    #[tokio::test]
    async fn operation_timeout_covers_body_and_exchange_is_not_retried() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200).set_delay(EXCHANGE_TIMEOUT * 2))
            .expect(1)
            .mount(&server)
            .await;
        let error = tokio::time::timeout(
            EXCHANGE_TIMEOUT * 3,
            exchange_code(
                &reqwest::Client::new(),
                &server.uri(),
                &bearer(),
                "secret-code",
                "secret-verifier",
                "http://127.0.0.1:1/callback",
            ),
        )
        .await
        .unwrap()
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

    /// A status server that answers every GET with `body`, counting calls.
    /// Send one raw request target to the listener and return the response.
    async fn send_callback(address: SocketAddr, target: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream
            .write_all(format!("GET {target} HTTP/1.1\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8_lossy(&response).into_owned()
    }

    #[tokio::test]
    async fn bind_loopback_prefers_ipv4_then_ipv6_and_errors_without_either() {
        let loopback = bind_loopback(TcpListener::bind).await.unwrap();
        let port = loopback.listener.local_addr().unwrap().port();
        assert_eq!(
            loopback.redirect_uri,
            format!("http://127.0.0.1:{port}/callback")
        );
        assert!(!loopback.state.is_empty());

        let tried = std::sync::Mutex::new(Vec::new());
        let ipv6 = bind_loopback(|address: SocketAddr| {
            tried.lock().unwrap().push(address);
            async move {
                if address.is_ipv4() {
                    Err(std::io::Error::other("fake IPv4 failure"))
                } else {
                    // Stand in for [::1] where the host has no IPv6.
                    TcpListener::bind(("127.0.0.1", 0)).await
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(
            *tried.lock().unwrap(),
            [
                "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
                "[::1]:0".parse().unwrap()
            ]
        );
        assert!(ipv6.redirect_uri.starts_with("http://127.0.0.1:"));
        if let Ok(listener) = TcpListener::bind("[::1]:0").await {
            drop(listener);
            let ipv6 = bind_loopback(|address: SocketAddr| async move {
                if address.is_ipv4() {
                    Err(std::io::Error::other("fake IPv4 failure"))
                } else {
                    TcpListener::bind(address).await
                }
            })
            .await
            .unwrap();
            let port = ipv6.listener.local_addr().unwrap().port();
            assert_eq!(ipv6.redirect_uri, format!("http://[::1]:{port}/callback"));
        }

        let error = bind_loopback(|_| async { Err(std::io::Error::other("fake")) })
            .await
            .err()
            .unwrap();
        assert_eq!(
            error.message,
            "could not start a local redirect listener on 127.0.0.1 or [::1]"
        );
    }

    #[tokio::test]
    async fn wait_for_redirect_takes_the_code_from_the_callback() {
        let created = created(4102444800000);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wait = wait_for_redirect(PendingApproval::new(&created).unwrap(), listener, "state");
        let notify = async {
            let response = send_callback(address, "/callback?code=test-code&state=state").await;
            assert!(response.starts_with("HTTP/1.1 200"));
        };
        let (code, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                .await
                .unwrap();
        assert_eq!(code.unwrap(), "test-code");
    }

    #[tokio::test]
    async fn wait_for_callback_ignores_unmatched_state_and_other_paths() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wait = wait_for_callback(listener, "state");
        let notify = async {
            for (target, status) in [
                ("/favicon.ico", "404"),
                ("/callback?code=forged&state=other", "400"),
                ("/callback?code=forged", "400"),
                ("/callback?state=", "400"),
                ("/callback?code=a%3Db&state=state", "200"),
            ] {
                let response = send_callback(address, target).await;
                assert!(
                    response.starts_with(&format!("HTTP/1.1 {status}")),
                    "{target}: {response}"
                );
            }
        };
        let (code, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                .await
                .unwrap();
        assert_eq!(code.unwrap(), "a=b");
    }

    #[tokio::test]
    async fn wait_for_callback_with_matching_state_and_bad_code_is_an_error() {
        for target in [
            "/callback?state=state",
            "/callback?code=&state=state",
            "/callback?code=a%20b&state=state",
            "/callback?code=a%0Ab&state=state",
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (result, response) = tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(
                    wait_for_callback(listener, "state"),
                    send_callback(address, target)
                )
            })
            .await
            .unwrap();
            assert!(response.starts_with("HTTP/1.1 400"), "{target}");
            let error = result.unwrap_err();
            assert!(
                matches!(error.kind, crate::cloud::client::CloudErrorKind::Generic),
                "{target}"
            );
            assert!(error.message.contains("no usable authorization code"));
        }
    }

    #[tokio::test]
    async fn wait_for_callback_with_matching_state_and_error_ends_the_wait() {
        for (target, status, message) in [
            (
                "/callback?error=access_denied&state=state",
                "200",
                "access request denied",
            ),
            (
                "/callback?error=server_error&state=state",
                "400",
                "access request redirect reported an error: server_error",
            ),
            (
                "/callback?error=%3Cscript%3E&state=state",
                "400",
                "access request redirect reported an error",
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (result, response) = tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(
                    wait_for_callback(listener, "state"),
                    send_callback(address, target)
                )
            })
            .await
            .unwrap();
            assert!(
                response.starts_with(&format!("HTTP/1.1 {status}")),
                "{target}"
            );
            let error = result.unwrap_err();
            assert!(
                matches!(error.kind, crate::cloud::client::CloudErrorKind::Generic),
                "{target}"
            );
            assert_eq!(error.message, message, "{target}");
        }
    }

    #[tokio::test]
    async fn wait_for_callback_ignores_error_with_unmatched_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wait = wait_for_callback(listener, "state");
        let notify = async {
            let forged = send_callback(address, "/callback?error=access_denied&state=other").await;
            assert!(forged.starts_with("HTTP/1.1 400"));
            let real = send_callback(address, "/callback?code=c&state=state").await;
            assert!(real.starts_with("HTTP/1.1 200"));
        };
        let (code, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                .await
                .unwrap();
        assert_eq!(code.unwrap(), "c");
    }

    #[tokio::test]
    async fn wait_for_redirect_reports_local_and_server_deadlines() {
        let created = created(4102444800000);
        let mut pending = PendingApproval::with_timeout(&created, Duration::from_secs(9)).unwrap();
        pending.deadline.monotonic = Instant::now() + Duration::from_millis(150);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wait = wait_for_redirect(pending, listener, "state");
        // An idle connection without a request line is cut off by the deadline.
        let idle = async {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            let mut response = Vec::new();
            let _ = stream.read_to_end(&mut response).await;
        };
        let (result, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, idle) })
                .await
                .unwrap();
        assert_eq!(
            result.unwrap_err().message,
            "access request approval wait timed out after 9 seconds"
        );

        let created = created_in(150);
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            wait_for_redirect(
                PendingApproval::new(&created).unwrap(),
                TcpListener::bind("127.0.0.1:0").await.unwrap(),
                "state",
            ),
        )
        .await
        .unwrap();
        assert_eq!(result.unwrap_err().message, "access request expired");
    }

    #[tokio::test]
    async fn wait_for_callback_reassembles_a_request_line_split_across_writes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wait = wait_for_callback(listener, "state");
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
        let (code, ()) =
            tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(wait, notify) })
                .await
                .unwrap();
        assert_eq!(code.unwrap(), "test-code");
    }

    #[tokio::test]
    async fn wait_for_callback_is_not_blocked_by_an_idle_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        // Accepted first and never sends a byte, like a browser preconnect.
        let idle = tokio::net::TcpStream::connect(address).await.unwrap();
        let started = Instant::now();
        let (code, response) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(wait_for_callback(listener, "state"), async {
                // Let the listener accept the idle connection first.
                tokio::time::sleep(Duration::from_millis(20)).await;
                send_callback(address, "/callback?code=c&state=state").await
            })
        })
        .await
        .unwrap();
        assert_eq!(code.unwrap(), "c");
        assert!(response.starts_with("HTTP/1.1 200"));
        // Served while the idle connection still held its slot.
        assert!(started.elapsed() < CALLBACK_IO_TIMEOUT);
        drop(idle);
    }

    #[tokio::test]
    async fn wait_for_callback_frees_slots_held_by_idle_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut idle = Vec::new();
        for _ in 0..MAX_CALLBACK_CONNECTIONS {
            idle.push(tokio::net::TcpStream::connect(address).await.unwrap());
        }
        let started = Instant::now();
        let (code, response) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(wait_for_callback(listener, "state"), async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                send_callback(address, "/callback?code=c&state=state").await
            })
        })
        .await
        .unwrap();
        assert_eq!(code.unwrap(), "c");
        assert!(response.starts_with("HTTP/1.1 200"));
        // Every slot was taken, so the callback waited for an idle one to time out.
        assert!(started.elapsed() >= CALLBACK_IO_TIMEOUT);
        drop(idle);
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
        let token_path = "/v1/organizations/org/accessRequests/authorizationCode/token";
        for status in [301, 302, 303, 307, 308] {
            let server = MockServer::start().await;
            let destination = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path(token_path))
                .respond_with(
                    ResponseTemplate::new(status)
                        .insert_header("Location", format!("{}/stolen", destination.uri())),
                )
                .expect(1)
                .mount(&server)
                .await;
            let exchanged = exchange_code(
                &handshake_client().unwrap(),
                &format!("{}{token_path}", server.uri()),
                &bearer(),
                "code-secret",
                "secret-verifier",
                "http://127.0.0.1:1/callback",
            )
            .await;
            let error = exchanged.err().unwrap();
            assert!(
                !error.message.contains("secret"),
                "redirect handling must not leak the verifier: {error}"
            );
            assert!(destination.received_requests().await.unwrap().is_empty());
        }
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
            parse_response::<TokenResponse>(response, "exchange", &[], &[]).await
        })
        .await;
        server.abort();
        assert!(matches!(result, Err(OperationError::Timeout)));

        use wiremock::{Mock, MockServer, ResponseTemplate};
        for field in ["error", "status"] {
            let server = MockServer::start().await;
            let mut body = serde_json::json!({"result":{"keyId":"k","key":"k","secret":"s"}});
            body[field] = if field == "error" {
                serde_json::json!("secret-echo")
            } else {
                serde_json::json!(500)
            };
            Mock::given(wiremock::matchers::method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&server)
                .await;
            let error = exchange_code(
                &client,
                &server.uri(),
                &bearer(),
                "code",
                "verifier",
                "http://127.0.0.1:1/callback",
            )
            .await
            .err()
            .unwrap();
            assert!(!error.message.contains("secret-echo"));
            assert!(error.message.contains("envelope"));
        }
    }

    #[test]
    fn approval_handoff_launches_once_and_survives_launcher_failure() {
        let mut calls = 0;
        approval_handoff("https://example.invalid/approve", |url| {
            assert_eq!(url, "https://example.invalid/approve");
            calls += 1;
            Ok(())
        });
        assert_eq!(calls, 1);
        // Launcher failure still permits opening the printed URL manually.
        approval_handoff("https://example.invalid/approve", |_| {
            Err(std::io::Error::other("fake launcher failure"))
        });
    }

    #[tokio::test]
    async fn exchange_maps_statuses_once_and_never_echoes_code_or_verifier() {
        use crate::cloud::client::CloudErrorKind;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for (status, message, kind) in [
            (
                400,
                "BAD_REQUEST: request is not approved",
                CloudErrorKind::Generic,
            ),
            (400, "BAD_REQUEST: code expired", CloudErrorKind::Generic),
            (
                400,
                "BAD_REQUEST: code already redeemed",
                CloudErrorKind::Generic,
            ),
            (
                400,
                "BAD_REQUEST: invalid_grant: redirect URI does not match the filed value",
                CloudErrorKind::Generic,
            ),
            (
                400,
                "BAD_REQUEST: request body property can't be validated: code secret-code",
                CloudErrorKind::Generic,
            ),
            (401, "UNAUTHORIZED: invalid code", CloudErrorKind::Generic),
            (
                401,
                "UNAUTHORIZED: PKCE verification failed secret-verifier",
                CloudErrorKind::Generic,
            ),
            (
                403,
                "FORBIDDEN: early access API endpoint",
                CloudErrorKind::Generic,
            ),
            (
                403,
                "FORBIDDEN: access requests require permission to create API keys",
                CloudErrorKind::Generic,
            ),
            (
                403,
                "FORBIDDEN: access requests require an organization migrated to RBAC v2",
                CloudErrorKind::Generic,
            ),
            (
                403,
                "FORBIDDEN: some other refusal",
                CloudErrorKind::Generic,
            ),
            (
                404,
                "NOT_FOUND: access request not found",
                CloudErrorKind::Generic,
            ),
            (
                500,
                "INTERNAL_SERVER_ERROR: secret-code",
                CloudErrorKind::Generic,
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(wiremock::matchers::method("POST"))
                .respond_with(
                    ResponseTemplate::new(status).set_body_json(serde_json::json!({
                        "error": message, "status": status, "requestId": "corr-1"
                    })),
                )
                .expect(1)
                .mount(&server)
                .await;
            let error = exchange_code(
                &reqwest::Client::new(),
                &server.uri(),
                &bearer(),
                "secret-code",
                "secret-verifier",
                "http://127.0.0.1:1/callback",
            )
            .await
            .err()
            .unwrap();
            assert_eq!(
                std::mem::discriminant(&error.kind),
                std::mem::discriminant(&kind),
                "{status} {message}"
            );
            assert!(error.message.contains(&format!("({status}")), "{error}");
            // The server's text is shown, minus the secrets it echoes.
            let shown = message.split(" secret").next().unwrap();
            assert!(
                error.message.contains(shown.trim_end_matches(": ")),
                "{error}"
            );
            assert!(!error.message.contains("secret-code"), "{error}");
            assert!(!error.message.contains("secret-verifier"), "{error}");
        }
    }

    #[test]
    fn server_error_message_surfaces_error_string_safely() {
        let body = |value: serde_json::Value| serde_json::to_vec(&value).unwrap();
        assert_eq!(
            server_error_message(
                &body(serde_json::json!({"error":"PKCE verification failed","status":401})),
                &[]
            )
            .as_deref(),
            Some("PKCE verification failed")
        );
        let echoed = server_error_message(
            &body(serde_json::json!({"error":"BAD_REQUEST: code 'code-1' verifier 'ver-1'"})),
            &["code-1", "ver-1", ""],
        )
        .unwrap();
        assert!(!echoed.contains("code-1") && !echoed.contains("ver-1"));
        assert!(echoed.contains("[redacted]"));
        let long = server_error_message(
            &body(serde_json::json!({"error": format!("a\n{}", "b".repeat(1000))})),
            &[],
        )
        .unwrap();
        assert_eq!(long.chars().count(), MAX_SERVER_ERROR_CHARS);
        assert!(!long.contains('\n'));
        for raw in [
            b"not json".to_vec(),
            body(serde_json::json!({"error": {"nested":"secret"}})),
            body(serde_json::json!({"message":"secret"})),
            body(serde_json::json!({"error":"  "})),
        ] {
            assert_eq!(server_error_message(&raw, &[]), None);
        }
    }

    #[tokio::test]
    async fn exchange_sends_code_verifier_and_redirect_uri() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::header("authorization", "Bearer fake"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "code": "redirect-code",
                "codeVerifier": "verifier",
                "redirectUri": "http://127.0.0.1:1234/callback"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"keyId":"key-id","key":"key","secret":"secret","keyExpiresAt":1794231456974_i64,"grantedRoleIds":["21111111-1111-1111-9999-000000000007"]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let issued = exchange_code(
            &reqwest::Client::new(),
            &server.uri(),
            &bearer(),
            "redirect-code",
            "verifier",
            "http://127.0.0.1:1234/callback",
        )
        .await
        .unwrap();
        assert_eq!(issued.key_id, "key-id");
        assert_eq!(issued.key_expires_at, Some(1794231456974));
    }

    #[test]
    fn key_expiry_formats_rfc3339_utc_and_rejects_out_of_range() {
        assert_eq!(
            key_expiry(Some(1794231456974)).unwrap().as_deref(),
            Some("2026-11-09T13:37:36Z")
        );
        assert_eq!(key_expiry(None).unwrap(), None);
        for value in [0, -1, MAX_EPOCH_MILLIS + 1, i64::MAX] {
            assert!(key_expiry(Some(value)).is_err(), "{value}");
        }
    }

    /// `args` with the required `--purpose` added right after `request` when
    /// the test does not pass one itself.
    fn with_purpose<'a>(args: &[&'a str]) -> Vec<&'a str> {
        let mut args = args.to_vec();
        if !args.iter().any(|arg| arg.starts_with("--purpose"))
            && let Some(at) = args.iter().position(|arg| *arg == "request")
        {
            args.insert(at + 1, "--purpose=test purpose");
        }
        args
    }

    fn summary_fixture() -> Vec<AssignableRole> {
        serde_json::from_value(serde_json::json!([
            {"id": "21111111-1111-1111-9999-000000000005", "name": "Service API Admin", "type": "system", "policies": [
                {"allowDeny": "ALLOW", "permissions": [
                    "control-plane:service:manage", "control-plane:service:delete",
                    "control-plane:service:view", "control-plane:postgres-service:manage",
                    "control-plane:organization:create-api-keys"
                ], "resources": ["instance/*", "postgres/*"]},
                {"allowDeny": "ALLOW", "permissions": ["control-plane:service:view-backups"], "resources": ["instance/*"]},
                {"allowDeny": "DENY", "permissions": ["control-plane:service:delete"], "resources": ["instance/prod"]}
            ]},
            {"id": "21111111-1111-1111-9999-000000000002", "name": "Viewer", "type": "system", "policies": [
                {"allowDeny": "ALLOW", "permissions": ["control-plane:service:view"], "resources": ["*"]}
            ]},
            {"id": "21111111-1111-1111-9999-000000000003", "name": "SQL", "type": "custom", "policies": [
                {"allowDeny": "ALLOW", "permissions": ["sql-console:database:access"], "resources": ["instance/a"], "tags": {"roleV2": "sql-console-admin"}}
            ]},
            {"id": "21111111-1111-1111-9999-000000000004", "name": "Another viewer", "type": "system", "policies": [
                {"allowDeny": "ALLOW", "permissions": ["control-plane:service:view"], "resources": ["*"]}
            ]}
        ]))
        .unwrap()
    }

    #[test]
    fn role_summary_orders_by_writes_then_total_then_name_and_groups_by_family() {
        let summaries = summarize_roles(&summary_fixture());
        let names: Vec<_> = summaries.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            ["Another viewer", "Viewer", "SQL", "Service API Admin"]
        );
        let admin = &summaries[3];
        assert_eq!(
            admin.write,
            vec![
                (
                    "service".to_string(),
                    vec!["manage".to_string(), "delete".to_string()]
                ),
                ("postgres-service".to_string(), vec!["manage".to_string()]),
                (
                    "organization".to_string(),
                    vec!["create-api-keys".to_string()]
                ),
            ]
        );
        assert_eq!(
            admin.read,
            vec![(
                "service".to_string(),
                vec!["view".to_string(), "view-backups".to_string()]
            )]
        );
        assert_eq!(admin.on, vec!["instance/*", "postgres/*"]);
        assert_eq!(
            admin.denied,
            vec![("service".to_string(), vec!["delete".to_string()])]
        );
        // A non-control-plane permission keeps its first segment as family
        // and carries the policy's roleV2 tag.
        assert_eq!(
            summaries[2].write,
            vec![(
                "sql-console".to_string(),
                vec!["database:access (sql-console-admin)".to_string()]
            )]
        );
    }

    #[test]
    fn role_summary_json_is_one_compact_role_per_line() {
        let text = summary_json(&summarize_roles(&summary_fixture()));
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0], "[");
        assert_eq!(lines[5], "]");
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 4);
        let admin = &parsed[3];
        let keys: Vec<_> = admin.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["id", "name", "write", "read", "on", "denied"]);
        assert_eq!(
            admin["write"]["service"],
            serde_json::json!(["manage", "delete"])
        );
        assert_eq!(admin["denied"]["service"], serde_json::json!(["delete"]));
        assert!(parsed[0].get("denied").is_none());
        assert_eq!(parsed[0]["write"], serde_json::json!({}));
        assert_eq!(summary_json(&[]), "[]");
    }

    #[test]
    fn role_summary_human_lists_every_permission_per_family() {
        let text = summary_human("org-1", &summarize_roles(&summary_fixture()));
        assert!(text.starts_with("Organization: org-1\n"));
        assert!(
            text.contains(
                "Service API Admin  21111111-1111-1111-9999-000000000005\n  \
             write  service: manage, delete\n         \
             postgres-service: manage\n         \
             organization: create-api-keys\n  \
             read   service: view, view-backups\n  \
             on     instance/*, postgres/*\n  \
             denied service: delete\n"
            ),
            "{text}"
        );
        assert!(text.contains("Viewer  21111111-1111-1111-9999-000000000002\n  write  none\n"));
        assert!(summary_human("org-1", &[]).contains("No assignable roles"));
    }

    #[test]
    fn roles_policies_flag_parses_only_on_roles() {
        let cli = Cli::try_parse_from(["chctl", "cloud", "access", "roles", "--policies"]).unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Access { command, .. } = cloud.command else {
            panic!("expected access command");
        };
        assert!(matches!(
            command,
            AccessCommands::Roles(AccessRolesArgs { policies: true })
        ));
        assert!(
            Cli::try_parse_from([
                "chctl",
                "cloud",
                "access",
                "request",
                "--purpose=x",
                "--policies"
            ])
            .is_err()
        );
    }

    #[test]
    fn approved_summary_names_granted_requested_expiry_and_project() {
        let menu = summary_fixture();
        let admin = "21111111-1111-1111-9999-000000000005";
        let viewer = "21111111-1111-1111-9999-000000000002";
        let grant = AccessGrant {
            api_key: "key".into(),
            key_id: "key-id".into(),
            organization_id: "org".into(),
            request_id: "request".into(),
            expires_at: Some("2026-11-19T09:58:23Z".into()),
            roles: vec![
                granted_role(admin, Some(&menu)),
                granted_role("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", Some(&menu)),
            ],
        };
        let expires = 1_795_082_303_000_i64; // 2026-11-19T09:58:23Z
        let text = approved_summary(
            "/home/dev/work/.clickhouse/credentials.json",
            "/home/dev/work",
            &grant,
            &[viewer.to_string()],
            Some(&menu),
            Some(expires),
            expires - 15 * 60_000,
        );
        assert_eq!(
            text,
            format!(
                "Approved. Key saved: /home/dev/work/.clickhouse/credentials.json\n  \
                 Granted: Service API Admin ({admin}), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\n  \
                 Requested: Viewer; the approver changed the roles.\n  \
                 Expires: 2026-11-19T09:58:23Z (15 min)\n  \
                 Active in /home/dev/work and below.\n"
            )
        );
        // Granted as requested: no Requested line; a failed lookup keeps ids.
        let same = AccessGrant {
            roles: vec![granted_role(admin, None)],
            ..grant
        };
        let text = approved_summary("p", "/w", &same, &[admin.to_string()], None, None, 0);
        assert_eq!(
            text,
            format!("Approved. Key saved: p\n  Granted: {admin}\n  Active in /w and below.\n")
        );
        assert!(same.roles[0].policies.is_none());
    }

    #[test]
    fn request_requires_a_purpose() {
        let error = Cli::try_parse_from(["chctl", "cloud", "access", "request"])
            .err()
            .unwrap();
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(error.to_string().contains("--purpose"), "{error}");
        assert_eq!(
            parse(&["chctl", "cloud", "access", "request", "--purpose", "why"])
                .purpose
                .as_deref(),
            Some("why")
        );
    }

    fn parse(args: &[&str]) -> AccessRequestArgs {
        let cli = Cli::try_parse_from(with_purpose(args)).expect("parse");
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
    fn request_timeout_defaults_and_validates_positive_seconds() {
        assert_eq!(
            parse(&["chctl", "cloud", "access", "request"]).timeout_seconds,
            120
        );
        for seconds in ["1", "600", "4294967295"] {
            assert_eq!(
                parse(&[
                    "chctl",
                    "cloud",
                    "access",
                    "request",
                    "--timeout-seconds",
                    seconds
                ])
                .timeout_seconds,
                seconds.parse::<u32>().unwrap()
            );
        }
        for seconds in ["0", "-1", "", "1.5", "5m", "4294967296"] {
            assert!(
                Cli::try_parse_from([
                    "chctl",
                    "cloud",
                    "access",
                    "request",
                    "--timeout-seconds",
                    seconds
                ])
                .is_err()
            );
        }
        assert!(
            Cli::try_parse_from([
                "chctl",
                "cloud",
                "access",
                "request",
                "--timeout-seconds",
                "1",
                "--timeout-seconds",
                "2"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "chctl",
                "cloud",
                "access",
                "roles",
                "--timeout-seconds",
                "1"
            ])
            .is_err()
        );
    }

    #[test]
    fn approval_timeout_override_preserves_earlier_server_expiry() {
        for seconds in [1, 300, 600] {
            let timeout = Duration::from_secs(seconds);
            let future = created(4102444800000);
            let before = Instant::now();
            let pending = PendingApproval::with_timeout(&future, timeout).unwrap();
            let after = Instant::now();
            assert!(pending.deadline.monotonic >= before + timeout);
            assert!(pending.deadline.monotonic <= after + timeout);
            assert_eq!(pending.deadline.epoch_millis, 4102444800000);

            let expiry = epoch_millis().unwrap() + 100;
            let sooner = created(expiry);
            let pending = PendingApproval::with_timeout(&sooner, timeout).unwrap();
            assert_eq!(pending.deadline.epoch_millis, expiry);
            assert!(pending.deadline.remaining().unwrap() <= Duration::from_millis(100));
        }
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
                .chain(with_purpose(args))
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
    fn request_flags_drop_agent_fields_and_validate_lengths_in_characters() {
        for flag in ["--tool", "--version", "--session-id"] {
            assert!(
                Cli::try_parse_from(["chctl", "cloud", "access", "request", flag, "x"]).is_err(),
                "{flag}"
            );
        }
        assert!(
            parse(&["chctl", "cloud", "access", "request"])
                .validate()
                .is_ok()
        );
        for (flag, max) in [("--purpose", 200), ("--key-name", 100)] {
            for (length, ok) in [(0, false), (1, true), (max, true), (max + 1, false)] {
                // Multi-byte characters: the limit counts characters, not bytes.
                let value = "é".repeat(length);
                let args = parse(&["chctl", "cloud", "access", "request", flag, &value]);
                assert_eq!(args.validate().is_ok(), ok, "{flag} {length}");
            }
        }
    }

    #[test]
    fn pkce_verifier_is_unreserved_and_challenge_matches_rfc7636_vector() {
        // RFC 7636 appendix B.
        assert_eq!(
            s256_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let pkce = generate_pkce();
        assert_eq!(pkce.challenge, s256_challenge(&pkce.verifier));
        assert_eq!(pkce.challenge.len(), 43);
        // 32 random bytes base64url-encode (no pad) to 43 chars.
        assert_eq!(pkce.verifier.len(), 43);
        assert!(
            pkce.verifier
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-._~".contains(&c))
        );
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

    fn keys(value: &serde_json::Value) -> Vec<String> {
        let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }

    #[test]
    fn create_body_is_flat_and_omits_absent_optional_fields() {
        let minimal = CreateAccessRequestBody {
            redirect_uri: "http://127.0.0.1:1234/callback",
            state: "state",
            code_challenge: "challenge",
            code_challenge_method: "S256",
            requested_role_ids: None,
            key_name: None,
            purpose: None,
        };
        assert_eq!(
            serde_json::to_value(&minimal).unwrap(),
            serde_json::json!({
                "redirectUri": "http://127.0.0.1:1234/callback",
                "state": "state",
                "codeChallenge": "challenge",
                "codeChallengeMethod": "S256"
            })
        );
        let roles = vec!["21111111-1111-1111-9999-000000000007".to_string()];
        let maximal = CreateAccessRequestBody {
            requested_role_ids: Some(&roles),
            key_name: Some("agent-key"),
            purpose: Some("ci"),
            ..minimal
        };
        let value = serde_json::to_value(&maximal).unwrap();
        assert_eq!(
            keys(&value),
            [
                "codeChallenge",
                "codeChallengeMethod",
                "keyName",
                "purpose",
                "redirectUri",
                "requestedRoleIds",
                "state"
            ]
        );
        assert_eq!(value["purpose"], "ci");
        assert_eq!(value["keyName"], "agent-key");
        assert_eq!(value["requestedRoleIds"], serde_json::json!(roles));
    }

    #[test]
    fn token_body_carries_only_code_verifier_and_redirect_uri() {
        let body = TokenBody {
            code: "code",
            code_verifier: "verifier",
            redirect_uri: "http://[::1]:1/callback",
        };
        assert_eq!(
            keys(&serde_json::to_value(&body).unwrap()),
            ["code", "codeVerifier", "redirectUri"]
        );
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
    fn token_response_reads_key_secret_and_expiry_from_result_envelope() {
        let issued: TokenResponse = unwrap_result(
            r#"{
                "result": { "keyId": "mgmt-1", "key": "PUBLICKEY", "secret": "SH_secret", "keyExpiresAt": 1794231456974 },
                "requestId": "corr-4"
            }"#,
        );
        assert_eq!(issued.key_id, "mgmt-1");
        assert_eq!(issued.key, "PUBLICKEY");
        assert_eq!(issued.secret, "SH_secret");
        assert_eq!(issued.key_expires_at, Some(1_794_231_456_974));
        let without: TokenResponse =
            unwrap_result(r#"{"result":{"keyId":"k","key":"k","secret":"s"}}"#);
        assert_eq!(without.key_expires_at, None);
    }
}
