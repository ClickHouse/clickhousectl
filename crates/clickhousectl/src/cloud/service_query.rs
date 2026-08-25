//! Auto-provisioning of per-service Query API endpoints.
//!
//! Creates a dedicated API key and binds it to the service's query endpoint
//! with role `sql_console_admin`. The key's `key_id`/`key_secret` are
//! persisted in `.clickhouse/credentials.json` keyed by service id, so later
//! `cloud service query` invocations can authenticate without contacting the
//! control plane.

use crate::cloud::api_keys::discard_api_key;
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials::{self, ServiceQueryKey};
use chrono::{DateTime, Utc};
use clickhouse_cloud_api::models::{
    ApiKeyPostRequest, ApiKeyPostRequestState, ApiKeyPostResponse,
    InstanceServiceQueryApiEndpointsPostRequest, IpAccessListEntry, QueryEndpointRole,
    ServiceQueryAPIEndpoint,
};

/// Default `allowedOrigins` for the query endpoint. The CLI is a non-browser
/// caller so CORS doesn't apply, but the API still requires a value.
const ALLOWED_ORIGINS: &str = "*";

enum EndpointCompensation {
    Delete,
    Restore(InstanceServiceQueryApiEndpointsPostRequest),
}

struct BoundQueryEndpoint {
    endpoint: ServiceQueryAPIEndpoint,
    compensation: EndpointCompensation,
}

/// Requires a response field the provisioning flow cannot proceed without.
fn require_field<T>(value: Option<T>, field: &str) -> CloudResult<T> {
    value.ok_or_else(|| {
        CloudError::new(format!(
            "the API response is missing required field '{field}'"
        ))
    })
}

/// The `key_id`/`key_secret` pair the query host authenticates with, taken
/// from the key-creation response. Both halves are required together: a key
/// id without its secret is as unusable as neither.
fn require_credential_pair(key_response: &ApiKeyPostResponse) -> CloudResult<(String, String)> {
    let key_id = require_field(key_response.key_id.clone(), "keyId")?;
    let key_secret = require_field(key_response.key_secret.clone(), "keySecret")?;
    Ok((key_id, key_secret))
}

fn build_service_query_key(
    organization_id: &str,
    api_key_id: String,
    key_id: String,
    key_secret: String,
    endpoint_id: Option<String>,
    service_name: &str,
    created_at: DateTime<Utc>,
) -> ServiceQueryKey {
    ServiceQueryKey {
        organization_id: Some(organization_id.to_string()),
        api_key_id: Some(api_key_id),
        key_id,
        key_secret,
        endpoint_id,
        service_name: service_name.to_string(),
        created_at,
    }
}

struct ProvisionedQueryKey {
    api_key_id: String,
    key_id: String,
    key_secret: String,
}

fn build_query_key_request(service_name: &str) -> ApiKeyPostRequest {
    ApiKeyPostRequest {
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
    }
}

async fn create_query_key(
    client: &CloudClient,
    org_id: &str,
    service_name: &str,
) -> CloudResult<ProvisionedQueryKey> {
    let key_response = client
        .create_api_key(org_id, &build_query_key_request(service_name))
        .await?;
    // `key_id`/`key_secret` authenticate to the query host. Endpoint and
    // management APIs identify the same key by the resource UUID in `key.id`.
    let api_key_id =
        require_field(key_response.key.as_ref().and_then(|key| key.id), "key.id")?.to_string();
    let (key_id, key_secret) = match require_credential_pair(&key_response) {
        Ok(pair) => pair,
        Err(error) => {
            discard_api_key(client, org_id, &api_key_id).await;
            return Err(error);
        }
    };

    Ok(ProvisionedQueryKey {
        api_key_id,
        key_id,
        key_secret,
    })
}

