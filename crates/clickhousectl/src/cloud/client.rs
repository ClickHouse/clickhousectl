use crate::cloud::output::{CloudErrorCode, CloudErrorDetail};
use crate::dotenv::DotenvVars;
use crate::failure::{ApiFailure, FailureKind, FailureStage};
use std::env;

pub(crate) const DEFAULT_BASE_URL: &str = "https://api.clickhouse.cloud/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CloudErrorKind {
    #[default]
    Generic,
    Auth,
    Usage,
}

#[derive(Debug)]
pub struct CloudError {
    pub message: String,
    pub kind: CloudErrorKind,
    /// Structural classification of the failure behind this error (#450),
    /// resolved from the library's typed error variant at the conversion
    /// boundary or from the local operation that failed — never from the
    /// message. `None` when no boundary claimed it, which
    /// [`CloudError::at_stage`] reports as [`FailureKind::Other`].
    pub failure: Option<ApiFailure>,
    /// Machine-readable form of this failure, for `--json` mode (#644).
    /// `None` for the ordinary case, where the message is the whole error;
    /// `Some` when the remedy is structured enough that an agent should not
    /// have to parse prose for it.
    pub details: Option<Box<CloudErrorDetail>>,
}

impl CloudError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: CloudErrorKind::Generic,
            failure: None,
            details: None,
        }
    }

    pub fn auth(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: CloudErrorKind::Auth,
            failure: None,
            details: None,
        }
    }

    /// Invalid user input discovered while reading command inputs.
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: CloudErrorKind::Usage,
            ..Self::new(message)
        }
    }

    /// Attach the classification of the failure this error stands for.
    pub fn with_failure(mut self, failure: ApiFailure) -> Self {
        self.failure = Some(failure);
        self
    }

    /// Attach the machine-readable form of this failure, which `--json` mode
    /// emits instead of the prose message.
    pub fn with_details(mut self, details: CloudErrorDetail) -> Self {
        self.details = Some(Box::new(details));
        self
    }

    /// Record this error against the stage whose boundary is returning it,
    /// and hand it back unchanged — the shape `map_err` wants:
    ///
    /// ```ignore
    /// client.create_api_key(org_id, &request)
    ///     .await
    ///     .map_err(|error| error.at_stage(FailureStage::KeyCreate))?;
    /// ```
    ///
    /// The stage comes from the call site (which owns it) and the kind from
    /// the error's own structural classification, so no category is ever
    /// derived from the message. Recording is first-write-wins, so wrapping an
    /// already-classified error at a coarser boundary is safe.
    pub fn at_stage(self, stage: FailureStage) -> Self {
        crate::failure::record(
            stage,
            self.failure
                .unwrap_or_else(|| ApiFailure::new(FailureKind::Other)),
        );
        self
    }
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CloudError {}

impl From<std::io::Error> for CloudError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string()).with_failure(ApiFailure::new(FailureKind::Io))
    }
}

impl CloudError {
    /// The file a flag names could not be read. The message names the flag
    /// and the path and keeps the OS reason, so a command with several file
    /// flags says which one failed; the failure stays [`FailureKind::Io`].
    pub fn file_read(flag: &str, path: &std::path::Path, error: std::io::Error) -> Self {
        Self::new(format!(
            "failed to read {flag} '{}': {error}",
            path.display()
        ))
        .with_failure(ApiFailure::new(FailureKind::Io))
    }
}

/// Read the file `flag` names as UTF-8 text; see [`CloudError::file_read`].
pub(crate) fn read_flag_file(flag: &str, path: impl AsRef<std::path::Path>) -> Result<String> {
    let path = path.as_ref();
    std::fs::read_to_string(path).map_err(|error| CloudError::file_read(flag, path, error))
}

/// Read the file `flag` names as bytes; see [`CloudError::file_read`].
pub(crate) fn read_flag_file_bytes(
    flag: &str,
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|error| CloudError::file_read(flag, path, error))
}

impl From<serde_json::Error> for CloudError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string()).with_failure(ApiFailure::new(FailureKind::Other))
    }
}

pub type Result<T> = std::result::Result<T, CloudError>;

enum AuthMode {
    Basic { key: String, secret: String },
    Bearer,
}

/// The resolved credential source that won precedence for a `CloudClient`.
///
/// Useful for debugging "which credential did we actually use?" questions.
/// See `CloudClient::auth_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
    /// `--api-key` / `--api-secret` CLI flags
    CliFlags,
    /// Project-local `.clickhouse/credentials.json`
    CredentialsFile,
    /// `CLICKHOUSE_CLOUD_API_KEY` / `CLICKHOUSE_CLOUD_API_SECRET` env vars
    EnvVars,
    /// OAuth tokens saved by `cloud auth login` (`~/.clickhouse/tokens.json`)
    OAuthTokens,
}

/// Credentials picked by the precedence ladder, paired with the auth scheme
/// the lib client should be built with.
enum ResolvedCreds {
    Basic { key: String, secret: String },
    Bearer { token: String },
}

/// One winning credential set: the keys/token, the source label, and the
/// API base URL the caller should talk to.
struct ResolvedAuth {
    creds: ResolvedCreds,
    source: AuthSource,
    base_url: String,
}

/// Lookup function for reading process environment variables. Production
/// callers pass a wrapper around `std::env::var`; tests pass a closure over
/// a synthetic map so precedence can be exercised without touching the real
/// environment (which would race with concurrently-running tests calling
/// `env::var`, the very reason `set_var` is `unsafe` in edition 2024).
type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

fn real_env_lookup(key: &str) -> Option<String> {
    env::var(key).ok()
}

/// Loader for the `.clickhouse/credentials.json` tier. Production passes
/// `credentials::load_credentials`; tests pass a closure over a synthetic
/// value (or `None`). Injected for the same reason as `EnvLookup`: the file
/// lives under the process cwd, which `cargo test` does not isolate, so a
/// developer's saved project credentials would otherwise win precedence over
/// the env tier these tests are exercising.
type CredentialsLookup<'a> = &'a dyn Fn() -> Option<crate::cloud::credentials::Credentials>;

/// Loader for the OAuth token tier (`~/.clickhouse/tokens.json`). Injected for
/// the same reason as `CredentialsLookup`.
type TokensLookup<'a> = &'a dyn Fn() -> std::result::Result<
    Option<crate::cloud::auth::TokenStore>,
    crate::cloud::auth::UnusableTokens,
>;

/// Treat empty as absent. An exported-but-empty variable (`CLICKHOUSE_..=`)
/// or a bare `KEY=` line in `.env` yields `Some("")`; collapsing it to `None`
/// here is the single chokepoint that keeps the resolver, the provenance
/// helper, and the status table from disagreeing: an empty value never
/// shadows a populated lower-precedence source, never resolves to empty
/// Basic-auth creds, and never counts as "present" in any of the three.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty())
}

fn env_or_dotenv(key: &str, dotenv: &DotenvVars, env_lookup: EnvLookup<'_>) -> Option<String> {
    non_empty(env_lookup(key)).or_else(|| non_empty(dotenv.get(key).map(String::from)))
}

fn resolve_auth(
    api_key: Option<&str>,
    api_secret: Option<&str>,
    url_override: Option<&str>,
) -> Result<ResolvedAuth> {
    resolve_auth_with_sources(
        api_key,
        api_secret,
        url_override,
        crate::dotenv::get(),
        &real_env_lookup,
        &crate::cloud::credentials::load_credentials,
        &crate::cloud::auth::load_tokens,
    )
}

/// [`resolve_auth`] without CLI flags or a URL override, reading the saved
/// OAuth login from the global file only, so resolving never writes or
/// removes a token file.
#[cfg(feature = "feedback")]
fn resolve_auth_read_only() -> Result<ResolvedAuth> {
    resolve_auth_with_sources(
        None,
        None,
        None,
        crate::dotenv::get(),
        &real_env_lookup,
        &crate::cloud::credentials::load_credentials,
        &crate::cloud::auth::load_global_tokens,
    )
}

/// Walk the precedence ladder once. Order: CLI flags, credentials file, env
/// vars (with `.env` fallback), OAuth tokens. Errors only when CLI flags
/// are half-set (key without secret or vice versa) or when nothing usable
/// is configured.
///
/// `env_lookup`, `load_credentials`, and `load_tokens` are the injection
/// points that let tests feed a controlled snapshot of every source without
/// mutating the process environment or reading the real `.clickhouse/` files
/// (credentials.json under cwd, tokens.json under the home dir) that `cargo
/// test` does not isolate.
fn resolve_auth_with_sources(
    api_key: Option<&str>,
    api_secret: Option<&str>,
    url_override: Option<&str>,
    dotenv: &DotenvVars,
    env_lookup: EnvLookup<'_>,
    load_credentials: CredentialsLookup<'_>,
    load_tokens: TokensLookup<'_>,
) -> Result<ResolvedAuth> {
    let normalized_default = || {
        url_override
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    };

    if api_key.is_some() || api_secret.is_some() {
        let key = api_key.map(String::from).ok_or_else(|| {
            CloudError::auth("API key required when --api-key or --api-secret is set")
        })?;
        let secret = api_secret.map(String::from).ok_or_else(|| {
            CloudError::auth("API secret required when --api-key or --api-secret is set")
        })?;
        return Ok(ResolvedAuth {
            creds: ResolvedCreds::Basic { key, secret },
            source: AuthSource::CliFlags,
            base_url: normalized_default(),
        });
    }

    if let Some(creds) = load_credentials()
        && let Some((key, secret)) = creds.api_credentials()
    {
        return Ok(ResolvedAuth {
            creds: ResolvedCreds::Basic {
                key: key.to_owned(),
                secret: secret.to_owned(),
            },
            source: AuthSource::CredentialsFile,
            base_url: saved_key_base_url(&creds, url_override)?,
        });
    }

    let env_key = env_or_dotenv("CLICKHOUSE_CLOUD_API_KEY", dotenv, env_lookup);
    let env_secret = env_or_dotenv("CLICKHOUSE_CLOUD_API_SECRET", dotenv, env_lookup);
    if let (Some(key), Some(secret)) = (env_key, env_secret) {
        return Ok(ResolvedAuth {
            creds: ResolvedCreds::Basic { key, secret },
            source: AuthSource::EnvVars,
            base_url: normalized_default(),
        });
    }

    let tokens =
        load_tokens().map_err(|_| CloudError::auth(crate::cloud::auth::OAUTH_LOGIN_REQUIRED))?;
    if let Some(tokens) = tokens {
        // `cloud::run` has already tried to refresh an expired login and kept
        // the file; name the expiry rather than claiming nothing is saved.
        if !crate::cloud::auth::is_token_valid(&tokens) {
            return Err(CloudError::auth(crate::cloud::auth::OAUTH_LOGIN_EXPIRED));
        }
        let base_url = url_override
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or(tokens.api_url);
        return Ok(ResolvedAuth {
            creds: ResolvedCreds::Bearer {
                token: tokens.access_token,
            },
            source: AuthSource::OAuthTokens,
            base_url,
        });
    }

    Err(CloudError::auth(
        "No credentials found. Run `clickhousectl cloud auth login` (OAuth, read-only), `clickhousectl cloud auth login --api-key KEY --api-secret SECRET` (read/write), set CLICKHOUSE_CLOUD_API_KEY + CLICKHOUSE_CLOUD_API_SECRET (also picked up from a `.env` file in the current directory), or use --api-key/--api-secret.\n\nLearn how to create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl",
    ))
}

