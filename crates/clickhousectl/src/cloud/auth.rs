use super::permissions::{Conditional, Declaration as Permission};
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("auth login", &[&op::WHOAMI_GET]).unscoped(),
    Permission::non_api(
        "auth logout",
        "Clears local credentials; no Cloud API call.",
    ),
    // The role of an API key in use without a saved grant is best effort:
    // many roles cannot read API keys, and status then says `role unknown`.
    Permission::api("auth status", &[&op::WHOAMI_GET])
        .when(&[Conditional::tolerated(
            "Showing the role of the API key in use",
            &[&op::OPENAPI_KEY_GET],
        )])
        .unscoped(),
    Permission::non_api("auth signup", "Opens account signup; no Cloud API call."),
    Permission::api("auth whoami", &[&op::WHOAMI_GET]).unscoped(),
];

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::print_human;
use crate::cloud::{
    AuthSource, dotenv_env_provenance, env_cred_presence, resolve_active_auth_source,
};
use crate::error::Error;
use crate::failure::{ApiFailure, FailureKind};
use clap::Subcommand;
use clickhouse_cloud_api::models::{ApiKey, Whoami};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AUDIENCE: &str = "clickhousectl";
const SCOPE: &str = "openid profile email offline_access";

const DEFAULT_API_URL: &str = "https://api.clickhouse.cloud/v1";

// With access negotiation compiled in, help and `auth status` point an OAuth
// login at `cloud access request` for writes; a default binary must not.
#[cfg(feature = "experimental-access-negotiation")]
const LOGIN_AFTER_HELP: &str = "\
CONTEXT FOR AGENTS:
  OAuth device flow: opens a browser, needs a human; the tokens are read-only.
  For writes afterwards, run `cloud access request`.
  In JSON mode the device code and URL go to stderr; stdout holds only the JSON report.
  Prints the verified identity.";
#[cfg(not(feature = "experimental-access-negotiation"))]
const LOGIN_AFTER_HELP: &str = "\
CONTEXT FOR AGENTS:
  No flags: OAuth device flow, opens a browser, needs a human; the tokens are read-only.
  In JSON mode the device code and URL go to stderr; stdout holds only the JSON report.
  --api-key/--api-secret: no browser, read+write; a key the API rejects is not saved (exit 4).
  Prints the verified identity; offline, the key is saved unverified with a warning.
  Create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl";

#[cfg(feature = "experimental-access-negotiation")]
const SIGNUP_AFTER_HELP: &str = "\
CONTEXT FOR AGENTS:
  Opens the ClickHouse Cloud sign-up page in a browser; a human must finish it.
  Next: `cloud auth login`";
#[cfg(not(feature = "experimental-access-negotiation"))]
const SIGNUP_AFTER_HELP: &str = "\
CONTEXT FOR AGENTS:
  Opens the ClickHouse Cloud sign-up page in a browser; a human must finish it.
  Next: `cloud auth login --api-key X --api-secret Y`";

/// What `auth status` reports as `write_access` while OAuth is the active source.
#[cfg(feature = "experimental-access-negotiation")]
const OAUTH_WRITE_ACCESS: Option<&str> = Some(
    "none: OAuth is read-only. Run `clickhousectl cloud access roles`, then \
     `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`; the user approves it in their browser.",
);
#[cfg(not(feature = "experimental-access-negotiation"))]
const OAUTH_WRITE_ACCESS: Option<&str> = None;

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in to ClickHouse Cloud
    #[command(after_help = LOGIN_AFTER_HELP)]
    Login {
        /// Prompt for an API key and secret instead of using flags
        #[arg(long)]
        interactive: bool,

        /// Cloud API key (requires --api-secret for auth login)
        #[arg(long, display_order = crate::cli::help_order::API_KEY)]
        api_key: Option<String>,

        /// Cloud API secret (requires --api-key for auth login)
        #[arg(long, display_order = crate::cli::help_order::API_SECRET)]
        api_secret: Option<String>,
    },
    /// Log out and clear saved credentials
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  With no flags, clears everything. Use --oauth to keep API keys, or --api-keys to keep OAuth tokens.")]
    Logout {
        /// Clear only OAuth tokens (keep API keys)
        #[arg(long, conflicts_with = "api_keys")]
        oauth: bool,

        /// Clear only API keys (keep OAuth tokens)
        #[arg(long, conflicts_with = "oauth")]
        api_keys: bool,
    },
    /// Show current authentication status
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Also checks the active credentials with whoami: verified, rejected or unavailable.
  Always exits 0, even when the credentials are rejected or the API is unreachable.")]
    Status,
    /// Show the identity behind the active credentials (Beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Works with OAuth or API key credentials and needs no --org-id.
  Use it to confirm who you are and which orgs you can use; exits 4 if the credentials are rejected.")]
    Whoami,
    /// Create a ClickHouse Cloud account
    #[command(after_help = SIGNUP_AFTER_HELP)]
    Signup,
}

impl AuthCommands {
    /// Usage errors for `auth login` that clap cannot see: the credential
    /// flags are global, so a pair split across command levels only meets
    /// after propagation, and `--interactive` needs a terminal to prompt on.
    pub fn login_validation_error(
        &self,
        has_terminal: bool,
    ) -> Option<(clap::error::ErrorKind, &'static str)> {
        let Self::Login {
            interactive,
            api_key,
            api_secret,
        } = self
        else {
            return None;
        };
        use clap::error::ErrorKind;
        if *interactive {
            return if api_key.is_some() || api_secret.is_some() {
                Some((
                    ErrorKind::ArgumentConflict,
                    "--interactive cannot be used with --api-key or --api-secret",
                ))
            } else if !has_terminal {
                Some((
                    ErrorKind::MissingRequiredArgument,
                    "--interactive requires a TTY. Use --api-key and --api-secret in \
                     non-interactive environments.",
                ))
            } else {
                None
            };
        }
        match (api_key.as_deref(), api_secret.as_deref()) {
            (Some(_), None) => Some((
                ErrorKind::MissingRequiredArgument,
                "--api-secret is required when --api-key is provided",
            )),
            (None, Some(_)) => Some((
                ErrorKind::MissingRequiredArgument,
                "--api-key is required when --api-secret is provided",
            )),
            (Some(""), _) => Some((ErrorKind::InvalidValue, "--api-key must not be empty")),
            (_, Some("")) => Some((ErrorKind::InvalidValue, "--api-secret must not be empty")),
            _ => None,
        }
    }

    pub fn is_write(&self) -> bool {
        match self {
            AuthCommands::Login { .. } => false,
            AuthCommands::Logout { .. } => false,
            AuthCommands::Status => false,
            AuthCommands::Signup => false,
            AuthCommands::Whoami => false,
        }
    }

    /// Whether this command calls the Cloud API, and so needs a `CloudClient`
    /// built from the resolved credentials. The others manage local state.
    pub fn needs_client(&self) -> bool {
        match self {
            AuthCommands::Login { .. }
            | AuthCommands::Logout { .. }
            | AuthCommands::Status
            | AuthCommands::Signup => false,
            AuthCommands::Whoami => true,
        }
    }
}

/// Run an auth command that calls the Cloud API (see [`AuthCommands::needs_client`]).
pub async fn run_with_client(
    client: &CloudClient,
    command: AuthCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        AuthCommands::Whoami => {
            let identity = client.get_whoami().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&identity)?);
            } else {
                print_human(&identity)?;
            }
            Ok(())
        }
        AuthCommands::Login { .. }
        | AuthCommands::Logout { .. }
        | AuthCommands::Status
        | AuthCommands::Signup => {
            unreachable!("local auth commands are handled before a client is built")
        }
    }
}

impl CloudClient {
    /// Resolve the caller behind the active credentials; not organization-scoped.
    /// Also identifies the key `cloud service query` binds to an endpoint (#1043).
    pub(crate) async fn get_whoami(&self) -> CloudResult<Whoami> {
        let response = self.api().whoami_get().await.map_err(|error| {
            // whoami answers any credential the API accepts, so its 403
            // refuses the credentials themselves; elsewhere a 403 refuses
            // only the request (#1225).
            let refused = matches!(error, clickhouse_cloud_api::Error::Api { status: 403, .. });
            let error = self.convert_error(error);
            if refused {
                super::CloudError {
                    kind: super::CloudErrorKind::Auth,
                    ..error
                }
            } else {
                error
            }
        })?;
        Self::unwrap_response(response)
    }
}