/// Ensure a query endpoint is provisioned for `service_id` and return the
/// persisted key. Provisioning is serialized across processes in this project;
/// after taking the lock, credentials are re-read so waiters reuse the winner's
/// key. The winner creates the API key, binds it to the query endpoint (merging
/// into any existing endpoint configuration) with read+write scope on this
/// service, then merges the key into the latest credentials under the shared
/// credentials mutation lock.
pub async fn ensure_service_query_setup(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    service_name: &str,
) -> CloudResult<ServiceQueryKey> {
    let _lock = credentials::lock_service_query_provisioning()?;
    if let Some(existing) = credentials::try_get_service_query_key(service_id)? {
        return Ok(existing);
    }

    let key = create_query_key(client, org_id, service_name).await?;

    let binding = match bind_query_endpoint(client, org_id, service_id, &key.api_key_id).await {
        Ok(binding) => binding,
        Err(e) => {
            // The key was created but never bound or persisted, so nothing
            // can use it.
            discard_api_key(client, org_id, &key.api_key_id).await;
            return Err(e);
        }
    };

    // The upsert succeeded, so the key is bound and fully usable. The echoed
    // `id` is diagnostic only, never an auth input: persist the record
    // without it rather than deleting a working credential and leaving a
    // dangling UUID in the endpoint's `openApiKeys`.
    let stored = build_service_query_key(
        org_id,
        key.api_key_id.clone(),
        key.key_id,
        key.key_secret,
        binding.endpoint.id,
        service_name,
        Utc::now(),
    );
    if let Err(persistence_error) = credentials::set_service_query_key(service_id, stored.clone()) {
        if let Err(compensation_error) = compensate_endpoint_binding(
            client,
            org_id,
            service_id,
            &key.api_key_id,
            &binding.compensation,
        )
        .await
        {
            return Err(CloudError::new(format!(
                "local credential persistence failed: {persistence_error}; query endpoint cleanup \
                 failed: {compensation_error}. API key {} remains bound and was \
                 retained for recovery",
                key.api_key_id
            )));
        }

        if let Err(cleanup_error) = client.delete_api_key(org_id, &key.api_key_id).await {
            return Err(CloudError::new(format!(
                "local credential persistence failed: {persistence_error}; the query endpoint \
                 binding was restored, but deleting API key {} failed: {cleanup_error}",
                key.api_key_id
            )));
        }
        return Err(persistence_error);
    }

    Ok(stored)
}

struct RepairEndpointConfiguration {
    id: Option<String>,
    roles: Vec<QueryEndpointRole>,
    open_api_keys: Vec<String>,
    allowed_origins: String,
}

impl RepairEndpointConfiguration {
    fn request(&self, open_api_keys: Vec<String>) -> InstanceServiceQueryApiEndpointsPostRequest {
        InstanceServiceQueryApiEndpointsPostRequest {
            roles: self.roles.clone(),
            open_api_keys,
            allowed_origins: self.allowed_origins.clone(),
        }
    }
}

fn repair_endpoint_configuration(
    endpoint: clickhouse_cloud_api::models::ServiceQueryAPIEndpoint,
    stored_endpoint_id: Option<&str>,
    api_key_id: &str,
) -> CloudResult<RepairEndpointConfiguration> {
    if let Some(stored_endpoint_id) = stored_endpoint_id {
        let current_endpoint_id = endpoint.id.as_deref().ok_or_else(|| {
            CloudError::new(
                "the query endpoint response omitted its id; refusing to repair a stored endpoint binding without confirming ownership",
            )
        })?;
        if current_endpoint_id != stored_endpoint_id {
            return Err(CloudError::new(format!(
                "the stored query key belongs to endpoint {stored_endpoint_id}, but the service now reports endpoint {current_endpoint_id}; refusing to modify the replacement endpoint"
            )));
        }
    }

    let open_api_keys = endpoint.open_api_keys.ok_or_else(|| {
        CloudError::new(
            "the query endpoint response omitted 'openApiKeys'; refusing to replace a binding when the other authorized keys are unknown",
        )
    })?;
    if !open_api_keys.iter().any(|key| key == api_key_id) {
        return Err(CloudError::new(format!(
            "the query endpoint no longer contains the stored API key binding {api_key_id}; refusing to add or remove credentials without an exact binding to replace"
        )));
    }

    Ok(RepairEndpointConfiguration {
        id: endpoint.id,
        roles: endpoint.roles.ok_or_else(|| {
            CloudError::new(
                "the query endpoint response omitted 'roles'; refusing to replace a binding when the endpoint permissions are unknown",
            )
        })?,
        open_api_keys,
        allowed_origins: endpoint.allowed_origins.ok_or_else(|| {
            CloudError::new(
                "the query endpoint response omitted 'allowedOrigins'; refusing to replace a binding when the endpoint configuration is unknown",
            )
        })?,
    })
}