/// The API a saved key pair is sent to (#1202). A key only works against the
/// API it was minted for, so a `--url` naming any other API is refused rather
/// than sending the key there; it does not fall through to a lower tier.
fn saved_key_base_url(
    creds: &crate::cloud::credentials::Credentials,
    url_override: Option<&str>,
) -> Result<String> {
    let saved = creds.api_base_url();
    match url_override.map(crate::cloud::auth::normalize_api_url) {
        Some(requested) if requested != saved => Err(CloudError::auth(format!(
            "The API key in {} is for {saved}, not {requested}. \
             Pass `--url {saved}` to use it, or save a key for this endpoint with \
             `clickhousectl cloud auth login --api-key KEY --api-secret SECRET --url {requested}`.",
            crate::cloud::credentials::credentials_path().display()
        ))),
        _ => Ok(saved),
    }
}

/// The stderr note for a command that sends a saved key to the non-production
/// API recorded with it, so the endpoint is never a surprise. `None` when the
/// user named the URL, or it is production.
pub(crate) fn stored_api_url_notice(
    source: AuthSource,
    url_override: Option<&str>,
    base_url: &str,
) -> Option<String> {
    (source == AuthSource::CredentialsFile
        && url_override.is_none()
        && base_url != DEFAULT_BASE_URL)
        .then(|| format!("note: using {base_url}, the API saved with this project's API key"))
}

/// Peek which credential source would win precedence right now without
/// actually building a `CloudClient`.
///
/// Used by `cloud auth status`, which has to render correctly even when no
/// credentials are configured (the case `CloudClient::new` errors out on).
/// Returns `None` if nothing usable is configured.
pub fn resolve_active_auth_source(
    api_key: Option<&str>,
    api_secret: Option<&str>,
) -> Option<AuthSource> {
    resolve_auth(api_key, api_secret, None)
        .ok()
        .map(|r| r.source)
}

/// The path of the `.env` file that supplied env-tier credentials, if any.
///
/// Returns `Some(path)` only when **both** `CLICKHOUSE_CLOUD_API_KEY` and
/// `CLICKHOUSE_CLOUD_API_SECRET` are absent from the real environment and
/// present in `.env`. If one is exported and the other comes from `.env`,
/// provenance is mixed and we return `None` so labels don't imply the file
/// was the sole source.
pub fn dotenv_env_provenance() -> Option<std::path::PathBuf> {
    dotenv_env_provenance_with_sources(crate::dotenv::get(), &real_env_lookup)
}

fn dotenv_env_provenance_with_sources(
    dotenv: &DotenvVars,
    env_lookup: EnvLookup<'_>,
) -> Option<std::path::PathBuf> {
    let real_key = non_empty(env_lookup("CLICKHOUSE_CLOUD_API_KEY")).is_some();
    let real_secret = non_empty(env_lookup("CLICKHOUSE_CLOUD_API_SECRET")).is_some();
    let dotenv_key = non_empty(dotenv.get("CLICKHOUSE_CLOUD_API_KEY").map(String::from)).is_some();
    let dotenv_secret =
        non_empty(dotenv.get("CLICKHOUSE_CLOUD_API_SECRET").map(String::from)).is_some();
    if !real_key && !real_secret && dotenv_key && dotenv_secret {
        dotenv.source_path().map(|p| p.to_path_buf())
    } else {
        None
    }
}

/// Per-key presence of env-tier credentials (shell env with `.env` fallback),
/// computed through the same `env_or_dotenv` merge the resolver uses so the
/// `cloud auth status` table can never disagree with which source actually
/// wins precedence. Empty values count as absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvCredPresence {
    pub key: bool,
    pub secret: bool,
}

pub fn env_cred_presence() -> EnvCredPresence {
    env_cred_presence_with_sources(crate::dotenv::get(), &real_env_lookup)
}

fn env_cred_presence_with_sources(
    dotenv: &DotenvVars,
    env_lookup: EnvLookup<'_>,
) -> EnvCredPresence {
    EnvCredPresence {
        key: env_or_dotenv("CLICKHOUSE_CLOUD_API_KEY", dotenv, env_lookup).is_some(),
        secret: env_or_dotenv("CLICKHOUSE_CLOUD_API_SECRET", dotenv, env_lookup).is_some(),
    }
}

impl AuthSource {
    /// Short label for the source (useful for tables / compact output).
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            AuthSource::CliFlags => "CLI flags",
            AuthSource::CredentialsFile => "Credentials file",
            AuthSource::EnvVars => "Env vars",
            AuthSource::OAuthTokens => "OAuth",
        }
    }

    /// One-line description including the concrete source (flag, path, env var names).
    pub fn describe(&self) -> String {
        match self {
            AuthSource::CliFlags => "CLI flags (--api-key, --api-secret)".to_string(),
            AuthSource::CredentialsFile => format!(
                "credentials file ({})",
                crate::cloud::credentials::credentials_path().display()
            ),
            AuthSource::EnvVars => {
                let base =
                    "environment variables (CLICKHOUSE_CLOUD_API_KEY, CLICKHOUSE_CLOUD_API_SECRET)";
                match dotenv_env_provenance() {
                    Some(path) => format!("{base} (loaded from {})", path.display()),
                    None => base.to_string(),
                }
            }
            AuthSource::OAuthTokens => format!(
                "OAuth tokens ({})",
                crate::cloud::auth::tokens_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "~/.clickhouse/tokens.json".to_string())
            ),
        }
    }
}

// ── by-identifier requests (issue #666) ───────────────────────────────────
//
// The API answers HTTP 400 `Invalid <thing> id string:"<id>"` for a
// syntactically valid UUID that resolves to nothing, so `cloud postgres get
// 00000000-0000-0000-0000-000000000000` reported a bad request rather than a
// missing resource. The fix is structural, not textual: nothing here reads the
// response prose (the message interpolates the id, so it could never become a
// typed library variant either). A by-identifier request has exactly one
// class of user-controlled input — the identifiers the CLI itself formatted
// into the URL path — so when every one of those is a well-formed UUID, "the
// identifier is invalid" is not what happened.
//
// A malformed identifier keeps the server's answer verbatim: there, "invalid"
// is both true and the more useful thing to say.

// The same not-found shape covers every other by-identifier path (#1237),
// keyed only on the typed HTTP 404 the API now returns for an identifier that
// resolves to nothing. Those paths never take the 400 arm above (#1226).

/// Which kind of resource a by-identifier request or a name lookup was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Service,
    PostgresService,
    Organization,
    Backup,
    Snapshot,
    ClickPipe,
    ReversePrivateEndpoint,
    SavedQuery,
    QueryApiEndpoint,
    ClickHouseSetting,
    JwtProvider,
    Udf,
    Member,
    Invitation,
    ApiKey,
    OrganizationRole,
    Byoc,
    Activity,
    ClickStackSource,
    ClickStackRole,
    ClickStackDashboard,
    ClickStackAlert,
    ClickStackWebhook,
    ClickStackSavedSearch,
}

impl ResourceKind {
    /// The noun the not-found message names the resource with.
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::PostgresService => "Postgres service",
            Self::Organization => "organization",
            Self::Backup => "backup",
            Self::Snapshot => "snapshot",
            Self::ClickPipe => "ClickPipe",
            Self::ReversePrivateEndpoint => "reverse private endpoint",
            Self::SavedQuery => "saved query",
            Self::QueryApiEndpoint => "Query API endpoint",
            Self::ClickHouseSetting => "ClickHouse setting",
            Self::JwtProvider => "JWT provider",
            Self::Udf => "UDF",
            Self::Member => "member",
            Self::Invitation => "invitation",
            Self::ApiKey => "API key",
            Self::OrganizationRole => "organization role",
            Self::Byoc => "BYOC infrastructure",
            Self::Activity => "activity",
            Self::ClickStackSource => "ClickStack source",
            Self::ClickStackRole => "ClickStack role",
            Self::ClickStackDashboard => "ClickStack dashboard",
            Self::ClickStackAlert => "ClickStack alert",
            Self::ClickStackWebhook => "ClickStack webhook",
            Self::ClickStackSavedSearch => "ClickStack saved search",
        }
    }

    /// The command that shows what does exist. BYOC infrastructure has no
    /// list of its own: `org get` shows the organization's `byocConfig`.
    const fn list_base(self) -> &'static str {
        match self {
            Self::Service => "clickhousectl cloud service list",
            Self::PostgresService => "clickhousectl cloud postgres list",
            Self::Organization => "clickhousectl cloud org list",
            Self::Backup => "clickhousectl cloud backup list",
            Self::Snapshot => "clickhousectl cloud service snapshot list",
            Self::ClickPipe => "clickhousectl cloud clickpipe list",
            Self::ReversePrivateEndpoint => {
                "clickhousectl cloud clickpipe reverse-private-endpoint list"
            }
            Self::SavedQuery => "clickhousectl cloud saved-query list",
            Self::QueryApiEndpoint => "clickhousectl cloud query-api-endpoint list",
            Self::ClickHouseSetting => "clickhousectl cloud service settings list",
            Self::JwtProvider => "clickhousectl cloud service jwt-provider list",
            Self::Udf => "clickhousectl cloud udf list",
            Self::Member => "clickhousectl cloud member list",
            Self::Invitation => "clickhousectl cloud invitation list",
            Self::ApiKey => "clickhousectl cloud key list",
            Self::OrganizationRole => "clickhousectl cloud org role list",
            Self::Byoc => "clickhousectl cloud org get",
            Self::Activity => "clickhousectl cloud activity list",
            Self::ClickStackSource => "clickhousectl cloud clickstack source list",
            Self::ClickStackRole => "clickhousectl cloud clickstack role list",
            Self::ClickStackDashboard => "clickhousectl cloud clickstack dashboard list",
            Self::ClickStackAlert => "clickhousectl cloud clickstack alert list",
            Self::ClickStackWebhook => "clickhousectl cloud clickstack webhook list",
            Self::ClickStackSavedSearch => "clickhousectl cloud clickstack saved-search list",
        }
    }

    /// The list command for resources of this kind in `scope`, for the
    /// structured detail.
    pub fn list_command(self, scope: Scope<'_>) -> String {
        let base = self.list_base();
        match scope {
            // Keyed on the kind too: organizations are listed without a
            // scope flag whatever scope the caller recorded.
            _ if self == Self::Organization => base.to_string(),
            Scope::Account => base.to_string(),
            Scope::Organization(org_id) => format!("{base} --org-id {org_id}"),
            Scope::Service { org_id, service_id } => {
                format!("{base} {service_id} --org-id {org_id}")
            }
        }
    }
}