pub async fn run(
    command: AuthCommands,
    api_key: Option<&str>,
    api_secret: Option<&str>,
    api_url: Option<&str>,
    debug: bool,
    json: bool,
) -> crate::error::Result<()> {
    match command {
        AuthCommands::Login {
            interactive,
            api_key,
            api_secret,
        } => {
            if interactive {
                let (key, secret) =
                    prompt_api_credentials().map_err(super::cloud_error_to_top_level)?;
                login_with_api_key(&key, &secret, api_url, json).await
            } else if api_key.is_some() || api_secret.is_some() {
                let key = api_key.ok_or_else(|| {
                    Error::AuthRequired(
                        "--api-key is required when --api-secret is provided".into(),
                    )
                })?;
                let secret = api_secret.ok_or_else(|| {
                    Error::AuthRequired(
                        "--api-secret is required when --api-key is provided".into(),
                    )
                })?;
                login_with_api_key(&key, &secret, api_url, json).await
            } else {
                let url = api_url.unwrap_or(DEFAULT_API_URL);
                let tokens = device_auth_login(url, json)
                    .await
                    .map_err(super::cloud_error_to_top_level)?;
                save_tokens(&tokens).map_err(super::cloud_error_to_top_level)?;
                let tokens_path = tokens_path().map_err(super::cloud_error_to_top_level)?;
                let report = oauth_login_report(&tokens, tokens_path.display().to_string()).await;
                if !json {
                    println!("Logged in successfully.");
                    println!("Tokens saved to {}", report.saved);
                }
                print_login_report(&report, json)
            }
        }
        AuthCommands::Whoami => unreachable!("whoami runs through run_with_client"),
        AuthCommands::Signup => {
            let api_url = api_url.unwrap_or("https://api.clickhouse.cloud");
            let parsed = url::Url::parse(api_url).map_err(|error| {
                super::cloud_error_to_top_level(CloudError::new(format!("Invalid URL: {error}")))
            })?;
            let host = parsed.host_str().unwrap_or("api.clickhouse.cloud");
            let base_host = host.strip_prefix("api.").unwrap_or(host);
            let url = format!(
                "https://console.{}/signUp?utm_source=clickhousectl",
                base_host
            );
            if !json {
                println!("Opening ClickHouse Cloud sign-up page...");
            }
            let report = SignupReport {
                browser_opened: open::that(&url).is_ok(),
                url,
            };
            report.write(&mut crate::stdout::stdout(), json)
        }
        AuthCommands::Logout { oauth, api_keys } => {
            // No flag (or both) clears everything.
            let clear_oauth = oauth || !api_keys;
            let clear_api_keys = api_keys || !oauth;
            if clear_oauth {
                clear_tokens();
            }
            if clear_api_keys {
                credentials::clear_credentials().map_err(super::cloud_error_to_top_level)?;
            }
            LogoutReport::new(clear_oauth, clear_api_keys).write(&mut crate::stdout::stdout(), json)
        }
        AuthCommands::Status => {
            use tabled::{Table, settings::Style};

            let active = resolve_active_auth_source(api_key, api_secret);
            let now = chrono::Utc::now();

            let mut oauth_warning = None;
            let oauth = match load_tokens() {
                Ok(Some(tokens)) if is_token_valid(&tokens) => OAuthState::Valid(tokens.api_url),
                Ok(Some(tokens)) => OAuthState::Expired(tokens.api_url),
                Err(UnusableTokens) => {
                    oauth_warning = Some(OAUTH_LOGIN_REQUIRED);
                    OAuthState::LoginRequired
                }
                Ok(None) => OAuthState::NotConfigured,
            };

            // An unreadable or corrupt file is reported, not shown as absent:
            // Cloud commands fail on it rather than falling through (#1327).
            let mut credentials_warning = None;
            let saved = match credentials::try_load_credentials() {
                Ok(saved) => saved,
                Err(error) => {
                    credentials_warning = Some(error.message);
                    None
                }
            };
            // The saved key's grant from `cloud access request`: its roles and expiry.
            #[cfg(feature = "experimental-access-negotiation")]
            let saved_grant = saved.as_ref().and_then(crate::cloud::grant::grant_of);
            let saved_key = match saved.as_ref() {
                _ if credentials_warning.is_some() => SavedKeyState::Unreadable,
                Some(creds) if creds.api_credentials().is_some() => {
                    let grant_scope = None;
                    let expired = None;
                    #[cfg(feature = "experimental-access-negotiation")]
                    let (grant_scope, expired) = match saved_grant {
                        Some(grant) => (
                            Some(crate::cloud::grant::status_scope(grant, now)),
                            crate::cloud::grant::status_expired(grant, now),
                        ),
                        None => (grant_scope, expired),
                    };
                    SavedKeyState::Complete {
                        api_url: creds.api_base_url(),
                        grant_scope,
                        expired,
                    }
                }
                Some(creds) if creds.api_key.is_some() || creds.api_secret.is_some() => {
                    SavedKeyState::Incomplete
                }
                _ => SavedKeyState::NotConfigured,
            };

            if debug {
                match active {
                    Some(source) => {
                        eprintln!("[debug] auth source: {}", source.describe())
                    }
                    None if api_key.is_some() != api_secret.is_some() => {
                        let missing = if api_key.is_some() {
                            "--api-secret"
                        } else {
                            "--api-key"
                        };
                        eprintln!(
                            "[debug] auth source: none (incomplete CLI flags: missing {missing})"
                        );
                    }
                    None if credentials_warning.is_some() => {
                        eprintln!("[debug] auth source: none (saved API key file unreadable)");
                    }
                    None if oauth_warning.is_some() => {
                        eprintln!("[debug] auth source: none (saved OAuth login unusable)");
                    }
                    None => eprintln!("[debug] auth source: none (no credentials configured)"),
                }
            }

            // A saved key with a grant takes its Scope from the grant.
            #[cfg(feature = "experimental-access-negotiation")]
            let saved_has_grant = saved_grant.is_some();
            #[cfg(not(feature = "experimental-access-negotiation"))]
            let saved_has_grant = false;

            let mut key_roles = KeyRoles::Unknown;
            let identity = match active {
                Some(source) => IdentityCheck::from(
                    match CloudClient::new_with_timeout(
                        api_key,
                        api_secret,
                        api_url,
                        VERIFY_TIMEOUT,
                    ) {
                        Ok(client) => {
                            let verification = classify_whoami(client.get_whoami().await);
                            // Any other API key in use: read its roles, once verified.
                            let ungranted_key = source != AuthSource::OAuthTokens
                                && !(source == AuthSource::CredentialsFile && saved_has_grant);
                            if ungranted_key && let Verification::Verified(whoami) = &verification {
                                key_roles = lookup_key_roles(&client, whoami).await;
                            }
                            verification
                        }
                        Err(error) => classify_whoami(Err(error)),
                    },
                ),
                None => IdentityCheck::skipped(),
            };

            let env_creds = env_cred_presence();
            let facts = StatusFacts {
                active,
                verification: identity.verification,
                flags: (api_key.is_some(), api_secret.is_some()),
                saved_key,
                env: (env_creds.key, env_creds.secret),
                env_provenance: dotenv_env_provenance(),
                oauth,
                in_use_key_scope: key_roles.scope(now),
            };
            let rows = facts.rows();
            let summary = facts.summary();

            // Only an active OAuth login lacks write access it could obtain;
            // JSON names the roles of an API key in use.
            let write_access: Option<String> = match active {
                Some(AuthSource::OAuthTokens) => OAUTH_WRITE_ACCESS.map(str::to_string),
                Some(_) if json => Some(key_roles.write_access()),
                _ => None,
            };
            // JSON names what the saved `cloud access request` key may write.
            #[cfg(feature = "experimental-access-negotiation")]
            let write_access = match saved_grant
                .filter(|_| active == Some(AuthSource::CredentialsFile))
            {
                Some(grant) if json => Some(crate::cloud::grant::status_write_access(grant, now)),
                _ => write_access,
            };

            if json {
                #[derive(Serialize)]
                struct StatusOutput<'a> {
                    sources: Vec<AuthRow>,
                    summary: &'a str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    write_access: Option<&'a str>,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    oauth_warning: Option<&'a str>,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    credentials_warning: Option<&'a str>,
                    #[serde(flatten)]
                    identity: &'a IdentityCheck,
                }
                let output = StatusOutput {
                    sources: rows,
                    summary: &summary,
                    write_access: write_access.as_deref(),
                    oauth_warning,
                    credentials_warning: credentials_warning.as_deref(),
                    identity: &identity,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{}", Table::new(rows).with(Style::markdown()));
                println!();
                println!("{summary}");
                if let Some(write_access) = &write_access {
                    println!();
                    println!("Write access: {write_access}");
                }
                if let Some(warning) = oauth_warning {
                    println!();
                    println!("{warning}");
                }
                if let Some(warning) = &credentials_warning {
                    println!();
                    println!("{warning}");
                }
                match (&identity.identity, &identity.warning) {
                    (Some(whoami), _) => {
                        println!();
                        println!("Identity:");
                        print_human(whoami)?;
                    }
                    (None, Some(warning)) => {
                        println!();
                        println!("Identity: {} ({warning})", identity.verification);
                    }
                    (None, None) => {}
                }
            }
            Ok(())
        }
    }
}

/// One credential source in the `auth status` table, in precedence order.
#[derive(Debug, Serialize, tabled::Tabled)]
struct AuthRow {
    #[tabled(rename = "Type")]
    #[serde(rename = "type")]
    auth_type: String,
    /// Whether the credential exists and works.
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Scope")]
    scope: String,
    /// `yes` for the one credential Cloud commands use, else `-`. The JSON
    /// field keeps its `active` name so parsers do not break.
    #[tabled(rename = "In use")]
    active: String,
    /// The API a saved source is sent to (#1202); flags and env
    /// vars carry no URL of their own.
    #[tabled(rename = "API URL", display = "display_api_url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    api_url: Option<String>,
}

fn display_api_url(api_url: &Option<String>) -> String {
    api_url.clone().unwrap_or_else(|| "-".into())
}

impl AuthRow {
    fn new(
        auth_type: &str,
        status: impl Into<String>,
        scope: impl Into<String>,
        in_use: bool,
        api_url: Option<String>,
    ) -> Self {
        Self {
            auth_type: auth_type.into(),
            status: status.into(),
            scope: scope.into(),
            active: if in_use { "yes" } else { "-" }.into(),
            api_url,
        }
    }
}

/// The saved OAuth login, as `auth status` sees it locally.
#[derive(Debug)]
enum OAuthState {
    /// Unexpired; carries its API URL.
    Valid(String),
    Expired(String),
    LoginRequired,
    NotConfigured,
}

/// The saved API key (`.clickhouse/credentials.json`), as `auth status` sees it.
#[derive(Debug)]
enum SavedKeyState {
    Unreadable,
    Incomplete,
    NotConfigured,
    Complete {
        api_url: String,
        /// The Scope from its `cloud access request` grant; `None` without
        /// one, when the Scope is the key's looked-up role.
        grant_scope: Option<String>,
        /// The grant's expiry, `%Y-%m-%dT%H:%MZ`, once it is past.
        expired: Option<String>,
    },
}

/// Everything `auth status` reports, gathered before rendering.
#[derive(Debug)]
struct StatusFacts {
    active: Option<AuthSource>,
    /// `IdentityCheck::verification` of the credential in use.
    verification: &'static str,
    /// Whether `--api-key` / `--api-secret` were passed.
    flags: (bool, bool),
    saved_key: SavedKeyState,
    /// Whether the env-tier key / secret are set.
    env: (bool, bool),
    env_provenance: Option<PathBuf>,
    oauth: OAuthState,
    /// The Scope of the API key in use when it has no saved grant
    /// ([`KeyRoles::scope`]); a key not in use is never looked up.
    in_use_key_scope: String,
}

/// The Scope of an API key whose roles `auth status` could not read.
const ROLE_UNKNOWN: &str = "role unknown";

/// The roles of the API key in use, read from its management record (the
/// `cloud key get` call) when it did not come from `cloud access request`.
#[derive(Debug, Clone, PartialEq)]
enum KeyRoles {
    /// Not looked up, or the lookup was refused or failed.
    Unknown,
    Known {
        /// Role names, else ids, in the API's order; may be empty.
        roles: Vec<String>,
        expires: Option<chrono::DateTime<chrono::Utc>>,
    },
}

impl KeyRoles {
    fn of(key: &ApiKey) -> Self {
        let Some(assigned) = &key.assigned_roles else {
            return Self::Unknown;
        };
        let roles = assigned
            .iter()
            .filter_map(|role| {
                role.role_name
                    .clone()
                    .filter(|name| !name.trim().is_empty())
                    .or_else(|| role.role_id.map(|id| id.to_string()))
            })
            .collect();
        Self::Known {
            roles,
            expires: key.expire_at,
        }
    }

    /// The Scope cell: `<roles>[, expires <time>]`, `no roles`, or `role unknown`.
    fn scope(&self, now: chrono::DateTime<chrono::Utc>) -> String {
        match self {
            Self::Unknown => ROLE_UNKNOWN.into(),
            Self::Known { roles, .. } if roles.is_empty() => "no roles".into(),
            Self::Known { roles, expires } => {
                let mut scope = roles.join(", ");
                if let Some(expires) = expires {
                    scope.push_str(&format!(", {}", expiry_phrase(*expires, now)));
                }
                scope
            }
        }
    }

    /// JSON `write_access`: `API key: <roles>[; expires <RFC 3339>]`.
    fn write_access(&self) -> String {
        match self {
            Self::Unknown => format!("API key: {ROLE_UNKNOWN}"),
            Self::Known { roles, .. } if roles.is_empty() => "API key: no roles".into(),
            Self::Known { roles, expires } => {
                let mut text = format!("API key: {}", roles.join(", "));
                if let Some(expires) = expires {
                    text.push_str(&format!(
                        "; expires {}",
                        expires.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                    ));
                }
                text
            }
        }
    }
}

