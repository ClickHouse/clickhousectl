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
mod query;
mod query_api_endpoints;
mod saved_queries;
mod services;
mod udfs;
mod whoami;

/// Authentication mode for the API client.
#[derive(Debug, Clone)]
enum Auth {
    Basic { key_id: String, key_secret: String },
    Bearer { token: String },
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

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        match &self.auth {
            Auth::Basic { key_id, key_secret } => builder.basic_auth(key_id, Some(key_secret)),
            Auth::Bearer { token } => builder.bearer_auth(token),
        }
    }
}