/// Where a resource lives: what its list command and message are scoped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope<'a> {
    /// Above every organization: only organizations live here.
    Account,
    Organization(&'a str),
    Service {
        org_id: &'a str,
        service_id: &'a str,
    },
}

impl Scope<'_> {
    /// The trailing scope clause of a not-found message.
    fn clause(self) -> String {
        match self {
            Scope::Account => String::new(),
            Scope::Organization(org_id) => format!(" (organization {org_id})"),
            Scope::Service { org_id, service_id } => {
                format!(" (service {service_id}, organization {org_id})")
            }
        }
    }
}

/// A `--name` (or `--email`) lookup that matched nothing in `scope`, reported
/// the way a by-identifier miss is (#1237).
pub fn name_not_found(kind: ResourceKind, scope: Scope<'_>, name: &str) -> CloudError {
    let message = format!("no {} found matching '{name}'", kind.noun());
    CloudError::new(message.clone()).with_details(CloudErrorDetail {
        code: CloudErrorCode::ResourceNotFound,
        message,
        host: None,
        port: None,
        command: Some(kind.list_command(scope)),
        api_key_id: None,
        ip_access_list: None,
    })
}

/// The identifiers a by-identifier request put into its request path.
///
/// Built only through [`ResourceLookup::in_org`],
/// [`ResourceLookup::in_service`] and [`ResourceLookup::organization`], so
/// the kind and the scope cannot disagree: an organization lookup is always
/// scoped to itself, and every other kind is always scoped to the
/// organization (and service) it was read from.
pub struct ResourceLookup<'a> {
    kind: ResourceKind,
    /// The resource's own identifier, as the user supplied it.
    id: &'a str,
    scope: Scope<'a>,
}

impl<'a> ResourceLookup<'a> {
    /// A resource looked up inside an organization: both identifiers are in
    /// the request path.
    pub fn in_org(kind: ResourceKind, id: &'a str, org_id: &'a str) -> Self {
        Self {
            kind,
            id,
            scope: Scope::Organization(org_id),
        }
    }

    /// A resource looked up inside a service, whose path also carries the
    /// service and its organization.
    pub fn in_service(
        kind: ResourceKind,
        id: &'a str,
        org_id: &'a str,
        service_id: &'a str,
    ) -> Self {
        Self {
            kind,
            id,
            scope: Scope::Service { org_id, service_id },
        }
    }

    /// An organization looked up by its own identifier, which is therefore
    /// also the only scope the request had.
    pub fn organization(id: &'a str) -> Self {
        Self {
            kind: ResourceKind::Organization,
            id,
            scope: Scope::Account,
        }
    }

    /// The organization the request was scoped to: the looked-up id itself
    /// for an organization lookup.
    fn org_id(&self) -> &'a str {
        match self.scope {
            Scope::Account => self.id,
            Scope::Organization(org_id) | Scope::Service { org_id, .. } => org_id,
        }
    }

    /// Whether the API rejected a request whose every path identifier is a
    /// well-formed UUID: 400 for an unknown ID (#666), or 404 for a deleted
    /// resource (#831). Use only when the path identifies the resource itself
    /// and the request has no other user-controlled input, never a subresource.
    ///
    /// The single implementation of that judgement: both
    /// [`CloudClient::convert_error_for_lookup`] and the callers that turn
    /// the rejection into an absent resource read it from here.
    pub fn rejected_well_formed_ids(&self, err: &clickhouse_cloud_api::Error) -> bool {
        matches!(
            err,
            clickhouse_cloud_api::Error::Api {
                status: 400 | 404,
                ..
            }
        ) && self
            .identifiers()
            .all(|id| uuid::Uuid::parse_str(id).is_ok())
    }

    /// The trailing scope clause of the not-found message. Keyed on the
    /// kind, not only the scope, so the message never reads
    /// "organization X (organization X)".
    fn scope_clause(&self) -> String {
        match self.kind {
            ResourceKind::Organization => String::new(),
            _ => self.scope.clause(),
        }
    }

    /// Every identifier the CLI formatted into the request path.
    fn identifiers(&self) -> impl Iterator<Item = &str> {
        let (org_id, service_id) = match self.scope {
            Scope::Account => (None, None),
            Scope::Organization(org_id) => (Some(org_id), None),
            Scope::Service { org_id, service_id } => (Some(org_id), Some(service_id)),
        };
        std::iter::once(self.id).chain(org_id).chain(service_id)
    }
}

/// Whether the API answered the typed "no such resource" status.
pub(crate) fn is_not_found(err: &clickhouse_cloud_api::Error) -> bool {
    matches!(err, clickhouse_cloud_api::Error::Api { status: 404, .. })
}

pub struct CloudClient {
    organization_id: Option<String>,
    organization_name: Option<String>,
    resolved_organization_id: tokio::sync::OnceCell<String>,
    lib_client: clickhouse_cloud_api::Client,
    auth_mode: AuthMode,
    auth_source: AuthSource,
    base_url: String,
}

/// Convert CLI base URL (with /v1 suffix) to library base URL (without /v1).
/// The library prefixes /v1 in its own path construction.
fn lib_base_url(cli_base_url: &str) -> String {
    cli_base_url
        .strip_suffix("/v1")
        .unwrap_or(cli_base_url)
        .to_string()
}

impl CloudClient {
    /// Set the shared cloud organization scope without performing a lookup.
    pub(super) fn with_organization_id(mut self, organization_id: Option<String>) -> Self {
        self.organization_id = organization_id;
        self.resolved_organization_id = tokio::sync::OnceCell::new();
        self
    }

    pub(super) fn with_organization_name(mut self, name: Option<String>) -> Self {
        self.organization_name = name;
        self.resolved_organization_id = tokio::sync::OnceCell::new();
        self
    }

    /// Resolve organization scope only when needed, caching successful detection.
    pub(super) async fn resolve_organization_id(&self) -> Result<String> {
        self.resolved_organization_id
            .get_or_try_init(|| async {
                match (&self.organization_id, &self.organization_name) {
                    (Some(_), Some(_)) => {
                        Err(CloudError::usage("--org-id conflicts with --org-name"))
                    }
                    (Some(id), None) => Ok(id.clone()),
                    (None, Some(name)) => {
                        let rows = self.list_organizations().await?;
                        // Names may collide, and an organization has no
                        // positional ID: only --org-id can choose one (#1200).
                        crate::cloud::shared::select_named_id_or(
                            ResourceKind::Organization,
                            Scope::Account,
                            name,
                            rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
                            |ids| format!("choose one with --org-id <ID>: {}", ids.join(", ")),
                        )
                    }
                    (None, None) => self.get_default_org_id().await,
                }
            })
            .await
            .cloned()
    }

    pub fn new(
        api_key: Option<&str>,
        api_secret: Option<&str>,
        url_override: Option<&str>,
    ) -> Result<Self> {
        Self::from_resolved_auth(resolve_auth(api_key, api_secret, url_override)?, None)
    }

    /// Like [`CloudClient::new`], but every request gives up after `timeout`,
    /// so a best-effort call cannot hang an otherwise local command.
    pub(crate) fn new_with_timeout(
        api_key: Option<&str>,
        api_secret: Option<&str>,
        url_override: Option<&str>,
        timeout: std::time::Duration,
    ) -> Result<Self> {
        Self::from_resolved_auth(
            resolve_auth(api_key, api_secret, url_override)?,
            Some(timeout),
        )
    }

    /// Like [`CloudClient::new_with_timeout`] with no flags or URL override,
    /// but resolving credentials without side effects: a legacy
    /// project-local OAuth login is ignored rather than migrated.
    #[cfg(feature = "feedback")]
    pub(crate) fn new_read_only_with_timeout(timeout: std::time::Duration) -> Result<Self> {
        Self::from_resolved_auth(resolve_auth_read_only()?, Some(timeout))
    }

    /// A client for exactly this API key pair, bypassing the precedence
    /// ladder: `auth login` verifies the pair before it is saved.
    pub(crate) fn for_api_key(
        key: &str,
        secret: &str,
        url_override: Option<&str>,
        timeout: std::time::Duration,
    ) -> Result<Self> {
        let resolved = ResolvedAuth {
            creds: ResolvedCreds::Basic {
                key: key.to_owned(),
                secret: secret.to_owned(),
            },
            source: AuthSource::CliFlags,
            base_url: url_override
                .map(crate::cloud::auth::normalize_api_url)
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        };
        Self::from_resolved_auth(resolved, Some(timeout))
    }

    /// A client for exactly these OAuth tokens, bypassing the precedence
    /// ladder: `auth login` reports who just logged in even when a
    /// higher-precedence API key would otherwise win.
    pub(crate) fn for_oauth_tokens(
        tokens: &crate::cloud::auth::TokenStore,
        timeout: std::time::Duration,
    ) -> Result<Self> {
        let resolved = ResolvedAuth {
            creds: ResolvedCreds::Bearer {
                token: tokens.access_token.clone(),
            },
            source: AuthSource::OAuthTokens,
            base_url: crate::cloud::auth::normalize_api_url(&tokens.api_url),
        };
        Self::from_resolved_auth(resolved, Some(timeout))
    }

    fn from_resolved_auth(
        resolved: ResolvedAuth,
        timeout: Option<std::time::Duration>,
    ) -> Result<Self> {
        let mut builder = crate::http::client_builder();
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
        let http = builder
            .build()
            .map_err(|e| CloudError::new(format!("Failed to create HTTP client: {}", e)))?;

        let lib_url = lib_base_url(&resolved.base_url);
        let (lib_client, auth_mode) = match resolved.creds {
            ResolvedCreds::Basic { key, secret } => (
                clickhouse_cloud_api::Client::with_http_client(
                    http,
                    lib_url,
                    key.clone(),
                    secret.clone(),
                ),
                AuthMode::Basic { key, secret },
            ),
            ResolvedCreds::Bearer { token } => (
                clickhouse_cloud_api::Client::with_http_client_bearer(http, lib_url, token),
                AuthMode::Bearer,
            ),
        };

        Ok(Self {
            lib_client,
            organization_id: None,
            organization_name: None,
            resolved_organization_id: tokio::sync::OnceCell::new(),
            auth_mode,
            auth_source: resolved.source,
            base_url: resolved.base_url,
        })
    }