/// The roles of the verified API key `whoami` names, or `Unknown` when the
/// lookup is refused or fails. Never an error: many roles cannot read keys.
async fn lookup_key_roles(client: &CloudClient, whoami: &Whoami) -> KeyRoles {
    let Whoami::WhoamiApiKey(key) = whoami else {
        return KeyRoles::Unknown;
    };
    let (Some(org_id), Some(key_id)) = (key.organization_id, key.key_id.as_deref()) else {
        return KeyRoles::Unknown;
    };
    match client.get_api_key(&org_id.to_string(), key_id).await {
        Ok(record) => KeyRoles::of(&record),
        Err(_) => KeyRoles::Unknown,
    }
}

/// `expires 09:56Z`, with the date (`2026-11-20T09:56Z`) when it is not today
/// in UTC, and `expired` once past.
pub fn expiry_phrase(
    expires: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let verb = if expires <= now { "expired" } else { "expires" };
    let time = if expires.date_naive() == now.date_naive() {
        expires.format("%H:%MZ").to_string()
    } else {
        expires.format("%Y-%m-%dT%H:%MZ").to_string()
    };
    format!("{verb} {time}")
}

#[cfg(feature = "experimental-access-negotiation")]
const OAUTH_IN_USE_SUMMARY: &str =
    "Commands here use the read-only OAuth login. For writes: `cloud access request`.";
#[cfg(not(feature = "experimental-access-negotiation"))]
const OAUTH_IN_USE_SUMMARY: &str = "Commands here use the read-only OAuth login.";

/// The second summary line when the saved key fails but OAuth is valid.
#[cfg(feature = "experimental-access-negotiation")]
const OAUTH_REPLACES_KEY: Option<&str> =
    Some("The OAuth login is valid: `cloud access request` works and replaces the key.");
#[cfg(not(feature = "experimental-access-negotiation"))]
const OAUTH_REPLACES_KEY: Option<&str> = None;

const NO_VALID_OAUTH: &str = "No valid OAuth login: `cloud auth login` (needs the user).";

impl StatusFacts {
    fn in_use(&self, source: AuthSource) -> bool {
        self.active == Some(source)
    }

    /// The Status of the credential in use, from the whoami check.
    fn in_use_status(&self) -> &'static str {
        match self.verification {
            "verified" => "Valid",
            "rejected" => "Rejected by the API",
            _ => "Configured (not verified)",
        }
    }

    /// The higher-priority source that keeps a configured `source` out of
    /// use: complete or incomplete flags, or a saved key file that is usable
    /// or unreadable (both block fallback).
    fn outranked_by(&self, source: AuthSource) -> Option<&'static str> {
        let flags = self.flags.0 || self.flags.1;
        if source != AuthSource::CliFlags && flags {
            return Some("CLI flags");
        }
        let saved = matches!(
            self.saved_key,
            SavedKeyState::Unreadable | SavedKeyState::Complete { .. }
        );
        (source == AuthSource::EnvVars && saved).then_some("saved API key")
    }

    /// The Scope of a configured API key source without a saved grant: the
    /// looked-up role of the key in use, else unknown.
    fn key_scope(&self, source: AuthSource) -> String {
        if self.in_use(source) {
            self.in_use_key_scope.clone()
        } else {
            ROLE_UNKNOWN.into()
        }
    }

    /// The Status of a configured API key source.
    fn key_status(&self, source: AuthSource, provenance: &str) -> String {
        if self.in_use(source) {
            return format!("{}{provenance}", self.in_use_status());
        }
        match self.outranked_by(source) {
            Some(by) => format!("Configured{provenance}, not in use (outranked by {by})"),
            None => format!("Configured{provenance}, not in use"),
        }
    }

    /// One row per source, in precedence order.
    fn rows(&self) -> Vec<AuthRow> {
        use AuthSource::*;
        let flags = match self.flags {
            (true, true) => AuthRow::new(
                "CLI flags",
                self.key_status(CliFlags, ""),
                self.key_scope(CliFlags),
                self.in_use(CliFlags),
                None,
            ),
            (true, false) => AuthRow::new(
                "CLI flags",
                "Incomplete (missing --api-secret)",
                "-",
                false,
                None,
            ),
            (false, true) => AuthRow::new(
                "CLI flags",
                "Incomplete (missing --api-key)",
                "-",
                false,
                None,
            ),
            (false, false) => AuthRow::new("CLI flags", "Not configured", "-", false, None),
        };
        let saved = match &self.saved_key {
            SavedKeyState::Unreadable => AuthRow::new("API key", "Unreadable", "-", false, None),
            SavedKeyState::Incomplete => AuthRow::new(
                "API key",
                "Incomplete (missing or empty API key/secret)",
                "-",
                false,
                None,
            ),
            SavedKeyState::NotConfigured => {
                AuthRow::new("API key", "Not configured", "-", false, None)
            }
            SavedKeyState::Complete {
                api_url,
                grant_scope,
                expired,
            } => {
                let status = match expired {
                    Some(time) => format!("Expired {time}"),
                    None => self.key_status(CredentialsFile, ""),
                };
                AuthRow::new(
                    "API key",
                    status,
                    grant_scope
                        .clone()
                        .unwrap_or_else(|| self.key_scope(CredentialsFile)),
                    self.in_use(CredentialsFile),
                    Some(api_url.clone()),
                )
            }
        };
        let env = match self.env {
            (true, true) => {
                let provenance = self
                    .env_provenance
                    .as_ref()
                    .map(|path| format!(" (from {})", path.display()))
                    .unwrap_or_default();
                AuthRow::new(
                    "Env vars",
                    self.key_status(EnvVars, &provenance),
                    self.key_scope(EnvVars),
                    self.in_use(EnvVars),
                    None,
                )
            }
            (true, false) => AuthRow::new(
                "Env vars",
                "Incomplete (missing CLICKHOUSE_CLOUD_API_SECRET)",
                "-",
                false,
                None,
            ),
            (false, true) => AuthRow::new(
                "Env vars",
                "Incomplete (missing CLICKHOUSE_CLOUD_API_KEY)",
                "-",
                false,
                None,
            ),
            (false, false) => AuthRow::new("Env vars", "Not configured", "-", false, None),
        };
        let oauth = match &self.oauth {
            OAuthState::Valid(api_url) => {
                let in_use = self.in_use(OAuthTokens);
                let status = if in_use && self.verification == "rejected" {
                    "Rejected by the API"
                } else {
                    "Valid"
                };
                AuthRow::new("OAuth", status, "read-only", in_use, Some(api_url.clone()))
            }
            OAuthState::Expired(api_url) => AuthRow::new(
                "OAuth",
                "Expired",
                "read-only",
                false,
                Some(api_url.clone()),
            ),
            OAuthState::LoginRequired => AuthRow::new("OAuth", "Login required", "-", false, None),
            OAuthState::NotConfigured => AuthRow::new("OAuth", "Not configured", "-", false, None),
        };
        vec![flags, saved, env, oauth]
    }

    /// Which credential commands here use and whether they work; one or two
    /// lines joined by `\n`.
    fn summary(&self) -> String {
        let failure = match (self.active, &self.saved_key) {
            (None, _) => return "Not logged in: `cloud auth login` (needs the user).".into(),
            (Some(AuthSource::CliFlags), _) => {
                return "Commands here use the API key from CLI flags.".into();
            }
            (Some(AuthSource::EnvVars), _) => {
                return "Commands here use the API key from environment variables.".into();
            }
            (Some(AuthSource::OAuthTokens), _) => return OAUTH_IN_USE_SUMMARY.into(),
            (
                Some(AuthSource::CredentialsFile),
                SavedKeyState::Complete {
                    expired: Some(_), ..
                },
            ) => "which has expired",
            (Some(AuthSource::CredentialsFile), _) if self.verification == "rejected" => {
                "which the API refused"
            }
            (Some(AuthSource::CredentialsFile), _) => {
                return "Commands here use the saved API key.".into();
            }
        };
        let mut lines = vec![format!(
            "Commands here use the saved API key, {failure}, so they fail."
        )];
        match (&self.oauth, OAUTH_REPLACES_KEY) {
            (OAuthState::Valid(_), Some(line)) => lines.push(line.into()),
            (OAuthState::Valid(_), None) => {}
            _ => lines.push(NO_VALID_OAUTH.into()),
        }
        lines.join("\n")
    }
}

/// How long `auth login`, `auth status` and `feedback` wait for whoami, so
/// working offline never hangs on the identity check.
pub(crate) const VERIFY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// The result of asking whoami about a credential set.
enum Verification {
    Verified(Whoami),
    /// The API refused the credentials (401/403).
    Rejected(super::CloudError),
    /// Anything else: offline, timeout, 5xx, or an unbuildable client.
    Unavailable(super::CloudError),
}

async fn verify_identity(client: CloudResult<CloudClient>) -> Verification {
    match client {
        Ok(client) => classify_whoami(client.get_whoami().await),
        Err(error) => classify_whoami(Err(error)),
    }
}

fn classify_whoami(result: CloudResult<Whoami>) -> Verification {
    match result {
        Ok(identity) => Verification::Verified(identity),
        Err(error) if error.kind == super::CloudErrorKind::Auth => Verification::Rejected(error),
        Err(error) => Verification::Unavailable(error),
    }
}

/// The identity part of `auth status`; also flattened into its JSON output.
#[derive(Serialize)]
struct IdentityCheck {
    identity: Option<Whoami>,
    /// `verified`, `rejected`, `unavailable`, or `skipped` (no active credentials).
    verification: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

impl IdentityCheck {
    fn skipped() -> Self {
        Self {
            identity: None,
            verification: "skipped",
            warning: None,
        }
    }
}

impl From<Verification> for IdentityCheck {
    fn from(verification: Verification) -> Self {
        match verification {
            Verification::Verified(identity) => Self {
                identity: Some(identity),
                verification: "verified",
                warning: None,
            },
            Verification::Rejected(error) => Self {
                identity: None,
                verification: "rejected",
                warning: Some(error.message),
            },
            Verification::Unavailable(error) => Self {
                identity: None,
                verification: "unavailable",
                warning: Some(error.message),
            },
        }
    }
}

/// What `auth signup` reports: the page it opened, or tried to.
#[derive(Debug, Serialize)]
struct SignupReport {
    url: String,
    browser_opened: bool,
}

impl SignupReport {
    fn write(&self, output: &mut dyn std::io::Write, json: bool) -> crate::error::Result<()> {
        if json {
            writeln!(output, "{}", serde_json::to_string_pretty(self)?)?;
        } else if !self.browser_opened {
            writeln!(output, "Could not open browser. Please visit: {}", self.url)?;
        }
        Ok(())
    }
}

/// What a saved credential source looks like after `auth logout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LogoutAction {
    Cleared,
    Unchanged,
}

impl LogoutAction {
    fn from_cleared(cleared: bool) -> Self {
        if cleared {
            Self::Cleared
        } else {
            Self::Unchanged
        }
    }
}

