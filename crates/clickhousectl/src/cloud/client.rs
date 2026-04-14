use crate::cloud::types::*;
use base64::Engine;
use reqwest::Client;
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
    Basic(String),
    Bearer(String),
}

pub struct CloudClient {
    /// Library client for migrated commands.
    lib_client: clickhouse_cloud_api::Client,
    /// Raw reqwest client for unmigrated commands (kept during transition).
    client: Client,
    auth_mode: AuthMode,
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
        let client = Client::builder()
            .user_agent(crate::user_agent::user_agent())
            .build()
            .map_err(|e| CloudError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        // Priority: CLI flags > file credentials > env vars > OAuth tokens
        // API keys are project-scoped (read/write); OAuth is user-scoped (read-only).
        if api_key.is_some() || api_secret.is_some() {
            let key = api_key.map(String::from).ok_or_else(|| CloudError {
                message: "API key required when --api-key or --api-secret is set".into(),
            })?;
            let secret = api_secret.map(String::from).ok_or_else(|| CloudError {
                message: "API secret required when --api-key or --api-secret is set".into(),
            })?;
            let base_url = url_override
                .map(crate::cloud::auth::normalize_api_url)
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
            let lib_client = clickhouse_cloud_api::Client::with_http_client(
                client.clone(),
                lib_base_url(&base_url),
                &key,
                &secret,
            );
            return Ok(Self {
                lib_client,
                client,
                auth_mode: Self::basic_auth(&key, &secret),
                base_url,
            });
        }

        let base_url = url_override
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        // Try file credentials
        if let Some(creds) = crate::cloud::credentials::load_credentials() {
            let lib_client = clickhouse_cloud_api::Client::with_http_client(
                client.clone(),
                lib_base_url(&base_url),
                &creds.api_key,
                &creds.api_secret,
            );
            return Ok(Self {
                lib_client,
                client,
                auth_mode: Self::basic_auth(&creds.api_key, &creds.api_secret),
                base_url,
            });
        }

        // Try env vars
        let env_key = env::var("CLICKHOUSE_CLOUD_API_KEY").ok();
        let env_secret = env::var("CLICKHOUSE_CLOUD_API_SECRET").ok();
        if let (Some(key), Some(secret)) = (env_key, env_secret) {
            let lib_client = clickhouse_cloud_api::Client::with_http_client(
                client.clone(),
                lib_base_url(&base_url),
                &key,
                &secret,
            );
            return Ok(Self {
                lib_client,
                client,
                auth_mode: Self::basic_auth(&key, &secret),
                base_url,
            });
        }

        // Fall back to OAuth tokens (read-only)
        if let Some(tokens) = crate::cloud::auth::load_tokens()
            && crate::cloud::auth::is_token_valid(&tokens)
        {
            let base_url = url_override
                .map(crate::cloud::auth::normalize_api_url)
                .unwrap_or(tokens.api_url.clone());
            let lib_client = clickhouse_cloud_api::Client::with_http_client_bearer(
                client.clone(),
                lib_base_url(&base_url),
                &tokens.access_token,
            );
            return Ok(Self {
                lib_client,
                client,
                auth_mode: AuthMode::Bearer(format!("Bearer {}", tokens.access_token)),
                base_url,
            });
        }

        Err(CloudError {
            message: "No credentials found. Run `clickhousectl cloud auth login` (OAuth, read-only), `clickhousectl cloud auth login --api-key KEY --api-secret SECRET` (read/write), set CLICKHOUSE_CLOUD_API_KEY + CLICKHOUSE_CLOUD_API_SECRET, or use --api-key/--api-secret.\n\nLearn how to create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl".into(),
        })
    }