/// Replace one service's stored query key without touching any other local
/// credential or endpoint binding. The same project lock as first-time
/// provisioning protects the read-modify-write sequence across processes. If
/// another repair replaced the expected stale key while this caller waited,
/// its winning credential is returned without another rotation.
pub async fn repair_service_query_setup(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    service_name: &str,
    expected_stale: ServiceQueryKey,
) -> CloudResult<ServiceQueryKey> {
    let _lock = credentials::lock_service_query_provisioning()?;
    let old = credentials::try_get_service_query_key(service_id)?
        .ok_or_else(|| {
            CloudError::new(format!(
                "no stored query key exists for service {service_id}; refusing to provision one under --repair-query-key"
            ))
        })?;
    let old_api_key_id = old.api_key_id.as_deref().ok_or_else(|| {
        CloudError::new(format!(
            "the stored query key for service {service_id} predates exact management API key IDs; refusing unsafe repair"
        ))
    })?;
    let old_org_id = old.organization_id.as_deref().ok_or_else(|| {
        CloudError::new(format!(
            "the stored query key for service {service_id} has no provisioning organization; refusing unsafe repair"
        ))
    })?;
    if old_org_id != org_id {
        return Err(CloudError::new(format!(
            "the stored query key for service {service_id} belongs to organization {old_org_id}, not {org_id}; refusing to repair it"
        )));
    }
    if old.api_key_id != expected_stale.api_key_id
        || old.key_id != expected_stale.key_id
        || old.key_secret != expected_stale.key_secret
    {
        return Ok(old);
    }

    let endpoint = client.get_query_endpoint(org_id, service_id).await?;
    let endpoint =
        repair_endpoint_configuration(endpoint, old.endpoint_id.as_deref(), old_api_key_id)?;
    let key = create_query_key(client, org_id, service_name).await?;

    let mut replaced = false;
    let replacement_keys = endpoint
        .open_api_keys
        .iter()
        .filter_map(|existing| {
            if existing == old_api_key_id {
                if replaced {
                    None
                } else {
                    replaced = true;
                    Some(key.api_key_id.clone())
                }
            } else {
                Some(existing.clone())
            }
        })
        .collect();
    let replacement_request = endpoint.request(replacement_keys);
    let rollback_request = endpoint.request(endpoint.open_api_keys.clone());
    let replacement_endpoint = match client
        .create_query_endpoint(org_id, service_id, &replacement_request)
        .await
    {
        Ok(endpoint) => endpoint,
        Err(error) => {
            discard_api_key(client, org_id, &key.api_key_id).await;
            return Err(error);
        }
    };

    let stored = build_service_query_key(
        org_id,
        key.api_key_id.clone(),
        key.key_id,
        key.key_secret,
        replacement_endpoint.id.or(endpoint.id),
        service_name,
        Utc::now(),
    );
    if let Err(save_error) = credentials::set_service_query_key(service_id, stored.clone()) {
        let rollback_error = client
            .create_query_endpoint(org_id, service_id, &rollback_request)
            .await
            .err();
        return match rollback_error {
            Some(rollback_error) => Err(CloudError::new(format!(
                "{save_error}; additionally failed to restore the previous query endpoint binding: {rollback_error}"
            ))),
            None => {
                discard_api_key(client, org_id, &key.api_key_id).await;
                Err(save_error)
            }
        };
    }

    client
        .delete_api_key_if_exists(org_id, old_api_key_id)
        .await
        .map_err(|mut error| {
            error.message = format!(
                "the replacement query key was stored, but the old API key {old_api_key_id} could not be deleted: {}",
                error.message
            );
            error
        })?;

    Ok(stored)
}