/// What `auth logout` did to each saved credential source.
#[derive(Debug, Serialize)]
struct LogoutReport {
    oauth_tokens: LogoutAction,
    api_keys: LogoutAction,
}

impl LogoutReport {
    fn new(oauth_tokens: bool, api_keys: bool) -> Self {
        Self {
            oauth_tokens: LogoutAction::from_cleared(oauth_tokens),
            api_keys: LogoutAction::from_cleared(api_keys),
        }
    }

    fn write(&self, output: &mut dyn std::io::Write, json: bool) -> crate::error::Result<()> {
        if json {
            writeln!(output, "{}", serde_json::to_string_pretty(self)?)?;
            return Ok(());
        }
        let line = match (self.oauth_tokens, self.api_keys) {
            (LogoutAction::Cleared, LogoutAction::Unchanged) => {
                "OAuth tokens cleared. API keys unchanged."
            }
            (LogoutAction::Unchanged, LogoutAction::Cleared) => {
                "API keys cleared. OAuth tokens unchanged."
            }
            _ => "Logged out. All saved credentials cleared.",
        };
        writeln!(output, "{line}")?;
        Ok(())
    }
}

/// What `auth login` reports once credentials are saved.
#[derive(Serialize)]
struct LoginReport {
    saved: String,
    identity: Option<Whoami>,
    /// `verified` or `unverified`.
    verification: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

impl LoginReport {
    fn new(saved: String, verification: Verification, unverified: &str) -> Self {
        match verification {
            Verification::Verified(identity) => Self {
                saved,
                identity: Some(identity),
                verification: "verified",
                warning: None,
            },
            Verification::Rejected(error) | Verification::Unavailable(error) => Self {
                saved,
                identity: None,
                verification: "unverified",
                warning: Some(format!("{unverified}: {}", error.message)),
            },
        }
    }
}

/// Verify an API key pair with whoami, then save it. Only a clear auth
/// rejection blocks the save; any other failure saves with a warning, so
/// credentials can still be set up offline.
async fn login_with_api_key(
    key: &str,
    secret: &str,
    api_url: Option<&str>,
    json: bool,
) -> crate::error::Result<()> {
    let verification = verify_identity(CloudClient::for_api_key(
        key,
        secret,
        api_url,
        VERIFY_TIMEOUT,
    ))
    .await;
    if let Verification::Rejected(error) = verification {
        return Err(Error::AuthRequired(format!(
            "{}\n\nCredentials were not saved.",
            error.message
        )));
    }
    let api_url = api_url
        .map(normalize_api_url)
        .unwrap_or_else(|| DEFAULT_API_URL.to_string());
    credentials::set_api_credentials(key.to_owned(), secret.to_owned(), &api_url)
        .map_err(super::cloud_error_to_top_level)?;
    let saved = credentials::credentials_path()?.display().to_string();
    let report = LoginReport::new(saved, verification, "could not verify the credentials");
    if !json {
        println!("Credentials saved to {}", report.saved);
    }
    print_login_report(&report, json)
}

/// Report who just logged in with OAuth. The device flow already proved the
/// token works, so a whoami failure only warns.
async fn oauth_login_report(tokens: &TokenStore, saved: String) -> LoginReport {
    let verification = verify_identity(CloudClient::for_oauth_tokens(tokens, VERIFY_TIMEOUT)).await;
    LoginReport::new(saved, verification, "could not fetch your identity")
}

fn print_login_report(report: &LoginReport, json: bool) -> crate::error::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    if let Some(identity) = &report.identity {
        println!("Logged in as:");
        print_human(identity)?;
    }
    if let Some(warning) = &report.warning {
        eprintln!("warning: {warning}");
    }
    Ok(())
}

fn prompt_api_credentials() -> CloudResult<(String, String)> {
    use std::io::Write;

    print!("API Key: ");
    crate::stdout::stdout().flush()?;
    let mut api_key = String::new();
    std::io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        return Err(CloudError::new("API key cannot be empty"));
    }

    print!("API Secret: ");
    crate::stdout::stdout().flush()?;
    let api_secret = rpassword::read_password()?;

    if api_secret.is_empty() {
        return Err(CloudError::new("API secret cannot be empty"));
    }

    Ok((api_key, api_secret))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuthConfig {
    #[serde(deserialize_with = "deserialize_auth_url")]
    auth_url: String,
    client_id: String,
}

const INVALID_AUTH_URL: &str = "Authorization URL must be an absolute HTTPS URL (HTTP is allowed only for localhost or a loopback IP), without userinfo, query, or fragment";

/// Validate before login or token loading, so refresh only uses stored config.
fn validated_auth_url(value: &str) -> Result<String, &'static str> {
    let parsed = url::Url::parse(value).map_err(|_| INVALID_AUTH_URL)?;
    let loopback = match parsed.host() {
        Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => return Err(INVALID_AUTH_URL),
    };
    if !(parsed.scheme() == "https" || (parsed.scheme() == "http" && loopback))
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(INVALID_AUTH_URL);
    }
    Ok(parsed.as_str().trim_end_matches('/').to_owned())
}

fn deserialize_auth_url<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validated_auth_url(&value).map_err(serde::de::Error::custom)
}

/// Bounds each OAuth request, body included. A token refresh holds the token
/// lock across its request, so a stalled issuer must not block every other
/// process waiting on that lock (#1444).
const OAUTH_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// Bounds establishing the connection, within [`OAUTH_REQUEST_TIMEOUT`].
const OAUTH_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// OAuth form bodies contain credentials; never forward them via redirects.
fn oauth_client() -> CloudResult<reqwest::Client> {
    oauth_client_with_timeout(crate::timing::duration_override(
        crate::timing::OAUTH_TIMEOUT_ENV,
        OAUTH_REQUEST_TIMEOUT,
    ))
}

fn oauth_client_with_timeout(timeout: std::time::Duration) -> CloudResult<reqwest::Client> {
    crate::http::clickhouse_client_builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(OAUTH_CONNECT_TIMEOUT.min(timeout))
        .timeout(timeout)
        .build()
        .map_err(oauth_http_error)
}

/// An OAuth request got no usable HTTP response. Classified from the reqwest
/// error's own kind, as the Cloud API boundary classifies its transport errors.
fn oauth_http_error(error: reqwest::Error) -> CloudError {
    let kind = if error.is_timeout() {
        FailureKind::Timeout
    } else {
        FailureKind::Transport
    };
    CloudError::new(error.to_string()).with_failure(ApiFailure::new(kind))
}

/// The OAuth server answered with a status it should not have.
fn oauth_status_error(message: String, status: reqwest::StatusCode) -> CloudError {
    CloudError::new(message).with_failure(ApiFailure::with_status(
        crate::failure::status_kind(status.as_u16()),
        status.as_u16(),
    ))
}

struct KnownAuthConfig {
    host: &'static str,
    auth_url: &'static str,
    client_id: &'static str,
}

impl From<&KnownAuthConfig> for AuthConfig {
    fn from(config: &KnownAuthConfig) -> Self {
        Self {
            auth_url: config.auth_url.to_owned(),
            client_id: config.client_id.to_owned(),
        }
    }
}

/// Known API host → authorization configuration mappings.
///
/// Any host not listed here requires `CLICKHOUSECTL_AUTH_URL` and
/// `CLICKHOUSECTL_CLIENT_ID` to be set; there is no built-in fallback.
const KNOWN_CONFIGS: &[KnownAuthConfig] = &[
    KnownAuthConfig {
        host: "api.clickhouse.cloud",
        auth_url: "https://auth.clickhouse.cloud",
        client_id: "9q6XAueAs47R4X5d1d6FbjbJqjsrA2ZJ",
    },
    KnownAuthConfig {
        host: "api.control-plane.clickhouse-staging.com",
        auth_url: "https://auth.control-plane.clickhouse-staging.com",
        client_id: "ZC8AupPshQt2UNO2hEDutnKitx4PhizY",
    },
    KnownAuthConfig {
        host: "api.control-plane.clickhouse-dev.com",
        auth_url: "https://auth.control-plane.clickhouse-dev.com",
        client_id: "bVVcrqNw1t5dya9WFzfnM7PSsAgmfzwY",
    },
];

/// Look up bundled configuration without consulting the environment.
fn known_auth_config_for_url(api_url: &str) -> Option<AuthConfig> {
    let parsed = url::Url::parse(api_url).ok()?;
    let host = parsed.host_str()?;

    KNOWN_CONFIGS
        .iter()
        .find(|config| host == config.host)
        .map(AuthConfig::from)
}

/// Injectable environment lookup keeps login tests independent of the shell.
type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Why a login could not resolve OAuth configuration for an API URL.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthConfigError {
    /// The API URL is malformed or has no host.
    InvalidApiUrl,
    /// Unknown host: the environment override is unset or empty. The payload
    /// names the missing variable(s).
    MissingEnvVars(&'static str),
    /// `CLICKHOUSECTL_AUTH_URL` was set but rejected by `validated_auth_url`;
    /// the payload is the validation reason.
    InvalidAuthUrl(&'static str),
}

impl std::fmt::Display for AuthConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidApiUrl => write!(f, "the URL is malformed or has no host"),
            Self::MissingEnvVars(missing) => {
                let known = KNOWN_CONFIGS
                    .iter()
                    .map(|config| config.host)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "known hosts: {known}; for any other host, set {missing}")
            }
            Self::InvalidAuthUrl(reason) => {
                write!(f, "CLICKHOUSECTL_AUTH_URL is invalid: {reason}")
            }
        }
    }
}

/// Resolve configuration for a new login, never for an existing session.
fn auth_config_for_url(api_url: &str) -> Result<AuthConfig, AuthConfigError> {
    auth_config_for_url_with(api_url, &|key| std::env::var(key).ok())
}

fn auth_config_for_url_with(
    api_url: &str,
    env_lookup: EnvLookup<'_>,
) -> Result<AuthConfig, AuthConfigError> {
    // Preserve rejection of malformed or hostless API URLs before reading env.
    let parsed = url::Url::parse(api_url).map_err(|_| AuthConfigError::InvalidApiUrl)?;
    if parsed.host_str().is_none() {
        return Err(AuthConfigError::InvalidApiUrl);
    }
    if let Some(config) = known_auth_config_for_url(api_url) {
        return Ok(config);
    }

    // Unknown host: require both env vars, with no built-in fallback.
    let env = |key: &str| env_lookup(key).filter(|v| !v.is_empty());
    match (
        env("CLICKHOUSECTL_AUTH_URL"),
        env("CLICKHOUSECTL_CLIENT_ID"),
    ) {
        (Some(auth_url), Some(client_id)) => {
            let auth_url =
                validated_auth_url(&auth_url).map_err(AuthConfigError::InvalidAuthUrl)?;
            Ok(AuthConfig {
                auth_url,
                client_id,
            })
        }
        (None, Some(_)) => Err(AuthConfigError::MissingEnvVars("CLICKHOUSECTL_AUTH_URL")),
        (Some(_), None) => Err(AuthConfigError::MissingEnvVars("CLICKHOUSECTL_CLIENT_ID")),
        (None, None) => Err(AuthConfigError::MissingEnvVars(
            "CLICKHOUSECTL_AUTH_URL and CLICKHOUSECTL_CLIENT_ID",
        )),
    }
}

