use crate::cloud::types::DeleteResponse;
use crate::dotenv::DotenvVars;
use std::env;

const DEFAULT_BASE_URL: &str = "https://api.clickhouse.cloud/v1";

#[derive(Debug)]
pub struct CloudError {
    pub message: String,
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CloudError {}

pub type Result<T> = std::result::Result<T, CloudError>;

enum AuthMode {
    Basic,
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
    /// OAuth tokens saved by `cloud auth login` (`.clickhouse/tokens.json`)
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

fn env_or_dotenv(key: &str, dotenv: &DotenvVars, env_lookup: EnvLookup<'_>) -> Option<String> {
    env_lookup(key).or_else(|| dotenv.get(key).map(String::from))
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
    )
}

/// Walk the precedence ladder once. Order: CLI flags, credentials file, env
/// vars (with `.env` fallback), OAuth tokens. Errors only when CLI flags
/// are half-set (key without secret or vice versa) or when nothing usable
/// is configured.
///
/// `env_lookup` is the injection point that lets tests feed a controlled
/// snapshot of the process environment without mutating it.
fn resolve_auth_with_sources(
    api_key: Option<&str>,
    api_secret: Option<&str>,
    url_override: Option<&str>,
    dotenv: &DotenvVars,
    env_lookup: EnvLookup<'_>,
) -> Result<ResolvedAuth> {
    let normalized_default = || {
        url_override
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    };

    if api_key.is_some() || api_secret.is_some() {
        let key = api_key.map(String::from).ok_or_else(|| CloudError {
            message: "API key required when --api-key or --api-secret is set".into(),
        })?;
        let secret = api_secret.map(String::from).ok_or_else(|| CloudError {
            message: "API secret required when --api-key or --api-secret is set".into(),
        })?;
        return Ok(ResolvedAuth {
            creds: ResolvedCreds::Basic { key, secret },
            source: AuthSource::CliFlags,
            base_url: normalized_default(),
        });
    }

    if let Some(creds) = crate::cloud::credentials::load_credentials()
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

    if let Some(tokens) = crate::cloud::auth::load_tokens()
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

    Err(CloudError {
        message: "No credentials found. Run `clickhousectl cloud auth login` (OAuth, read-only), `clickhousectl cloud auth login --api-key KEY --api-secret SECRET` (read/write), set CLICKHOUSE_CLOUD_API_KEY + CLICKHOUSE_CLOUD_API_SECRET (also picked up from a `.env` file in the current directory), or use --api-key/--api-secret.\n\nLearn how to create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl".into(),
    })
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
    let real_key = env_lookup("CLICKHOUSE_CLOUD_API_KEY").is_some();
    let real_secret = env_lookup("CLICKHOUSE_CLOUD_API_SECRET").is_some();
    if !real_key
        && !real_secret
        && dotenv.get("CLICKHOUSE_CLOUD_API_KEY").is_some()
        && dotenv.get("CLICKHOUSE_CLOUD_API_SECRET").is_some()
    {
        dotenv.source_path().map(|p| p.to_path_buf())
    } else {
        None
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
                let base = "environment variables (CLICKHOUSE_CLOUD_API_KEY, CLICKHOUSE_CLOUD_API_SECRET)";
                match dotenv_env_provenance() {
                    Some(path) => format!("{base} (loaded from {})", path.display()),
                    None => base.to_string(),
                }
            }
            AuthSource::OAuthTokens => format!(
                "OAuth tokens ({})",
                crate::cloud::auth::tokens_path().display()
            ),
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
        let http = reqwest::Client::builder()
            .user_agent(crate::user_agent::user_agent())
            .build()
            .map_err(|e| CloudError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let resolved = resolve_auth(api_key, api_secret, url_override)?;
        let lib_url = lib_base_url(&resolved.base_url);
        let (lib_client, auth_mode) = match &resolved.creds {
            ResolvedCreds::Basic { key, secret } => (
                clickhouse_cloud_api::Client::with_http_client(http, lib_url, key, secret),
                AuthMode::Basic,
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

    /// Returns true if the client is using OAuth Bearer token authentication.
    /// Bearer auth is read-only and cannot perform write operations.
    pub fn is_bearer_auth(&self) -> bool {
        matches!(self.auth_mode, AuthMode::Bearer)
    }

    /// The credential source that won precedence when constructing this client.
    pub fn auth_source(&self) -> AuthSource {
        self.auth_source
    }

    /// The API base URL the client is talking to (includes the `/v1` suffix).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Access the library client for migrated commands.
    pub fn api(&self) -> &clickhouse_cloud_api::Client {
        &self.lib_client
    }

    /// Unwrap an `ApiResponse<T>` into `T`, returning an error if the result is empty.
    pub fn unwrap_response<T>(response: clickhouse_cloud_api::models::ApiResponse<T>) -> Result<T> {
        response.result.ok_or_else(|| CloudError {
            message: "Empty response from API".into(),
        })
    }

    /// Convert a library error into a `CloudError`, appending OAuth hints when relevant.
    pub fn convert_error(&self, err: clickhouse_cloud_api::Error) -> CloudError {
        let message = match &err {
            clickhouse_cloud_api::Error::Api { status, message } => {
                let mut msg = message.clone();
                if *status == 403 && self.is_bearer_auth() {
                    msg.push_str(
                        "\n\nHint: You are authenticated via OAuth, which provides read-only access. \
                         Use API key authentication for write operations:\n  \
                         clickhousectl cloud auth login --api-key YOUR_KEY --api-secret YOUR_SECRET\n\n\
                         Learn how to create API keys:\n  \
                         https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl",
                    );
                }
                msg
            }
            other => other.to_string(),
        };
        CloudError { message }
    }

    // Organization endpoints (delegated to library client)
    pub async fn list_organizations(
        &self,
    ) -> Result<Vec<clickhouse_cloud_api::models::Organization>> {
        let response = self
            .api()
            .organization_get_list()
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_organization(
        &self,
        org_id: &str,
    ) -> Result<clickhouse_cloud_api::models::Organization> {
        let response = self
            .api()
            .organization_get(org_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Service endpoints (delegated to library client)
    pub async fn list_services(
        &self,
        org_id: &str,
    ) -> Result<Vec<clickhouse_cloud_api::models::Service>> {
        let response = self
            .api()
            .instance_get_list(org_id, &[])
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn list_services_filtered(
        &self,
        org_id: &str,
        filters: &[String],
    ) -> Result<Vec<clickhouse_cloud_api::models::Service>> {
        let filter_refs: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
        let response = self
            .api()
            .instance_get_list(org_id, &filter_refs)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<clickhouse_cloud_api::models::Service> {
        let response = self
            .api()
            .instance_get(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn create_service(
        &self,
        org_id: &str,
        request: &clickhouse_cloud_api::models::ServicePostRequest,
    ) -> Result<clickhouse_cloud_api::models::ServicePostResponse> {
        let response = self
            .api()
            .instance_create(org_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_service(&self, org_id: &str, service_id: &str) -> Result<DeleteResponse> {
        let response = self
            .api()
            .instance_delete(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(DeleteResponse {
            status: response.status.unwrap_or(0.0),
            request_id: response.request_id.unwrap_or_default(),
        })
    }

    pub async fn change_service_state(
        &self,
        org_id: &str,
        service_id: &str,
        command: clickhouse_cloud_api::models::ServiceStatePatchRequestCommand,
    ) -> Result<clickhouse_cloud_api::models::Service> {
        use clickhouse_cloud_api::models::ServiceStatePatchRequest;
        let request = ServiceStatePatchRequest {
            command: Some(command),
        };
        let response = self
            .api()
            .instance_state_update(org_id, service_id, &request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Backup endpoints (delegated to library client)
    pub async fn list_backups(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<Vec<clickhouse_cloud_api::models::Backup>> {
        let response = self
            .api()
            .backup_get_list(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_backup(
        &self,
        org_id: &str,
        service_id: &str,
        backup_id: &str,
    ) -> Result<clickhouse_cloud_api::models::Backup> {
        let response = self
            .api()
            .backup_get(org_id, service_id, backup_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Update service
    pub async fn update_service(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ServicePatchRequest,
    ) -> Result<clickhouse_cloud_api::models::Service> {
        let response = self
            .api()
            .instance_update(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Replica scaling
    pub async fn update_replica_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ServiceReplicaScalingPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::ServiceScalingPatchResponse> {
        let response = self
            .api()
            .instance_replica_scaling_update(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Reset password
    pub async fn reset_password(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ServicePasswordPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::ServicePasswordPatchResponse> {
        let response = self
            .api()
            .instance_password_update(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Query endpoint (delegated to library client)
    pub async fn get_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = self
            .api()
            .instance_query_endpoint_get(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn create_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::InstanceServiceQueryApiEndpointsPostRequest,
    ) -> Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = self
            .api()
            .instance_query_endpoint_upsert(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<DeleteResponse> {
        let response = self
            .api()
            .instance_query_endpoint_delete(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(DeleteResponse {
            status: response.status.unwrap_or(0.0),
            request_id: response.request_id.unwrap_or_default(),
        })
    }

    // Private endpoint (delegated to library client)
    pub async fn create_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ServicPrivateEndpointePostRequest,
    ) -> Result<clickhouse_cloud_api::models::InstancePrivateEndpoint> {
        let response = self
            .api()
            .instance_private_endpoint_create(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Private endpoint config (delegated to library client)
    pub async fn get_service_private_endpoint_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<clickhouse_cloud_api::models::PrivateEndpointConfig> {
        let response = self
            .api()
            .instance_private_endpoint_config_get(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_prometheus(
        &self,
        org_id: &str,
        service_id: &str,
        filtered_metrics: Option<bool>,
    ) -> Result<String> {
        let filtered = filtered_metrics.map(|b| b.to_string());
        self.api()
            .instance_prometheus_get(org_id, service_id, filtered.as_deref())
            .await
            .map_err(|e| self.convert_error(e))
    }

    // Organization endpoints (delegated to library client)
    pub async fn update_organization(
        &self,
        org_id: &str,
        request: &clickhouse_cloud_api::models::OrganizationPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::Organization> {
        let response = self
            .api()
            .organization_update(org_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_org_prometheus(
        &self,
        org_id: &str,
        filtered_metrics: Option<bool>,
    ) -> Result<String> {
        let fm_str = filtered_metrics.map(|b| if b { "true" } else { "false" });
        self.api()
            .organization_prometheus_get(org_id, fm_str)
            .await
            .map_err(|e| self.convert_error(e))
    }

    pub async fn get_org_usage(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[String],
    ) -> Result<clickhouse_cloud_api::models::UsageCost> {
        let filter_refs: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
        let response = self
            .api()
            .usage_cost_get(org_id, from_date, to_date, &filter_refs)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Phase 4 - Member endpoints (delegated to library client)
    pub async fn list_members(
        &self,
        org_id: &str,
    ) -> Result<Vec<clickhouse_cloud_api::models::Member>> {
        let response = self
            .api()
            .member_get_list(org_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_member(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> Result<clickhouse_cloud_api::models::Member> {
        let response = self
            .api()
            .member_get(org_id, user_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn update_member(
        &self,
        org_id: &str,
        user_id: &str,
        request: &clickhouse_cloud_api::models::MemberPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::Member> {
        let response = self
            .api()
            .member_update(org_id, user_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_member(&self, org_id: &str, user_id: &str) -> Result<DeleteResponse> {
        let response = self
            .api()
            .member_delete(org_id, user_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(DeleteResponse {
            status: response.status.unwrap_or(0.0),
            request_id: response.request_id.unwrap_or_default(),
        })
    }

    // Phase 4 - Invitation endpoints (delegated to library client)
    pub async fn list_invitations(
        &self,
        org_id: &str,
    ) -> Result<Vec<clickhouse_cloud_api::models::Invitation>> {
        let response = self
            .api()
            .invitation_get_list(org_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn create_invitation(
        &self,
        org_id: &str,
        request: &clickhouse_cloud_api::models::InvitationPostRequest,
    ) -> Result<clickhouse_cloud_api::models::Invitation> {
        let response = self
            .api()
            .invitation_create(org_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> Result<clickhouse_cloud_api::models::Invitation> {
        let response = self
            .api()
            .invitation_get(org_id, invitation_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> Result<DeleteResponse> {
        let response = self
            .api()
            .invitation_delete(org_id, invitation_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(DeleteResponse {
            status: response.status.unwrap_or(0.0),
            request_id: response.request_id.unwrap_or_default(),
        })
    }

    // Phase 5 - API Key endpoints (delegated to library client)
    pub async fn list_api_keys(
        &self,
        org_id: &str,
    ) -> Result<Vec<clickhouse_cloud_api::models::ApiKey>> {
        let response = self
            .api()
            .openapi_key_get_list(org_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn create_api_key(
        &self,
        org_id: &str,
        request: &clickhouse_cloud_api::models::ApiKeyPostRequest,
    ) -> Result<clickhouse_cloud_api::models::ApiKeyPostResponse> {
        let response = self
            .api()
            .openapi_key_create(org_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_api_key(
        &self,
        org_id: &str,
        key_id: &str,
    ) -> Result<clickhouse_cloud_api::models::ApiKey> {
        let response = self
            .api()
            .openapi_key_get(org_id, key_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn update_api_key(
        &self,
        org_id: &str,
        key_id: &str,
        request: &clickhouse_cloud_api::models::ApiKeyPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::ApiKey> {
        let response = self
            .api()
            .openapi_key_update(org_id, key_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_api_key(&self, org_id: &str, key_id: &str) -> Result<DeleteResponse> {
        let response = self
            .api()
            .openapi_key_delete(org_id, key_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(DeleteResponse {
            status: response.status.unwrap_or(0.0),
            request_id: response.request_id.unwrap_or_default(),
        })
    }

    // Phase 6 - Activity endpoints
    pub async fn list_activities(
        &self,
        org_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<Vec<clickhouse_cloud_api::models::Activity>> {
        let response = self
            .api()
            .activity_get_list(org_id, from_date, to_date)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_activity(
        &self,
        org_id: &str,
        activity_id: &str,
    ) -> Result<clickhouse_cloud_api::models::Activity> {
        let response = self
            .api()
            .activity_get(org_id, activity_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Backup Config endpoints (delegated to library client)
    pub async fn get_backup_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<clickhouse_cloud_api::models::BackupConfiguration> {
        let response = self
            .api()
            .backup_configuration_get(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn update_backup_config(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::BackupConfigurationPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::BackupConfiguration> {
        let response = self
            .api()
            .backup_configuration_update(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // ClickPipe endpoints (delegated to library client)
    pub async fn list_clickpipes(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<Vec<clickhouse_cloud_api::models::ClickPipe>> {
        let response = self
            .api()
            .click_pipe_get_list(org_id, service_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .api()
            .click_pipe_get(org_id, service_id, clickpipe_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn create_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipePostRequest,
    ) -> Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .api()
            .click_pipe_create(org_id, service_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> Result<DeleteResponse> {
        let response = self
            .api()
            .click_pipe_delete(org_id, service_id, clickpipe_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(DeleteResponse {
            status: response.status.unwrap_or(0.0),
            request_id: response.request_id.unwrap_or_default(),
        })
    }

    pub async fn change_clickpipe_state(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        command: clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand,
    ) -> Result<clickhouse_cloud_api::models::ClickPipe> {
        use clickhouse_cloud_api::models::ClickPipeStatePatchRequest;
        let request = ClickPipeStatePatchRequest {
            command: Some(command),
        };
        let response = self
            .api()
            .click_pipe_state_update(org_id, service_id, clickpipe_id, &request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeScalingPatchRequest,
    ) -> Result<clickhouse_cloud_api::models::ClickPipe> {
        let response = self
            .api()
            .click_pipe_scaling_update(org_id, service_id, clickpipe_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe_settings(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> Result<clickhouse_cloud_api::models::ClickPipeSettings> {
        let response = self
            .api()
            .click_pipe_settings_get(org_id, service_id, clickpipe_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_settings(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        request: &clickhouse_cloud_api::models::ClickPipeSettingsPutRequest,
    ) -> Result<clickhouse_cloud_api::models::ClickPipeSettings> {
        let response = self
            .api()
            .click_pipe_settings_update(org_id, service_id, clickpipe_id, request)
            .await
            .map_err(|e| self.convert_error(e))?;
        Self::unwrap_response(response)
    }

    // Helper to get the default organization
    pub async fn get_default_org_id(&self) -> Result<String> {
        let orgs = self.list_organizations().await?;
        match orgs.len() {
            0 => Err(CloudError {
                message: "No organization found for this API key".into(),
            }),
            1 => Ok(orgs[0].id.to_string()),
            _ => Err(CloudError {
                message: "Multiple organizations found. Specify --org-id to choose one. \
                          Use `clickhousectl cloud org list` to see your organizations."
                    .into(),
            }),
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
            auth_mode: AuthMode::Basic,
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
            lib_base_url("https://api.clickhouse-staging.com/v1"),
            "https://api.clickhouse-staging.com"
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
            status: Some(200.0),
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
                status: Some(200.0),
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
        assert!(AuthSource::CredentialsFile.describe().contains("credentials"));
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

    // ── Dotenv resolver tests ──────────────────────────────────────────────
    //
    // Precedence is exercised by feeding `resolve_auth_with_sources` a
    // synthetic `(env_map, dotenv)` pair. We deliberately do NOT mutate the
    // real process environment: `std::env::set_var` is `unsafe` in edition
    // 2024 because it races with `getenv` across threads, and a mutex
    // around the test body cannot prevent concurrently-running tests from
    // calling `env::var` and tripping that race.

    fn dotenv_with(pairs: &[(&str, &str)]) -> DotenvVars {
        let mut map = std::collections::HashMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.to_string());
        }
        DotenvVars::from_map_for_tests(map, Some(std::path::PathBuf::from("/synthetic/.env")))
    }

    fn env_map(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn lookup_from(map: &std::collections::HashMap<String, String>) -> impl Fn(&str) -> Option<String> + '_ {
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn dotenv_only_resolves_to_env_source() {
        let dotenv = dotenv_with(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "dot_k"),
            ("CLICKHOUSE_CLOUD_API_SECRET", "dot_s"),
        ]);
        let env = env_map(&[]);
        let lookup = lookup_from(&env);
        let resolved =
            resolve_auth_with_sources(None, None, None, &dotenv, &lookup).unwrap();
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
        let resolved =
            resolve_auth_with_sources(None, None, None, &dotenv, &lookup).unwrap();
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
        let resolved =
            resolve_auth_with_sources(None, None, None, &dotenv, &lookup).unwrap();
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
}
