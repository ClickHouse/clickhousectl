//! HTTP client for the ClickHouse Cloud API.
//!
//! Auto-generated from the OpenAPI specification.

use crate::error::Error;

mod activity;
mod api_keys;
mod backups;
mod clickpipes;
mod clickstack;
mod organizations;
mod postgres;
mod services;
mod udfs;

/// Authentication mode for the API client.
#[derive(Debug, Clone)]
enum Auth {
    Basic { key_id: String, key_secret: String },
    Bearer { token: String },
}

/// Credentials for a single Query API request. Basic carries a per-service
/// Query API key; Bearer carries the user's OAuth token.
enum QueryAuth<'a> {
    Basic {
        key_id: &'a str,
        key_secret: &'a str,
    },
    Bearer {
        token: &'a str,
    },
}

/// ClickHouse Cloud API client.
///
/// Supports both HTTP Basic Auth (API key/secret) and Bearer token (OAuth) authentication.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    auth: Auth,
    /// Explicit Query API host override; see [`Client::with_query_host`].
    query_host: Option<String>,
}

/// Derive the Query API host from a management API base URL by swapping the
/// `api.` host prefix for `queries.`, so each environment talks to its own
/// query host. Staging and dev serve the management API under an extra
/// `control-plane.` label that the query host doesn't have, so it is
/// dropped too:
///
/// - `https://api.clickhouse.cloud` → `https://queries.clickhouse.cloud`
/// - `https://api.control-plane.clickhouse-staging.com` →
///   `https://queries.clickhouse-staging.com`
///
/// Returns `None` when the base URL isn't of that shape (e.g. a localhost
/// test server).
fn derive_query_host(base_url: &str) -> Option<String> {
    let parsed = url::Url::parse(base_url).ok()?;
    let rest = parsed.host_str()?.strip_prefix("api.")?;
    let rest = rest.strip_prefix("control-plane.").unwrap_or(rest);
    let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    Some(format!("{}://queries.{}{}", parsed.scheme(), rest, port))
}

/// The ClickHouse error code and details a Query API failure body carries, or
/// `None` when the body is not a SQL-level error report.
fn query_api_sql_error(body: &str) -> Option<(String, String)> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let error = value.get("error")?;
    let code = match error.get("code")? {
        serde_json::Value::String(code) if !code.is_empty() => code.clone(),
        serde_json::Value::Number(code) => code.to_string(),
        _ => return None,
    };
    let details = error.get("details")?.as_str()?;
    if details.is_empty() {
        return None;
    }
    Some((code, details.to_string()))
}

/// Classify a Query API failure body: a SQL-level rejection becomes
/// [`Error::Sql`], anything else stays an [`Error::Api`]. The rendered text is
/// identical to what the single `Api` variant produced before, so callers that
/// only print the error see no change; callers that need to know *what kind of
/// failure this was* read the variant instead of the message.
fn query_api_error(status: reqwest::StatusCode, body: &str) -> Error {
    if query_api_reports_timeout(status, body) {
        return Error::QueryTimeout;
    }
    if let Some((code, details)) = query_api_sql_error(body) {
        return Error::Sql {
            status: status.as_u16(),
            code,
            details,
        };
    }
    Error::Api {
        status: status.as_u16(),
        message: if body.is_empty() {
            format!("Query API returned HTTP {status} with an empty response body")
        } else {
            format!("Query API returned HTTP {status}: {body}")
        },
    }
}

/// Whether a Query API failure is the gateway giving up on the statement:
/// HTTP 500 whose body is exactly `{"error": "Timeout error."}`.
///
/// The body is parsed as JSON and the `error` field compared in full, the same
/// shape as [`query_api_reports_stopped_service`]. A substring match on
/// `Timeout` would also fire on a ClickHouse error *about* a timeout that the
/// service itself reported, which is a different failure with a different
/// remedy.
fn query_api_reports_timeout(status: reqwest::StatusCode, body: &str) -> bool {
    const TIMEOUT_MESSAGE: &str = "Timeout error.";

    status == reqwest::StatusCode::INTERNAL_SERVER_ERROR
        && serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .is_some_and(|value| {
                value.get("error").and_then(serde_json::Value::as_str) == Some(TIMEOUT_MESSAGE)
            })
}