/// Capture an existing endpoint exactly enough to restore it if local
/// persistence fails after binding. Every request field is replaced by an
/// upsert, so binding is unsafe when the GET omits any of them.
fn existing_endpoint_request(
    endpoint: Option<ServiceQueryAPIEndpoint>,
) -> CloudResult<InstanceServiceQueryApiEndpointsPostRequest> {
    let incomplete = |field: &str| {
        CloudError::new(format!(
            "the query endpoint response is missing field '{field}', so its pre-bind state cannot \
             be safely restored; refusing to bind a new key"
        ))
    };
    let endpoint = endpoint.ok_or_else(|| incomplete("result"))?;
    let open_api_keys = endpoint
        .open_api_keys
        .ok_or_else(|| incomplete("openApiKeys"))?;
    let allowed_origins = endpoint
        .allowed_origins
        .ok_or_else(|| incomplete("allowedOrigins"))?;
    let roles = endpoint.roles.ok_or_else(|| incomplete("roles"))?;
    Ok(InstanceServiceQueryApiEndpointsPostRequest {
        allowed_origins,
        open_api_keys,
        roles,
    })
}

/// Bind `api_key_uuid` to the service's query endpoint, merging into the
/// endpoint's existing `openApiKeys` so we don't silently revoke other
/// key bindings the user set up. Only the key list is merged: the upsert
/// still replaces `roles` and `allowedOrigins` with this module's values.
async fn bind_query_endpoint(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_uuid: &str,
) -> CloudResult<BoundQueryEndpoint> {
    let (mut open_api_keys, compensation) = match client
        .api()
        .instance_query_endpoint_get(org_id, service_id)
        .await
    {
        Ok(resp) => {
            let request = existing_endpoint_request(resp.result)?;
            (
                request.open_api_keys.clone(),
                EndpointCompensation::Restore(request),
            )
        }
        // Only a 404 means there is no endpoint yet, so this binding is the
        // first one and starts from an empty list.
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => {
            (Vec::new(), EndpointCompensation::Delete)
        }
        Err(e) => return Err(client.convert_error_for_organization(e, org_id)),
    };
    if !open_api_keys.iter().any(|k| k == api_key_uuid) {
        open_api_keys.push(api_key_uuid.to_string());
    }

    let endpoint_request = InstanceServiceQueryApiEndpointsPostRequest {
        // The binding grants read/write SQL access only through this service's
        // endpoint; it does not assign an organization-level role to the key.
        roles: vec![QueryEndpointRole::SqlConsoleAdmin],
        open_api_keys,
        allowed_origins: ALLOWED_ORIGINS.to_string(),
    };

    let endpoint = client
        .create_query_endpoint(org_id, service_id, &endpoint_request)
        .await?;
    Ok(BoundQueryEndpoint {
        endpoint,
        compensation,
    })
}

