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

    // Helper to get the default organization
    pub async fn get_default_org_id(&self) -> Result<String> {
        let orgs = self.list_organizations().await?;
        orgs.first()
            .map(|o| o.id.clone())
            .ok_or_else(|| CloudError {
                message: "No organization found for this API key".into(),
            })
    }
}