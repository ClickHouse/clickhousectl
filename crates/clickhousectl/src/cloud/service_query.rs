//! Auto-provisioning of per-service Query API endpoints.
//!
//! Creates a dedicated read-only API key and binds it to the service's query
//! endpoint with role `sql_console_read_only`. The key's `key_id`/`key_secret`
//! are persisted in `.clickhouse/credentials.json` keyed by service id, so
//! later `cloud service query` invocations can authenticate without contacting
//! the control plane.

use crate::cloud::client::CloudClient;
use crate::cloud::credentials::{self, ServiceQueryKey};
use chrono::Utc;
use clickhouse_cloud_api::models::{
    ApiKeyHashData, ApiKeyPostRequest, ApiKeyPostRequestState,
    InstanceServiceQueryApiEndpointsPostRequest, IpAccessListEntry,
};

/// The role attached to the query endpoint binding. Limits the key to
/// read-only SQL through the query endpoint, regardless of any future
/// org-level role assignments.
const READ_ONLY_ROLE: &str = "sql_console_read_only";

/// Default `allowedOrigins` for the query endpoint. The CLI is a non-browser
/// caller so CORS doesn't apply, but the API still requires a value.
const ALLOWED_ORIGINS: &str = "*";

/// Ensure a read-only query endpoint is provisioned for `service_id` and return
/// the persisted key. If a key is already cached locally, returns it
/// unchanged; otherwise creates the API key, binds it to the query endpoint
/// (merging into any existing endpoint configuration), and saves it to
/// `.clickhouse/credentials.json`.
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
        hash_data: ApiKeyHashData::default(),
        ip_access_list: vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some(format!(
                "clickhousectl auto-provisioned key for service {service_name}"
            )),
        }],
        // `roles` is deprecated but the API still enforces minLength=1.
        // `query_endpoints` is the legacy role for keys whose only job is
        // to authenticate against a Query API endpoint binding.
        roles: vec!["query_endpoints".to_string()],
        state: ApiKeyPostRequestState::Enabled,
    };

    let key_response = client.create_api_key(org_id, &key_request).await?;
    let key_id = key_response.key_id.clone();
    let key_secret = key_response.key_secret.clone();

    // Merge our new key_id into any existing endpoint config so we don't
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
    if !open_api_keys.contains(&key_id) {
        open_api_keys.push(key_id.clone());
    }

    let endpoint_request = InstanceServiceQueryApiEndpointsPostRequest {
        roles: vec![READ_ONLY_ROLE.to_string()],
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