    /// An API-key client against `base_url`, for unit tests that drive a
    /// handler against a local mock server.
    #[cfg(test)]
    pub(crate) fn for_tests(base_url: &str, query_host: Option<&str>) -> Self {
        let http = reqwest::Client::builder().build().unwrap();
        let mut lib_client = clickhouse_cloud_api::Client::with_http_client(
            http,
            lib_base_url(base_url),
            "test_key",
            "test_secret",
        );
        if let Some(query_host) = query_host {
            lib_client = lib_client.with_query_host(query_host);
        }
        Self {
            lib_client,
            organization_id: None,
            organization_name: None,
            resolved_organization_id: tokio::sync::OnceCell::new(),
            auth_mode: AuthMode::Basic {
                key: "test_key".into(),
                secret: "test_secret".into(),
            },
            auth_source: AuthSource::CliFlags,
            base_url: base_url.to_string(),
        }
    }

    /// Returns true if the client is using OAuth Bearer token authentication.
    /// Bearer auth is read-only and cannot perform write operations.
    pub fn is_bearer_auth(&self) -> bool {
        matches!(&self.auth_mode, AuthMode::Bearer)
    }

    /// The active API key pair, for authenticating directly to a Query API
    /// endpoint that already authorizes this key.
    pub(crate) fn basic_auth_credentials(&self) -> Option<(&str, &str)> {
        match &self.auth_mode {
            AuthMode::Basic { key, secret } => Some((key, secret)),
            AuthMode::Bearer => None,
        }
    }

    /// The credential source that won precedence when constructing this client.
    pub fn auth_source(&self) -> AuthSource {
        self.auth_source
    }

    /// The API base URL the client is talking to (includes the `/v1` suffix).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Access the library client from domain-specific wrapper methods.
    pub fn api(&self) -> &clickhouse_cloud_api::Client {
        &self.lib_client
    }

    /// Unwrap an `ApiResponse<T>` into `T`, returning an error if the result is empty.
    pub fn unwrap_response<T>(response: clickhouse_cloud_api::models::ApiResponse<T>) -> Result<T> {
        response
            .result
            .ok_or_else(|| CloudError::new("Empty response from API"))
    }

    /// Read every page of a list whose response carries a top-level `nextCursor`.
    ///
    /// `fetch` returns one page's response for the given cursor (`None` first). Pages are
    /// read until `nextCursor` is absent; an empty page or empty cursor is not a terminator,
    /// since cursors are opaque. A repeated cursor fails instead of looping; `noun` names the
    /// list in that error.
    pub async fn collect_cursor_pages<T, F, Fut>(noun: &str, mut fetch: F) -> Result<Vec<T>>
    where
        F: FnMut(Option<String>) -> Fut,
        Fut:
            std::future::Future<Output = Result<clickhouse_cloud_api::models::ApiResponse<Vec<T>>>>,
    {
        let mut items = Vec::new();
        let mut cursor = None;
        let mut seen_cursors = std::collections::HashSet::new();
        loop {
            let mut response = fetch(cursor.take()).await?;
            let next = response.next_cursor.take();
            items.extend(Self::unwrap_response(response)?);
            match next {
                None => return Ok(items),
                Some(next) if !seen_cursors.insert(next.clone()) => {
                    return Err(CloudError::new(format!(
                        "{noun} list returned a repeated cursor"
                    )));
                }
                Some(next) => cursor = Some(next),
            }
        }
    }

    /// Convert a library error into a `CloudError`.
    pub fn convert_error(&self, err: clickhouse_cloud_api::Error) -> CloudError {
        self.convert_error_with_organization(err, None)
    }

    /// Add safe request scope to otherwise context-free organization errors.
    pub fn convert_error_for_organization(
        &self,
        err: clickhouse_cloud_api::Error,
        org_id: &str,
    ) -> CloudError {
        self.convert_error_with_organization(err, Some(org_id))
    }

    /// Re-present a by-identifier request's failure as a not-found when the
    /// API rejected identifiers that are all well-formed UUIDs (#666).
    ///
    /// The discriminator is structural: the HTTP status the library reported,
    /// plus `Uuid::parse_str` over inputs the CLI already held, both read
    /// through [`ResourceLookup::rejected_well_formed_ids`]. No part of it
    /// comes from the response message, which is appended verbatim so nothing
    /// the server said is lost.
    ///
    /// Conversion still goes through [`Self::convert_error_with_organization`],
    /// so the telemetry classification (#450) is inherited unchanged and
    /// carried across the rewrite: the original HTTP status stays intact, and only the
    /// user-facing message changes.
    pub fn convert_error_for_lookup(
        &self,
        err: clickhouse_cloud_api::Error,
        lookup: ResourceLookup<'_>,
    ) -> CloudError {
        if lookup.rejected_well_formed_ids(&err) {
            self.not_found_error(err, &lookup)
        } else {
            self.convert_error_for_organization(err, lookup.org_id())
        }
    }

    /// Report a 404 from a request addressed to `lookup` as that resource
    /// being missing (#1237); any other failure converts as usual.
    ///
    /// Only the typed 404 counts, never a 400 (#1226). Use it where a 404
    /// can only mean `lookup` is missing: the resource itself, or an
    /// organization-scoped resource, whose message names the organization.
    /// A resource inside a service uses [`Self::convert_error_in_service`].
    pub fn convert_error_for_resource(
        &self,
        err: clickhouse_cloud_api::Error,
        lookup: ResourceLookup<'_>,
    ) -> CloudError {
        if is_not_found(&err) {
            self.not_found_error(err, &lookup)
        } else {
            self.convert_error_for_organization(err, lookup.org_id())
        }
    }

    /// [`Self::convert_error_for_resource`] for a resource inside a service,
    /// where a 404 can mean the resource or its service is missing. A 404
    /// reads the service to tell which; when that read fails too, the
    /// original error is reported unchanged.
    pub async fn convert_error_in_service(
        &self,
        err: clickhouse_cloud_api::Error,
        lookup: ResourceLookup<'_>,
    ) -> CloudError {
        let Scope::Service { org_id, service_id } = lookup.scope else {
            return self.convert_error_for_resource(err, lookup);
        };
        self.convert_error_under_service(err, org_id, service_id, Some(&lookup))
            .await
    }

    /// Send a request addressed to a resource inside a service, converting
    /// its failure with [`Self::convert_error_in_service`].
    pub async fn request_in_service<T>(
        &self,
        lookup: ResourceLookup<'_>,
        request: impl std::future::Future<Output = std::result::Result<T, clickhouse_cloud_api::Error>>,
    ) -> Result<T> {
        match request.await {
            Ok(response) => Ok(response),
            Err(error) => Err(self.convert_error_in_service(error, lookup).await),
        }
    }

    /// Send a request whose path names only the service but whose body may
    /// name other resources, such as a create or an update. A 404 is the
    /// service being missing only once a read of the service confirms it;
    /// otherwise the original error is reported unchanged.
    pub async fn request_on_service<T>(
        &self,
        org_id: &str,
        service_id: &str,
        request: impl std::future::Future<Output = std::result::Result<T, clickhouse_cloud_api::Error>>,
    ) -> Result<T> {
        match request.await {
            Ok(response) => Ok(response),
            Err(error) => Err(self
                .convert_error_under_service(error, org_id, service_id, None)
                .await),
        }
    }

    /// A 404 from a request under a service: the service is missing when a
    /// read of it says so, else `leaf` (when there is one) is. Any other
    /// failure, or a read that fails too, converts as usual.
    async fn convert_error_under_service(
        &self,
        err: clickhouse_cloud_api::Error,
        org_id: &str,
        service_id: &str,
        leaf: Option<&ResourceLookup<'_>>,
    ) -> CloudError {
        if !is_not_found(&err) {
            return self.convert_error_for_organization(err, org_id);
        }
        match (self.get_service_if_exists(org_id, service_id).await, leaf) {
            (Ok(None), _) => self.not_found_error(
                err,
                &ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
            ),
            (Ok(Some(_)), Some(leaf)) => self.not_found_error(err, leaf),
            _ => self.convert_error_for_organization(err, org_id),
        }
    }

    /// The not-found error for `lookup`, once the caller has decided that is
    /// what the API's rejection means. The server's message is appended
    /// verbatim, and the classification (#450) is kept.
    fn not_found_error(
        &self,
        err: clickhouse_cloud_api::Error,
        lookup: &ResourceLookup<'_>,
    ) -> CloudError {
        // The lookup message supplies its own scope; leave the server detail
        // untouched rather than appending the organization a second time.
        let error = self.convert_error_with_organization(err, None);
        let message = format!(
            "No such {}: {}{}. The API rejected the identifier: {}",
            lookup.kind.noun(),
            lookup.id,
            lookup.scope_clause(),
            error.message,
        );
        CloudError {
            message: message.clone(),
            ..error
        }
        .with_details(CloudErrorDetail {
            code: CloudErrorCode::ResourceNotFound,
            message,
            host: None,
            port: None,
            command: Some(lookup.kind.list_command(lookup.scope)),
            api_key_id: None,
            ip_access_list: None,
        })
    }

    /// The single boundary where a typed library error becomes a
    /// `CloudError`, so it is also the single place the failure
    /// classification (#450) is attached: every cloud command inherits the
    /// same variant-derived category without doing anything.
    fn convert_error_with_organization(
        &self,
        err: clickhouse_cloud_api::Error,
        org_id: Option<&str>,
    ) -> CloudError {
        let failure = crate::failure::classify_api_error(&err);
        self.convert_error_message(err, org_id)
            .with_failure(failure)
    }

