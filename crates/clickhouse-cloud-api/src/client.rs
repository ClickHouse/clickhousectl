//! HTTP client for the ClickHouse Cloud API.
//!
//! Auto-generated from the OpenAPI specification.

use crate::error::Error;
use crate::models::*;

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
    extra_query_params: Vec<(String, String)>,
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
            extra_query_params: Vec::new(),
        }
    }

    /// Create a new client with Bearer token authentication and a custom base URL.
    pub fn with_bearer_token(
        base_url: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            auth: Auth::Bearer {
                token: token.into(),
            },
            extra_query_params: Vec::new(),
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
            extra_query_params: Vec::new(),
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
            extra_query_params: Vec::new(),
        }
    }

    /// Attach extra query parameters that should be appended to every request
    /// this client makes. Useful for callers that want to surface a CLI- or
    /// runtime-level signal (e.g. an `agent` tag) to the API for analytics.
    ///
    /// Multiple calls accumulate; existing params are preserved.
    pub fn with_extra_query_params<I, K, V>(mut self, params: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.extra_query_params
            .extend(params.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
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
        let mut builder = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        builder = match &self.auth {
            Auth::Basic { key_id, key_secret } => builder.basic_auth(key_id, Some(key_secret)),
            Auth::Bearer { token } => builder.bearer_auth(token),
        };
        if !self.extra_query_params.is_empty() {
            builder = builder.query(&self.extra_query_params);
        }
        builder
    }

    /// Get list of available organizations
    pub async fn organization_get_list(
        &self,
    ) -> Result<ApiResponse<Vec<Organization>>, Error> {
        let path = "/v1/organizations".to_string();
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization details
    pub async fn organization_get(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Organization>, Error> {
        let path = format!("/v1/organizations/{organization_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update organization details
    pub async fn organization_update(
        &self,
        organization_id: &str,
        body: &OrganizationPatchRequest,
    ) -> Result<ApiResponse<Organization>, Error> {
        let path = format!("/v1/organizations/{organization_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List of organization activities
    pub async fn activity_get_list(
        &self,
        organization_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<ApiResponse<Vec<Activity>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/activities");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = from_date {
            req = req.query(&[("from_date", v)]);
        }
        if let Some(v) = to_date {
            req = req.query(&[("to_date", v)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Organization activity
    pub async fn activity_get(
        &self,
        organization_id: &str,
        activity_id: &str,
    ) -> Result<ApiResponse<Activity>, Error> {
        let path = format!("/v1/organizations/{organization_id}/activities/{activity_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_create(
        &self,
        organization_id: &str,
        body: &ByocInfrastructurePostRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Remove a BYOC infrastructure
    pub async fn organization_byoc_infrastructure_delete(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_update(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
        body: &ByocInfrastructurePatchRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List all invitations
    pub async fn invitation_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<Invitation>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create an invitation
    pub async fn invitation_create(
        &self,
        organization_id: &str,
        body: &InvitationPostRequest,
    ) -> Result<ApiResponse<Invitation>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get invitation details
    pub async fn invitation_get(
        &self,
        organization_id: &str,
        invitation_id: &str,
    ) -> Result<ApiResponse<Invitation>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations/{invitation_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete organization invitation
    pub async fn invitation_delete(
        &self,
        organization_id: &str,
        invitation_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations/{invitation_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get list of all keys
    pub async fn openapi_key_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<ApiKey>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create key
    pub async fn openapi_key_create(
        &self,
        organization_id: &str,
        body: &ApiKeyPostRequest,
    ) -> Result<ApiResponse<ApiKeyPostResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get key details
    pub async fn openapi_key_get(
        &self,
        organization_id: &str,
        key_id: &str,
    ) -> Result<ApiResponse<ApiKey>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys/{key_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update key
    pub async fn openapi_key_update(
        &self,
        organization_id: &str,
        key_id: &str,
        body: &ApiKeyPatchRequest,
    ) -> Result<ApiResponse<ApiKey>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys/{key_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete key
    pub async fn openapi_key_delete(
        &self,
        organization_id: &str,
        key_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/keys/{key_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List organization members
    pub async fn member_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<Member>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get member details
    pub async fn member_get(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<ApiResponse<Member>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members/{user_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update organization member
    pub async fn member_update(
        &self,
        organization_id: &str,
        user_id: &str,
        body: &MemberPatchRequest,
    ) -> Result<ApiResponse<Member>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members/{user_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Remove an organization member
    pub async fn member_delete(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members/{user_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create new Postgres service
    pub async fn postgres_service_create(
        &self,
        organization_id: &str,
        body: &PostgresServicePostRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List of organization Postgres services
    pub async fn postgres_service_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<PostgresServiceListItem>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get PostgreSQL service details
    pub async fn postgres_service_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete a PostgreSQL service
    pub async fn postgres_service_delete(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update a PostgreSQL service
    pub async fn postgres_service_patch(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServicePatchRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get Postgres CA certs
    pub async fn postgres_service_certs_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/caCertificates");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(body_text)
    }

    /// Get PostgreSQL service configuration
    pub async fn postgres_instance_config_get(
        &self,
        organization_id: &str,
        postgres_id: &str,
    ) -> Result<ApiResponse<PostgresInstanceConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/config");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Replace Postgres service configuration
    pub async fn postgres_instance_config_post(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresInstanceConfig,
    ) -> Result<ApiResponse<PostgresInstanceUpdateConfigResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/config");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update Postgres service configuration
    pub async fn postgres_instance_config_patch(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresInstanceConfig,
    ) -> Result<ApiResponse<PostgresInstanceUpdateConfigResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/config");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update Postgres superuser password
    pub async fn postgres_service_set_password(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceSetPassword,
    ) -> Result<ApiResponse<PostgresServicePasswordResource>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/password");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create a read replica for a Postgres service
    pub async fn postgres_instance_create_read_replica(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceReadReplicaRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/readReplica");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Restore a Postgres service
    pub async fn postgres_instance_restore(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceRestoreRequest,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/restoredService");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update Postgres service state
    pub async fn postgres_service_patch_state(
        &self,
        organization_id: &str,
        postgres_id: &str,
        body: &PostgresServiceSetState,
    ) -> Result<ApiResponse<PostgresService>, Error> {
        let path = format!("/v1/organizations/{organization_id}/postgres/{postgres_id}/state");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get private endpoint configuration for region within cloud provider for an organization
    #[deprecated]
    #[allow(deprecated)]
    pub async fn organization_private_endpoint_config_get_list(
        &self,
        organization_id: &str,
        cloud_provider: &str,
        region_id: &str,
    ) -> Result<ApiResponse<OrganizationCloudRegionPrivateEndpointConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/privateEndpointConfig");
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[("cloud_provider", cloud_provider)]);
        req = req.query(&[("region_id", region_id)]);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization metrics
    pub async fn organization_prometheus_get(
        &self,
        organization_id: &str,
        filtered_metrics: Option<&str>,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/prometheus");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = filtered_metrics {
            req = req.query(&[("filtered_metrics", v)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text),
            });
        }
        Ok(resp.text().await?)
    }

    /// List of organization services
    pub async fn instance_get_list(
        &self,
        organization_id: &str,
        filters: &[&str],
    ) -> Result<ApiResponse<Vec<Service>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services");
        let mut req = self.request(reqwest::Method::GET, &path);
        for f in filters {
            req = req.query(&[("filter", f)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create new service
    pub async fn instance_create(
        &self,
        organization_id: &str,
        body: &ServicePostRequest,
    ) -> Result<ApiResponse<ServicePostResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get service details
    pub async fn instance_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update service basic details
    pub async fn instance_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicePatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete service
    pub async fn instance_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get service backup bucket
    pub async fn backup_bucket_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create service backup bucket
    pub async fn backup_bucket_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupBucketPostRequest,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update service backup bucket
    pub async fn backup_bucket_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupBucketPatchRequest,
    ) -> Result<ApiResponse<BackupBucket>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete service backup bucket
    pub async fn backup_bucket_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backupBucket");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get service backup configuration
    pub async fn backup_configuration_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<BackupConfiguration>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backupConfiguration");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update service backup configuration
    pub async fn backup_configuration_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &BackupConfigurationPatchRequest,
    ) -> Result<ApiResponse<BackupConfiguration>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backupConfiguration");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List of service backups
    pub async fn backup_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<Backup>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backups");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get backup details
    pub async fn backup_get(
        &self,
        organization_id: &str,
        service_id: &str,
        backup_id: &str,
    ) -> Result<ApiResponse<Backup>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/backups/{backup_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List ClickPipes
    pub async fn click_pipe_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickPipe>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create ClickPipe
    pub async fn click_pipe_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickPipePostRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get ClickPipe
    pub async fn click_pipe_get(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe
    pub async fn click_pipe_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipePatchRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete ClickPipe
    pub async fn click_pipe_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe scaling
    pub async fn click_pipe_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipeScalingPatchRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/scaling");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get ClickPipe settings
    pub async fn click_pipe_settings_get(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
    ) -> Result<ApiResponse<ClickPipeSettings>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/settings");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe settings
    pub async fn click_pipe_settings_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipeSettingsPutRequest,
    ) -> Result<ApiResponse<ClickPipeSettings>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/settings");
        let mut req = self.request(reqwest::Method::PUT, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update ClickPipe state
    pub async fn click_pipe_state_update(
        &self,
        organization_id: &str,
        service_id: &str,
        click_pipe_id: &str,
        body: &ClickPipeStatePatchRequest,
    ) -> Result<ApiResponse<ClickPipe>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipes/{click_pipe_id}/state");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get CDC ClickPipes scaling
    pub async fn click_pipe_cdc_scaling_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ClickPipesCdcScaling>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipesCdcScaling");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update CDC ClickPipes scaling
    pub async fn click_pipe_cdc_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickPipesCdcScalingPatchRequest,
    ) -> Result<ApiResponse<ClickPipesCdcScaling>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipesCdcScaling");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List reverse private endpoints
    pub async fn click_pipe_reverse_private_endpoint_get_list(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ReversePrivateEndpoint>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &CreateReversePrivateEndpoint,
    ) -> Result<ApiResponse<ReversePrivateEndpoint>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
        reverse_private_endpoint_id: &str,
    ) -> Result<ApiResponse<ReversePrivateEndpoint>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints/{reverse_private_endpoint_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete reverse private endpoint
    pub async fn click_pipe_reverse_private_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        reverse_private_endpoint_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickpipesReversePrivateEndpoints/{reverse_private_endpoint_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: List Alerts
    pub async fn click_stack_list_alerts(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackAlertResponse>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Create Alert
    pub async fn click_stack_create_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateAlertRequest,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Get Alert
    pub async fn click_stack_get_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts/{click_stack_alert_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Update Alert
    pub async fn click_stack_update_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
        body: &ClickStackUpdateAlertRequest,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts/{click_stack_alert_id}");
        let mut req = self.request(reqwest::Method::PUT, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Delete Alert
    pub async fn click_stack_delete_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts/{click_stack_alert_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: List Dashboards
    pub async fn click_stack_list_dashboards(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackDashboardResponse>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Create Dashboard
    pub async fn click_stack_create_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Get Dashboard
    pub async fn click_stack_get_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/{click_stack_dashboard_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Update Dashboard
    pub async fn click_stack_update_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
        body: &ClickStackUpdateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/{click_stack_dashboard_id}");
        let mut req = self.request(reqwest::Method::PUT, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: Delete Dashboard
    pub async fn click_stack_delete_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/{click_stack_dashboard_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: List Sources
    pub async fn click_stack_list_sources(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackSource>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/sources");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// ClickStack: List Webhooks
    pub async fn click_stack_list_webhooks(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackWebhook>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/webhooks");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update service password
    pub async fn instance_password_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicePasswordPatchRequest,
    ) -> Result<ApiResponse<ServicePasswordPatchResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/password");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create a private endpoint
    pub async fn instance_private_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicPrivateEndpointePostRequest,
    ) -> Result<ApiResponse<InstancePrivateEndpoint>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/privateEndpoint");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get private endpoint configuration
    pub async fn instance_private_endpoint_config_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<PrivateEndpointConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/privateEndpointConfig");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get service metrics
    pub async fn instance_prometheus_get(
        &self,
        organization_id: &str,
        service_id: &str,
        filtered_metrics: Option<&str>,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/prometheus");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = filtered_metrics {
            req = req.query(&[("filtered_metrics", v)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text),
            });
        }
        Ok(resp.text().await?)
    }

    /// Update service auto scaling settings
    pub async fn instance_replica_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceReplicaScalingPatchRequest,
    ) -> Result<ApiResponse<ServiceScalingPatchResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/replicaScaling");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update service auto scaling settings
    #[deprecated]
    #[allow(deprecated)]
    pub async fn instance_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceScalingPatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/scaling");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get the service query endpoint for a given instance
    pub async fn instance_query_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceQueryAPIEndpoint>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/serviceQueryEndpoint");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete the service query endpoint for a given instance
    pub async fn instance_query_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/serviceQueryEndpoint");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Upsert the service query endpoint for a given instance
    pub async fn instance_query_endpoint_upsert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &InstanceServiceQueryApiEndpointsPostRequest,
    ) -> Result<ApiResponse<ServiceQueryAPIEndpoint>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/serviceQueryEndpoint");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update service state
    pub async fn instance_state_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceStatePatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/state");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization usage costs
    pub async fn usage_cost_get(
        &self,
        organization_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[&str],
    ) -> Result<ApiResponse<UsageCost>, Error> {
        let path = format!("/v1/organizations/{organization_id}/usageCost");
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[("from_date", from_date)]);
        req = req.query(&[("to_date", to_date)]);
        for f in filters {
            req = req.query(&[("filter", f)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

}