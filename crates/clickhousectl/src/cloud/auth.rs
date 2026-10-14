use super::permissions::Declaration as Permission;
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("auth login", &[&op::WHOAMI_GET]).unscoped(),
    Permission::non_api(
        "auth logout",
        "Clears local credentials; no Cloud API call.",
    ),
    Permission::api("auth status", &[&op::WHOAMI_GET]).unscoped(),
    Permission::non_api("auth signup", "Opens account signup; no Cloud API call."),
    Permission::api("auth whoami", &[&op::WHOAMI_GET]).unscoped(),
];

use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::{eprint_line, print_human};
use crate::cloud::{
    AuthSource, dotenv_env_provenance, env_cred_presence, resolve_active_auth_source,
};
use crate::error::Error;
use clap::Subcommand;
use clickhouse_cloud_api::models::Whoami;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AUDIENCE: &str = "clickhousectl";
const SCOPE: &str = "openid profile email offline_access";

const DEFAULT_API_URL: &str = "https://api.clickhouse.cloud/v1";

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in to ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  No flags: OAuth device flow, opens a browser, needs a human; the tokens are read-only.
  --api-key/--api-secret: no browser, read+write; a key the API rejects is not saved (exit 4).
  Prints the verified identity; offline, the key is saved unverified with a warning.
  Create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl")]
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
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Opens the ClickHouse Cloud sign-up page in a browser; a human must finish it.
  Next: `cloud auth login --api-key X --api-secret Y`")]
    Signup,
}