    fn convert_error_message(
        &self,
        err: clickhouse_cloud_api::Error,
        org_id: Option<&str>,
    ) -> CloudError {
        match &err {
            clickhouse_cloud_api::Error::Api { status, message }
            | clickhouse_cloud_api::Error::UdfAttachmentUnavailable {
                status, message, ..
            } => {
                let mut msg = message.clone();
                let trimmed_message = message.trim();
                if *status == 404
                    && matches!(
                        trimmed_message.to_ascii_uppercase().as_str(),
                        "NOT_FOUND" | "NOT FOUND" | "NOT_FOUND: NOT FOUND"
                    )
                    && let Some(org_id) = org_id
                {
                    msg = format!("{trimmed_message}: request scoped to organization {org_id}");
                }
                // Only a 401 says the credentials themselves were refused. A
                // 403 is a refusal of this request (a quota, a precondition,
                // a policy or a missing grant) that logging in again cannot
                // fix (#1225), so it stays a generic `http_4xx` failure.
                // Writes under OAuth never get here: dispatch blocks them
                // first with their own API-key guidance.
                if *status == 401 {
                    CloudError::auth(msg)
                } else {
                    CloudError::new(msg)
                }
            }
            other => CloudError::new(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_LIB_BASE_URL: &str = "https://api.clickhouse.cloud";

    #[test]
    fn typed_udf_dependency_error_keeps_message_and_http_classification() {
        let error =
            test_client().convert_error(clickhouse_cloud_api::Error::UdfAttachmentUnavailable {
                status: 424,
                message: "service unavailable".into(),
                response: Box::default(),
            });
        assert_eq!(error.to_string(), "service unavailable");
        assert_eq!(
            error.failure.unwrap().kind,
            crate::failure::FailureKind::Http4xx
        );
    }

    fn test_client() -> CloudClient {
        let http = reqwest::Client::builder().build().unwrap();
        let lib_client = clickhouse_cloud_api::Client::with_http_client(
            http,
            DEFAULT_LIB_BASE_URL,
            "test_key",
            "test_secret",
        );
        CloudClient {
            lib_client,
            organization_id: None,
            organization_name: None,
            resolved_organization_id: tokio::sync::OnceCell::new(),
            auth_mode: AuthMode::Basic {
                key: "test_key".into(),
                secret: "test_secret".into(),
            },
            auth_source: AuthSource::CliFlags,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    #[tokio::test]
    async fn shared_organization_scope_is_lazy_and_caches_detection() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        let client = CloudClient::for_tests(&server.uri(), None);
        assert!(server.received_requests().await.unwrap().is_empty());
        let organization = "00000000-0000-0000-0000-000000000001";
        Mock::given(method("GET"))
            .and(path("/v1/organizations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [{"id": organization}]
            })))
            .expect(1)
            .mount(&server)
            .await;
        for _ in 0..2 {
            assert_eq!(
                client.resolve_organization_id().await.unwrap(),
                organization
            );
        }
        let client = client.with_organization_id(Some("explicit-org".into()));
        assert_eq!(
            client.resolve_organization_id().await.unwrap(),
            "explicit-org"
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[test]
    fn is_bearer_auth_returns_true_for_bearer() {
        let http = reqwest::Client::builder().build().unwrap();
        let lib_client = clickhouse_cloud_api::Client::with_http_client_bearer(
            http,
            DEFAULT_LIB_BASE_URL,
            "test_token",
        );
        let client = CloudClient {
            lib_client,
            organization_id: None,
            organization_name: None,
            resolved_organization_id: tokio::sync::OnceCell::new(),
            auth_mode: AuthMode::Bearer,
            auth_source: AuthSource::OAuthTokens,
            base_url: DEFAULT_BASE_URL.to_string(),
        };
        assert!(client.is_bearer_auth());
    }

    #[test]
    fn is_bearer_auth_returns_false_for_basic() {
        let client = test_client();
        assert!(!client.is_bearer_auth());
    }

    #[test]
    fn basic_auth_credentials_returns_the_active_pair() {
        let client = test_client();
        assert_eq!(
            client.basic_auth_credentials(),
            Some(("test_key", "test_secret"))
        );
    }

    #[test]
    fn lib_base_url_strips_v1_suffix() {
        assert_eq!(
            lib_base_url("https://api.clickhouse.cloud/v1"),
            "https://api.clickhouse.cloud"
        );
    }

    #[test]
    fn lib_base_url_preserves_url_without_v1() {
        assert_eq!(
            lib_base_url("https://api.clickhouse.cloud"),
            "https://api.clickhouse.cloud"
        );
    }

    #[test]
    fn lib_base_url_strips_v1_from_staging() {
        assert_eq!(
            lib_base_url("https://api.control-plane.clickhouse-staging.com/v1"),
            "https://api.control-plane.clickhouse-staging.com"
        );
    }

    #[test]
    fn api_returns_library_client_ref() {
        let client = test_client();
        // Verify api() returns a reference without panicking
        let _api = client.api();
    }

    #[test]
    fn unwrap_response_extracts_result() {
        let response = clickhouse_cloud_api::models::ApiResponse {
            status: Some(200),
            request_id: None,
            result: Some(vec!["hello".to_string()]),
            error: None,
            ..Default::default()
        };
        let result = CloudClient::unwrap_response(response).unwrap();
        assert_eq!(result, vec!["hello".to_string()]);
    }

    fn cursor_page(
        result: Vec<u8>,
        next: Option<&str>,
    ) -> Result<clickhouse_cloud_api::models::ApiResponse<Vec<u8>>> {
        Ok(clickhouse_cloud_api::models::ApiResponse {
            result: Some(result),
            next_cursor: next.map(str::to_owned),
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn collect_cursor_pages_follows_opaque_cursors_until_absent() {
        let mut requested = Vec::new();
        let items = CloudClient::collect_cursor_pages("thing", |cursor| {
            requested.push(cursor.clone());
            let page = match cursor.as_deref() {
                None => cursor_page(vec![1], Some("")),
                Some("") => cursor_page(vec![], Some("last")),
                Some("last") => cursor_page(vec![2, 3], None),
                Some(other) => panic!("unexpected cursor {other}"),
            };
            async move { page }
        })
        .await
        .unwrap();
        assert_eq!(items, vec![1, 2, 3]);
        assert_eq!(
            requested,
            vec![None, Some(String::new()), Some("last".to_owned())]
        );
    }

    #[tokio::test]
    async fn collect_cursor_pages_rejects_repeats_and_propagates_errors() {
        let mut calls = 0;
        let error = CloudClient::collect_cursor_pages("thing", |_| {
            calls += 1;
            // Bound the fake so a regression fails instead of paging forever.
            assert!(calls <= 2, "paged past a repeated cursor");
            async { cursor_page(vec![], Some("same")) }
        })
        .await
        .unwrap_err();
        // The repeated-cursor error names the list it was reading.
        assert!(error.message.contains("thing"), "{error}");
        assert_eq!(calls, 2);

        let error = CloudClient::collect_cursor_pages::<u8, _, _>("thing", |cursor| async move {
            match cursor {
                None => cursor_page(vec![1], Some("next")),
                Some(_) => Err(CloudError::new("page two failed")),
            }
        })
        .await
        .unwrap_err();
        assert_eq!(error.message, "page two failed");

        let error = CloudClient::collect_cursor_pages::<u8, _, _>("thing", |_| async {
            Ok(clickhouse_cloud_api::models::ApiResponse::default())
        })
        .await
        .unwrap_err();
        // An empty page body fails through the same unwrap as a single read.
        let unwrap_error = CloudClient::unwrap_response(
            clickhouse_cloud_api::models::ApiResponse::<Vec<u8>>::default(),
        )
        .unwrap_err();
        assert_eq!(error.message, unwrap_error.message);
        assert_eq!(error.kind, CloudErrorKind::Generic);
    }

    #[test]
    fn unwrap_response_errors_on_empty_result() {
        let response: clickhouse_cloud_api::models::ApiResponse<String> =
            clickhouse_cloud_api::models::ApiResponse {
                status: Some(200),
                request_id: None,
                result: None,
                error: None,
                ..Default::default()
            };
        let err = CloudClient::unwrap_response(response).unwrap_err();
        assert_eq!(err.kind, CloudErrorKind::Generic);
        assert_eq!(err.failure, None);
    }

    /// A 403 refuses the request, not the credentials (#1225): under an API
    /// key or OAuth it stays generic, with the server's text as the message.
    #[test]
    fn convert_error_reports_403_as_a_generic_refusal() {
        let http = reqwest::Client::builder().build().unwrap();
        let bearer = CloudClient {
            lib_client: clickhouse_cloud_api::Client::with_http_client_bearer(
                http,
                DEFAULT_LIB_BASE_URL,
                "test_token",
            ),
            organization_id: None,
            organization_name: None,
            resolved_organization_id: tokio::sync::OnceCell::new(),
            auth_mode: AuthMode::Bearer,
            auth_source: AuthSource::OAuthTokens,
            base_url: DEFAULT_BASE_URL.to_string(),
        };
        for client in [test_client(), bearer] {
            let err = client.convert_error(clickhouse_cloud_api::Error::Api {
                status: 403,
                message: "FORBIDDEN: quota".into(),
            });
            assert_eq!(err.kind, CloudErrorKind::Generic);
            assert_eq!(err.message, "FORBIDDEN: quota");
            assert_eq!(
                err.failure,
                Some(ApiFailure::with_status(FailureKind::Http4xx, 403))
            );
        }
    }

    #[test]
    fn auth_source_label_and_describe() {
        let sources = [
            AuthSource::CliFlags,
            AuthSource::CredentialsFile,
            AuthSource::EnvVars,
            AuthSource::OAuthTokens,
        ];
        let labels: std::collections::HashSet<_> = sources.iter().map(AuthSource::label).collect();
        assert_eq!(labels.len(), sources.len(), "labels must be distinct");
        assert!(labels.iter().all(|label| !label.is_empty()));

        // Each description names the concrete source it was read from.
        let cli = AuthSource::CliFlags.describe();
        assert!(
            cli.contains("--api-key") && cli.contains("--api-secret"),
            "{cli}"
        );
        let env = AuthSource::EnvVars.describe();
        assert!(
            env.contains("CLICKHOUSE_CLOUD_API_KEY") && env.contains("CLICKHOUSE_CLOUD_API_SECRET"),
            "{env}"
        );
        let file = AuthSource::CredentialsFile.describe();
        let path = crate::cloud::credentials::credentials_path();
        assert!(file.contains(&path.display().to_string()), "{file}");
        let tokens = AuthSource::OAuthTokens.describe();
        assert!(tokens.contains("tokens.json"), "{tokens}");
    }

    #[test]
    fn auth_source_accessor_returns_cli_flags_default_in_test_client() {
        let client = test_client();
        assert_eq!(client.auth_source(), AuthSource::CliFlags);
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn convert_error_flags_401_as_auth() {
        let err = test_client().convert_error(clickhouse_cloud_api::Error::Api {
            status: 401,
            message: "Unauthorized".into(),
        });
        assert_eq!(err.kind, CloudErrorKind::Auth);
    }

    #[test]
    fn convert_error_treats_other_status_as_generic() {
        let err = test_client().convert_error(clickhouse_cloud_api::Error::Api {
            status: 500,
            message: "Internal Server Error".into(),
        });
        assert_eq!(err.kind, CloudErrorKind::Generic);
    }

    /// Conversion is the single boundary where a typed library error becomes a
    /// `CloudError`, so every converted error carries its classification
    /// (#450) whether or not a stage ever records it.
    #[test]
    fn convert_error_attaches_the_failure_classification() {
        let err = test_client().convert_error(clickhouse_cloud_api::Error::Api {
            status: 429,
            message: "TOO_MANY_REQUESTS".into(),
        });
        assert_eq!(
            err.failure,
            Some(ApiFailure::with_status(FailureKind::RateLimited, 429))
        );

        // A locally-raised error has no classification until a boundary
        // claims one, and local I/O is classified by its own conversion.
        assert_eq!(CloudError::new("boom").failure, None);
        assert_eq!(
            CloudError::from(std::io::Error::other("disk gone")).failure,
            Some(ApiFailure::new(FailureKind::Io))
        );
    }

    #[test]
    fn flag_file_read_errors_name_the_flag_and_path_and_stay_io() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.pem");
        for error in [
            read_flag_file("--ca-certificate", &missing).unwrap_err(),
            read_flag_file_bytes("--ca-certificate", &missing).unwrap_err(),
        ] {
            assert_eq!(error.kind, CloudErrorKind::Generic);
            assert_eq!(error.failure, Some(ApiFailure::new(FailureKind::Io)));
            assert!(error.message.contains("--ca-certificate"), "{error}");
            assert!(error.message.contains(missing.to_str().unwrap()), "{error}");
        }
    }

    #[test]
    fn convert_error_adds_organization_scope_to_bare_not_found() {
        let err = test_client().convert_error_for_organization(
            clickhouse_cloud_api::Error::Api {
                status: 404,
                message: " NOT_FOUND\n".into(),
            },
            "00000000-0000-4000-8000-000000000001",
        );
        // The trimmed server text leads, and the organization it was scoped
        // to follows.
        assert!(err.message.starts_with("NOT_FOUND"), "{}", err.message);
        assert!(
            err.message.contains("00000000-0000-4000-8000-000000000001"),
            "{}",
            err.message
        );
        assert!(!err.message.contains('\n'), "{}", err.message);
        assert_eq!(err.kind, CloudErrorKind::Generic);
    }

    #[test]
    fn convert_error_preserves_detailed_not_found() {
        let err = test_client().convert_error_for_organization(
            clickhouse_cloud_api::Error::Api {
                status: 404,
                message: "Service svc-1 not found".into(),
            },
            "org-1",
        );
        assert_eq!(err.message, "Service svc-1 not found");
    }

    // ── by-identifier reads (issue #666) ──────────────────────────────────
    //
    // The API answers 400 with "Invalid <thing> id" for a well-formed UUID
    // that resolves to nothing. These pin the refinement to its two
    // structural inputs — the status and whether the CLI's own path
    // identifiers parse as UUIDs — and pin that nothing else moves.

    /// A well-formed UUID that resolves to nothing. All-zero is still a
    /// syntactically valid UUID, so it must count as well-formed.
    const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";
    const ORG_UUID: &str = "00000000-0000-4000-8000-000000000001";

    fn invalid_id_400(thing: &str, id: &str) -> clickhouse_cloud_api::Error {
        clickhouse_cloud_api::Error::Api {
            status: 400,
            message: format!("BAD_REQUEST: Invalid {thing} id string:\"{id}\""),
        }
    }

    /// A refined lookup names the missing ID before the server's verbatim
    /// answer, names each scope identifier once, and suggests the list command
    /// for its kind. The ID appears once on its own and once inside the
    /// server text, so a scope clause that repeated it would add a third.
    fn assert_missing_resource(err: &CloudError, scope: &[&str], server: &str, command: &str) {
        let details = err.details.as_deref().expect("a structured detail");
        assert_eq!(details.code, CloudErrorCode::ResourceNotFound);
        assert_eq!(details.command.as_deref(), Some(command));
        assert_eq!(err.message.matches(NIL_UUID).count(), 2, "{}", err.message);
        for id in scope {
            assert_eq!(err.message.matches(id).count(), 1, "{}", err.message);
        }
        let id_at = err.message.find(NIL_UUID).unwrap();
        let server_at = err.message.find(server).expect("server text kept");
        assert!(id_at < server_at, "{}", err.message);
        assert!(
            err.message.ends_with(&format!("{server}{NIL_UUID}\"")),
            "{}",
            err.message
        );
    }

    #[test]
    fn lookup_400_over_well_formed_ids_reads_as_a_missing_service() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("service", NIL_UUID),
            ResourceLookup::in_org(ResourceKind::Service, NIL_UUID, ORG_UUID),
        );
        assert_missing_resource(
            &err,
            &[ORG_UUID],
            "BAD_REQUEST: Invalid service id string:\"",
            &format!("clickhousectl cloud service list --org-id {ORG_UUID}"),
        );
        assert_eq!(err.kind, CloudErrorKind::Generic);
    }

    #[test]
    fn lookup_400_over_well_formed_ids_reads_as_a_missing_postgres_service() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("Postgres service", NIL_UUID),
            ResourceLookup::in_org(ResourceKind::PostgresService, NIL_UUID, ORG_UUID),
        );
        assert_missing_resource(
            &err,
            &[ORG_UUID],
            "BAD_REQUEST: Invalid Postgres service id string:\"",
            &format!("clickhousectl cloud postgres list --org-id {ORG_UUID}"),
        );
    }

    /// The organization is itself the identifier being looked up, so the
    /// message must not repeat it as request scope.
    #[test]
    fn lookup_400_over_a_well_formed_id_reads_as_a_missing_organization() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("organization", NIL_UUID),
            ResourceLookup::organization(NIL_UUID),
        );
        assert_missing_resource(
            &err,
            &[],
            "BAD_REQUEST: Invalid organization id string:\"",
            "clickhousectl cloud org list",
        );
    }

    /// The organization parenthetical is keyed on the kind, not on the two
    /// identifiers happening to be equal: an organization lookup never names
    /// an organization as the scope it was read from, however it was built.
    #[test]
    fn an_organization_lookup_never_names_a_scope() {
        let client = test_client();
        for lookup in [
            ResourceLookup::organization(NIL_UUID),
            ResourceLookup::in_org(ResourceKind::Organization, NIL_UUID, ORG_UUID),
        ] {
            let err =
                client.convert_error_for_lookup(invalid_id_400("organization", NIL_UUID), lookup);
            assert_missing_resource(
                &err,
                &[],
                "BAD_REQUEST: Invalid organization id string:\"",
                "clickhousectl cloud org list",
            );
            assert!(!err.message.contains(ORG_UUID), "{}", err.message);
        }
    }

    /// Only the message changes: the server did answer 400, so the telemetry
    /// classification must stay exactly what the status says (#450).
    #[test]
    fn lookup_refinement_keeps_the_failure_classification() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("service", NIL_UUID),
            ResourceLookup::in_org(ResourceKind::Service, NIL_UUID, ORG_UUID),
        );
        assert_eq!(
            err.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 400))
        );
    }

    /// `--json` gets a stable code plus the command that lists what does
    /// exist, and the same text human mode prints.
    #[test]
    fn lookup_refinement_carries_a_structured_detail() {
        let client = test_client();
        let err = client.convert_error_for_lookup(
            invalid_id_400("Postgres service", NIL_UUID),
            ResourceLookup::in_org(ResourceKind::PostgresService, NIL_UUID, ORG_UUID),
        );
        let details = err.details.as_deref().expect("a structured detail");
        assert_eq!(details.code, CloudErrorCode::ResourceNotFound);
        assert_eq!(details.message, err.message);
        assert_eq!(
            details.command.as_deref(),
            Some(format!("clickhousectl cloud postgres list --org-id {ORG_UUID}").as_str())
        );
        assert_eq!(details.api_key_id, None);
        assert_eq!(details.ip_access_list, None);

        // An organization lookup has no scope flag to suggest.
        let err = client.convert_error_for_lookup(
            invalid_id_400("organization", NIL_UUID),
            ResourceLookup::organization(NIL_UUID),
        );
        assert_eq!(
            err.details.as_deref().unwrap().command.as_deref(),
            Some("clickhousectl cloud org list")
        );
    }

    /// A malformed identifier keeps the server's answer: "invalid" is then
    /// both true and more useful than "no such service".
    #[test]
    fn lookup_400_over_a_malformed_id_is_left_alone() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("service", "not-a-uuid"),
            ResourceLookup::in_org(ResourceKind::Service, "not-a-uuid", ORG_UUID),
        );
        assert_eq!(
            err.message,
            "BAD_REQUEST: Invalid service id string:\"not-a-uuid\""
        );
        assert!(err.details.is_none());
    }

    /// Every path identifier has to parse, not just the resource's own: a
    /// malformed organization is as plausible a cause of the 400.
    #[test]
    fn lookup_400_with_a_malformed_organization_is_left_alone() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("organization", "org-1"),
            ResourceLookup::in_org(ResourceKind::Service, NIL_UUID, "org-1"),
        );
        assert_eq!(
            err.message,
            "BAD_REQUEST: Invalid organization id string:\"org-1\""
        );
        assert!(err.details.is_none());
    }

    #[test]
    fn lookup_404_refines_only_well_formed_ids_and_preserves_typed_failure() {
        for message in ["NOT_FOUND: Not Found", "deleted", "arbitrary server detail"] {
            for kind in [
                ResourceKind::Service,
                ResourceKind::PostgresService,
                ResourceKind::Organization,
            ] {
                let lookup = if kind == ResourceKind::Organization {
                    ResourceLookup::organization(NIL_UUID)
                } else {
                    ResourceLookup::in_org(kind, NIL_UUID, ORG_UUID)
                };
                let err = test_client().convert_error_for_lookup(
                    clickhouse_cloud_api::Error::Api {
                        status: 404,
                        message: message.into(),
                    },
                    lookup,
                );
                let detail = err.details.expect("structured missing resource");
                assert_eq!(detail.code, CloudErrorCode::ResourceNotFound);
                assert!(detail.message.contains(message));
                assert_eq!(
                    err.failure,
                    Some(ApiFailure::with_status(FailureKind::Http4xx, 404))
                );
            }
        }
        for (id, org_id) in [("service-name", ORG_UUID), (NIL_UUID, "org-name")] {
            let err = test_client().convert_error_for_lookup(
                clickhouse_cloud_api::Error::Api {
                    status: 404,
                    message: "original detail".into(),
                },
                ResourceLookup::in_org(ResourceKind::Service, id, org_id),
            );
            assert!(err.details.is_none());
            assert_eq!(err.message, "original detail");
        }
    }

    #[test]
    fn lookup_refusals_are_not_missing_resources_even_with_not_found_text() {
        for (status, kind) in [(401, CloudErrorKind::Auth), (403, CloudErrorKind::Generic)] {
            let err = test_client().convert_error_for_lookup(
                clickhouse_cloud_api::Error::Api {
                    status,
                    message: "NOT_FOUND: Not Found".into(),
                },
                ResourceLookup::in_org(ResourceKind::Service, NIL_UUID, ORG_UUID),
            );
            assert_eq!(err.kind, kind);
            assert!(err.details.is_none());
            assert_eq!(
                err.failure,
                Some(ApiFailure::with_status(FailureKind::Http4xx, status))
            );
        }
    }

    /// Other statuses and non-API errors cannot imply a missing resource.
    #[test]
    fn lookup_leaves_every_other_status_unchanged() {
        let client = test_client();
        let lookup = || ResourceLookup::in_org(ResourceKind::Service, NIL_UUID, ORG_UUID);

        let err = client.convert_error_for_lookup(
            clickhouse_cloud_api::Error::Api {
                status: 500,
                message: "Internal Server Error".into(),
            },
            lookup(),
        );
        assert_eq!(err.message, "Internal Server Error");
        assert!(err.details.is_none());

        // A non-API failure has no status to reason from at all.
        let err = client.convert_error_for_lookup(
            clickhouse_cloud_api::Error::AuthMismatch("nope".into()),
            lookup(),
        );
        assert!(err.details.is_none());
        assert_eq!(
            err.message,
            clickhouse_cloud_api::Error::AuthMismatch("nope".into()).to_string()
        );
    }

    fn not_found_404() -> clickhouse_cloud_api::Error {
        clickhouse_cloud_api::Error::Api {
            status: 404,
            message: format!("NOT_FOUND: {NIL_UUID}"),
        }
    }

    /// Only the typed 404 is a missing resource on these paths; a 400 keeps
    /// the server's answer (#1226), and both keep their classification.
    #[test]
    fn resource_404_is_missing_but_400_is_not() {
        let lookup = || ResourceLookup::in_org(ResourceKind::Member, NIL_UUID, ORG_UUID);
        let client = test_client();
        let missing = client.convert_error_for_resource(not_found_404(), lookup());
        let detail = missing
            .details
            .as_deref()
            .expect("structured missing resource");
        assert_eq!(detail.code, CloudErrorCode::ResourceNotFound);
        assert_eq!(
            detail.command.as_deref(),
            Some(format!("clickhousectl cloud member list --org-id {ORG_UUID}").as_str())
        );
        assert_eq!(
            missing.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 404))
        );
        let rejected =
            client.convert_error_for_resource(invalid_id_400("member", NIL_UUID), lookup());
        assert!(rejected.details.is_none());
        assert_eq!(
            rejected.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 400))
        );
    }

    #[test]
    fn list_commands_follow_the_scope_except_for_organizations() {
        let service = Scope::Service {
            org_id: ORG_UUID,
            service_id: NIL_UUID,
        };
        assert_eq!(
            ResourceKind::ClickPipe.list_command(service),
            format!("clickhousectl cloud clickpipe list {NIL_UUID} --org-id {ORG_UUID}")
        );
        assert_eq!(
            ResourceKind::ApiKey.list_command(Scope::Organization(ORG_UUID)),
            format!("clickhousectl cloud key list --org-id {ORG_UUID}")
        );
        for scope in [Scope::Account, Scope::Organization(ORG_UUID), service] {
            assert_eq!(
                ResourceKind::Organization.list_command(scope),
                "clickhousectl cloud org list"
            );
        }
    }

    /// A 404 under a service reads the service to tell which is missing; a
    /// failed read, or a service-level request whose service exists, keeps
    /// the original error rather than guessing.
    #[tokio::test]
    async fn a_404_under_a_service_names_only_what_a_read_confirms_missing() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let service_path = format!("/v1/organizations/{ORG_UUID}/services/{NIL_UUID}");
        let leaf =
            || ResourceLookup::in_service(ResourceKind::ClickPipe, NIL_UUID, ORG_UUID, NIL_UUID);
        for (service_status, leaf_command, service_level_command) in [
            (
                404,
                Some(format!(
                    "clickhousectl cloud service list --org-id {ORG_UUID}"
                )),
                Some(format!(
                    "clickhousectl cloud service list --org-id {ORG_UUID}"
                )),
            ),
            (
                200,
                Some(format!(
                    "clickhousectl cloud clickpipe list {NIL_UUID} --org-id {ORG_UUID}"
                )),
                None,
            ),
            (500, None, None),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(service_path.clone()))
                .respond_with(
                    ResponseTemplate::new(service_status)
                        .set_body_json(serde_json::json!({"result": {"id": NIL_UUID}})),
                )
                .mount(&server)
                .await;
            let client = CloudClient::for_tests(&server.uri(), None);
            let command = |error: CloudError| {
                assert_eq!(
                    error.failure,
                    Some(ApiFailure::with_status(FailureKind::Http4xx, 404))
                );
                error.details.and_then(|detail| detail.command)
            };
            let in_service = client
                .convert_error_in_service(not_found_404(), leaf())
                .await;
            assert_eq!(command(in_service), leaf_command, "{service_status}");
            let on_service = client
                .request_on_service(ORG_UUID, NIL_UUID, async { Err::<(), _>(not_found_404()) })
                .await
                .unwrap_err();
            assert_eq!(
                command(on_service),
                service_level_command,
                "{service_status}"
            );
        }
    }

    #[test]
    fn convert_error_treats_non_api_error_as_generic() {
        let err =
            test_client().convert_error(clickhouse_cloud_api::Error::AuthMismatch("nope".into()));
        assert_eq!(err.kind, CloudErrorKind::Generic);
    }

    // ── Dotenv resolver tests ──────────────────────────────────────────────
    //
    // Precedence is exercised by feeding `resolve_auth_with_sources` a
    // synthetic `(env_map, dotenv)` pair. We deliberately do NOT mutate the
    // real process environment: `std::env::set_var` is `unsafe` in edition
    // 2024 because it races with `getenv` across threads, and a mutex
    // around the test body cannot prevent concurrently-running tests from
    // calling `env::var` and tripping that race.
    //
    // The credentials-file and OAuth-token tiers are injected as no-op
    // loaders here. Both sit *above* the env tier in the ladder and read
    // files under the process cwd, which `cargo test` does not isolate —
    // without stubbing them, a developer's saved `.clickhouse/credentials.json`
    // would short-circuit the resolver before it ever reaches the env/dotenv
    // logic these tests assert on.

    fn dotenv_with(pairs: &[(&str, &str)]) -> DotenvVars {
        let mut map = std::collections::HashMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.to_string());
        }
        DotenvVars::from_map_for_tests(map, Some(std::path::PathBuf::from("/synthetic/.env")))
    }

    fn env_map(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn lookup_from(
        map: &std::collections::HashMap<String, String>,
    ) -> impl Fn(&str) -> Option<String> + '_ {
        move |k: &str| map.get(k).cloned()
    }

    // No-op loaders for the file-backed tiers, so the env/dotenv precedence
    // tests don't depend on whatever `.clickhouse/` files happen to live under
    // the test cwd.
    fn no_credentials() -> Option<crate::cloud::credentials::Credentials> {
        None
    }

    fn no_tokens() -> std::result::Result<
        Option<crate::cloud::auth::TokenStore>,
        crate::cloud::auth::UnusableTokens,
    > {
        Ok(None)
    }

    // A populated credentials file, for asserting the file tier wins over env.
    fn some_credentials() -> Option<crate::cloud::credentials::Credentials> {
        Some(crate::cloud::credentials::Credentials {
            api_key: Some("file_k".to_string()),
            api_secret: Some("file_s".to_string()),
            ..Default::default()
        })
    }

    /// An expired saved login (#1224) is an auth error naming the expiry, not
    /// the "no credentials" error an absent login gets.
    #[test]
    fn expired_oauth_login_reports_expiry_not_missing_credentials() {
        let expired = || {
            serde_json::from_value(serde_json::json!({
                "access_token": "expired", "refresh_token": "", "expires_at": 1,
                "api_url": DEFAULT_BASE_URL,
                "auth_config": {"auth_url": "https://auth.example", "client_id": "client"}
            }))
            .map(Some)
            .map_err(|_| crate::cloud::auth::UnusableTokens)
        };
        let dotenv = dotenv_with(&[]);
        let resolve = |tokens: TokensLookup<'_>| {
            resolve_auth_with_sources(
                None,
                None,
                None,
                &dotenv,
                &|_| None,
                &no_credentials,
                tokens,
            )
            .err()
            .expect("no usable credentials")
        };
        let error = resolve(&expired);
        assert_eq!(error.kind, CloudErrorKind::Auth);
        assert_eq!(error.message, crate::cloud::auth::OAUTH_LOGIN_EXPIRED);
        let error = resolve(&no_tokens);
        assert_eq!(error.kind, CloudErrorKind::Auth);
        assert_ne!(error.message, crate::cloud::auth::OAUTH_LOGIN_EXPIRED);
    }

    #[test]
    fn empty_saved_credentials_do_not_shadow_usable_sources() {
        for (key, secret) in [("", ""), ("", "file_s"), ("file_k", "")] {
            let saved = || {
                Some(crate::cloud::credentials::Credentials {
                    api_key: Some(key.to_owned()),
                    api_secret: Some(secret.to_owned()),
                    ..Default::default()
                })
            };
            let dotenv = dotenv_with(&[]);
            let env = env_map(&[
                ("CLICKHOUSE_CLOUD_API_KEY", "env_k"),
                ("CLICKHOUSE_CLOUD_API_SECRET", "env_s"),
            ]);
            let resolved = resolve_auth_with_sources(
                None,
                None,
                None,
                &dotenv,
                &lookup_from(&env),
                &saved,
                &no_tokens,
            )
            .unwrap();
            assert_eq!(resolved.source, AuthSource::EnvVars);
            let ResolvedCreds::Basic { key, secret } = resolved.creds else {
                panic!("expected environment API key credentials");
            };
            assert_eq!(key, "env_k");
            assert_eq!(secret, "env_s");
            let result =
                resolve_auth_with_sources(None, None, None, &dotenv, &|_| None, &saved, &no_tokens);
            assert!(matches!(
                result,
                Err(CloudError {
                    kind: CloudErrorKind::Auth,
                    ..
                })
            ));
        }
    }

    #[test]
    fn credentials_file_overrides_env() {
        // Both the credentials file and the env tier are fully populated.
        // The file sits above env in the ladder, so it must win.
        let dotenv = dotenv_with(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "dot_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "dot_s"),
        ]);
        let env = env_map(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "shell_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "shell_s"),
        ]);
        let lookup = lookup_from(&env);
        let resolved = resolve_auth_with_sources(
            None,
            None,
            None,
            &dotenv,
            &lookup,
            &some_credentials,
            &no_tokens,
        )
        .unwrap();
        assert_eq!(resolved.source, AuthSource::CredentialsFile);
        match resolved.creds {
            ResolvedCreds::Basic { key, secret } => {
                assert_eq!(key, "file_k");
                assert_eq!(secret, "file_s");
            }
            _ => panic!("expected Basic creds"),
        }
    }

    const DEV_URL: &str = "https://api.control-plane.clickhouse-dev.com/v1";

    fn saved_key_for(api_url: Option<&str>) -> crate::cloud::credentials::Credentials {
        crate::cloud::credentials::Credentials {
            api_key: Some("file_k".to_string()),
            api_secret: Some("file_s".to_string()),
            api_url: api_url.map(String::from),
            ..Default::default()
        }
    }

    /// Every row of the #1202 table: where a saved key is sent for each
    /// combination of its recorded API and `--url`, or that it is refused.
    #[test]
    fn saved_key_goes_only_to_the_api_it_was_saved_for() {
        let other = "https://api.control-plane.clickhouse-staging.com/v1";
        let cases: [(Option<&str>, Option<&str>, Option<&str>); 7] = [
            (None, None, Some(DEFAULT_BASE_URL)),
            (
                None,
                Some("https://api.clickhouse.cloud"),
                Some(DEFAULT_BASE_URL),
            ),
            (None, Some(DEV_URL), None),
            (Some(DEV_URL), None, Some(DEV_URL)),
            // `--url` is compared normalized, so a missing `/v1` still matches.
            (
                Some(DEV_URL),
                Some("https://api.control-plane.clickhouse-dev.com/"),
                Some(DEV_URL),
            ),
            (Some(DEV_URL), Some(other), None),
            (Some(DEV_URL), Some(DEFAULT_BASE_URL), None),
        ];
        // Env vars are usable, so a refusal shows the mismatch does not fall
        // through to a lower tier.
        let env = env_map(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "env_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "env_s"),
        ]);
        for (saved_url, url_override, expected) in cases {
            let saved = || Some(saved_key_for(saved_url));
            let result = resolve_auth_with_sources(
                None,
                None,
                url_override,
                &dotenv_with(&[]),
                &lookup_from(&env),
                &saved,
                &no_tokens,
            );
            match expected {
                Some(base_url) => {
                    let resolved = result.unwrap_or_else(|error| {
                        panic!("{saved_url:?} with {url_override:?}: {error}")
                    });
                    assert_eq!(resolved.source, AuthSource::CredentialsFile);
                    assert_eq!(resolved.base_url, base_url);
                }
                None => {
                    let Err(error) = result else {
                        panic!("{saved_url:?} with {url_override:?} was not refused");
                    };
                    assert_eq!(error.kind, CloudErrorKind::Auth);
                }
            }
        }
    }

    /// CLI-flag and env-var keys carry no URL, so a saved key's URL never
    /// steers them.
    #[test]
    fn unsaved_keys_ignore_a_saved_api_url() {
        let env = env_map(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "env_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "env_s"),
        ]);
        let saved = || Some(saved_key_for(Some(DEV_URL)));
        let flags = resolve_auth_with_sources(
            Some("k"),
            Some("s"),
            None,
            &dotenv_with(&[]),
            &lookup_from(&env),
            &saved,
            &no_tokens,
        )
        .unwrap();
        assert_eq!(flags.source, AuthSource::CliFlags);
        assert_eq!(flags.base_url, DEFAULT_BASE_URL);
        let env_tier = resolve_auth_with_sources(
            None,
            None,
            None,
            &dotenv_with(&[]),
            &lookup_from(&env),
            &no_credentials,
            &no_tokens,
        )
        .unwrap();
        assert_eq!(env_tier.source, AuthSource::EnvVars);
        assert_eq!(env_tier.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn stored_api_url_notice_only_for_an_implicit_non_prod_saved_key() {
        assert!(stored_api_url_notice(AuthSource::CredentialsFile, None, DEV_URL).is_some());
        for (source, url_override, base_url) in [
            (AuthSource::CredentialsFile, None, DEFAULT_BASE_URL),
            (AuthSource::CredentialsFile, Some(DEV_URL), DEV_URL),
            (AuthSource::EnvVars, None, DEV_URL),
            (AuthSource::OAuthTokens, None, DEV_URL),
        ] {
            assert!(
                stored_api_url_notice(source, url_override, base_url).is_none(),
                "{source:?} {url_override:?} {base_url}"
            );
        }
    }

    #[test]
    fn dotenv_only_resolves_to_env_source() {
        let dotenv = dotenv_with(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "dot_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "dot_s"),
        ]);
        let env = env_map(&[]);
        let lookup = lookup_from(&env);
        let resolved = resolve_auth_with_sources(
            None,
            None,
            None,
            &dotenv,
            &lookup,
            &no_credentials,
            &no_tokens,
        )
        .unwrap();
        assert_eq!(resolved.source, AuthSource::EnvVars);
        match resolved.creds {
            ResolvedCreds::Basic { key, secret } => {
                assert_eq!(key, "dot_k");
                assert_eq!(secret, "dot_s");
            }
            _ => panic!("expected Basic creds"),
        }
        // Both creds came from .env → provenance helper should surface the path.
        assert_eq!(
            dotenv_env_provenance_with_sources(&dotenv, &lookup)
                .unwrap()
                .display()
                .to_string(),
            "/synthetic/.env"
        );
    }

    #[test]
    fn real_env_overrides_dotenv() {
        let dotenv = dotenv_with(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "dot_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "dot_s"),
        ]);
        let env = env_map(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "shell_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "shell_s"),
        ]);
        let lookup = lookup_from(&env);
        let resolved = resolve_auth_with_sources(
            None,
            None,
            None,
            &dotenv,
            &lookup,
            &no_credentials,
            &no_tokens,
        )
        .unwrap();
        match resolved.creds {
            ResolvedCreds::Basic { key, secret } => {
                assert_eq!(key, "shell_k");
                assert_eq!(secret, "shell_s");
            }
            _ => panic!("expected Basic creds"),
        }
        // Real env supplied both: provenance is shell, not .env.
        assert!(dotenv_env_provenance_with_sources(&dotenv, &lookup).is_none());
    }

    #[test]
    fn mixed_real_and_dotenv() {
        // Key from shell, secret comes from .env.
        let dotenv = dotenv_with(&[("CLICKHOUSE_CLOUD_API_SECRET", "dot_s")]);
        let env = env_map(&[("CLICKHOUSE_CLOUD_API_KEY", "shell_k")]);
        let lookup = lookup_from(&env);
        let resolved = resolve_auth_with_sources(
            None,
            None,
            None,
            &dotenv,
            &lookup,
            &no_credentials,
            &no_tokens,
        )
        .unwrap();
        match resolved.creds {
            ResolvedCreds::Basic { key, secret } => {
                assert_eq!(key, "shell_k");
                assert_eq!(secret, "dot_s");
            }
            _ => panic!("expected Basic creds"),
        }
        // Mixed provenance: helper must return None so the status line
        // doesn't imply .env was the sole source.
        assert!(dotenv_env_provenance_with_sources(&dotenv, &lookup).is_none());
    }

    // ── Empty-is-absent ────────────────────────────────────────────────────
    //
    // An exported-but-empty shell var (or a bare `KEY=` line) must not count
    // as a credential: it can't shadow a populated `.env` value, can't resolve
    // to empty Basic-auth creds, and can't register as "present". All three
    // sites route through `non_empty`/`env_or_dotenv`, so these assert the
    // behavior once per surface.

    #[test]
    fn empty_shell_does_not_shadow_dotenv() {
        let dotenv = dotenv_with(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "dot_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "dot_s"),
        ]);
        // Both shell vars exported but empty — should be treated as absent.
        let env = env_map(&[
            ("CLICKHOUSE_CLOUD_API_KEY", ""),
            ("CLICKHOUSE_CLOUD_API_SECRET", ""),
        ]);
        let lookup = lookup_from(&env);
        let resolved = resolve_auth_with_sources(
            None,
            None,
            None,
            &dotenv,
            &lookup,
            &no_credentials,
            &no_tokens,
        )
        .unwrap();
        match resolved.creds {
            ResolvedCreds::Basic { key, secret } => {
                assert_eq!(key, "dot_k");
                assert_eq!(secret, "dot_s");
            }
            _ => panic!("expected Basic creds"),
        }
        // Empty real vars are absent, so provenance is purely .env.
        assert_eq!(
            dotenv_env_provenance_with_sources(&dotenv, &lookup)
                .unwrap()
                .display()
                .to_string(),
            "/synthetic/.env"
        );
        // And the status table sees both creds present.
        let presence = env_cred_presence_with_sources(&dotenv, &lookup);
        assert!(presence.key && presence.secret);
    }

    #[test]
    fn empty_dotenv_value_is_absent() {
        // `.env` has the key but its value is empty; secret is populated.
        let dotenv = dotenv_with(&[
            ("CLICKHOUSE_CLOUD_API_KEY", ""),
            ("CLICKHOUSE_CLOUD_API_SECRET", "dot_s"),
        ]);
        let env = env_map(&[]);
        let lookup = lookup_from(&env);
        // The empty key isn't a usable credential, so env-tier doesn't fully
        // resolve and provenance must not claim .env was the sole source.
        assert!(dotenv_env_provenance_with_sources(&dotenv, &lookup).is_none());
        let presence = env_cred_presence_with_sources(&dotenv, &lookup);
        assert!(!presence.key);
        assert!(presence.secret);
    }

    #[test]
    fn all_empty_registers_as_absent() {
        let dotenv = dotenv_with(&[("CLICKHOUSE_CLOUD_API_KEY", "")]);
        let env = env_map(&[("CLICKHOUSE_CLOUD_API_SECRET", "")]);
        let lookup = lookup_from(&env);
        let presence = env_cred_presence_with_sources(&dotenv, &lookup);
        assert!(!presence.key);
        assert!(!presence.secret);
        assert!(dotenv_env_provenance_with_sources(&dotenv, &lookup).is_none());
    }
}
