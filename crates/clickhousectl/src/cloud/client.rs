use crate::cloud::output::{CloudErrorCode, CloudErrorDetail};
use crate::dotenv::DotenvVars;
use crate::failure::{ApiFailure, FailureKind, FailureStage};
use std::env;

const DEFAULT_BASE_URL: &str = "https://api.clickhouse.cloud/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CloudErrorKind {
    #[default]
    Generic,
    Auth,
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
type TokensLookup<'a> = &'a dyn Fn() -> Option<crate::cloud::auth::TokenStore>;

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
        && let (Some(key), Some(secret)) = (creds.api_key, creds.api_secret)
    {
        return Ok(ResolvedAuth {
            creds: ResolvedCreds::Basic { key, secret },
            source: AuthSource::CredentialsFile,
            base_url: normalized_default(),
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

    if let Some(tokens) = load_tokens()
        && crate::cloud::auth::is_token_valid(&tokens)
    {
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

/// Peek which credential source would win precedence right now without
/// actually building a `CloudClient`.
///
/// Used by `cloud auth status`, which has to render correctly even when no
/// credentials are configured (the case `CloudClient::new` errors out on).
/// Returns `None` if nothing usable is configured.
pub fn resolve_active_auth_source() -> Option<AuthSource> {
    resolve_auth(None, None, None).ok().map(|r| r.source)
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

// ── by-identifier reads (issue #666) ───────────────────────────────────────
//
// The API answers HTTP 400 `Invalid <thing> id string:"<id>"` for a
// syntactically valid UUID that resolves to nothing, so `cloud postgres get
// 00000000-0000-0000-0000-000000000000` reported a bad request rather than a
// missing resource. The fix is structural, not textual: nothing here reads the
// response prose (the message interpolates the id, so it could never become a
// typed library variant either). A GET-by-identifier request has exactly one
// class of user-controlled input — the identifiers the CLI itself formatted
// into the URL path — so when every one of those is a well-formed UUID, "the
// identifier is invalid" is not what happened.
//
// A malformed identifier keeps the server's answer verbatim: there, "invalid"
// is both true and the more useful thing to say.

/// Which resource a by-identifier read was looking up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Service,
    PostgresService,
    Organization,
}

impl ResourceKind {
    /// The noun the not-found message names the resource with.
    const fn noun(self) -> &'static str {
        match self {
            ResourceKind::Service => "service",
            ResourceKind::PostgresService => "Postgres service",
            ResourceKind::Organization => "organization",
        }
    }

    /// The command that lists what does exist, for the structured detail.
    /// Carries an organization id at most, which the message already names.
    fn list_command(self, org_id: Option<&str>) -> String {
        let base = match self {
            ResourceKind::Service => "clickhousectl cloud service list",
            ResourceKind::PostgresService => "clickhousectl cloud postgres list",
            ResourceKind::Organization => "clickhousectl cloud org list",
        };
        match (self, org_id) {
            // The organization *is* the identifier being looked up, so the
            // command that lists the alternatives takes no scope flag.
            (ResourceKind::Organization, _) | (_, None) => base.to_string(),
            (_, Some(org_id)) => format!("{base} --org-id {org_id}"),
        }
    }
}

/// The identifiers a by-identifier read put into its request path.
pub struct ResourceLookup<'a> {
    pub kind: ResourceKind,
    /// The resource's own identifier, as the user supplied it.
    pub id: &'a str,
    /// The organization the request was scoped to. Equal to `id` for an
    /// organization lookup, whose only path identifier is the organization.
    pub org_id: Option<&'a str>,
}

impl ResourceLookup<'_> {
    /// Every identifier the CLI formatted into the request path.
    fn identifiers(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id).chain(self.org_id)
    }

    /// The trailing scope clause, when the request was scoped to something
    /// other than the resource being looked up.
    fn scope_clause(&self) -> String {
        match self.org_id {
            // Skipped when the organization is the resource, so the message
            // never reads "organization X (organization X)".
            Some(org_id) if org_id != self.id => format!(" (organization {org_id})"),
            _ => String::new(),
        }
    }
}