impl AuthCommands {
    pub fn login_validation_error(&self) -> Option<(clap::error::ErrorKind, &'static str)> {
        let Self::Login {
            api_key,
            api_secret,
            ..
        } = self
        else {
            return None;
        };
        use clap::error::ErrorKind;
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
        let response = self
            .api()
            .whoami_get()
            .await
            .map_err(|error| self.convert_error(error))?;
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
                    prompt_api_credentials().map_err(|error| Error::Cloud(error.to_string()))?;
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
                let url = api_url.unwrap_or("https://api.clickhouse.cloud");
                let tokens = device_auth_login(url)
                    .await
                    .map_err(|error| Error::Cloud(error.to_string()))?;
                save_tokens(&tokens).map_err(|error| Error::Cloud(error.to_string()))?;
                let tokens_path = tokens_path().map_err(|error| Error::Cloud(error.to_string()))?;
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
            let parsed = url::Url::parse(api_url)
                .map_err(|error| Error::Cloud(format!("Invalid URL: {}", error)))?;
            let host = parsed.host_str().unwrap_or("api.clickhouse.cloud");
            let base_host = host.strip_prefix("api.").unwrap_or(host);
            let url = format!(
                "https://console.{}/signUp?utm_source=clickhousectl",
                base_host
            );
            println!("Opening ClickHouse Cloud sign-up page...");
            if open::that(&url).is_err() {
                println!("Could not open browser. Please visit: {}", url);
            }
            Ok(())
        }
        AuthCommands::Logout { oauth, api_keys } => {
            match (oauth, api_keys) {
                (true, false) => {
                    clear_tokens();
                    println!("OAuth tokens cleared. API keys unchanged.");
                }
                (false, true) => {
                    credentials::clear_credentials()
                        .map_err(|error| Error::Cloud(error.to_string()))?;
                    println!("API keys cleared. OAuth tokens unchanged.");
                }
                _ => {
                    clear_tokens();
                    credentials::clear_credentials()
                        .map_err(|error| Error::Cloud(error.to_string()))?;
                    println!("Logged out. All saved credentials cleared.");
                }
            }
            Ok(())
        }
        AuthCommands::Status => {
            use tabled::{Table, Tabled, settings::Style};

            #[derive(Serialize, Tabled)]
            struct AuthRow {
                #[tabled(rename = "Type")]
                #[serde(rename = "type")]
                auth_type: String,
                #[tabled(rename = "Status")]
                status: String,
                #[tabled(rename = "Scope")]
                scope: String,
                #[tabled(rename = "Active")]
                active: String,
            }

            let active = resolve_active_auth_source(api_key, api_secret);
            let mark = |source: AuthSource| -> String {
                if active == Some(source) {
                    "yes".into()
                } else {
                    "-".into()
                }
            };

            let configured_status = |source: AuthSource| -> String {
                if active == Some(source) {
                    "Active".into()
                } else {
                    "Configured (inactive)".into()
                }
            };

            let mut rows = Vec::new();
            let (status, scope) = match (api_key.is_some(), api_secret.is_some()) {
                (true, true) => (configured_status(AuthSource::CliFlags), "read/write"),
                (true, false) => ("Incomplete (missing --api-secret)".into(), "-"),
                (false, true) => ("Incomplete (missing --api-key)".into(), "-"),
                (false, false) => ("Not configured".into(), "-"),
            };
            rows.push(AuthRow {
                auth_type: "CLI flags".into(),
                status,
                scope: scope.into(),
                active: mark(AuthSource::CliFlags),
            });

            match load_tokens() {
                Some(tokens) if is_token_valid(&tokens) => {
                    rows.push(AuthRow {
                        auth_type: "OAuth".into(),
                        status: configured_status(AuthSource::OAuthTokens),
                        scope: "read-only".into(),
                        active: mark(AuthSource::OAuthTokens),
                    });
                }
                Some(_) => {
                    rows.push(AuthRow {
                        auth_type: "OAuth".into(),
                        status: "Expired".into(),
                        scope: "read-only".into(),
                        active: "-".into(),
                    });
                }
                None => {
                    rows.push(AuthRow {
                        auth_type: "OAuth".into(),
                        status: "Not configured".into(),
                        scope: "-".into(),
                        active: "-".into(),
                    });
                }
            }

            let saved = credentials::load_credentials();
            let (status, scope) = match saved.as_ref() {
                Some(creds) if creds.api_credentials().is_some() => {
                    (configured_status(AuthSource::CredentialsFile), "read/write")
                }
                Some(creds) if creds.api_key.is_some() || creds.api_secret.is_some() => {
                    ("Incomplete (missing or empty API key/secret)".into(), "-")
                }
                _ => ("Not configured".into(), "-"),
            };
            rows.push(AuthRow {
                auth_type: "API key".into(),
                status,
                scope: scope.into(),
                active: mark(AuthSource::CredentialsFile),
            });

            let env_creds = env_cred_presence();
            match (env_creds.key, env_creds.secret) {
                (true, true) => {
                    let provenance = dotenv_env_provenance()
                        .map(|path| format!(" (from {})", path.display()))
                        .unwrap_or_default();
                    let status = match active {
                        Some(AuthSource::EnvVars) => format!("Active{provenance}"),
                        Some(AuthSource::CliFlags) => {
                            format!("Configured{provenance} (inactive, outranked by CLI flags)")
                        }
                        Some(AuthSource::CredentialsFile) => format!(
                            "Configured{provenance} (inactive, outranked by credentials file)"
                        ),
                        _ => format!("Configured{provenance} (inactive)"),
                    };
                    rows.push(AuthRow {
                        auth_type: "Env vars".into(),
                        status,
                        scope: "read/write".into(),
                        active: mark(AuthSource::EnvVars),
                    });
                }
                (true, false) => {
                    rows.push(AuthRow {
                        auth_type: "Env vars".into(),
                        status: "Incomplete (missing CLICKHOUSE_CLOUD_API_SECRET)".into(),
                        scope: "-".into(),
                        active: "-".into(),
                    });
                }
                (false, true) => {
                    rows.push(AuthRow {
                        auth_type: "Env vars".into(),
                        status: "Incomplete (missing CLICKHOUSE_CLOUD_API_KEY)".into(),
                        scope: "-".into(),
                        active: "-".into(),
                    });
                }
                (false, false) => {
                    rows.push(AuthRow {
                        auth_type: "Env vars".into(),
                        status: "Not configured".into(),
                        scope: "-".into(),
                        active: "-".into(),
                    });
                }
            }

            if debug {
                match active {
                    Some(source) => {
                        eprint_line(format!("[debug] auth source: {}", source.describe()))
                    }
                    None if api_key.is_some() != api_secret.is_some() => {
                        let missing = if api_key.is_some() {
                            "--api-secret"
                        } else {
                            "--api-key"
                        };
                        eprint_line(format!(
                            "[debug] auth source: none (incomplete CLI flags: missing {missing})"
                        ));
                    }
                    None => eprint_line("[debug] auth source: none (no credentials configured)"),
                }
            }

            let identity = match active {
                Some(_) => IdentityCheck::from(
                    verify_identity(CloudClient::new_with_timeout(
                        api_key,
                        api_secret,
                        api_url,
                        VERIFY_TIMEOUT,
                    ))
                    .await,
                ),
                None => IdentityCheck::skipped(),
            };

            if json {
                #[derive(Serialize)]
                struct StatusOutput<'a> {
                    sources: Vec<AuthRow>,
                    #[serde(flatten)]
                    identity: &'a IdentityCheck,
                }
                let output = StatusOutput {
                    sources: rows,
                    identity: &identity,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{}", Table::new(rows).with(Style::markdown()));
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

/// How long `auth login` and `auth status` wait for whoami, so working
/// offline never hangs on the identity check.
const VERIFY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// The result of asking whoami about a credential set.
enum Verification {
    Verified(Whoami),
    /// The API refused the credentials (401/403).
    Rejected(super::CloudError),
    /// Anything else: offline, timeout, 5xx, or an unbuildable client.
    Unavailable(super::CloudError),
}

async fn verify_identity(client: CloudResult<CloudClient>) -> Verification {
    let result = match client {
        Ok(client) => client.get_whoami().await,
        Err(error) => Err(error),
    };
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
    credentials::set_api_credentials(key.to_owned(), secret.to_owned())
        .map_err(|error| Error::Cloud(error.to_string()))?;
    let saved = credentials::credentials_path().display().to_string();
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
        eprint_line(format!("warning: {warning}"));
    }
    Ok(())
}

fn prompt_api_credentials() -> std::result::Result<(String, String), Box<dyn std::error::Error>> {
    use std::io::Write;

    print!("API Key: ");
    crate::stdout::stdout().flush()?;
    let mut api_key = String::new();
    std::io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        return Err("API key cannot be empty".into());
    }

    print!("API Secret: ");
    crate::stdout::stdout().flush()?;
    let api_secret = rpassword::read_password()?;

    if api_secret.is_empty() {
        return Err("API secret cannot be empty".into());
    }

    Ok((api_key, api_secret))
}

struct AuthConfig {
    auth_url: &'static str,
    client_id: &'static str,
}

/// Known API host → auth configuration mappings.
const KNOWN_CONFIGS: &[(&str, AuthConfig)] = &[
    (
        "api.clickhouse.cloud",
        AuthConfig {
            auth_url: "https://auth.clickhouse.cloud",
            client_id: "9q6XAueAs47R4X5d1d6FbjbJqjsrA2ZJ",
        },
    ),
    (
        "api.control-plane.clickhouse-staging.com",
        AuthConfig {
            auth_url: "https://auth.control-plane.clickhouse-staging.com",
            client_id: "ZC8AupPshQt2UNO2hEDutnKitx4PhizY",
        },
    ),
    (
        "api.control-plane.clickhouse-dev.com",
        AuthConfig {
            auth_url: "https://auth.control-plane.clickhouse-dev.com",
            client_id: "bVVcrqNw1t5dya9WFzfnM7PSsAgmfzwY",
        },
    ),
];

fn auth_config_for_url(api_url: &str) -> Option<&'static AuthConfig> {
    let parsed = url::Url::parse(api_url).ok()?;
    let host = parsed.host_str()?;
    KNOWN_CONFIGS
        .iter()
        .find(|(known_host, _)| host == *known_host)
        .map(|(_, config)| config)
}