fn query_api_reports_stopped_service(status: reqwest::StatusCode, body: &str) -> bool {
    const STOPPED_SERVICE_MESSAGE: &str =
        "ClickHouse service is currently unavailable. Please try again later.";

    status == reqwest::StatusCode::NOT_FOUND
        && serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .is_some_and(|value| {
                value.get("error").and_then(serde_json::Value::as_str)
                    == Some(STOPPED_SERVICE_MESSAGE)
            })
}

impl Client {
    /// Create a new client with the default base URL (`https://api.clickhouse.cloud`).
    pub fn new(key_id: impl Into<String>, key_secret: impl Into<String>) -> Self {
        Self::with_base_url("https://api.clickhouse.cloud", key_id, key_secret)
    }

    /// Create a new client with a custom base URL.
    pub fn with_base_url(
        base_url: impl Into<String>,
        key_id: impl Into<String>,
        key_secret: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            auth: Auth::Basic {
                key_id: key_id.into(),
                key_secret: key_secret.into(),
            },
            query_host: None,
        }
    }

    /// Create a new client with Bearer token authentication and a custom base URL.
    pub fn with_bearer_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            auth: Auth::Bearer {
                token: token.into(),
            },
            query_host: None,
        }
    }

    /// Create a new client with a pre-built HTTP client and Basic auth.
    ///
    /// Use this when you need to customize the underlying `reqwest::Client`
    /// (e.g. to set a custom user-agent or timeout).
    pub fn with_http_client(
        http: reqwest::Client,
        base_url: impl Into<String>,
        key_id: impl Into<String>,
        key_secret: impl Into<String>,
    ) -> Self {
        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            auth: Auth::Basic {
                key_id: key_id.into(),
                key_secret: key_secret.into(),
            },
            query_host: None,
        }
    }

    /// Create a new client with a pre-built HTTP client and Bearer auth.
    ///
    /// Use this when you need to customize the underlying `reqwest::Client`
    /// (e.g. to set a custom user-agent or timeout).
    pub fn with_http_client_bearer(
        http: reqwest::Client,
        base_url: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            auth: Auth::Bearer {
                token: token.into(),
            },
            query_host: None,
        }
    }

    /// Replace the Bearer token without rebuilding the client.
    ///
    /// Useful for refreshing an expired OAuth token.
    /// Returns an error if the client is using Basic auth.
    pub fn set_bearer_token(&mut self, token: impl Into<String>) -> Result<(), Error> {
        match &mut self.auth {
            Auth::Bearer { token: t } => {
                *t = token.into();
                Ok(())
            }
            Auth::Basic { .. } => Err(Error::AuthMismatch(
                "set_bearer_token called on a Basic-auth client".into(),
            )),
        }
    }

    /// Override the Query API host used by [`Client::run_query`] and
    /// [`Client::run_query_bearer`].
    ///
    /// When not set, the host is taken from the `CLICKHOUSE_CLOUD_QUERY_HOST`
    /// env var if present, otherwise derived from the client's base URL
    /// (`api.<domain>` → `queries.<domain>`), falling back to the production
    /// host `https://queries.clickhouse.cloud`.
    pub fn with_query_host(mut self, host: impl Into<String>) -> Self {
        self.query_host = Some(host.into().trim_end_matches('/').to_string());
        self
    }

    /// Resolve the Query API host: explicit override, then env var, then
    /// derivation from the base URL, then the production default.
    fn resolved_query_host(&self) -> String {
        if let Some(host) = &self.query_host {
            return host.clone();
        }
        if let Ok(host) = std::env::var("CLICKHOUSE_CLOUD_QUERY_HOST") {
            return host;
        }
        derive_query_host(&self.base_url)
            .unwrap_or_else(|| "https://queries.clickhouse.cloud".to_string())
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        match &self.auth {
            Auth::Basic { key_id, key_secret } => builder.basic_auth(key_id, Some(key_secret)),
            Auth::Bearer { token } => builder.bearer_auth(token),
        }
    }

    /// Run a SQL statement against a service's Query API endpoint.
    ///
    /// Hits the environment's query host (see [`Client::with_query_host`]
    /// for resolution order) using Basic auth with the provided
    /// `key_id`/`key_secret` — a per-service key bound to a query endpoint
    /// with role `sql_console_read_only` (or `sql_console_admin`). This
    /// bypasses the client's primary auth because Query API keys are scoped
    /// to a single service.
    ///
    /// `wake_service` resends the wake confirmation the query host asks for
    /// when the target service is idled — see [`Error::ServiceIdle`].
    ///
    /// Returns the streaming response so the caller can forward it to
    /// stdout or buffer it into memory.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_query(
        &self,
        service_id: &str,
        key_id: &str,
        key_secret: &str,
        sql: &str,
        database: Option<&str>,
        format: &str,
        wake_service: bool,
    ) -> Result<reqwest::Response, Error> {
        self.run_query_with(
            QueryAuth::Basic { key_id, key_secret },
            service_id,
            sql,
            database,
            format,
            wake_service,
        )
        .await
    }

    /// Run a SQL statement against a service's Query API endpoint using the
    /// client's own OAuth Bearer token.
    ///
    /// Unlike [`Client::run_query`], no per-service Query API key and no
    /// query-endpoint configuration are needed: the Query API authenticates
    /// the user's identity directly and grants read-only SQL access (SELECT
    /// and other read statements only; no INSERT, DDL, or other writes).
    ///
    /// `wake_service` resends the wake confirmation the query host asks for
    /// when the target service is idled — see [`Error::ServiceIdle`].
    ///
    /// Returns an error if the client is using Basic auth.
    pub async fn run_query_bearer(
        &self,
        service_id: &str,
        sql: &str,
        database: Option<&str>,
        format: &str,
        wake_service: bool,
    ) -> Result<reqwest::Response, Error> {
        let token = match &self.auth {
            Auth::Bearer { token } => token,
            Auth::Basic { .. } => {
                return Err(Error::AuthMismatch(
                    "run_query_bearer called on a Basic-auth client".into(),
                ));
            }
        };
        self.run_query_with(
            QueryAuth::Bearer { token },
            service_id,
            sql,
            database,
            format,
            wake_service,
        )
        .await
    }

    async fn run_query_with(
        &self,
        auth: QueryAuth<'_>,
        service_id: &str,
        sql: &str,
        database: Option<&str>,
        format: &str,
        wake_service: bool,
    ) -> Result<reqwest::Response, Error> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct RunQueryBody<'a> {
            run_id: String,
            sql: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            database: Option<&'a str>,
        }

        let url = format!(
            "{}/service/{}/run",
            self.resolved_query_host().trim_end_matches('/'),
            service_id,
        );

        let body = RunQueryBody {
            run_id: uuid::Uuid::new_v4().to_string(),
            sql,
            database,
        };

        let request = self
            .http
            .post(url)
            .query(&[("format", format)])
            .header("content-type", "text/plain;charset=UTF-8")
            .header("x-service-type", "clickhouse");
        // `wake-service: true` is the wake confirmation the query host asks
        // for via a 206 `Confirm wake service` response (the SQL console
        // sends it after prompting the user).
        let request = if wake_service {
            request.header("wake-service", "true")
        } else {
            request
        };
        // `auth-provider: custom` tells the query host the credentials are a
        // custom (user-provisioned) Query API key. Bearer tokens carry their
        // own provider information, so the header is omitted for them.
        let request = match auth {
            QueryAuth::Basic { key_id, key_secret } => request
                .basic_auth(key_id, Some(key_secret))
                .header("auth-provider", "custom"),
            QueryAuth::Bearer { token } => request.bearer_auth(token),
        };

        let response = request.json(&body).send().await?;

        let status = response.status();
        // 206 means the service can't take the query in its current state:
        // `Confirm wake service` for an idled service (resend with the
        // wake confirmation to wake it and run the query), `Service is
        // stopped` for one that must be started explicitly.
        if status.as_u16() == 206 {
            let body_text = response.text().await.map_err(|error| Error::Api {
                status: status.as_u16(),
                message: format!(
                    "Query API returned HTTP {status}, but its response body could not be read: {error}"
                ),
            })?;
            #[derive(serde::Deserialize)]
            struct StateBody {
                data: Option<String>,
            }
            let data = serde_json::from_str::<StateBody>(&body_text)
                .ok()
                .and_then(|b| b.data);
            return Err(match data.as_deref() {
                Some("Confirm wake service") => Error::ServiceIdle,
                Some("Service is stopped") => Error::ServiceStopped,
                _ => query_api_error(status, &body_text),
            });
        }
        if !status.is_success() {
            let body_text = response.text().await.map_err(|error| Error::Api {
                status: status.as_u16(),
                message: format!(
                    "Query API returned HTTP {status}, but its response body could not be read: {error}"
                ),
            })?;
            if query_api_reports_stopped_service(status, &body_text) {
                return Err(Error::ServiceStopped);
            }
            return Err(query_api_error(status, &body_text));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, derive_query_host, query_api_error};

    #[test]
    fn derive_query_host_prod() {
        assert_eq!(
            derive_query_host("https://api.clickhouse.cloud").as_deref(),
            Some("https://queries.clickhouse.cloud")
        );
    }

    #[test]
    fn derive_query_host_staging() {
        assert_eq!(
            derive_query_host("https://api.control-plane.clickhouse-staging.com").as_deref(),
            Some("https://queries.clickhouse-staging.com")
        );
    }

    #[test]
    fn derive_query_host_dev() {
        assert_eq!(
            derive_query_host("https://api.control-plane.clickhouse-dev.com").as_deref(),
            Some("https://queries.clickhouse-dev.com")
        );
    }

    #[test]
    fn derive_query_host_plain_api_prefix_without_control_plane() {
        assert_eq!(
            derive_query_host("https://api.clickhouse-staging.com").as_deref(),
            Some("https://queries.clickhouse-staging.com")
        );
    }

    #[test]
    fn derive_query_host_non_api_host_is_none() {
        assert_eq!(derive_query_host("http://127.0.0.1:8123"), None);
        assert_eq!(derive_query_host("https://example.com"), None);
    }

    #[test]
    fn derive_query_host_invalid_url_is_none() {
        assert_eq!(derive_query_host("not a url"), None);
    }

    #[test]
    fn derive_query_host_preserves_non_default_port() {
        assert_eq!(
            derive_query_host("https://api.mycorp.example.com:8443").as_deref(),
            Some("https://queries.mycorp.example.com:8443")
        );
        // Default ports are normalized away by the URL parser and stay off
        // the derived host.
        assert_eq!(
            derive_query_host("https://api.clickhouse.cloud:443").as_deref(),
            Some("https://queries.clickhouse.cloud")
        );
    }

    #[test]
    fn query_api_error_extracts_documented_sql_error() {
        let body = r#"{"error":{"code":"62","details":"Syntax error","extra":"ignored"}}"#;
        let error = query_api_error(reqwest::StatusCode::BAD_REQUEST, body);
        assert!(
            matches!(
                &error,
                Error::Sql { status: 400, code, details }
                    if code == "62" && details == "Syntax error"
            ),
            "expected a typed SQL error, got {error:?}"
        );
        // The rendered text is what it always was, so nothing user-facing
        // changes with the variant split.
        assert_eq!(error.to_string(), "SQL error 62: Syntax error");
    }

    #[test]
    fn query_api_error_accepts_numeric_codes() {
        let body = r#"{"error":{"code":241,"details":"Memory limit exceeded"}}"#;
        let error = query_api_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, body);
        assert!(
            matches!(
                &error,
                Error::Sql { status: 500, code, details }
                    if code == "241" && details == "Memory limit exceeded"
            ),
            "expected a typed SQL error, got {error:?}"
        );
        assert_eq!(error.to_string(), "SQL error 241: Memory limit exceeded");
    }

    #[test]
    fn query_api_error_preserves_status_and_unrecognized_body() {
        let malformed = r#"{"error":{"code":"62","details":"truncated"#;
        let error = query_api_error(reqwest::StatusCode::BAD_REQUEST, malformed);
        assert!(
            matches!(&error, Error::Api { status: 400, .. }),
            "a body that is not a SQL error report must stay an API error: {error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!("API error (status 400): Query API returned HTTP 400 Bad Request: {malformed}")
        );

        let error = query_api_error(reqwest::StatusCode::BAD_GATEWAY, "upstream failed");
        assert!(matches!(&error, Error::Api { status: 502, .. }));
        assert_eq!(
            error.to_string(),
            "API error (status 502): Query API returned HTTP 502 Bad Gateway: upstream failed"
        );
    }

    #[test]
    fn query_api_error_recognizes_the_gateway_timeout() {
        let error = query_api_error(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"error":"Timeout error."}"#,
        );
        assert!(
            matches!(&error, Error::QueryTimeout),
            "the gateway timeout must be its own variant, got {error:?}"
        );
        // The text is what a user sees, so it is pinned: it says the response
        // was lost, not that the statement was.
        assert_eq!(
            error.to_string(),
            "the query timed out at the Query API gateway; the statement may still be running on \
             the service"
        );
    }

    #[test]
    fn query_api_error_keeps_other_500s_generic() {
        for body in [
            // A different gateway message.
            r#"{"error":"Internal error."}"#,
            // A body that merely mentions a timeout: the service reported it,
            // the gateway did not give up.
            r#"{"error":"Timeout exceeded while reading from socket"}"#,
            // Right message, wrong shape.
            r#"{"error":{"message":"Timeout error."}}"#,
            // Not JSON at all.
            "Timeout error.",
            "",
        ] {
            let error = query_api_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, body);
            assert!(
                matches!(&error, Error::Api { status: 500, .. }),
                "only the exact gateway timeout body may become QueryTimeout: {body} -> {error:?}"
            );
        }
    }

    #[test]
    fn query_api_error_only_reads_the_timeout_out_of_a_500() {
        // The same body under any other status is not the gateway timeout.
        for status in [
            reqwest::StatusCode::PARTIAL_CONTENT,
            reqwest::StatusCode::BAD_REQUEST,
            reqwest::StatusCode::BAD_GATEWAY,
            reqwest::StatusCode::GATEWAY_TIMEOUT,
        ] {
            let error = query_api_error(status, r#"{"error":"Timeout error."}"#);
            assert!(
                matches!(&error, Error::Api { .. }),
                "status {status} must not produce QueryTimeout: {error:?}"
            );
        }
    }

    #[test]
    fn query_api_error_needs_both_code_and_details_to_be_sql() {
        for body in [
            r#"{"error":{"code":"62"}}"#,
            r#"{"error":{"details":"Syntax error"}}"#,
            r#"{"error":{"code":"","details":"Syntax error"}}"#,
            r#"{"error":{"code":"62","details":""}}"#,
            r#"{"error":"ClickHouse service is currently unavailable."}"#,
            "",
        ] {
            assert!(
                matches!(
                    query_api_error(reqwest::StatusCode::BAD_REQUEST, body),
                    Error::Api { .. }
                ),
                "partial SQL error body must not become Error::Sql: {body}"
            );
        }
    }
}
