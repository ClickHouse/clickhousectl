//! Auto-provisioning of per-service Query API endpoints.
//!
//! Creates a dedicated API key and binds it to the service's query endpoint
//! with role `sql_console_admin`. The key's `key_id`/`key_secret` are
//! persisted in `.clickhouse/credentials.json` keyed by service id, so later
//! `cloud service query` invocations can authenticate without contacting the
//! control plane.

use crate::cloud::client::CloudClient;
use crate::cloud::credentials::{self, ServiceQueryKey};
use chrono::Utc;
use clickhouse_cloud_api::models::{
    ApiKeyPostRequest, ApiKeyPostRequestState,
    InstanceServiceQueryApiEndpointsPostRequest, IpAccessListEntry,
};

/// The role attached to the query endpoint binding. Grants the key read +
/// write SQL access through the query endpoint, scoped to this single
/// service. The binding (not the API key) is what enforces the scope, so the
/// key cannot reach other services in the org regardless of any future
/// org-level role assignments.
const QUERY_ENDPOINT_ROLE: &str = "sql_console_admin";

/// Default `allowedOrigins` for the query endpoint. The CLI is a non-browser
/// caller so CORS doesn't apply, but the API still requires a value.
const ALLOWED_ORIGINS: &str = "*";

/// Ensure a query endpoint is provisioned for `service_id` and return the
/// persisted key. If a key is already cached locally, returns it unchanged;
/// otherwise creates the API key, binds it to the query endpoint (merging
/// into any existing endpoint configuration) with read+write scope on this
/// service, and saves it to `.clickhouse/credentials.json`.
pub async fn ensure_service_query_setup(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    service_name: &str,
) -> Result<ServiceQueryKey, Box<dyn std::error::Error>> {
    if let Some(existing) = credentials::get_service_query_key(service_id) {
        return Ok(existing);
    }

    let key_request = ApiKeyPostRequest {
        name: format!("clickhousectl-query-{service_name}"),
        assigned_role_ids: vec![],
        expire_at: None,
        hash_data: None,
        ip_access_list: vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some(format!(
                "clickhousectl auto-provisioned key for service {service_name}"
            )),
        }],
        roles: None,
        state: ApiKeyPostRequestState::Enabled,
    };

    let key_response = client.create_api_key(org_id, &key_request).await?;
    let key_id = key_response.key_id.clone();
    let key_secret = key_response.key_secret.clone();
    // `key_id`/`key_secret` are the credential pair used for query auth.
    // The endpoint binding's `openApiKeys` array, by contrast, references
    // API keys by their resource UUID — the same value the management
    // endpoints (GET/DELETE /v1/.../keys/{keyId}) accept.
    let api_key_uuid = key_response.key.id.to_string();

    // Merge our new key UUID into any existing endpoint config so we don't
    // silently revoke other bindings the user set up.
    let mut open_api_keys = match client
        .api()
        .instance_query_endpoint_get(org_id, service_id)
        .await
    {
        Ok(resp) => resp.result.map(|ep| ep.open_api_keys).unwrap_or_default(),
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Vec::new(),
        Err(e) => return Err(client.convert_error(e).into()),
    };
    if !open_api_keys.contains(&api_key_uuid) {
        open_api_keys.push(api_key_uuid);
    }

    let endpoint_request = InstanceServiceQueryApiEndpointsPostRequest {
        roles: vec![QUERY_ENDPOINT_ROLE.to_string()],
        open_api_keys,
        allowed_origins: ALLOWED_ORIGINS.to_string(),
    };

    let endpoint = client
        .create_query_endpoint(org_id, service_id, &endpoint_request)
        .await?;

    let stored = ServiceQueryKey {
        key_id,
        key_secret,
        endpoint_id: endpoint.id,
        service_name: service_name.to_string(),
        created_at: Utc::now(),
    };
    credentials::set_service_query_key(service_id, stored.clone())?;

    Ok(stored)
}