/// Normalize a user-provided URL into the API base URL we store and use.
/// Ensures it has a scheme and the /v1 path suffix.
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
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
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
    #[allow(dead_code)]
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[allow(dead_code)]
    error_description: Option<String>,
}

/// Global OAuth token store: `~/.clickhouse/tokens.json`.
///
/// OAuth login is user identity (not org-scoped like API keys), so tokens live
/// in the CLI's global home alongside installed versions and configs, rather
/// than per-project under `./.clickhouse/`.
pub fn tokens_path() -> Result<PathBuf, crate::error::Error> {
    Ok(crate::paths::base_dir()?.join("tokens.json"))
}

/// Legacy per-project token store: `<cwd>/.clickhouse/tokens.json`.
///
/// Earlier versions wrote OAuth tokens here (per-project). `load_tokens`
/// migrates a legacy file to the global path on first sight, and
/// `clear_tokens` removes both so logging out from an old project dir doesn't
/// leave a file that re-migrates.
fn legacy_tokens_path() -> PathBuf {
    crate::init::local_dir().join("tokens.json")
}

pub fn load_tokens() -> Option<TokenStore> {
    let global = tokens_path().ok()?;
    let legacy = legacy_tokens_path();
    load_tokens_from(&global, &legacy)
}

/// Path-injected core of [`load_tokens`]. Reads the global token file first;
/// if absent/unreadable, attempts a one-time migration from `legacy_path`
/// (write the global copy, best-effort delete the legacy file).
fn load_tokens_from(
    global_path: &std::path::Path,
    legacy_path: &std::path::Path,
) -> Option<TokenStore> {
    if let Ok(data) = std::fs::read_to_string(global_path)
        && let Ok(tokens) = serde_json::from_str::<TokenStore>(&data)
    {
        return Some(tokens);
    }

    // Global file missing or unreadable — try a one-time migration from the
    // legacy cwd-based path. Best-effort: write the global copy, then delete
    // the legacy file so it can't re-migrate on the next command.
    if let Ok(data) = std::fs::read_to_string(legacy_path)
        && let Ok(tokens) = serde_json::from_str::<TokenStore>(&data)
    {
        if save_tokens_to(global_path, &tokens).is_ok() {
            let _ = std::fs::remove_file(legacy_path);
        }
        return Some(tokens);
    }

    None
}

