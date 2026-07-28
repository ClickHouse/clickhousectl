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
    ApiKeyPostRequest, ApiKeyPostRequestState, ApiKeyPostResponse,
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

/// Requires a response field the provisioning flow cannot proceed without.
fn require_field<T>(value: Option<T>, field: &str) -> Result<T, Box<dyn std::error::Error>> {
    value.ok_or_else(|| format!("the API response is missing required field '{field}'").into())
}

/// The `key_id`/`key_secret` pair the query host authenticates with, taken
/// from the key-creation response. Both halves are required together: a key
/// id without its secret is as unusable as neither.
fn require_credential_pair(
    key_response: &ApiKeyPostResponse,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let key_id = require_field(key_response.key_id.clone(), "keyId")?;
    let key_secret = require_field(key_response.key_secret.clone(), "keySecret")?;
    Ok((key_id, key_secret))
}

/// Discard the API key created for a provisioning attempt that then failed,
/// so a later retry doesn't leave an orphaned key behind per attempt. Best
/// effort: the caller is already returning an error, and a key we couldn't
/// delete is no worse than the one we'd otherwise leave behind.
async fn discard_api_key(client: &CloudClient, org_id: &str, api_key_uuid: &str) {
    let _ = client.delete_api_key(org_id, api_key_uuid).await;
}

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
        #[cfg(feature = "deprecated-fields")]
        roles: None,
        state: ApiKeyPostRequestState::Enabled,
    };

    let key_response = client.create_api_key(org_id, &key_request).await?;
    // `key_id`/`key_secret` are the credential pair used for query auth.
    // The endpoint binding's `openApiKeys` array, by contrast, references
    // API keys by their resource UUID — the same value the management
    // endpoints (GET/DELETE /v1/.../keys/{keyId}) accept. Resolve the UUID
    // first: every failure past this point deletes the key it identifies, so
    // an absent `key.id` is the only one with no cleanup available — we
    // cannot name the key we just created.
    let api_key_uuid =
        require_field(key_response.key.as_ref().and_then(|key| key.id), "key.id")?.to_string();

    // Every response field is `Option<T>`, and an absent credential cannot be
    // substituted with a placeholder: fail loudly instead of persisting an
    // empty key pair that every later query would reject.
    let (key_id, key_secret) = match require_credential_pair(&key_response) {
        Ok(pair) => pair,
        Err(e) => {
            // The key exists but we can't authenticate with it, so it is
            // dead weight in the org: discard it before failing.
            discard_api_key(client, org_id, &api_key_uuid).await;
            return Err(e);
        }
    };

    let endpoint = match bind_query_endpoint(client, org_id, service_id, &api_key_uuid).await {
        Ok(endpoint) => endpoint,
        Err(e) => {
            // The key was created but never bound or persisted, so nothing
            // can use it.
            discard_api_key(client, org_id, &api_key_uuid).await;
            return Err(e);
        }
    };

    let endpoint_id = match require_field(endpoint.id, "id") {
        Ok(id) => id,
        Err(e) => {
            // The binding took effect but we can't persist it, so the key
            // is again unusable. Discarding it leaves a dangling UUID in the
            // endpoint's `openApiKeys`, which is harmless — a retry merges a
            // fresh key into the same endpoint (see `bind_query_endpoint`).
            discard_api_key(client, org_id, &api_key_uuid).await;
            return Err(e);
        }
    };

    let stored = ServiceQueryKey {
        key_id,
        key_secret,
        endpoint_id,
        service_name: service_name.to_string(),
        created_at: Utc::now(),
    };
    credentials::set_service_query_key(service_id, stored.clone())?;

    Ok(stored)
}

/// Bind `api_key_uuid` to the service's query endpoint, merging into any
/// existing endpoint configuration so we don't silently revoke other
/// bindings the user set up.
async fn bind_query_endpoint(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_uuid: &str,
) -> Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint, Box<dyn std::error::Error>> {
    let mut open_api_keys = match client
        .api()
        .instance_query_endpoint_get(org_id, service_id)
        .await
    {
        Ok(resp) => resp
            .result
            .and_then(|ep| ep.open_api_keys)
            .unwrap_or_default(),
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Vec::new(),
        Err(e) => return Err(client.convert_error(e).into()),
    };
    if !open_api_keys.iter().any(|k| k == api_key_uuid) {
        open_api_keys.push(api_key_uuid.to_string());
    }

    let endpoint_request = InstanceServiceQueryApiEndpointsPostRequest {
        roles: vec![QUERY_ENDPOINT_ROLE.to_string()],
        open_api_keys,
        allowed_origins: ALLOWED_ORIGINS.to_string(),
    };

    Ok(client
        .create_query_endpoint(org_id, service_id, &endpoint_request)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_response(key_id: Option<&str>, key_secret: Option<&str>) -> ApiKeyPostResponse {
        ApiKeyPostResponse {
            key: None,
            key_id: key_id.map(str::to_string),
            key_secret: key_secret.map(str::to_string),
        }
    }

    #[test]
    fn credential_pair_is_returned_when_both_halves_are_present() {
        let (key_id, key_secret) =
            require_credential_pair(&key_response(Some("k-1"), Some("s-1"))).unwrap();
        assert_eq!(key_id, "k-1");
        assert_eq!(key_secret, "s-1");
    }

    #[test]
    fn credential_pair_fails_naming_the_absent_key_id() {
        let err = require_credential_pair(&key_response(None, Some("s-1"))).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the API response is missing required field 'keyId'"
        );
    }

    #[test]
    fn credential_pair_fails_naming_the_absent_key_secret() {
        let err = require_credential_pair(&key_response(Some("k-1"), None)).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the API response is missing required field 'keySecret'"
        );
    }
}