/// Normalize a user-provided URL into the API base URL we store and use:
/// trailing slashes are trimmed and a `/v1` path suffix is added if missing.
/// The scheme is left as given.
pub fn normalize_api_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if url.ends_with("/v1") {
        url.to_string()
    } else {
        format!("{url}/v1")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStore {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    /// The API base URL these tokens were issued for (e.g. "https://api.clickhouse.cloud/v1").
    pub api_url: String,
    /// Complete authorization configuration resolved at login, never at refresh.
    auth_config: AuthConfig,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// Global OAuth token store: `~/.clickhouse/tokens.json`.
///
/// OAuth login is user identity (not org-scoped like API keys), so tokens live
/// in the CLI's global home alongside installed versions and configs, rather
/// than per-project under `./.clickhouse/`.
pub fn tokens_path() -> CloudResult<PathBuf> {
    let base = crate::paths::base_dir().map_err(|error| match error {
        Error::Io(error) => CloudError::from(error),
        error => CloudError::new(error.to_string()),
    })?;
    Ok(base.join("tokens.json"))
}

/// A saved login could not be read or decoded. Never reconstruct its issuer
/// from the environment or built-in configuration; a new login is required.
#[derive(Debug)]
pub struct UnusableTokens;

pub(super) const OAUTH_LOGIN_REQUIRED: &str =
    "Saved OAuth login can't be used. Run `clickhousectl cloud auth login` to log in again.";

/// A saved login is readable but expired, and refreshing it was not possible
/// (no refresh token, or the refresh failed). The file is kept.
pub(super) const OAUTH_LOGIN_EXPIRED: &str = "Saved OAuth login has expired and could not be refreshed. Run `clickhousectl cloud auth login` to log in again.";

/// The saved OAuth login. Reading never writes or removes a token file.
pub fn load_tokens() -> Result<Option<TokenStore>, UnusableTokens> {
    read_tokens(&tokens_path().map_err(|_| UnusableTokens)?)
}

fn read_tokens(path: &std::path::Path) -> Result<Option<TokenStore>, UnusableTokens> {
    match std::fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data)
            .map(Some)
            .map_err(|_| UnusableTokens),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(UnusableTokens),
    }
}

pub fn save_tokens(tokens: &TokenStore) -> CloudResult<()> {
    let path = tokens_path()?;
    let _lock = lock_tokens(&path)?;
    save_tokens_to(&path, tokens)
}

/// Write the token file atomically and owner-only. Callers hold the
/// [`lock_tokens`] lock.
fn save_tokens_to(path: &std::path::Path, tokens: &TokenStore) -> CloudResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(tokens)?;
    crate::paths::atomic_write(path, json.as_bytes(), 0o600)?;
    Ok(())
}

/// Serializes token-file mutations across processes, so parallel commands
/// refresh a near-expiry login once instead of racing on the refresh token.
/// The lock is held by the open handle and released when it is dropped; it
/// lives beside the token file as `tokens.lock`. Not reentrant: a holder
/// writes with [`save_tokens_to`], never [`save_tokens`].
fn lock_tokens(tokens: &std::path::Path) -> std::io::Result<std::fs::File> {
    if let Some(parent) = tokens.parent() {
        std::fs::create_dir_all(parent)?;
    }
    credentials::open_lock_file(&tokens.with_extension("lock"))
}

pub fn clear_tokens() {
    if let Ok(path) = tokens_path() {
        // Logout removes the file even if the lock cannot be taken.
        let _lock = lock_tokens(&path);
        let _ = std::fs::remove_file(path);
    }
}

pub fn is_token_valid(tokens: &TokenStore) -> bool {
    let now = chrono::Utc::now().timestamp();
    tokens.expires_at > now + 60
}

