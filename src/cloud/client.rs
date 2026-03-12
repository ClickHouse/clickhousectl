use crate::cloud::types::*;
use base64::Engine;
use reqwest::Client;
use std::env;

const BASE_URL: &str = "https://api.clickhouse.cloud/v1";

pub fn user_agent() -> String {
    format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"))
}

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

pub struct CloudClient {
    client: Client,
    auth_header: String,
}

impl CloudClient {
    pub fn new(api_key: Option<&str>, api_secret: Option<&str>) -> Result<Self> {
        let file_creds = crate::cloud::credentials::load_credentials();

        let key = api_key
            .map(String::from)
            .or_else(|| file_creds.as_ref().map(|c| c.api_key.clone()))
            .or_else(|| env::var("CLICKHOUSE_CLOUD_API_KEY").ok())
            .ok_or_else(|| CloudError {
                message: "API key required. Run `clickhousectl cloud auth`, set CLICKHOUSE_CLOUD_API_KEY, or use --api-key".into(),
            })?;

        let secret = api_secret
            .map(String::from)
            .or_else(|| file_creds.as_ref().map(|c| c.api_secret.clone()))
            .or_else(|| env::var("CLICKHOUSE_CLOUD_API_SECRET").ok())
            .ok_or_else(|| CloudError {
                message: "API secret required. Run `clickhousectl cloud auth`, set CLICKHOUSE_CLOUD_API_SECRET, or use --api-secret"
                    .into(),
            })?;

        let credentials = format!("{}:{}", key, secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
        let auth_header = format!("Basic {}", encoded);

        let client = Client::builder()
            .user_agent(user_agent())
            .build()
            .map_err(|e| CloudError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            client,
            auth_header,
        })
    }