pub struct CloudClient {
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
    pub fn new(
        api_key: Option<&str>,
        api_secret: Option<&str>,
        url_override: Option<&str>,
    ) -> Result<Self> {
        let http = crate::http::client_builder()
            .build()
            .map_err(|e| CloudError::new(format!("Failed to create HTTP client: {}", e)))?;

        let resolved = resolve_auth(api_key, api_secret, url_override)?;
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

    /// Convert a library error into a `CloudError`, appending OAuth hints when relevant.
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

    /// Re-present a by-identifier read's failure as a not-found when the API
    /// rejected identifiers that are all well-formed UUIDs (#666).
    ///
    /// The discriminator is structural: the HTTP status the library reported,
    /// plus `Uuid::parse_str` over inputs the CLI already held. No part of it
    /// comes from the response message, which is appended verbatim so nothing
    /// the server said is lost.
    ///
    /// Conversion still goes through [`Self::convert_error_with_organization`],
    /// so the telemetry classification (#450) is inherited unchanged and
    /// carried across the rewrite: the server did answer 400, and only the
    /// user-facing message changes.
    pub fn convert_error_for_lookup(
        &self,
        err: clickhouse_cloud_api::Error,
        lookup: ResourceLookup<'_>,
    ) -> CloudError {
        let rejected_well_formed_ids =
            matches!(&err, clickhouse_cloud_api::Error::Api { status: 400, .. })
                && lookup
                    .identifiers()
                    .all(|id| uuid::Uuid::parse_str(id).is_ok());
        let error = self.convert_error_with_organization(err, lookup.org_id);
        if !rejected_well_formed_ids {
            return error;
        }
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
            command: Some(lookup.kind.list_command(lookup.org_id)),
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
            clickhouse_cloud_api::Error::Api { status, message } => {
                let mut msg = message.clone();
                let trimmed_message = message.trim();
                if *status == 404
                    && matches!(
                        trimmed_message.to_ascii_uppercase().as_str(),
                        "NOT_FOUND" | "NOT FOUND"
                    )
                    && let Some(org_id) = org_id
                {
                    msg = format!("{trimmed_message}: request scoped to organization {org_id}");
                }
                if *status == 403 && self.is_bearer_auth() {
                    msg.push_str(
                        "\n\nHint: You are authenticated via OAuth, which provides read-only access. \
                         Use API key authentication for write operations:\n  \
                         clickhousectl cloud auth login --api-key YOUR_KEY --api-secret YOUR_SECRET\n\n\
                         Learn how to create API keys:\n  \
                         https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl",
                    );
                }
                if matches!(*status, 401 | 403) {
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
            auth_mode: AuthMode::Basic {
                key: "test_key".into(),
                secret: "test_secret".into(),
            },
            auth_source: AuthSource::CliFlags,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
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
        };
        let result = CloudClient::unwrap_response(response).unwrap();
        assert_eq!(result, vec!["hello".to_string()]);
    }

    #[test]
    fn unwrap_response_errors_on_empty_result() {
        let response: clickhouse_cloud_api::models::ApiResponse<String> =
            clickhouse_cloud_api::models::ApiResponse {
                status: Some(200),
                request_id: None,
                result: None,
                error: None,
            };
        let err = CloudClient::unwrap_response(response).unwrap_err();
        assert_eq!(err.message, "Empty response from API");
    }

    #[test]
    fn convert_error_includes_oauth_hint_for_403_bearer() {
        let http = reqwest::Client::builder().build().unwrap();
        let lib_client = clickhouse_cloud_api::Client::with_http_client_bearer(
            http,
            DEFAULT_LIB_BASE_URL,
            "test_token",
        );
        let client = CloudClient {
            lib_client,
            auth_mode: AuthMode::Bearer,
            auth_source: AuthSource::OAuthTokens,
            base_url: DEFAULT_BASE_URL.to_string(),
        };
        let err = client.convert_error(clickhouse_cloud_api::Error::Api {
            status: 403,
            message: "Forbidden".into(),
        });
        assert!(
            err.message
                .contains("Hint: You are authenticated via OAuth")
        );
    }

    #[test]
    fn auth_source_label_and_describe() {
        assert_eq!(AuthSource::CliFlags.label(), "CLI flags");
        assert_eq!(AuthSource::CredentialsFile.label(), "Credentials file");
        assert_eq!(AuthSource::EnvVars.label(), "Env vars");
        assert_eq!(AuthSource::OAuthTokens.label(), "OAuth");

        assert!(AuthSource::CliFlags.describe().contains("--api-key"));
        assert!(
            AuthSource::EnvVars
                .describe()
                .contains("CLICKHOUSE_CLOUD_API_KEY")
        );
        assert!(
            AuthSource::CredentialsFile
                .describe()
                .contains("credentials")
        );
        assert!(AuthSource::OAuthTokens.describe().contains("OAuth"));
    }

    #[test]
    fn auth_source_accessor_returns_cli_flags_default_in_test_client() {
        let client = test_client();
        assert_eq!(client.auth_source(), AuthSource::CliFlags);
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn convert_error_no_hint_for_403_basic() {
        let client = test_client();
        let err = client.convert_error(clickhouse_cloud_api::Error::Api {
            status: 403,
            message: "Forbidden".into(),
        });
        assert!(!err.message.contains("Hint:"));
        assert_eq!(err.message, "Forbidden");
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
    fn convert_error_flags_403_as_auth() {
        let err = test_client().convert_error(clickhouse_cloud_api::Error::Api {
            status: 403,
            message: "Forbidden".into(),
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

        // The OAuth hint rewrites the message; the classification is
        // unaffected by it.
        let err = test_client().convert_error(clickhouse_cloud_api::Error::Api {
            status: 403,
            message: "Forbidden".into(),
        });
        assert_eq!(
            err.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 403))
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
    fn convert_error_adds_organization_scope_to_bare_not_found() {
        let err = test_client().convert_error_for_organization(
            clickhouse_cloud_api::Error::Api {
                status: 404,
                message: " NOT_FOUND\n".into(),
            },
            "00000000-0000-4000-8000-000000000001",
        );
        assert_eq!(
            err.message,
            "NOT_FOUND: request scoped to organization 00000000-0000-4000-8000-000000000001"
        );
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

    #[test]
    fn lookup_400_over_well_formed_ids_reads_as_a_missing_service() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("service", NIL_UUID),
            ResourceLookup {
                kind: ResourceKind::Service,
                id: NIL_UUID,
                org_id: Some(ORG_UUID),
            },
        );
        assert_eq!(
            err.message,
            format!(
                "No such service: {NIL_UUID} (organization {ORG_UUID}). \
                 The API rejected the identifier: \
                 BAD_REQUEST: Invalid service id string:\"{NIL_UUID}\""
            )
        );
        assert_eq!(err.kind, CloudErrorKind::Generic);
    }

    #[test]
    fn lookup_400_over_well_formed_ids_reads_as_a_missing_postgres_service() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("Postgres service", NIL_UUID),
            ResourceLookup {
                kind: ResourceKind::PostgresService,
                id: NIL_UUID,
                org_id: Some(ORG_UUID),
            },
        );
        assert_eq!(
            err.message,
            format!(
                "No such Postgres service: {NIL_UUID} (organization {ORG_UUID}). \
                 The API rejected the identifier: \
                 BAD_REQUEST: Invalid Postgres service id string:\"{NIL_UUID}\""
            )
        );
    }

    /// The organization is itself the identifier being looked up, so the
    /// message must not repeat it as request scope.
    #[test]
    fn lookup_400_over_a_well_formed_id_reads_as_a_missing_organization() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("organization", NIL_UUID),
            ResourceLookup {
                kind: ResourceKind::Organization,
                id: NIL_UUID,
                org_id: Some(NIL_UUID),
            },
        );
        assert_eq!(
            err.message,
            format!(
                "No such organization: {NIL_UUID}. \
                 The API rejected the identifier: \
                 BAD_REQUEST: Invalid organization id string:\"{NIL_UUID}\""
            )
        );
        assert!(!err.message.contains("(organization"));
    }

    /// Only the message changes: the server did answer 400, so the telemetry
    /// classification must stay exactly what the status says (#450).
    #[test]
    fn lookup_refinement_keeps_the_failure_classification() {
        let err = test_client().convert_error_for_lookup(
            invalid_id_400("service", NIL_UUID),
            ResourceLookup {
                kind: ResourceKind::Service,
                id: NIL_UUID,
                org_id: Some(ORG_UUID),
            },
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
            ResourceLookup {
                kind: ResourceKind::PostgresService,
                id: NIL_UUID,
                org_id: Some(ORG_UUID),
            },
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
            ResourceLookup {
                kind: ResourceKind::Organization,
                id: NIL_UUID,
                org_id: Some(NIL_UUID),
            },
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
            ResourceLookup {
                kind: ResourceKind::Service,
                id: "not-a-uuid",
                org_id: Some(ORG_UUID),
            },
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
            ResourceLookup {
                kind: ResourceKind::Service,
                id: NIL_UUID,
                org_id: Some("org-1"),
            },
        );
        assert_eq!(
            err.message,
            "BAD_REQUEST: Invalid organization id string:\"org-1\""
        );
        assert!(err.details.is_none());
    }

    /// Any other status is somebody else's story. A 404 keeps the existing
    /// organization-scope enrichment, and a 5xx is untouched.
    #[test]
    fn lookup_leaves_every_other_status_unchanged() {
        let client = test_client();
        let lookup = || ResourceLookup {
            kind: ResourceKind::Service,
            id: NIL_UUID,
            org_id: Some(ORG_UUID),
        };

        let err = client.convert_error_for_lookup(
            clickhouse_cloud_api::Error::Api {
                status: 404,
                message: "NOT_FOUND".into(),
            },
            lookup(),
        );
        assert_eq!(
            err.message,
            format!("NOT_FOUND: request scoped to organization {ORG_UUID}")
        );
        assert!(err.details.is_none());

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
        assert!(!err.message.starts_with("No such"));
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

    fn no_tokens() -> Option<crate::cloud::auth::TokenStore> {
        None
    }

    // A populated credentials file, for asserting the file tier wins over env.
    fn some_credentials() -> Option<crate::cloud::credentials::Credentials> {
        Some(crate::cloud::credentials::Credentials {
            api_key: Some("file_k".to_string()),
            api_secret: Some("file_s".to_string()),
            ..Default::default()
        })
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