    fn basic_auth(key: &str, secret: &str) -> AuthMode {
        let credentials = format!("{}:{}", key, secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
        AuthMode::Basic(format!("Basic {}", encoded))
    }

    /// Returns true if the client is using OAuth Bearer token authentication.
    /// Bearer auth is read-only and cannot perform write operations.
    pub fn is_bearer_auth(&self) -> bool {
        matches!(self.auth_mode, AuthMode::Bearer(_))
    }

    /// Access the library client for migrated commands.
    pub fn api(&self) -> &clickhouse_cloud_api::Client {
        &self.lib_client
    }

    /// Unwrap an `ApiResponse<T>` into `T`, returning an error if the result is empty.
    pub fn unwrap_response<T>(
        response: clickhouse_cloud_api::models::ApiResponse<T>,
    ) -> Result<T> {
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

    fn auth_header_value(&self) -> &str {
        match &self.auth_mode {
            AuthMode::Basic(v) | AuthMode::Bearer(v) => v,
        }
    }

    /// If using OAuth and the response is 403, append a hint about API key auth.
    fn maybe_append_oauth_hint(&self, message: &mut String, status: reqwest::StatusCode) {
        if status == reqwest::StatusCode::FORBIDDEN && self.is_bearer_auth() {
            message.push_str(
                "\n\nHint: You are authenticated via OAuth, which provides read-only access. \
                 Use API key authentication for write operations:\n  \
                 clickhousectl cloud auth login --api-key YOUR_KEY --api-secret YOUR_SECRET\n\n\
                 Learn how to create API keys:\n  \
                 https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl",
            );
        }
    }

    /// Send a request and parse the JSON response body.
    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T> {
        let response = req
            .header("Authorization", self.auth_header_value())
            .send()
            .await
            .map_err(|e| CloudError {
                message: format!("Request failed: {}", e),
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| CloudError {
            message: format!("Failed to read response: {}", e),
        })?;

        if !status.is_success() {
            let mut message = if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body)
                && let Some(err) = api_resp.error
            {
                err.message
            } else {
                format!("API error ({}): {}", status, body)
            };
            self.maybe_append_oauth_hint(&mut message, status);
            return Err(CloudError { message });
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&body).map_err(|e| CloudError {
            message: format!("Failed to parse response: {} - Body: {}", e, body),
        })?;

        api_response.result.ok_or_else(|| CloudError {
            message: "Empty response from API".into(),
        })
    }

    /// Send a request expecting no response body.
    async fn request_no_body(&self, req: reqwest::RequestBuilder) -> Result<()> {
        let response = req
            .header("Authorization", self.auth_header_value())
            .send()
            .await
            .map_err(|e| CloudError {
                message: format!("Request failed: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let mut message = if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body)
                && let Some(err) = api_resp.error
            {
                err.message
            } else {
                format!("API error ({}): {}", status, body)
            };
            self.maybe_append_oauth_hint(&mut message, status);
            return Err(CloudError { message });
        }

        Ok(())
    }

    /// Send a request and return a plaintext response body.
    async fn request_text(&self, req: reqwest::RequestBuilder) -> Result<String> {
        let response = req
            .header("Authorization", self.auth_header_value())
            .send()
            .await
            .map_err(|e| CloudError {
                message: format!("Request failed: {}", e),
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| CloudError {
            message: format!("Failed to read response: {}", e),
        })?;

        if !status.is_success() {
            let mut message = if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body)
                && let Some(err) = api_resp.error
            {
                err.message
            } else {
                format!("API error ({}): {}", status, body)
            };
            self.maybe_append_oauth_hint(&mut message, status);
            return Err(CloudError { message });
        }

        Ok(body)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn list_services_url(&self, org_id: &str, filters: &[String]) -> String {
        let path = format!("/organizations/{}/services", org_id);
        let query: Vec<String> = filters
            .iter()
            .map(|f| format!("filter={}", urlencoding::encode(f)))
            .collect();
        if query.is_empty() {
            self.url(&path)
        } else {
            format!("{}?{}", self.url(&path), query.join("&"))
        }
    }

    fn service_prometheus_url(
        &self,
        org_id: &str,
        service_id: &str,
        filtered_metrics: Option<bool>,
    ) -> String {
        let path = format!(
            "/organizations/{}/services/{}/prometheus",
            org_id, service_id
        );
        match filtered_metrics {
            Some(value) => format!("{}?filtered_metrics={}", self.url(&path), value),
            None => self.url(&path),
        }
    }


    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(self.client.get(self.url(path))).await
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(self.client.post(self.url(path)).json(body))
            .await
    }

    async fn patch<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(self.client.patch(self.url(path)).json(body))
            .await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.request_no_body(self.client.delete(self.url(path)))
            .await
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

    // Service endpoints
    pub async fn list_services(&self, org_id: &str) -> Result<Vec<Service>> {
        self.get(&format!("/organizations/{}/services", org_id))
            .await
    }

    pub async fn list_services_filtered(
        &self,
        org_id: &str,
        filters: &[String],
    ) -> Result<Vec<Service>> {
        self.request(self.client.get(self.list_services_url(org_id, filters)))
            .await
    }

    pub async fn get_service(&self, org_id: &str, service_id: &str) -> Result<Service> {
        self.get(&format!(
            "/organizations/{}/services/{}",
            org_id, service_id
        ))
        .await
    }

    pub async fn create_service(
        &self,
        org_id: &str,
        request: &CreateServiceRequest,
    ) -> Result<CreateServiceResponse> {
        self.post(&format!("/organizations/{}/services", org_id), request)
            .await
    }

    pub async fn delete_service(&self, org_id: &str, service_id: &str) -> Result<DeleteResponse> {
        let body = self
            .request_text(self.client.delete(self.url(&format!(
                "/organizations/{}/services/{}",
                org_id, service_id
            ))))
            .await?;

        serde_json::from_str::<DeleteResponse>(&body).map_err(|e| CloudError {
            message: format!("Failed to parse delete response: {} - Body: {}", e, body),
        })
    }

    pub async fn change_service_state(
        &self,
        org_id: &str,
        service_id: &str,
        command: ServiceStateCommand,
    ) -> Result<Service> {
        let request = StateChangeRequest { command };
        self.patch(
            &format!("/organizations/{}/services/{}/state", org_id, service_id),
            &request,
        )
        .await
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
        request: &UpdateServiceRequest,
    ) -> Result<Service> {
        self.patch(
            &format!("/organizations/{}/services/{}", org_id, service_id),
            request,
        )
        .await
    }

    // Replica scaling
    pub async fn update_replica_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ReplicaScalingRequest,
    ) -> Result<ServiceScalingPatchResponse> {
        self.patch(
            &format!(
                "/organizations/{}/services/{}/replicaScaling",
                org_id, service_id
            ),
            request,
        )
        .await
    }

    // Reset password
    pub async fn reset_password(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicePasswordPatchRequest,
    ) -> Result<ServicePasswordPatchResponse> {
        self.patch(
            &format!("/organizations/{}/services/{}/password", org_id, service_id),
            request,
        )
        .await
    }

    // Query endpoint
    pub async fn get_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<ServiceQueryEndpoint> {
        self.get(&format!(
            "/organizations/{}/services/{}/serviceQueryEndpoint",
            org_id, service_id
        ))
        .await
    }

    pub async fn create_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &CreateQueryEndpointRequest,
    ) -> Result<ServiceQueryEndpoint> {
        self.post(
            &format!(
                "/organizations/{}/services/{}/serviceQueryEndpoint",
                org_id, service_id
            ),
            request,
        )
        .await
    }

    pub async fn delete_query_endpoint(&self, org_id: &str, service_id: &str) -> Result<()> {
        self.delete(&format!(
            "/organizations/{}/services/{}/serviceQueryEndpoint",
            org_id, service_id
        ))
        .await
    }

    // Private endpoint
    pub async fn create_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &CreatePrivateEndpointRequest,
    ) -> Result<InstancePrivateEndpoint> {
        self.post(
            &format!(
                "/organizations/{}/services/{}/privateEndpoint",
                org_id, service_id
            ),
            request,
        )
        .await
    }

    // Private endpoint config
    pub async fn get_service_private_endpoint_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<PrivateEndpointConfig> {
        self.get(&format!(
            "/organizations/{}/services/{}/privateEndpointConfig",
            org_id, service_id
        ))
        .await
    }

    pub async fn get_service_prometheus(
        &self,
        org_id: &str,
        service_id: &str,
        filtered_metrics: Option<bool>,
    ) -> Result<String> {
        self.request_text(self.client.get(self.service_prometheus_url(
            org_id,
            service_id,
            filtered_metrics,
        )))
        .await
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

    pub async fn delete_member(&self, org_id: &str, user_id: &str) -> Result<()> {
        self.api()
            .member_delete(org_id, user_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(())
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

    pub async fn delete_invitation(&self, org_id: &str, invitation_id: &str) -> Result<()> {
        self.api()
            .invitation_delete(org_id, invitation_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(())
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

    pub async fn delete_api_key(&self, org_id: &str, key_id: &str) -> Result<()> {
        self.api()
            .openapi_key_delete(org_id, key_id)
            .await
            .map_err(|e| self.convert_error(e))?;
        Ok(())
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

    // Phase 6 - Backup Config endpoints
    pub async fn get_backup_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<BackupConfiguration> {
        self.get(&format!(
            "/organizations/{}/services/{}/backupConfiguration",
            org_id, service_id
        ))
        .await
    }

    pub async fn update_backup_config(
        &self,
        org_id: &str,
        service_id: &str,
        request: &UpdateBackupConfigRequest,
    ) -> Result<BackupConfiguration> {
        self.patch(
            &format!(
                "/organizations/{}/services/{}/backupConfiguration",
                org_id, service_id
            ),
            request,
        )
        .await
    }

    // Helper to get the default organization
    pub async fn get_default_org_id(&self) -> Result<String> {
        let orgs = self.list_organizations().await?;
        match orgs.len() {
            0 => Err(CloudError {
                message: "No organization found for this API key".into(),
            }),
            1 => {
                let id = orgs[0].id.as_ref().ok_or_else(|| CloudError {
                    message: "Organization missing ID".into(),
                })?;
                Ok(id.to_string())
            }
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
        let http = Client::builder().build().unwrap();
        let lib_client = clickhouse_cloud_api::Client::with_http_client(
            http.clone(),
            DEFAULT_LIB_BASE_URL,
            "test_key",
            "test_secret",
        );
        CloudClient {
            lib_client,
            client: http,
            auth_mode: AuthMode::Basic("Basic test".to_string()),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    #[test]
    fn list_services_url_includes_repeated_filters() {
        let client = test_client();
        let url = client.list_services_url(
            "org-1",
            &["tag:env=prod".to_string(), "state=running".to_string()],
        );
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/services?filter=tag%3Aenv%3Dprod&filter=state%3Drunning"
        );
    }

    #[test]
    fn list_services_url_omits_query_without_filters() {
        let client = test_client();
        let url = client.list_services_url("org-1", &[]);
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/services"
        );
    }

    #[test]
    fn service_prometheus_url_supports_filtered_metrics_query() {
        let client = test_client();
        let url = client.service_prometheus_url("org-1", "svc-1", Some(false));
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/services/svc-1/prometheus?filtered_metrics=false"
        );
    }

    #[test]
    fn service_prometheus_url_omits_filtered_metrics_when_not_set() {
        let client = test_client();
        let url = client.service_prometheus_url("org-1", "svc-1", None);
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/services/svc-1/prometheus"
        );
    }

    #[test]
    fn is_bearer_auth_returns_true_for_bearer() {
        let http = Client::builder().build().unwrap();
        let lib_client = clickhouse_cloud_api::Client::with_http_client_bearer(
            http.clone(),
            DEFAULT_LIB_BASE_URL,
            "test_token",
        );
        let client = CloudClient {
            lib_client,
            client: http,
            auth_mode: AuthMode::Bearer("Bearer test".to_string()),
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
        let http = Client::builder().build().unwrap();
        let lib_client = clickhouse_cloud_api::Client::with_http_client_bearer(
            http.clone(),
            DEFAULT_LIB_BASE_URL,
            "test_token",
        );
        let client = CloudClient {
            lib_client,
            client: http,
            auth_mode: AuthMode::Bearer("Bearer test".to_string()),
            base_url: DEFAULT_BASE_URL.to_string(),
        };
        let err = client.convert_error(clickhouse_cloud_api::Error::Api {
            status: 403,
            message: "Forbidden".into(),
        });
        assert!(err.message.contains("Hint: You are authenticated via OAuth"));
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
}