pub fn save_tokens(tokens: &TokenStore) -> Result<(), Box<dyn std::error::Error>> {
    let path = tokens_path()?;
    save_tokens_to(&path, tokens)
}

fn save_tokens_to(
    path: &std::path::Path,
    tokens: &TokenStore,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(tokens)?;
    std::fs::write(path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn clear_tokens() {
    if let Ok(path) = tokens_path() {
        let _ = std::fs::remove_file(path);
    }
    let _ = std::fs::remove_file(legacy_tokens_path());
}

pub fn is_token_valid(tokens: &TokenStore) -> bool {
    let now = chrono::Utc::now().timestamp();
    tokens.expires_at > now + 60
}

pub async fn device_auth_login(api_url: &str) -> Result<TokenStore, Box<dyn std::error::Error>> {
    let api_url = normalize_api_url(api_url);
    let config = auth_config_for_url(&api_url).ok_or_else(|| {
        format!(
            "Unknown API host in URL '{}'. Known hosts: {}",
            api_url,
            KNOWN_CONFIGS
                .iter()
                .map(|(h, _)| *h)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let client = crate::http::client_builder().build()?;

    // Step 1: Request device code
    let form_body = format!(
        "client_id={}&scope={}&audience={}",
        urlencoding::encode(config.client_id),
        urlencoding::encode(SCOPE),
        urlencoding::encode(AUDIENCE),
    );
    let resp = client
        .post(format!("{}/oauth/device/code", config.auth_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await?;

    let status = resp.status();
    let resp_body = resp.bytes().await?;

    if !status.is_success() {
        return Err(format!(
            "Failed to start device authorization: {}",
            String::from_utf8_lossy(&resp_body)
        )
        .into());
    }

    let device_resp: DeviceCodeResponse = serde_json::from_slice(&resp_body)?;

    // Step 2: Display instructions
    let verification_url = device_resp
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&device_resp.verification_uri);

    println!("Login to ClickHouse Cloud:");
    println!();
    println!("  Code: {}", device_resp.user_code);
    println!("  URL:  {verification_url}");
    println!();

    // Best-effort browser open
    if open::that(verification_url).is_ok() {
        println!("Browser opened. Waiting for authentication...");
    } else {
        println!("Open the URL above in your browser. Waiting for authentication...");
    }

    // Step 3: Poll for token
    let mut interval = device_resp.interval;
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(device_resp.expires_in);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        if std::time::Instant::now() > deadline {
            return Err("Device authorization timed out".into());
        }

        let poll_body = format!(
            "grant_type={}&device_code={}&client_id={}",
            urlencoding::encode("urn:ietf:params:oauth:grant-type:device_code"),
            urlencoding::encode(&device_resp.device_code),
            urlencoding::encode(config.client_id),
        );
        let resp = client
            .post(format!("{}/oauth/token", config.auth_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(poll_body)
            .send()
            .await?;

        let status = resp.status();
        let body_bytes = resp.bytes().await?;
        let body = String::from_utf8_lossy(&body_bytes);

        if status.is_success() {
            let token_resp: TokenResponse = serde_json::from_str(&body)?;
            let now = chrono::Utc::now().timestamp();
            let tokens = TokenStore {
                access_token: token_resp.access_token,
                refresh_token: token_resp.refresh_token.unwrap_or_default(),
                expires_at: now + token_resp.expires_in,
                api_url: api_url.clone(),
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
            "expired_token" => return Err("Device code expired. Please try again.".into()),
            "access_denied" => return Err("Authorization denied by user.".into()),
            _ => {
                return Err(format!(
                    "Authorization failed: {} ({})",
                    error_resp.error,
                    error_resp.error_description.unwrap_or_default()
                )
                .into());
            }
        }
    }
}

pub async fn refresh_access_token(
    tokens: &TokenStore,
) -> Result<TokenStore, Box<dyn std::error::Error>> {
    let config = auth_config_for_url(&tokens.api_url)
        .ok_or_else(|| format!("Cannot refresh: unknown API host in '{}'", tokens.api_url))?;

    let client = crate::http::client_builder().build()?;

    let form_body = format!(
        "grant_type={}&client_id={}&refresh_token={}",
        urlencoding::encode("refresh_token"),
        urlencoding::encode(config.client_id),
        urlencoding::encode(&tokens.refresh_token),
    );
    let resp = client
        .post(format!("{}/oauth/token", config.auth_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await?;

    let status = resp.status();
    let resp_body = resp.bytes().await?;

    if !status.is_success() {
        return Err(format!(
            "Token refresh failed: {}",
            String::from_utf8_lossy(&resp_body)
        )
        .into());
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
    })
}

/// If tokens exist and are near-expiry, refresh them. Returns Ok(()) even if
/// no tokens are present (the user may be using API keys instead).
pub async fn ensure_fresh_tokens(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let Some(tokens) = load_tokens() else {
        return Ok(());
    };

    if is_token_valid(&tokens) {
        return Ok(());
    }

    if tokens.refresh_token.is_empty() {
        clear_tokens();
        return Ok(());
    }

    match refresh_access_token(&tokens).await {
        Ok(new_tokens) => {
            save_tokens(&new_tokens)?;
        }
        Err(_) => {
            // Refresh failed — clear stale tokens so we fall back to API keys
            clear_tokens();
            if !json {
                eprint_line("Warning: OAuth token refresh failed. Tokens cleared.");
                eprint_line(
                    "Run `clickhousectl cloud auth login` to re-authenticate, or use API keys.",
                );
            }
        }
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
        assert_eq!(
            Cli::try_parse_from(["clickhousectl", "cloud", "auth", "whoami", "extra"])
                .err()
                .unwrap()
                .kind(),
            clap::error::ErrorKind::UnknownArgument
        );
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
    fn test_token_serialization() {
        let tokens = TokenStore {
            access_token: "access123".to_string(),
            refresh_token: "refresh456".to_string(),
            expires_at: 1700000000,
            api_url: "https://api.clickhouse.cloud/v1".into(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let parsed: TokenStore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "access123");
        assert_eq!(parsed.refresh_token, "refresh456");
        assert_eq!(parsed.expires_at, 1700000000);
        assert_eq!(parsed.api_url, "https://api.clickhouse.cloud/v1");
    }

    #[test]
    fn test_token_serialization_with_custom_url() {
        let tokens = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: 1700000000,
            api_url: "https://api.control-plane.clickhouse-staging.com/v1".into(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        assert!(json.contains("clickhouse-staging.com"));
        let parsed: TokenStore = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.api_url,
            "https://api.control-plane.clickhouse-staging.com/v1"
        );
    }

    #[test]
    fn test_token_deserialization_defaults_to_production() {
        let json = r#"{"access_token":"a","refresh_token":"r","expires_at":1700000000}"#;
        let parsed: TokenStore = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.api_url, "https://api.clickhouse.cloud/v1");
    }

    #[test]
    fn test_token_validity() {
        let now = chrono::Utc::now().timestamp();

        let valid = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now + 3600,
            api_url: DEFAULT_API_URL.into(),
        };
        assert!(is_token_valid(&valid));

        let expired = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now - 10,
            api_url: DEFAULT_API_URL.into(),
        };
        assert!(!is_token_valid(&expired));

        let near_expiry = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now + 30, // within 60s buffer
            api_url: DEFAULT_API_URL.into(),
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
        }
    }

    #[test]
    fn load_tokens_migrates_legacy_file_when_global_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("global/.clickhouse/tokens.json");
        let legacy = tmp.path().join("legacy/.clickhouse/tokens.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();

        let tokens = sample_tokens();
        std::fs::write(&legacy, serde_json::to_string_pretty(&tokens).unwrap()).unwrap();

        let loaded = load_tokens_from(&global, &legacy).expect("legacy file should migrate");
        assert_eq!(loaded.access_token, "a");

        // Migration writes the global copy and deletes the legacy file.
        assert!(global.exists(), "global tokens file should be created");
        assert!(
            std::fs::read_to_string(&global)
                .unwrap()
                .contains("\"access_token\": \"a\""),
            "global file should hold the migrated tokens"
        );
        assert!(!legacy.exists(), "legacy file should be deleted");
    }

    #[test]
    fn load_tokens_prefers_global_over_legacy() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("global/.clickhouse/tokens.json");
        let legacy = tmp.path().join("legacy/.clickhouse/tokens.json");
        std::fs::create_dir_all(global.parent().unwrap()).unwrap();
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();

        let mut global_tokens = sample_tokens();
        global_tokens.access_token = "global-wins".into();
        std::fs::write(
            &global,
            serde_json::to_string_pretty(&global_tokens).unwrap(),
        )
        .unwrap();

        let mut legacy_tokens = sample_tokens();
        legacy_tokens.access_token = "legacy-ignored".into();
        std::fs::write(
            &legacy,
            serde_json::to_string_pretty(&legacy_tokens).unwrap(),
        )
        .unwrap();

        let loaded = load_tokens_from(&global, &legacy).expect("global file should load");
        assert_eq!(loaded.access_token, "global-wins");
        assert!(legacy.exists(), "legacy file must not be touched");
    }

    #[test]
    fn load_tokens_returns_none_when_both_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("global/.clickhouse/tokens.json");
        let legacy = tmp.path().join("legacy/.clickhouse/tokens.json");
        assert!(load_tokens_from(&global, &legacy).is_none());
    }

    #[test]
    fn load_tokens_returns_none_when_both_unparseable() {
        let tmp = tempfile::tempdir().unwrap();
        let global = tmp.path().join("global/.clickhouse/tokens.json");
        let legacy = tmp.path().join("legacy/.clickhouse/tokens.json");
        std::fs::create_dir_all(global.parent().unwrap()).unwrap();
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&global, b"not json").unwrap();
        std::fs::write(&legacy, b"also not json").unwrap();
        assert!(load_tokens_from(&global, &legacy).is_none());
    }

    #[test]
    fn save_tokens_creates_parent_dirs_and_sets_permissions() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/dir/tokens.json");
        save_tokens_to(&path, &sample_tokens()).unwrap();
        assert!(path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "tokens file should be 0o600");
        }
    }

    #[test]
    fn test_auth_config_lookup() {
        assert!(auth_config_for_url("https://api.clickhouse.cloud/v1").is_some());
        assert!(
            auth_config_for_url("https://api.control-plane.clickhouse-staging.com/v1").is_some()
        );
        assert!(auth_config_for_url("https://api.control-plane.clickhouse-dev.com/v1").is_some());
        assert!(auth_config_for_url("https://api.unknown.com/v1").is_none());

        // Verify distinct configs
        let prod = auth_config_for_url("https://api.clickhouse.cloud/v1").unwrap();
        let staging =
            auth_config_for_url("https://api.control-plane.clickhouse-staging.com/v1").unwrap();
        let dev = auth_config_for_url("https://api.control-plane.clickhouse-dev.com/v1").unwrap();
        assert_ne!(prod.client_id, staging.client_id);
        assert_ne!(prod.client_id, dev.client_id);
        assert_ne!(staging.client_id, dev.client_id);

        // Must not match on substring — these are hostile URLs that embed a known host
        assert!(auth_config_for_url("https://api.clickhouse.cloud.evil.com/v1").is_none());
        assert!(auth_config_for_url("https://evil-api.clickhouse.cloud.attacker.com/v1").is_none());
        assert!(auth_config_for_url("https://not-api.clickhouse-staging.com.bad.com/v1").is_none());
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
            let warning = report.warning.unwrap();
            assert!(
                warning.starts_with("could not fetch your identity: "),
                "{warning}"
            );
        }
    }
}