    /// Send a request and parse the JSON response body.
    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T> {
        let response = req
            .header("Authorization", &self.auth_header)
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
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body) {
                if let Some(err) = api_resp.error {
                    return Err(CloudError {
                        message: err.message,
                    });
                }
            }
            return Err(CloudError {
                message: format!("API error ({}): {}", status, body),
            });
        }

        let api_response: ApiResponse<T> =
            serde_json::from_str(&body).map_err(|e| CloudError {
                message: format!("Failed to parse response: {} - Body: {}", e, body),
            })?;

        api_response.result.ok_or_else(|| CloudError {
            message: "Empty response from API".into(),
        })
    }

    /// Send a request expecting no response body.
    async fn request_no_body(&self, req: reqwest::RequestBuilder) -> Result<()> {
        let response = req
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .map_err(|e| CloudError {
                message: format!("Request failed: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body) {
                if let Some(err) = api_resp.error {
                    return Err(CloudError {
                        message: err.message,
                    });
                }
            }
            return Err(CloudError {
                message: format!("API error ({}): {}", status, body),
            });
        }

        Ok(())
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", BASE_URL, path)
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(self.client.get(&self.url(path))).await
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(self.client.post(&self.url(path)).json(body))
            .await
    }

    async fn patch<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(self.client.patch(&self.url(path)).json(body))
            .await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.request_no_body(self.client.delete(&self.url(path)))
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
        let path = format!("/organizations/{}/services", org_id);
        let query: Vec<String> = filters
            .iter()
            .map(|f| format!("filter={}", urlencoding::encode(f)))
            .collect();
        let full_url = if query.is_empty() {
            self.url(&path)
        } else {
            format!("{}?{}", self.url(&path), query.join("&"))
        };
        self.request(self.client.get(full_url)).await
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

    pub async fn delete_service(&self, org_id: &str, service_id: &str) -> Result<()> {
        self.delete(&format!(
            "/organizations/{}/services/{}",
            org_id, service_id
        ))
        .await
    }

    pub async fn change_service_state(
        &self,
        org_id: &str,
        service_id: &str,
        command: &str,
    ) -> Result<Service> {
        let request = StateChangeRequest {
            command: command.to_string(),
        };
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
    ) -> Result<Service> {
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
    ) -> Result<PasswordResetResponse> {
        // The API expects an empty JSON object
        self.patch(
            &format!(
                "/organizations/{}/services/{}/password",
                org_id, service_id
            ),
            &serde_json::json!({}),
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
    ) -> Result<PrivateEndpoint> {
        self.post(
            &format!(
                "/organizations/{}/services/{}/privateEndpoint",
                org_id, service_id
            ),
            request,
        )
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

    pub async fn get_org_prometheus(&self, org_id: &str) -> Result<OrgPrometheus> {
        self.get(&format!("/organizations/{}/prometheus", org_id))
            .await
    }

    pub async fn get_org_usage(&self, org_id: &str) -> Result<UsageCost> {
        self.get(&format!("/organizations/{}/usageCost", org_id))
            .await
    }

    // Phase 4 - Member endpoints
    pub async fn list_members(&self, org_id: &str) -> Result<Vec<Member>> {
        self.get(&format!("/organizations/{}/members", org_id))
            .await
    }

    pub async fn get_member(&self, org_id: &str, user_id: &str) -> Result<Member> {
        self.get(&format!(
            "/organizations/{}/members/{}",
            org_id, user_id
        ))
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
        self.delete(&format!(
            "/organizations/{}/members/{}",
            org_id, user_id
        ))
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
        self.post(
            &format!("/organizations/{}/invitations", org_id),
            request,
        )
        .await
    }

    pub async fn get_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> Result<Invitation> {
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
        self.get(&format!(
            "/organizations/{}/keys/{}",
            org_id, key_id
        ))
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
        self.delete(&format!(
            "/organizations/{}/keys/{}",
            org_id, key_id
        ))
        .await
    }

    // Phase 6 - Activity endpoints
    pub async fn list_activities(
        &self,
        org_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<Vec<Activity>> {
        let path = format!("/organizations/{}/activities", org_id);
        let mut params = Vec::new();
        if let Some(from) = from_date {
            params.push(format!("from_date={}", urlencoding::encode(from)));
        }
        if let Some(to) = to_date {
            params.push(format!("to_date={}", urlencoding::encode(to)));
        }
        let full_url = if params.is_empty() {
            self.url(&path)
        } else {
            format!("{}?{}", self.url(&path), params.join("&"))
        };
        self.request(self.client.get(full_url)).await
    }

    pub async fn get_activity(&self, org_id: &str, activity_id: &str) -> Result<Activity> {
        self.get(&format!(
            "/organizations/{}/activities/{}",
            org_id, activity_id
        ))
        .await
    }

    // Phase 6 - BYOC endpoints
    pub async fn create_byoc(
        &self,
        org_id: &str,
        request: &CreateByocRequest,
    ) -> Result<ByocInfrastructure> {
        self.post(
            &format!("/organizations/{}/byocInfrastructure", org_id),
            request,
        )
        .await
    }

    pub async fn update_byoc(
        &self,
        org_id: &str,
        byoc_id: &str,
        request: &UpdateByocRequest,
    ) -> Result<ByocInfrastructure> {
        self.patch(
            &format!(
                "/organizations/{}/byocInfrastructure/{}",
                org_id, byoc_id
            ),
            request,
        )
        .await
    }

    pub async fn delete_byoc(&self, org_id: &str, byoc_id: &str) -> Result<()> {
        self.delete(&format!(
            "/organizations/{}/byocInfrastructure/{}",
            org_id, byoc_id
        ))
        .await
    }

    // Phase 6 - Backup Bucket endpoints
    pub async fn list_backup_buckets(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<Vec<BackupBucket>> {
        self.get(&format!(
            "/organizations/{}/services/{}/backupBucket",
            org_id, service_id
        ))
        .await
    }

    pub async fn create_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
        request: &CreateBackupBucketRequest,
    ) -> Result<BackupBucket> {
        self.post(
            &format!(
                "/organizations/{}/services/{}/backupBucket",
                org_id, service_id
            ),
            request,
        )
        .await
    }

    pub async fn update_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
        bucket_id: &str,
        request: &UpdateBackupBucketRequest,
    ) -> Result<BackupBucket> {
        self.patch(
            &format!(
                "/organizations/{}/services/{}/backupBucket/{}",
                org_id, service_id, bucket_id
            ),
            request,
        )
        .await
    }

    pub async fn delete_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
        bucket_id: &str,
    ) -> Result<()> {
        self.delete(&format!(
            "/organizations/{}/services/{}/backupBucket/{}",
            org_id, service_id, bucket_id
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

    // Phase 6 - Service Prometheus endpoints
    pub async fn get_service_prometheus(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<PrometheusConfig> {
        self.get(&format!(
            "/organizations/{}/services/{}/prometheus",
            org_id, service_id
        ))
        .await
    }

    pub async fn setup_service_prometheus(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> Result<PrometheusConfig> {
        self.post(
            &format!(
                "/organizations/{}/services/{}/prometheus",
                org_id, service_id
            ),
            &SetupPrometheusRequest {},
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