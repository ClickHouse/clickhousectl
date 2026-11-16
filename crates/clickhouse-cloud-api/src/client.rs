//! HTTP client for the ClickHouse Cloud API.

use crate::error::Error;
use crate::meta::OperationMetadata;

mod activity;
mod api_keys;
mod backups;
mod byoc;
mod clickpipes;
mod clickstack;
mod invitations;
mod jwt_providers;
mod members;
mod organizations;
mod postgres;
mod query;
mod query_api_endpoints;
mod rbac;
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

    /// Start a request for a Cloud API operation.
    ///
    /// The method and path template come from the operation's generated
    /// [`OperationMetadata`], so a hand-typed path cannot drift from the spec.
    /// `params` fill the template's `{placeholder}` segments in order; each is
    /// sent as one percent-encoded segment (see [`endpoint_url`]).
    fn request(
        &self,
        operation: &OperationMetadata,
        params: &[&str],
    ) -> Result<reqwest::RequestBuilder, Error> {
        let method = reqwest::Method::from_bytes(operation.method.as_bytes())
            .expect("generated operation metadata holds a valid HTTP method");
        let url = endpoint_url(&self.base_url, operation.path, params)?;
        let builder = self.http.request(method, url);
        Ok(match &self.auth {
            Auth::Basic { key_id, key_secret } => builder.basic_auth(key_id, Some(key_secret)),
            Auth::Bearer { token } => builder.bearer_auth(token),
        })
    }
}

/// Join `base` and a path `template`, sending each parameter as one segment.
///
/// `template` is a `/`-separated path whose `{placeholder}` segments are
/// replaced, in order, by `params`. A parameter is percent-encoded, so `/`,
/// `?`, `#` and `%` inside an ID stay inside its segment and cannot re-target
/// the request. URL parsing always resolves `.` and `..` segments (encoded or
/// not), and an empty segment changes the route, so such a parameter is
/// rejected with [`Error::InvalidPathParameter`] before any request is made.
///
/// A `base` that is not a valid base URL is passed through unchanged, so the
/// HTTP client reports it when the request is sent, as it always has.
fn endpoint_url(base: &str, template: &str, params: &[&str]) -> Result<String, Error> {
    let placeholders = template
        .split('/')
        .filter(|segment| segment.starts_with('{'))
        .count();
    assert_eq!(
        placeholders,
        params.len(),
        "path template {template} takes {placeholders} parameter(s)"
    );
    if let Some(bad) = params.iter().find(|p| matches!(**p, "" | "." | "..")) {
        return Err(Error::InvalidPathParameter((*bad).to_string()));
    }
    let Ok(mut url) = url::Url::parse(base) else {
        return Ok(format!("{base}{template}"));
    };
    let mut params = params.iter();
    match url.path_segments_mut() {
        Ok(mut segments) => {
            segments.pop_if_empty();
            for segment in template.split('/').filter(|s| !s.is_empty()) {
                if segment.starts_with('{') {
                    segments.push(params.next().expect("counted above"));
                } else {
                    segments.push(segment);
                }
            }
        }
        Err(()) => return Ok(format!("{base}{template}")),
    }
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://api.example.test";
    const TEMPLATE: &str = "/v1/organizations/{organizationId}/services/{serviceId}";

    fn url(params: &[&str]) -> String {
        endpoint_url(BASE, TEMPLATE, params).unwrap()
    }

    #[test]
    fn plain_ids_are_unchanged() {
        assert_eq!(
            url(&["org-1", "6b1f8c1e-2f8a-4c39-9a51-0f6f1d3c2b7a"]),
            "https://api.example.test/v1/organizations/org-1/services/6b1f8c1e-2f8a-4c39-9a51-0f6f1d3c2b7a"
        );
    }

    #[test]
    fn reserved_characters_stay_inside_their_segment() {
        assert_eq!(
            url(&["org-1", "svc-1/../../keys/key-9"]),
            "https://api.example.test/v1/organizations/org-1/services/svc-1%2F..%2F..%2Fkeys%2Fkey-9"
        );
        assert_eq!(
            url(&["org-1", "a?b#c%d"]),
            "https://api.example.test/v1/organizations/org-1/services/a%3Fb%23c%25d"
        );
        assert_eq!(
            url(&["org-1", "%2e%2e"]),
            "https://api.example.test/v1/organizations/org-1/services/%252e%252e"
        );
    }

    #[test]
    fn dot_and_empty_segments_are_rejected() {
        for bad in ["..", ".", ""] {
            let err = endpoint_url(BASE, TEMPLATE, &["org-1", bad]).unwrap_err();
            assert!(
                matches!(&err, Error::InvalidPathParameter(value) if value == bad),
                "{bad:?}: {err:?}"
            );
        }
    }

    #[test]
    fn base_path_prefix_is_kept() {
        assert_eq!(
            endpoint_url("http://127.0.0.1:8080/api", "/v1/whoami", &[]).unwrap(),
            "http://127.0.0.1:8080/api/v1/whoami"
        );
    }

    #[test]
    #[should_panic(expected = "takes 2 parameter(s)")]
    fn parameter_count_must_match_the_template() {
        let _ = endpoint_url(BASE, TEMPLATE, &["org-1"]);
    }
}