async fn compensate_endpoint_binding(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_uuid: &str,
    compensation: &EndpointCompensation,
) -> CloudResult<()> {
    let current = match client
        .api()
        .instance_query_endpoint_get(org_id, service_id)
        .await
    {
        Ok(response) => response.result,
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => {
            return match compensation {
                EndpointCompensation::Delete => Ok(()),
                EndpointCompensation::Restore(request) => client
                    .create_query_endpoint(org_id, service_id, request)
                    .await
                    .map(|_| ()),
            };
        }
        Err(error) => return Err(client.convert_error_for_organization(error, org_id)),
    };

    match compensation {
        EndpointCompensation::Delete => {
            let mut request = existing_endpoint_request(current)?;
            if request.allowed_origins == ALLOWED_ORIGINS
                && request.roles == [QueryEndpointRole::SqlConsoleAdmin]
                && request.open_api_keys == [api_key_uuid]
            {
                client.delete_query_endpoint(org_id, service_id).await?;
            } else {
                let previous_len = request.open_api_keys.len();
                request.open_api_keys.retain(|key| key != api_key_uuid);
                if request.open_api_keys.len() == previous_len {
                    return Ok(());
                }
                client
                    .create_query_endpoint(org_id, service_id, &request)
                    .await?;
            }
        }
        EndpointCompensation::Restore(previous) => {
            let current = current.ok_or_else(|| {
                CloudError::new(
                    "the query endpoint response is missing field 'result', so the newly bound \
                     key cannot be safely removed",
                )
            })?;
            let mut request = previous.clone();
            request.open_api_keys = current.open_api_keys.ok_or_else(|| {
                CloudError::new(
                    "the query endpoint response is missing field 'openApiKeys', so the newly \
                     bound key cannot be safely removed",
                )
            })?;
            request.open_api_keys.retain(|key| key != api_key_uuid);
            client
                .create_query_endpoint(org_id, service_id, &request)
                .await?;
        }
    }
    Ok(())
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

    #[test]
    fn stored_query_key_keeps_the_management_resource_ownership() {
        let created_at = DateTime::parse_from_rfc3339("2026-05-11T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let key = build_service_query_key(
            "org-1",
            "api-key-uuid".into(),
            "query-key-id".into(),
            "query-key-secret".into(),
            Some("endpoint-id".into()),
            "demo",
            created_at,
        );

        assert_eq!(key.organization_id.as_deref(), Some("org-1"));
        assert_eq!(key.api_key_id.as_deref(), Some("api-key-uuid"));
        assert_eq!(key.key_id, "query-key-id");
        assert_eq!(key.key_secret, "query-key-secret");
        assert_eq!(key.endpoint_id.as_deref(), Some("endpoint-id"));
        assert_eq!(key.service_name, "demo");
        assert_eq!(key.created_at, created_at);
    }

    fn endpoint(open_api_keys: Option<Vec<&str>>) -> ServiceQueryAPIEndpoint {
        ServiceQueryAPIEndpoint {
            allowed_origins: Some("https://example.com".to_string()),
            id: Some("ep-1".to_string()),
            open_api_keys: open_api_keys.map(|keys| keys.into_iter().map(str::to_string).collect()),
            roles: Some(vec![QueryEndpointRole::SqlConsoleReadOnly]),
        }
    }

    #[test]
    fn existing_keys_are_returned_when_the_endpoint_reports_them() {
        assert_eq!(
            existing_endpoint_request(Some(endpoint(Some(vec!["uuid-a", "uuid-b"]))))
                .unwrap()
                .open_api_keys,
            vec!["uuid-a".to_string(), "uuid-b".to_string()]
        );
    }

    #[test]
    fn an_explicitly_empty_key_list_is_a_real_answer() {
        assert!(
            existing_endpoint_request(Some(endpoint(Some(vec![]))))
                .unwrap()
                .open_api_keys
                .is_empty()
        );
    }

    #[test]
    fn absent_open_api_keys_is_refused_rather_than_treated_as_empty() {
        let err = existing_endpoint_request(Some(endpoint(None))).unwrap_err();
        assert!(
            err.to_string().contains("'openApiKeys'") && err.to_string().contains("restored"),
            "error should name the field and the rollback consequence: {err}",
        );
    }

    #[test]
    fn absent_result_is_refused_rather_than_treated_as_empty() {
        let err = existing_endpoint_request(None).unwrap_err();
        assert!(
            err.to_string().contains("'result'"),
            "error should name the field: {err}",
        );
    }
}