/// Run the OAuth device flow. The code and URL are instructions for a human,
/// so in JSON mode they go to stderr and stdout keeps only the login report.
pub async fn device_auth_login(api_url: &str, json: bool) -> CloudResult<TokenStore> {
    let api_url = normalize_api_url(api_url);
    let config = auth_config_for_url(&api_url).map_err(|error| {
        CloudError::new(format!(
            "Cannot resolve OAuth configuration for '{api_url}': {error}"
        ))
    })?;

    let client = oauth_client()?;

    // Step 1: Request device code
    let form_body = format!(
        "client_id={}&scope={}&audience={}",
        urlencoding::encode(&config.client_id),
        urlencoding::encode(SCOPE),
        urlencoding::encode(AUDIENCE),
    );
    let resp = client
        .post(format!("{}/oauth/device/code", config.auth_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await
        .map_err(oauth_http_error)?;

    let status = resp.status();
    let resp_body = resp.bytes().await.map_err(oauth_http_error)?;

    if !status.is_success() {
        return Err(oauth_status_error(
            format!(
                "Failed to start device authorization: {}",
                String::from_utf8_lossy(&resp_body)
            ),
            status,
        ));
    }

    let device_resp: DeviceCodeResponse = serde_json::from_slice(&resp_body)?;

    // Step 2: Display instructions
    let verification_url = device_resp
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&device_resp.verification_uri);

    let say = |line: &str| {
        if json {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    };
    say("Login to ClickHouse Cloud:");
    say("");
    say(&format!("  Code: {}", device_resp.user_code));
    say(&format!("  URL:  {verification_url}"));
    say("");

    // Best-effort browser open
    if open::that(verification_url).is_ok() {
        say("Browser opened. Waiting for authentication...");
    } else {
        say("Open the URL above in your browser. Waiting for authentication...");
    }

    // Step 3: Poll for token
    let mut interval = device_resp.interval;
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(device_resp.expires_in);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        if std::time::Instant::now() > deadline {
            return Err(CloudError::new("Device authorization timed out"));
        }

        let poll_body = format!(
            "grant_type={}&device_code={}&client_id={}",
            urlencoding::encode("urn:ietf:params:oauth:grant-type:device_code"),
            urlencoding::encode(&device_resp.device_code),
            urlencoding::encode(&config.client_id),
        );
        let resp = client
            .post(format!("{}/oauth/token", config.auth_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(poll_body)
            .send()
            .await
            .map_err(oauth_http_error)?;

        let status = resp.status();
        let body_bytes = resp.bytes().await.map_err(oauth_http_error)?;
        let body = String::from_utf8_lossy(&body_bytes);

        if status.is_success() {
            let token_resp: TokenResponse = serde_json::from_str(&body)?;
            let now = chrono::Utc::now().timestamp();
            let tokens = TokenStore {
                access_token: token_resp.access_token,
                refresh_token: token_resp.refresh_token.unwrap_or_default(),
                expires_at: now + token_resp.expires_in,
                api_url: api_url.clone(),
                auth_config: config,
            };
            return Ok(tokens);
        }

        let error_resp: TokenErrorResponse = serde_json::from_str(&body)?;
        match error_resp.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += 5;
                continue;
            }
            "expired_token" => {
                return Err(CloudError::new("Device code expired. Please try again."));
            }
            "access_denied" => return Err(CloudError::new("Authorization denied by user.")),
            _ => {
                return Err(CloudError::new(format!(
                    "Authorization failed: {} ({})",
                    error_resp.error,
                    error_resp.error_description.unwrap_or_default()
                )));
            }
        }
    }
}

pub async fn refresh_access_token(tokens: &TokenStore) -> CloudResult<TokenStore> {
    refresh_access_token_with(&oauth_client()?, tokens).await
}

async fn refresh_access_token_with(
    client: &reqwest::Client,
    tokens: &TokenStore,
) -> CloudResult<TokenStore> {
    let config = &tokens.auth_config;

    let form_body = format!(
        "grant_type={}&client_id={}&refresh_token={}",
        urlencoding::encode("refresh_token"),
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&tokens.refresh_token),
    );
    let resp = client
        .post(format!("{}/oauth/token", config.auth_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await
        .map_err(oauth_http_error)?;

    let status = resp.status();
    let resp_body = resp.bytes().await.map_err(oauth_http_error)?;

    if !status.is_success() {
        return Err(oauth_status_error(
            format!(
                "Token refresh failed: {}",
                String::from_utf8_lossy(&resp_body)
            ),
            status,
        ));
    }

    let token_resp: TokenResponse = serde_json::from_slice(&resp_body)?;
    let now = chrono::Utc::now().timestamp();

    Ok(TokenStore {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .unwrap_or_else(|| tokens.refresh_token.clone()),
        expires_at: now + token_resp.expires_in,
        api_url: tokens.api_url.clone(),
        auth_config: config.clone(),
    })
}

/// If tokens exist and are near-expiry, refresh them. Returns Ok(()) even if
/// no tokens are present (the user may be using API keys instead).
///
/// Never removes the saved login (#1224): a refresh can fail for a transient
/// reason, and an expired file keeps `auth status` showing `Expired` and lets
/// the credential resolver report [`OAUTH_LOGIN_EXPIRED`] instead of "no
/// credentials". Only `auth logout` and a new login replace the file.
pub async fn ensure_fresh_tokens(json: bool) -> CloudResult<()> {
    // Leave unusable files intact. The credential resolver reports the re-login
    // error only if no higher-precedence credential source wins.
    let Ok(Some(tokens)) = load_tokens() else {
        return Ok(());
    };
    if is_token_valid(&tokens) || tokens.refresh_token.is_empty() {
        return Ok(());
    }
    refresh_saved_tokens(&tokens_path()?, json).await
}

/// Refresh the login at `path` under [`lock_tokens`]. The file is re-read once
/// the lock is held: another process may have refreshed it while this one
/// waited, and its refresh token may already be spent.
async fn refresh_saved_tokens(path: &std::path::Path, json: bool) -> CloudResult<()> {
    let lock_path = path.to_path_buf();
    // Waiting can last as long as another process's refresh request.
    let _lock = tokio::task::spawn_blocking(move || lock_tokens(&lock_path))
        .await
        .map_err(|error| {
            CloudError::new(format!("failed to wait for the token lock: {error}"))
        })??;

    let Ok(Some(tokens)) = read_tokens(path) else {
        return Ok(());
    };
    if is_token_valid(&tokens) || tokens.refresh_token.is_empty() {
        return Ok(());
    }

    match refresh_access_token(&tokens).await {
        Ok(new_tokens) => {
            save_tokens_to(path, &new_tokens)?;
        }
        // JSON mode keeps stderr to the single error envelope; the resolver's
        // expired-login error carries the action in both modes.
        Err(_) if !json => {
            eprintln!("Warning: OAuth token refresh failed. The saved login was kept.");
        }
        Err(_) => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[derive(Parser)]
    struct AuthCli {
        #[command(subcommand)]
        command: AuthCommands,
    }

    #[test]
    fn interactive_login_is_a_usage_error_with_credentials_or_without_a_tty() {
        use clap::error::ErrorKind;
        let login = |args: &[&str]| {
            let mut argv = vec!["clickhousectl", "login"];
            argv.extend(args);
            AuthCli::try_parse_from(argv).unwrap().command
        };
        for has_terminal in [true, false] {
            for flags in [
                &["--interactive", "--api-key", "k"][..],
                &["--interactive", "--api-secret", "s"],
                &["--interactive", "--api-key", "k", "--api-secret", "s"],
            ] {
                let (kind, _) = login(flags)
                    .login_validation_error(has_terminal)
                    .expect("credentials conflict with --interactive");
                assert_eq!(kind, ErrorKind::ArgumentConflict, "{flags:?}");
            }
        }
        let (kind, _) = login(&["--interactive"])
            .login_validation_error(false)
            .expect("no TTY to prompt on");
        assert_eq!(kind, ErrorKind::MissingRequiredArgument);
        assert!(
            login(&["--interactive"])
                .login_validation_error(true)
                .is_none()
        );
    }

    #[test]
    fn parses_auth_login_and_classifies_every_command_as_runtime_read_only() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "auth",
            "login",
            "--api-key",
            "key",
            "--api-secret",
            "secret",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        assert_eq!(args.api_key.as_deref(), Some("key"));
        assert_eq!(args.api_secret.as_deref(), Some("secret"));
        assert!(!args.json);
        assert!(!args.debug);
        assert!(args.url.is_none());
        let crate::cloud::cli::CloudCommands::Auth { command } = args.command else {
            panic!("expected auth command");
        };
        let crate::cloud::cli::AuthCommands::Login {
            interactive,
            api_key,
            api_secret,
        } = command
        else {
            panic!("expected login");
        };
        assert!(!interactive);
        assert_eq!(api_key.as_deref(), Some("key"));
        assert_eq!(api_secret.as_deref(), Some("secret"));

        let commands = [
            AuthCli::try_parse_from(["clickhousectl", "login"])
                .unwrap()
                .command,
            AuthCli::try_parse_from(["clickhousectl", "logout"])
                .unwrap()
                .command,
            AuthCli::try_parse_from(["clickhousectl", "status"])
                .unwrap()
                .command,
            AuthCli::try_parse_from(["clickhousectl", "signup"])
                .unwrap()
                .command,
            AuthCli::try_parse_from(["clickhousectl", "whoami"])
                .unwrap()
                .command,
        ];
        assert!(commands.iter().all(|command| !command.is_write()));
        // Only whoami calls the Cloud API; the rest manage local state.
        let remote: Vec<_> = commands
            .iter()
            .map(|command| matches!(command, AuthCommands::Whoami))
            .collect();
        assert_eq!(
            commands
                .iter()
                .map(AuthCommands::needs_client)
                .collect::<Vec<_>>(),
            remote
        );
        assert_eq!(remote.iter().filter(|remote| **remote).count(), 1);
    }

    #[test]
    fn auth_whoami_takes_shared_credentials_and_rejects_arguments() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "auth",
            "whoami",
            "--api-key",
            "key",
            "--api-secret",
            "secret",
            "--json",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        assert_eq!(args.api_key.as_deref(), Some("key"));
        assert_eq!(args.api_secret.as_deref(), Some("secret"));
        assert!(args.json);
        assert!(!args.command.is_write_command());
        assert!(matches!(
            args.command,
            crate::cloud::cli::CloudCommands::Auth {
                command: AuthCommands::Whoami
            }
        ));
    }

    #[test]
    fn auth_status_receives_credentials_from_every_command_level() {
        for args in [
            vec![
                "cloud",
                "--api-key",
                "key",
                "--api-secret",
                "secret",
                "auth",
                "status",
            ],
            vec![
                "cloud",
                "auth",
                "--api-key",
                "key",
                "--api-secret",
                "secret",
                "status",
            ],
            vec![
                "cloud",
                "auth",
                "status",
                "--api-key",
                "key",
                "--api-secret",
                "secret",
            ],
        ] {
            let cli = Cli::try_parse_from(std::iter::once("clickhousectl").chain(args)).unwrap();
            let Commands::Cloud(args) = cli.command else {
                panic!("expected cloud command");
            };
            assert_eq!(args.api_key.as_deref(), Some("key"));
            assert_eq!(args.api_secret.as_deref(), Some("secret"));
            assert!(matches!(
                args.command,
                crate::cloud::cli::CloudCommands::Auth {
                    command: AuthCommands::Status
                }
            ));
        }
    }

    #[test]
    fn token_deserialization_requires_complete_configuration() {
        let complete = serde_json::to_value(sample_tokens()).unwrap();
        for missing in ["api_url", "auth_config"] {
            let mut record = complete.clone();
            record.as_object_mut().unwrap().remove(missing);
            assert!(serde_json::from_value::<TokenStore>(record).is_err());
        }
        for missing in ["auth_url", "client_id"] {
            let mut record = complete.clone();
            record["auth_config"]
                .as_object_mut()
                .unwrap()
                .remove(missing);
            assert!(serde_json::from_value::<TokenStore>(record).is_err());
        }
        let mut record = complete;
        record["auth_config"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<TokenStore>(record).is_err());
    }

    #[test]
    fn load_tokens_rejects_old_records_for_known_and_custom_hosts() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("tokens.json");
        for api_url in [DEFAULT_API_URL, "https://custom.example.com/v1"] {
            let old = serde_json::json!({
                "access_token": "a", "refresh_token": "r",
                "expires_at": 4102444800_i64, "api_url": api_url
            });
            std::fs::write(&global, serde_json::to_vec(&old).unwrap()).unwrap();
            assert!(read_tokens(&global).is_err());
        }
    }

    /// OAuth requests carry the typed class of their failure, so `--json`
    /// reports `transport`/`http_5xx` rather than `other` (#1312).
    #[tokio::test]
    async fn oauth_request_failures_keep_their_class() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&issuer)
            .await;
        let closed = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}", listener.local_addr().unwrap())
        };
        for (auth_url, kind, status) in [
            (issuer.uri(), FailureKind::Http5xx, Some(503)),
            (closed, FailureKind::Transport, None),
        ] {
            let mut tokens = sample_tokens();
            tokens.auth_config = AuthConfig {
                auth_url,
                client_id: "client".into(),
            };
            let error = refresh_access_token(&tokens).await.unwrap_err();
            let failure = error.failure.expect("classified");
            assert_eq!((failure.kind, failure.http_status), (kind, status));
        }
    }

    /// A stalled issuer fails the request within the bound, as a timeout: a
    /// refresh holds the token lock across it (#1444).
    #[tokio::test]
    async fn a_stalled_oauth_request_times_out_as_a_timeout() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(30)))
            .mount(&issuer)
            .await;
        let tokens = TokenStore {
            auth_config: AuthConfig {
                auth_url: issuer.uri(),
                client_id: "client".into(),
            },
            ..sample_tokens()
        };
        let client = oauth_client_with_timeout(std::time::Duration::from_millis(50)).unwrap();
        let started = std::time::Instant::now();
        let error = refresh_access_token_with(&client, &tokens)
            .await
            .unwrap_err();
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);
        assert_eq!(
            error.failure.expect("classified").kind,
            FailureKind::Timeout
        );
    }

    #[tokio::test]
    async fn refresh_uses_saved_config_even_for_a_known_api_host() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("client_id=saved-client"))
            .and(body_string_contains("refresh_token=r"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "refreshed", "expires_in": 3600, "token_type": "Bearer"
            })))
            .expect(1)
            .mount(&issuer)
            .await;
        let mut tokens = sample_tokens();
        tokens.auth_config = AuthConfig {
            auth_url: issuer.uri(),
            client_id: "saved-client".into(),
        };
        let refreshed = refresh_access_token(&tokens).await.unwrap();
        assert_eq!(refreshed.api_url, DEFAULT_API_URL);
        assert_eq!(refreshed.auth_config, tokens.auth_config);
        assert_eq!(refreshed.access_token, "refreshed");
    }

    #[test]
    fn test_token_validity() {
        let now = chrono::Utc::now().timestamp();

        let valid = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now + 3600,
            api_url: DEFAULT_API_URL.into(),
            auth_config: sample_auth_config(),
        };
        assert!(is_token_valid(&valid));

        let expired = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now - 10,
            api_url: DEFAULT_API_URL.into(),
            auth_config: sample_auth_config(),
        };
        assert!(!is_token_valid(&expired));

        let near_expiry = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now + 30, // within 60s buffer
            api_url: DEFAULT_API_URL.into(),
            auth_config: sample_auth_config(),
        };
        assert!(!is_token_valid(&near_expiry));
    }

    #[test]
    fn test_tokens_path() {
        // Tokens live in the global ~/.clickhouse/ home, not the cwd, so the
        // path must end with .clickhouse/tokens.json under the home directory
        // and be independent of the current working directory.
        let path = tokens_path().expect("home dir should be resolvable");
        assert!(
            path.ends_with(".clickhouse/tokens.json"),
            "expected path to end with .clickhouse/tokens.json, got {}",
            path.display()
        );
        let home = dirs::home_dir().expect("home dir should be resolvable");
        assert!(
            path.starts_with(home.join(".clickhouse")),
            "expected path under {}/.clickhouse, got {}",
            home.display(),
            path.display()
        );
    }

    fn sample_tokens() -> TokenStore {
        TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: chrono::Utc::now().timestamp() + 3600,
            api_url: DEFAULT_API_URL.into(),
            auth_config: sample_auth_config(),
        }
    }

    fn sample_auth_config() -> AuthConfig {
        AuthConfig {
            auth_url: "https://auth.example.com".into(),
            client_id: "test-client".into(),
        }
    }

    #[test]
    fn load_tokens_returns_none_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("global/.clickhouse/tokens.json");
        assert!(read_tokens(&global).unwrap().is_none());
    }

    #[test]
    fn load_tokens_reports_unusable_files_without_modifying_them() {
        for data in [b"not json".as_slice(), b"{}", b"\xff"] {
            let tmp = tempfile::tempdir().unwrap();
            let global = tmp.path().join("global.json");
            std::fs::write(&global, data).unwrap();
            assert!(read_tokens(&global).is_err());
            assert_eq!(std::fs::read(&global).unwrap(), data);
        }
    }

    #[test]
    fn unreadable_token_path_requires_login() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("global.json");
        // A directory reliably fails read_to_string even when tests run as root.
        std::fs::create_dir(&global).unwrap();
        assert!(read_tokens(&global).is_err());
    }

    #[test]
    fn save_tokens_creates_parent_dirs_and_sets_permissions() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/dir/tokens.json");
        save_tokens_to(&path, &sample_tokens()).unwrap();
        let replacement = TokenStore {
            access_token: "replaced".into(),
            ..sample_tokens()
        };
        save_tokens_to(&path, &replacement).unwrap();
        assert_eq!(
            read_tokens(&path).unwrap().unwrap().access_token,
            "replaced"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "tokens file should be 0o600");
        }
        // Only the token file is left: the temporary file was renamed over it.
        let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["tokens.json"]);
    }

    /// The saved login is re-read under the lock, so a login another process
    /// refreshed while this one waited is not refreshed again.
    #[tokio::test]
    async fn locked_refresh_skips_a_login_already_fresh_on_disk() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "refreshed", "expires_in": 3600, "token_type": "Bearer"
            })))
            .expect(1)
            .mount(&issuer)
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("tokens.json");
        let fresh = TokenStore {
            auth_config: AuthConfig {
                auth_url: issuer.uri(),
                client_id: "c".into(),
            },
            ..sample_tokens()
        };

        save_tokens_to(&file, &fresh).unwrap();
        refresh_saved_tokens(&file, true).await.unwrap();
        assert_eq!(read_tokens(&file).unwrap().unwrap().access_token, "a");

        let expired = TokenStore {
            expires_at: chrono::Utc::now().timestamp() - 10,
            ..fresh
        };
        save_tokens_to(&file, &expired).unwrap();
        refresh_saved_tokens(&file, true).await.unwrap();
        assert_eq!(
            read_tokens(&file).unwrap().unwrap().access_token,
            "refreshed"
        );
    }

    /// Neither override is set: the error names both variables.
    fn assert_missing_both_env_vars(error: &AuthConfigError) {
        let AuthConfigError::MissingEnvVars(missing) = error else {
            panic!("expected MissingEnvVars, got {error:?}");
        };
        assert!(
            missing.contains("CLICKHOUSECTL_AUTH_URL")
                && missing.contains("CLICKHOUSECTL_CLIENT_ID"),
            "{missing}"
        );
    }

    #[test]
    fn test_auth_config_lookup() {
        let lookup = |url: &str| auth_config_for_url_with(url, &|_| None);
        assert!(lookup("https://api.clickhouse.cloud/v1").is_ok());
        assert!(lookup("https://api.control-plane.clickhouse-staging.com/v1").is_ok());
        assert!(lookup("https://api.control-plane.clickhouse-dev.com/v1").is_ok());
        assert_missing_both_env_vars(&lookup("https://api.unknown.com/v1").unwrap_err());

        // Verify distinct configs without reading the process environment.
        let prod = lookup("https://api.clickhouse.cloud/v1").unwrap();
        let staging = lookup("https://api.control-plane.clickhouse-staging.com/v1").unwrap();
        let dev = lookup("https://api.control-plane.clickhouse-dev.com/v1").unwrap();
        assert_ne!(prod.client_id, staging.client_id);
        assert_ne!(prod.client_id, dev.client_id);
        assert_ne!(staging.client_id, dev.client_id);

        // Must not match known hosts by substring, regardless of the real env.
        assert!(lookup("https://api.clickhouse.cloud.evil.com/v1").is_err());
        assert!(lookup("https://evil-api.clickhouse.cloud.attacker.com/v1").is_err());
        assert!(lookup("https://not-api.clickhouse-staging.com.bad.com/v1").is_err());
    }

    #[test]
    fn auth_config_unset_and_invalid_auth_url_are_distinct_errors() {
        let api_url = "https://api.example.com/v1";

        // Env vars simply not set: the error names the variables and never
        // carries the URL-format advice.
        let error = auth_config_for_url_with(api_url, &|_| None).unwrap_err();
        assert_missing_both_env_vars(&error);
        assert!(!error.to_string().contains(INVALID_AUTH_URL), "{error}");

        // Set but invalid: the error carries the validation reason.
        let error = auth_config_for_url_with(api_url, &|key| match key {
            "CLICKHOUSECTL_AUTH_URL" => Some("http://auth.example.com".to_owned()),
            "CLICKHOUSECTL_CLIENT_ID" => Some("custom-client".to_owned()),
            _ => panic!("unexpected environment lookup: {key}"),
        })
        .unwrap_err();
        assert_eq!(error, AuthConfigError::InvalidAuthUrl(INVALID_AUTH_URL));
        assert!(error.to_string().contains(INVALID_AUTH_URL), "{error}");
    }

    #[test]
    fn known_auth_config_converts_named_fields_to_owned_values() {
        let known = KnownAuthConfig {
            host: "api.example.com",
            auth_url: "https://auth.example.com",
            client_id: "example-client",
        };
        assert_eq!(
            AuthConfig::from(&known),
            AuthConfig {
                auth_url: "https://auth.example.com".into(),
                client_id: "example-client".into(),
            }
        );
    }

    #[test]
    fn auth_config_known_hosts_ignore_environment() {
        for known in KNOWN_CONFIGS {
            let api_url = format!("https://{}/v1", known.host);
            let config = auth_config_for_url_with(&api_url, &|_| {
                panic!("built-in host must not read environment overrides")
            })
            .unwrap();
            assert_eq!(config.auth_url, known.auth_url);
            assert_eq!(config.client_id, known.client_id);
        }
    }

    #[test]
    fn auth_config_custom_host_requires_both_nonempty_values() {
        for (auth_url, client_id) in [
            (None, None),
            (Some("https://auth.example.com"), None),
            (None, Some("client")),
            (Some(""), Some("client")),
            (Some("https://auth.example.com"), Some("")),
            (Some(""), Some("")),
        ] {
            let config = auth_config_for_url_with("https://api.example.com/v1", &|key| match key {
                "CLICKHOUSECTL_AUTH_URL" => auth_url.map(str::to_owned),
                "CLICKHOUSECTL_CLIENT_ID" => client_id.map(str::to_owned),
                _ => panic!("unexpected environment lookup: {key}"),
            });
            assert!(
                matches!(config, Err(AuthConfigError::MissingEnvVars(_))),
                "{auth_url:?}, {client_id:?}"
            );
        }
    }

    #[test]
    fn auth_config_custom_host_uses_environment_and_trims_trailing_slashes() {
        for auth_url in [
            "https://auth.example.com",
            "https://auth.example.com/",
            "https://auth.example.com///",
        ] {
            let config = auth_config_for_url_with("https://api.example.com/v1", &|key| match key {
                "CLICKHOUSECTL_AUTH_URL" => Some(auth_url.to_owned()),
                "CLICKHOUSECTL_CLIENT_ID" => Some("custom-client".into()),
                _ => panic!("unexpected environment lookup: {key}"),
            })
            .unwrap();
            assert_eq!(config.auth_url, "https://auth.example.com");
            assert_eq!(config.client_id, "custom-client");
        }
    }

    #[test]
    fn authorization_url_requires_https_except_for_loopback() {
        for value in [
            "https://auth.example.com",
            "https://auth.example.com/tenant",
            "https://127.0.0.1:8443",
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://127.255.255.254",
            "http://[::1]:8080",
        ] {
            assert!(validated_auth_url(value).is_ok(), "{value}");
        }
        for value in [
            "",
            "not a URL",
            "/relative",
            "//auth.example.com",
            "file:///tmp/auth",
            "ftp://localhost",
            "http://auth.example.com",
            "http://0.0.0.0",
            "http://192.168.1.1",
            "http://[::]",
            "http://[2001:db8::1]",
            "http://localhost.evil.example",
            "http://127.0.0.1.evil.example",
            "http://localhost@evil.example",
            "https://user:password@auth.example.com",
            "https://auth.example.com?redirect=http://evil.example",
            "https://auth.example.com#fragment",
        ] {
            assert!(validated_auth_url(value).is_err(), "{value}");
            // An empty value is filtered out as unset before validation, so
            // it reports the missing variable rather than an invalid URL.
            let expected = if value.is_empty() {
                AuthConfigError::MissingEnvVars("CLICKHOUSECTL_AUTH_URL")
            } else {
                AuthConfigError::InvalidAuthUrl(INVALID_AUTH_URL)
            };
            assert_eq!(
                auth_config_for_url_with("https://api.example.com", &|key| {
                    Some(
                        if key == "CLICKHOUSECTL_AUTH_URL" {
                            value
                        } else {
                            "client"
                        }
                        .into(),
                    )
                })
                .unwrap_err(),
                expected,
                "{value}"
            );
            let mut record = serde_json::to_value(sample_tokens()).unwrap();
            record["auth_config"]["auth_url"] = serde_json::json!(value);
            assert!(
                serde_json::from_value::<TokenStore>(record).is_err(),
                "{value}"
            );
        }
        for known in KNOWN_CONFIGS {
            assert!(validated_auth_url(known.auth_url).is_ok());
        }
    }

    #[test]
    fn auth_config_invalid_api_url_never_reads_environment() {
        for api_url in ["not a URL", "file:///tmp/api", "mailto:user@example.com"] {
            assert!(
                matches!(
                    auth_config_for_url_with(api_url, &|_| {
                        panic!("invalid API URL must not read environment overrides")
                    }),
                    Err(AuthConfigError::InvalidApiUrl)
                ),
                "{api_url}"
            );
        }
    }

    /// `auth signup` opens a real browser, so its JSON contract is checked
    /// here rather than in a subprocess (#1233).
    #[test]
    fn signup_json_report_is_one_object_with_the_url_and_browser_outcome() {
        for browser_opened in [true, false] {
            let report = SignupReport {
                url: "https://console.clickhouse.cloud/signUp".into(),
                browser_opened,
            };
            let mut output = Vec::new();
            report.write(&mut output, true).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(
                value,
                serde_json::json!({
                    "url": "https://console.clickhouse.cloud/signUp",
                    "browser_opened": browser_opened,
                })
            );
        }
    }

    #[test]
    fn test_normalize_api_url() {
        assert_eq!(
            normalize_api_url("https://api.clickhouse.cloud"),
            "https://api.clickhouse.cloud/v1"
        );
        assert_eq!(
            normalize_api_url("https://api.clickhouse.cloud/v1"),
            "https://api.clickhouse.cloud/v1"
        );
        assert_eq!(
            normalize_api_url("https://api.clickhouse.cloud/"),
            "https://api.clickhouse.cloud/v1"
        );
        assert_eq!(
            normalize_api_url("https://api.control-plane.clickhouse-staging.com/v1/"),
            "https://api.control-plane.clickhouse-staging.com/v1"
        );
    }

    fn tokens_for(server: &wiremock::MockServer) -> TokenStore {
        TokenStore {
            access_token: "fresh-bearer".into(),
            api_url: normalize_api_url(&server.uri()),
            ..sample_tokens()
        }
    }

    #[tokio::test]
    async fn oauth_login_report_names_the_user_and_organizations() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let user = serde_json::json!({
            "actorType": "user",
            "email": "ada@example.com",
            "name": "Ada Lovelace",
            "organizations": [
                {"organizationId": "55555555-6666-4777-8888-999999999999", "organizationName": "Engines"}
            ]
        });
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .and(header("authorization", "Bearer fresh-bearer"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"status": 200, "result": user})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let report = oauth_login_report(&tokens_for(&server), "tokens.json".into()).await;
        let report = serde_json::to_value(&report).unwrap();
        assert_eq!(
            report,
            serde_json::json!({
                "saved": "tokens.json",
                "identity": user,
                "verification": "verified",
            })
        );
    }

    #[tokio::test]
    async fn oauth_login_report_only_warns_when_whoami_fails() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        for status in [401, 500] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/whoami"))
                .respond_with(
                    ResponseTemplate::new(status).set_body_json(
                        serde_json::json!({"status": status, "error": "whoami failed"}),
                    ),
                )
                .expect(1)
                .mount(&server)
                .await;

            let report = oauth_login_report(&tokens_for(&server), "tokens.json".into()).await;
            assert_eq!(report.verification, "unverified", "{status}");
            assert!(report.identity.is_none());
            // The warning relays the API's own error.
            let warning = report.warning.unwrap();
            assert!(warning.contains("whoami failed"), "{warning}");
        }
    }

    /// `auth status` facts with nothing configured; tests set what they need.
    fn status_facts() -> StatusFacts {
        StatusFacts {
            active: None,
            verification: "skipped",
            flags: (false, false),
            saved_key: SavedKeyState::NotConfigured,
            env: (false, false),
            env_provenance: None,
            oauth: OAuthState::NotConfigured,
            in_use_key_scope: ROLE_UNKNOWN.into(),
        }
    }

    fn saved_key(expired: Option<&str>) -> SavedKeyState {
        SavedKeyState::Complete {
            api_url: "https://api.example/v1".into(),
            grant_scope: Some("Service API Reader".into()),
            expired: expired.map(String::from),
        }
    }

    /// (type, status, scope, in use) per row.
    fn status_cells(facts: &StatusFacts) -> Vec<(String, String, String, String)> {
        facts
            .rows()
            .into_iter()
            .map(|row| (row.auth_type, row.status, row.scope, row.active))
            .collect()
    }

    fn cell<'a>(
        cells: &'a [(String, String, String, String)],
        source: &str,
    ) -> &'a (String, String, String, String) {
        cells.iter().find(|row| row.0 == source).unwrap()
    }

    #[test]
    fn status_rows_follow_precedence_and_the_table_says_in_use() {
        let facts = status_facts();
        let types: Vec<_> = status_cells(&facts).into_iter().map(|row| row.0).collect();
        assert_eq!(types, ["CLI flags", "API key", "Env vars", "OAuth"]);
        let table = tabled::Table::new(facts.rows()).to_string();
        assert!(table.lines().nth(1).unwrap().contains("In use"), "{table}");
        let json = serde_json::to_value(facts.rows()).unwrap();
        assert_eq!(json[0]["active"], "-");
        assert!(json[0].get("in_use").is_none());
    }

    #[test]
    fn status_without_credentials_says_not_logged_in() {
        let facts = status_facts();
        for row in status_cells(&facts) {
            assert_eq!(row.1, "Not configured", "{row:?}");
            assert_eq!(row.3, "-");
        }
        assert_eq!(
            facts.summary(),
            "Not logged in: `cloud auth login` (needs the user)."
        );
    }

    #[test]
    fn expired_saved_key_is_expired_and_points_at_the_valid_oauth_login() {
        let facts = StatusFacts {
            active: Some(AuthSource::CredentialsFile),
            verification: "rejected",
            saved_key: saved_key(Some("2026-11-18T17:45Z")),
            oauth: OAuthState::Valid("https://api.example/v1".into()),
            ..status_facts()
        };
        let cells = status_cells(&facts);
        let saved = cell(&cells, "API key");
        assert_eq!(saved.1, "Expired 2026-11-18T17:45Z");
        assert_eq!(saved.2, "Service API Reader");
        assert_eq!(saved.3, "yes");
        let oauth = cell(&cells, "OAuth");
        assert_eq!((oauth.1.as_str(), oauth.3.as_str()), ("Valid", "-"));
        let summary = facts.summary();
        let lines: Vec<_> = summary.lines().collect();
        assert_eq!(
            lines[0],
            "Commands here use the saved API key, which has expired, so they fail."
        );
        #[cfg(feature = "experimental-access-negotiation")]
        assert_eq!(
            lines[1..],
            ["The OAuth login is valid: `cloud access request` works and replaces the key."]
        );
        #[cfg(not(feature = "experimental-access-negotiation"))]
        assert_eq!(lines.len(), 1);

        // Expired is shown even when flags outrank the saved key.
        let outranked = StatusFacts {
            active: Some(AuthSource::CliFlags),
            verification: "verified",
            flags: (true, true),
            ..facts
        };
        let cells = status_cells(&outranked);
        assert_eq!(cell(&cells, "API key").1, "Expired 2026-11-18T17:45Z");
        assert_eq!(cell(&cells, "CLI flags").1, "Valid");
    }

    #[test]
    fn rejected_saved_key_without_oauth_needs_a_login() {
        let facts = StatusFacts {
            active: Some(AuthSource::CredentialsFile),
            verification: "rejected",
            saved_key: saved_key(None),
            oauth: OAuthState::Expired("https://api.example/v1".into()),
            ..status_facts()
        };
        let cells = status_cells(&facts);
        assert_eq!(cell(&cells, "API key").1, "Rejected by the API");
        assert_eq!(cell(&cells, "OAuth").1, "Expired");
        assert_eq!(
            facts.summary(),
            "Commands here use the saved API key, which the API refused, so they fail.\n\
             No valid OAuth login: `cloud auth login` (needs the user)."
        );
    }

    #[test]
    fn valid_saved_key_in_use_outranks_env_vars() {
        let facts = StatusFacts {
            active: Some(AuthSource::CredentialsFile),
            verification: "verified",
            saved_key: saved_key(None),
            env: (true, true),
            env_provenance: Some(PathBuf::from("/x/.env")),
            oauth: OAuthState::Valid("https://api.example/v1".into()),
            ..status_facts()
        };
        let cells = status_cells(&facts);
        assert_eq!(cell(&cells, "API key").1, "Valid");
        assert_eq!(cell(&cells, "API key").3, "yes");
        assert_eq!(
            cell(&cells, "Env vars").1,
            "Configured (from /x/.env), not in use (outranked by saved API key)"
        );
        assert_eq!(cell(&cells, "Env vars").3, "-");
        assert_eq!(cell(&cells, "OAuth").1, "Valid");
        assert_eq!(facts.summary(), "Commands here use the saved API key.");
    }

    #[test]
    fn flags_in_use_outrank_every_api_key_source() {
        let facts = StatusFacts {
            active: Some(AuthSource::CliFlags),
            verification: "unavailable",
            flags: (true, true),
            saved_key: saved_key(None),
            env: (true, true),
            ..status_facts()
        };
        let cells = status_cells(&facts);
        assert_eq!(cell(&cells, "CLI flags").1, "Configured (not verified)");
        for source in ["API key", "Env vars"] {
            assert_eq!(
                cell(&cells, source).1,
                "Configured, not in use (outranked by CLI flags)"
            );
        }
        assert_eq!(
            facts.summary(),
            "Commands here use the API key from CLI flags."
        );
        let env = StatusFacts {
            active: Some(AuthSource::EnvVars),
            verification: "verified",
            env: (true, true),
            env_provenance: Some(PathBuf::from("/x/.env")),
            ..status_facts()
        };
        assert_eq!(
            cell(&status_cells(&env), "Env vars").1,
            "Valid (from /x/.env)"
        );
        assert_eq!(
            env.summary(),
            "Commands here use the API key from environment variables."
        );
    }

    fn key_record(value: serde_json::Value) -> ApiKey {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn key_roles_come_from_the_key_record_or_are_unknown() {
        let now = "2026-11-19T12:00:00Z".parse().unwrap();
        let one = KeyRoles::of(&key_record(serde_json::json!({
            "assignedRoles": [{"roleName": "Service API Reader"}]
        })));
        assert_eq!(one.scope(now), "Service API Reader");
        assert_eq!(one.write_access(), "API key: Service API Reader");

        // A role without a name shows its id; the key's expiry follows the roles.
        let several = KeyRoles::of(&key_record(serde_json::json!({
            "assignedRoles": [
                {"roleName": "Service API Reader"},
                {"roleId": "11111111-2222-4333-8444-555555555555", "roleName": " "}
            ],
            "expireAt": "2026-11-19T17:45:00Z"
        })));
        assert_eq!(
            several.scope(now),
            "Service API Reader, 11111111-2222-4333-8444-555555555555, expires 17:45Z"
        );
        assert_eq!(
            several.write_access(),
            "API key: Service API Reader, 11111111-2222-4333-8444-555555555555; \
             expires 2026-11-19T17:45:00Z"
        );

        let none = KeyRoles::of(&key_record(serde_json::json!({"assignedRoles": []})));
        assert_eq!(none.scope(now), "no roles");
        assert_eq!(none.write_access(), "API key: no roles");

        // A record without assignedRoles says nothing about them.
        assert_eq!(
            KeyRoles::of(&key_record(serde_json::json!({}))),
            KeyRoles::Unknown
        );
        assert_eq!(KeyRoles::Unknown.scope(now), "role unknown");
        assert_eq!(KeyRoles::Unknown.write_access(), "API key: role unknown");
    }

    #[test]
    fn only_the_api_key_in_use_shows_its_looked_up_role() {
        let ungranted = SavedKeyState::Complete {
            api_url: "https://api.example/v1".into(),
            grant_scope: None,
            expired: None,
        };
        let flags = StatusFacts {
            active: Some(AuthSource::CliFlags),
            verification: "verified",
            flags: (true, true),
            saved_key: ungranted,
            env: (true, true),
            in_use_key_scope: "Organization Admin".into(),
            ..status_facts()
        };
        let cells = status_cells(&flags);
        assert_eq!(cell(&cells, "CLI flags").2, "Organization Admin");
        // Configured keys not in use are never looked up.
        assert_eq!(cell(&cells, "API key").2, "role unknown");
        assert_eq!(cell(&cells, "Env vars").2, "role unknown");

        // An outranked saved key with a grant keeps the grant's Scope.
        let granted = StatusFacts {
            saved_key: saved_key(None),
            ..flags
        };
        assert_eq!(
            cell(&status_cells(&granted), "API key").2,
            "Service API Reader"
        );

        let saved = StatusFacts {
            active: Some(AuthSource::CredentialsFile),
            flags: (false, false),
            saved_key: SavedKeyState::Complete {
                api_url: "https://api.example/v1".into(),
                grant_scope: None,
                expired: None,
            },
            in_use_key_scope: "no roles".into(),
            ..granted
        };
        let cells = status_cells(&saved);
        assert_eq!(cell(&cells, "API key").2, "no roles");
        assert_eq!(cell(&cells, "Env vars").2, "role unknown");
    }

    #[test]
    fn oauth_only_is_valid_and_read_only() {
        let facts = StatusFacts {
            active: Some(AuthSource::OAuthTokens),
            verification: "verified",
            oauth: OAuthState::Valid("https://api.example/v1".into()),
            ..status_facts()
        };
        let cells = status_cells(&facts);
        let oauth = cell(&cells, "OAuth");
        assert_eq!(
            (oauth.1.as_str(), oauth.2.as_str(), oauth.3.as_str()),
            ("Valid", "read-only", "yes")
        );
        #[cfg(feature = "experimental-access-negotiation")]
        let expected =
            "Commands here use the read-only OAuth login. For writes: `cloud access request`.";
        #[cfg(not(feature = "experimental-access-negotiation"))]
        let expected = "Commands here use the read-only OAuth login.";
        assert_eq!(facts.summary(), expected);
        let rejected = StatusFacts {
            verification: "rejected",
            ..facts
        };
        assert_eq!(
            cell(&status_cells(&rejected), "OAuth").1,
            "Rejected by the API"
        );
    }
}
