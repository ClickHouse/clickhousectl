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
    client: Client,
    auth_mode: AuthMode,
    base_url: String,
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

        // Priority: CLI flags > OAuth tokens > file credentials > env vars
        // If explicit API key flags are provided, use Basic auth
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
            return Ok(Self {
                client,
                auth_mode: Self::basic_auth(&key, &secret),
                base_url,
            });
        }

        // Try OAuth tokens
        if let Some(tokens) = crate::cloud::auth::load_tokens()
            && crate::cloud::auth::is_token_valid(&tokens)
        {
            let base_url = url_override
                .map(crate::cloud::auth::normalize_api_url)
                .unwrap_or(tokens.api_url.clone());
            return Ok(Self {
                client,
                auth_mode: AuthMode::Bearer(format!("Bearer {}", tokens.access_token)),
                base_url,
            });
        }

        let base_url = url_override
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        // Try file credentials
        if let Some(creds) = crate::cloud::credentials::load_credentials() {
            return Ok(Self {
                client,
                auth_mode: Self::basic_auth(&creds.api_key, &creds.api_secret),
                base_url,
            });
        }

        // Try env vars
        let env_key = env::var("CLICKHOUSE_CLOUD_API_KEY").ok();
        let env_secret = env::var("CLICKHOUSE_CLOUD_API_SECRET").ok();
        if let (Some(key), Some(secret)) = (env_key, env_secret) {
            return Ok(Self {
                client,
                auth_mode: Self::basic_auth(&key, &secret),
                base_url,
            });
        }

        Err(CloudError {
            message: "No credentials found. Run `clickhousectl cloud auth login` (OAuth), `clickhousectl cloud auth keys` (API key/secret), set CLICKHOUSE_CLOUD_API_KEY + CLICKHOUSE_CLOUD_API_SECRET, or use --api-key/--api-secret".into(),
        })
    }

    fn basic_auth(key: &str, secret: &str) -> AuthMode {
        let credentials = format!("{}:{}", key, secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
        AuthMode::Basic(format!("Basic {}", encoded))
    }

    fn auth_header_value(&self) -> &str {
        match &self.auth_mode {
            AuthMode::Basic(v) | AuthMode::Bearer(v) => v,
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
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body)
                && let Some(err) = api_resp.error
            {
                return Err(CloudError {
                    message: err.message,
                });
            }
            return Err(CloudError {
                message: format!("API error ({}): {}", status, body),
            });
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
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body)
                && let Some(err) = api_resp.error
            {
                return Err(CloudError {
                    message: err.message,
                });
            }
            return Err(CloudError {
                message: format!("API error ({}): {}", status, body),
            });
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
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body)
                && let Some(err) = api_resp.error
            {
                return Err(CloudError {
                    message: err.message,
                });
            }
            return Err(CloudError {
                message: format!("API error ({}): {}", status, body),
            });
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

    fn org_prometheus_url(&self, org_id: &str, filtered_metrics: Option<bool>) -> String {
        let path = format!("/organizations/{}/prometheus", org_id);
        match filtered_metrics {
            Some(value) => format!("{}?filtered_metrics={}", self.url(&path), value),
            None => self.url(&path),
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

    fn org_usage_url(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[String],
    ) -> String {
        let path = format!("/organizations/{}/usageCost", org_id);
        let mut params = vec![
            format!("from_date={}", urlencoding::encode(from_date)),
            format!("to_date={}", urlencoding::encode(to_date)),
        ];
        params.extend(
            filters
                .iter()
                .map(|f| format!("filter={}", urlencoding::encode(f))),
        );
        format!("{}?{}", self.url(&path), params.join("&"))
    }

    fn activities_url(
        &self,
        org_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> String {
        let path = format!("/organizations/{}/activities", org_id);
        let mut params = Vec::new();
        if let Some(from) = from_date {
            params.push(format!("from_date={}", urlencoding::encode(from)));
        }
        if let Some(to) = to_date {
            params.push(format!("to_date={}", urlencoding::encode(to)));
        }
        if params.is_empty() {
            self.url(&path)
        } else {
            format!("{}?{}", self.url(&path), params.join("&"))
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

    // Organization endpoints
    pub async fn list_organizations(&self) -> Result<Vec<Organization>> {
        self.get("/organizations").await
    }

    pub async fn get_organization(&self, org_id: &str) -> Result<Organization> {
        self.get(&format!("/organizations/{}", org_id)).await
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

    // Backup endpoints
    pub async fn list_backups(&self, org_id: &str, service_id: &str) -> Result<Vec<Backup>> {
        self.get(&format!(
            "/organizations/{}/services/{}/backups",
            org_id, service_id
        ))
        .await
    }

    pub async fn get_backup(
        &self,
        org_id: &str,
        service_id: &str,
        backup_id: &str,
    ) -> Result<Backup> {
        self.get(&format!(
            "/organizations/{}/services/{}/backups/{}",
            org_id, service_id, backup_id
        ))
        .await
    }

    // ClickPipe endpoints
    pub async fn list_clickpipes(&self, org_id: &str, service_id: &str) -> Result<Vec<ClickPipe>> {
        self.get(&format!(
            "/organizations/{}/services/{}/clickpipes",
            org_id, service_id
        ))
        .await
    }

    pub async fn create_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        request: &CreateClickPipeRequest,
    ) -> Result<ClickPipe> {
        self.post(
            &format!(
                "/organizations/{}/services/{}/clickpipes",
                org_id, service_id
            ),
            request,
        )
        .await
    }

    pub async fn get_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> Result<ClickPipe> {
        self.get(&format!(
            "/organizations/{}/services/{}/clickpipes/{}",
            org_id, service_id, clickpipe_id
        ))
        .await
    }

    pub async fn delete_clickpipe(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
    ) -> Result<()> {
        self.delete(&format!(
            "/organizations/{}/services/{}/clickpipes/{}",
            org_id, service_id, clickpipe_id
        ))
        .await
    }

    pub async fn change_clickpipe_state(
        &self,
        org_id: &str,
        service_id: &str,
        clickpipe_id: &str,
        command: &str,
    ) -> Result<ClickPipe> {
        self.patch(
            &format!(
                "/organizations/{}/services/{}/clickpipes/{}/state",
                org_id, service_id, clickpipe_id
            ),
            &serde_json::json!({ "command": command }),
        )
        .await
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

    // Phase 3 - Org endpoints
    pub async fn update_organization(
        &self,
        org_id: &str,
        request: &UpdateOrgRequest,
    ) -> Result<Organization> {
        self.patch(&format!("/organizations/{}", org_id), request)
            .await
    }

    pub async fn get_org_prometheus(
        &self,
        org_id: &str,
        filtered_metrics: Option<bool>,
    ) -> Result<String> {
        self.request_text(
            self.client
                .get(self.org_prometheus_url(org_id, filtered_metrics)),
        )
        .await
    }

    pub async fn get_org_usage(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[String],
    ) -> Result<UsageCost> {
        self.request(
            self.client
                .get(self.org_usage_url(org_id, from_date, to_date, filters)),
        )
        .await
    }

    // Phase 4 - Member endpoints
    pub async fn list_members(&self, org_id: &str) -> Result<Vec<Member>> {
        self.get(&format!("/organizations/{}/members", org_id))
            .await
    }

    pub async fn get_member(&self, org_id: &str, user_id: &str) -> Result<Member> {
        self.get(&format!("/organizations/{}/members/{}", org_id, user_id))
            .await
    }

    pub async fn update_member(
        &self,
        org_id: &str,
        user_id: &str,
        request: &UpdateMemberRequest,
    ) -> Result<Member> {
        self.patch(
            &format!("/organizations/{}/members/{}", org_id, user_id),
            request,
        )
        .await
    }

    pub async fn delete_member(&self, org_id: &str, user_id: &str) -> Result<()> {
        self.delete(&format!("/organizations/{}/members/{}", org_id, user_id))
            .await
    }

    // Phase 4 - Invitation endpoints
    pub async fn list_invitations(&self, org_id: &str) -> Result<Vec<Invitation>> {
        self.get(&format!("/organizations/{}/invitations", org_id))
            .await
    }

    pub async fn create_invitation(
        &self,
        org_id: &str,
        request: &CreateInvitationRequest,
    ) -> Result<Invitation> {
        self.post(&format!("/organizations/{}/invitations", org_id), request)
            .await
    }

    pub async fn get_invitation(&self, org_id: &str, invitation_id: &str) -> Result<Invitation> {
        self.get(&format!(
            "/organizations/{}/invitations/{}",
            org_id, invitation_id
        ))
        .await
    }

    pub async fn delete_invitation(&self, org_id: &str, invitation_id: &str) -> Result<()> {
        self.delete(&format!(
            "/organizations/{}/invitations/{}",
            org_id, invitation_id
        ))
        .await
    }

    // Phase 5 - API Key endpoints
    pub async fn list_api_keys(&self, org_id: &str) -> Result<Vec<ApiKey>> {
        self.get(&format!("/organizations/{}/keys", org_id)).await
    }

    pub async fn create_api_key(
        &self,
        org_id: &str,
        request: &CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        self.post(&format!("/organizations/{}/keys", org_id), request)
            .await
    }

    pub async fn get_api_key(&self, org_id: &str, key_id: &str) -> Result<ApiKey> {
        self.get(&format!("/organizations/{}/keys/{}", org_id, key_id))
            .await
    }

    pub async fn update_api_key(
        &self,
        org_id: &str,
        key_id: &str,
        request: &UpdateApiKeyRequest,
    ) -> Result<ApiKey> {
        self.patch(
            &format!("/organizations/{}/keys/{}", org_id, key_id),
            request,
        )
        .await
    }

    pub async fn delete_api_key(&self, org_id: &str, key_id: &str) -> Result<()> {
        self.delete(&format!("/organizations/{}/keys/{}", org_id, key_id))
            .await
    }

    // Phase 6 - Activity endpoints
    pub async fn list_activities(
        &self,
        org_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<Vec<Activity>> {
        self.request(
            self.client
                .get(self.activities_url(org_id, from_date, to_date)),
        )
        .await
    }

    pub async fn get_activity(&self, org_id: &str, activity_id: &str) -> Result<Activity> {
        self.get(&format!(
            "/organizations/{}/activities/{}",
            org_id, activity_id
        ))
        .await
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
            1 => Ok(orgs[0].id.clone()),
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

    fn test_client() -> CloudClient {
        CloudClient {
            client: Client::builder().build().unwrap(),
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
    fn activities_url_includes_optional_date_filters() {
        let client = test_client();
        let url = client.activities_url("org-1", Some("2024-01-01"), Some("2024-01-31"));
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/activities?from_date=2024-01-01&to_date=2024-01-31"
        );
    }

    #[test]
    fn activities_url_omits_query_without_dates() {
        let client = test_client();
        let url = client.activities_url("org-1", None, None);
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/activities"
        );
    }

    #[test]
    fn org_usage_url_requires_dates_and_supports_filters() {
        let client = test_client();
        let url = client.org_usage_url(
            "org-1",
            "2024-01-01T00:00:00Z",
            "2024-01-31T23:59:59Z",
            &[
                "entityType=service".to_string(),
                "entityName=my svc".to_string(),
            ],
        );
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/usageCost?from_date=2024-01-01T00%3A00%3A00Z&to_date=2024-01-31T23%3A59%3A59Z&filter=entityType%3Dservice&filter=entityName%3Dmy%20svc"
        );
    }

    #[test]
    fn org_usage_url_includes_only_required_dates_without_filters() {
        let client = test_client();
        let url = client.org_usage_url("org-1", "2024-01-01", "2024-01-31", &[]);
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/usageCost?from_date=2024-01-01&to_date=2024-01-31"
        );
    }

    #[test]
    fn org_prometheus_url_supports_filtered_metrics_query() {
        let client = test_client();
        let url = client.org_prometheus_url("org-1", Some(true));
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/prometheus?filtered_metrics=true"
        );
    }

    #[test]
    fn org_prometheus_url_omits_filtered_metrics_when_not_set() {
        let client = test_client();
        let url = client.org_prometheus_url("org-1", None);
        assert_eq!(
            url,
            "https://api.clickhouse.cloud/v1/organizations/org-1/prometheus"
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
}
