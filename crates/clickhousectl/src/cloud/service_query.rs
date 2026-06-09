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

/// Polling cadence while waiting for a service to become ready.
const READY_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Upper bound on the readiness wait: 120 polls × 5s ≈ 10 minutes, well
/// above normal provisioning time. On timeout the caller falls back to its
/// "retry later" path rather than blocking forever.
const READY_POLL_MAX_ATTEMPTS: u32 = 120;

/// What a service state means for binding a query endpoint. The control
/// plane returns 500 "Internal error" on the endpoint upsert while the
/// service is still provisioning, so callers must hold off until the
/// service is ready (issue #242).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadyState {
    /// Provisioned — safe to bind the query endpoint.
    Ready,
    /// Still coming up — keep polling.
    Pending,
    /// Won't become ready by waiting (stopped/failed/terminating/unknown).
    Unavailable,
}

fn classify_ready_state(state: &str) -> ReadyState {
    match state {
        "running" | "idle" | "partially_running" => ReadyState::Ready,
        "provisioning" | "starting" | "awaking" => ReadyState::Pending,
        _ => ReadyState::Unavailable,
    }
}

/// Poll until `service_id` reaches a state where the query endpoint can be
/// bound, printing each state transition to stderr. Errors out (rather than
/// waiting) on states that won't resolve on their own, and after
/// [`READY_POLL_MAX_ATTEMPTS`] polls.
pub async fn wait_for_service_ready(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_state = String::new();
    for _ in 0..READY_POLL_MAX_ATTEMPTS {
        let svc = client.get_service(org_id, service_id).await?;
        let state = svc.state.to_string();
        if state != last_state {
            eprintln!("  state: {state}");
            last_state = state.clone();
        }
        match classify_ready_state(&state) {
            ReadyState::Ready => return Ok(()),
            ReadyState::Pending => tokio::time::sleep(READY_POLL_INTERVAL).await,
            ReadyState::Unavailable => {
                return Err(format!(
                    "service is in state '{state}' and will not become ready by waiting"
                )
                .into())
            }
        }
    }
    Err("timed out waiting for service to become ready".into())
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
    let key_id = key_response.key_id.clone();
    let key_secret = key_response.key_secret.clone();
    // `key_id`/`key_secret` are the credential pair used for query auth.
    // The endpoint binding's `openApiKeys` array, by contrast, references
    // API keys by their resource UUID — the same value the management
    // endpoints (GET/DELETE /v1/.../keys/{keyId}) accept.
    let api_key_uuid = key_response.key.id.to_string();

    let endpoint = match bind_query_endpoint(client, org_id, service_id, &api_key_uuid).await {
        Ok(endpoint) => endpoint,
        Err(e) => {
            // The key was created but never bound or persisted, so nothing
            // can use it. Delete it (best-effort) so a later retry doesn't
            // leave an orphaned key behind per attempt.
            let _ = client.delete_api_key(org_id, &api_key_uuid).await;
            return Err(e);
        }
    };

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
        Ok(resp) => resp.result.map(|ep| ep.open_api_keys).unwrap_or_default(),
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

    #[test]
    fn ready_states() {
        for state in ["running", "idle", "partially_running"] {
            assert_eq!(classify_ready_state(state), ReadyState::Ready, "{state}");
        }
    }

    #[test]
    fn pending_states() {
        for state in ["provisioning", "starting", "awaking"] {
            assert_eq!(classify_ready_state(state), ReadyState::Pending, "{state}");
        }
    }

    #[test]
    fn unavailable_states() {
        for state in [
            "stopping",
            "stopped",
            "terminating",
            "terminated",
            "softdeleting",
            "softdeleted",
            "degraded",
            "failed",
            "some-future-state",
        ] {
            assert_eq!(
                classify_ready_state(state),
                ReadyState::Unavailable,
                "{state}"
            );
        }
    }
}
