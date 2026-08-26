//! Auto-provisioning of per-service Query API endpoints.
//!
//! Creates a dedicated API key and binds it to the service's query endpoint
//! with role `sql_console_admin`. The key's `key_id`/`key_secret` are
//! persisted in `.clickhouse/credentials.json` keyed by service id, so later
//! `cloud service query` invocations can authenticate without contacting the
//! control plane.

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials::{self, ServiceQueryKey};
use chrono::{DateTime, Utc};
use clickhouse_cloud_api::models::{
    ApiKeyPostRequest, ApiKeyPostRequestState, ApiKeyPostResponse,
    InstanceServiceQueryApiEndpointsPostRequest, IpAccessListEntry, QueryEndpointRole,
    ServiceQueryAPIEndpoint,
};
use serde::Serialize;

/// Default `allowedOrigins` for the query endpoint. The CLI is a non-browser
/// caller so CORS doesn't apply, but the API still requires a value.
const ALLOWED_ORIGINS: &str = "*";

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
    api_key_id: &str,
    key_id: String,
    key_secret: String,
    endpoint_id: Option<String>,
    service_name: &str,
    created_at: DateTime<Utc>,
) -> ServiceQueryKey {
    ServiceQueryKey {
        organization_id: Some(organization_id.to_string()),
        api_key_id: Some(api_key_id.to_string()),
        key_id,
        key_secret,
        endpoint_id,
        pending_cleanup_api_key_ids: vec![],
        service_name: service_name.to_string(),
        created_at,
    }
}

fn build_query_api_key_request(service_name: &str) -> ApiKeyPostRequest {
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
) -> CloudResult<ServiceQueryKey> {
    if let Some(existing) = credentials::try_get_service_query_key(service_id)? {
        return Ok(existing);
    }

    // Serialize the complete read-create-bind-save transaction across CLI
    // processes in this project. A waiter must re-read after acquisition: the
    // process that held the lock may have completed provisioning while it
    // waited.
    let provisioning_lock = credentials::lock_query_provisioning().await?;
    if let Some(existing) = credentials::try_get_service_query_key(service_id)? {
        return Ok(existing);
    }

    let key_request = build_query_api_key_request(service_name);

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
            return fail_after_key_creation(client, org_id, &api_key_uuid, e).await;
        }
    };

    let endpoint = match bind_query_endpoint(client, org_id, service_id, &api_key_uuid).await {
        Ok(endpoint) => endpoint,
        Err(e) => {
            // The key was created but never bound or persisted, so nothing
            // can use it.
            return fail_after_key_creation(client, org_id, &api_key_uuid, e).await;
        }
    };

    // The upsert succeeded, so the key is bound and fully usable. The echoed
    // `id` is diagnostic only, never an auth input: persist the record
    // without it rather than deleting a working credential and leaving a
    // dangling UUID in the endpoint's `openApiKeys`.
    let stored = build_service_query_key(
        org_id,
        &api_key_uuid,
        key_id,
        key_secret,
        endpoint.id,
        service_name,
        Utc::now(),
    );
    if let Err(error) =
        credentials::set_service_query_key(service_id, stored.clone(), &provisioning_lock)
    {
        return fail_after_endpoint_binding(client, org_id, service_id, &api_key_uuid, error).await;
    }

    Ok(stored)
}

async fn fail_after_endpoint_binding<T>(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_id: &str,
    persistence_error: CloudError,
) -> CloudResult<T> {
    if let Err(unbind_error) = unbind_query_endpoint(client, org_id, service_id, api_key_id).await {
        return Err(CloudError {
            message: format!(
                "local credential persistence failed: {persistence_error}; additionally, failed \
                 to remove API key {api_key_id} from the query endpoint: {unbind_error}. The key \
                 was retained for recovery"
            ),
            kind: persistence_error.kind,
        });
    }

    fail_after_key_creation(client, org_id, api_key_id, persistence_error).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryKeyRepairResult {
    pub(crate) status: &'static str,
    pub(crate) service_id: String,
    pub(crate) organization_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) replaced_api_key_id: Option<String>,
    pub(crate) api_key_id: String,
    pub(crate) endpoint_id: String,
}

#[derive(Clone)]
enum RepairEndpointState {
    Existing {
        endpoint_id: String,
        original_request: InstanceServiceQueryApiEndpointsPostRequest,
    },
    Missing,
}

fn require_owned_query_key<'a>(
    key: &'a ServiceQueryKey,
    org_id: &str,
    service_id: &str,
) -> CloudResult<(&'a str, &'a str)> {
    let key_org_id = key.organization_id.as_deref().ok_or_else(|| {
        CloudError::new(format!(
            "the stored query key for service {service_id} has no ownership organization; \
             refusing to repair a legacy or non-owned record"
        ))
    })?;
    if key_org_id != org_id {
        return Err(CloudError::new(format!(
            "the stored query key for service {service_id} belongs to organization {key_org_id}, \
             not {org_id}; refusing to repair it"
        )));
    }
    let api_key_id = key.api_key_id.as_deref().ok_or_else(|| {
        CloudError::new(format!(
            "the stored query key for service {service_id} has no exact management API key ID; \
             refusing to repair a legacy or non-owned record"
        ))
    })?;
    let endpoint_id = key.endpoint_id.as_deref().ok_or_else(|| {
        CloudError::new(format!(
            "the stored query key for service {service_id} has no exact query endpoint ID; \
             refusing to repair a record whose endpoint ownership cannot be verified"
        ))
    })?;
    if key
        .pending_cleanup_api_key_ids
        .iter()
        .any(|pending| pending == api_key_id)
    {
        return Err(CloudError::new(format!(
            "the stored query key for service {service_id} marks its active management API key \
             for cleanup; refusing to delete it"
        )));
    }
    Ok((api_key_id, endpoint_id))
}

fn inspect_repair_endpoint(
    endpoint: Option<ServiceQueryAPIEndpoint>,
    expected_endpoint_id: &str,
    service_id: &str,
) -> CloudResult<RepairEndpointState> {
    let Some(endpoint) = endpoint else {
        return Ok(RepairEndpointState::Missing);
    };
    let endpoint_id = require_field(endpoint.id, "query endpoint id")?;
    if endpoint_id != expected_endpoint_id {
        return Err(CloudError::new(format!(
            "query endpoint {endpoint_id} for service {service_id} does not match the owned \
             endpoint {expected_endpoint_id}; refusing to modify its bindings"
        )));
    }
    let original_request = InstanceServiceQueryApiEndpointsPostRequest {
        roles: require_field(endpoint.roles, "query endpoint roles")?,
        open_api_keys: require_field(endpoint.open_api_keys, "query endpoint openApiKeys")?,
        allowed_origins: require_field(endpoint.allowed_origins, "query endpoint allowedOrigins")?,
    };
    Ok(RepairEndpointState::Existing {
        endpoint_id,
        original_request,
    })
}

fn build_repair_endpoint_request(
    state: &RepairEndpointState,
    old_api_key_id: &str,
    new_api_key_id: &str,
) -> InstanceServiceQueryApiEndpointsPostRequest {
    match state {
        RepairEndpointState::Existing {
            original_request, ..
        } => {
            let mut request = original_request.clone();
            request
                .open_api_keys
                .retain(|key_id| key_id != old_api_key_id);
            if !request
                .open_api_keys
                .iter()
                .any(|key_id| key_id == new_api_key_id)
            {
                request.open_api_keys.push(new_api_key_id.to_string());
            }
            request
        }
        RepairEndpointState::Missing => InstanceServiceQueryApiEndpointsPostRequest {
            roles: vec![QueryEndpointRole::SqlConsoleAdmin],
            open_api_keys: vec![new_api_key_id.to_string()],
            allowed_origins: ALLOWED_ORIGINS.to_string(),
        },
    }
}

async fn restore_repair_endpoint(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    state: &RepairEndpointState,
) -> CloudResult<()> {
    match state {
        RepairEndpointState::Existing {
            original_request, ..
        } => {
            client
                .create_query_endpoint(org_id, service_id, original_request)
                .await?;
        }
        RepairEndpointState::Missing => {
            client
                .delete_query_endpoint_if_exists(org_id, service_id)
                .await?;
        }
    }
    Ok(())
}

async fn fail_after_repair_binding<T>(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    new_api_key_id: &str,
    endpoint_state: &RepairEndpointState,
    mut repair_error: CloudError,
) -> CloudResult<T> {
    if let Err(rollback_error) =
        restore_repair_endpoint(client, org_id, service_id, endpoint_state).await
    {
        repair_error.message = format!(
            "{repair_error}; additionally, failed to restore the original query endpoint \
             bindings: {rollback_error}"
        );
    }
    if let Err(cleanup_error) = client
        .delete_api_key_if_exists(org_id, new_api_key_id)
        .await
    {
        repair_error.message = format!(
            "{repair_error}; additionally, failed to delete newly created API key \
             {new_api_key_id}: {cleanup_error}"
        );
    }
    Err(repair_error)
}

pub async fn repair_service_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
) -> CloudResult<QueryKeyRepairResult> {
    let provisioning_lock = credentials::lock_query_provisioning().await?;
    let mut old_key = credentials::try_get_service_query_key(service_id)?.ok_or_else(|| {
        CloudError::new(format!(
            "no stored query key exists for service {service_id}; run a query normally before \
             requesting repair"
        ))
    })?;
    let (old_api_key_id, expected_endpoint_id) =
        require_owned_query_key(&old_key, org_id, service_id)?;
    let old_api_key_id = old_api_key_id.to_string();
    let expected_endpoint_id = expected_endpoint_id.to_string();

    if !old_key.pending_cleanup_api_key_ids.is_empty() {
        for pending_id in &old_key.pending_cleanup_api_key_ids {
            client
                .delete_api_key_if_exists(org_id, pending_id)
                .await
                .map_err(|mut error| {
                    error.message = format!(
                        "failed to finish query-key repair for service {service_id}: could not \
                         delete superseded API key {pending_id}: {}",
                        error.message
                    );
                    error
                })?;
        }
        let cleaned_ids = std::mem::take(&mut old_key.pending_cleanup_api_key_ids);
        credentials::set_service_query_key(service_id, old_key, &provisioning_lock)?;
        return Ok(QueryKeyRepairResult {
            status: "cleanup_completed",
            service_id: service_id.to_string(),
            organization_id: org_id.to_string(),
            replaced_api_key_id: cleaned_ids.last().cloned(),
            api_key_id: old_api_key_id,
            endpoint_id: expected_endpoint_id,
        });
    }

    let endpoint = client
        .get_query_endpoint_for_binding(org_id, service_id)
        .await?;
    let endpoint_state = inspect_repair_endpoint(endpoint, &expected_endpoint_id, service_id)?;

    let key_request = build_query_api_key_request(&old_key.service_name);
    let key_response = client.create_api_key(org_id, &key_request).await?;
    let new_api_key_id =
        require_field(key_response.key.as_ref().and_then(|key| key.id), "key.id")?.to_string();
    let (new_key_id, new_key_secret) = match require_credential_pair(&key_response) {
        Ok(pair) => pair,
        Err(error) => {
            return fail_after_key_creation(client, org_id, &new_api_key_id, error).await;
        }
    };

    let endpoint_request =
        build_repair_endpoint_request(&endpoint_state, &old_api_key_id, &new_api_key_id);
    let repaired_endpoint = match client
        .create_query_endpoint(org_id, service_id, &endpoint_request)
        .await
    {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return fail_after_repair_binding(
                client,
                org_id,
                service_id,
                &new_api_key_id,
                &endpoint_state,
                error,
            )
            .await;
        }
    };
    let repaired_endpoint_id = match require_field(repaired_endpoint.id, "query endpoint id") {
        Ok(endpoint_id) => endpoint_id,
        Err(error) => {
            return fail_after_repair_binding(
                client,
                org_id,
                service_id,
                &new_api_key_id,
                &endpoint_state,
                error,
            )
            .await;
        }
    };
    if let RepairEndpointState::Existing { endpoint_id, .. } = &endpoint_state
        && repaired_endpoint_id != *endpoint_id
    {
        return fail_after_repair_binding(
            client,
            org_id,
            service_id,
            &new_api_key_id,
            &endpoint_state,
            CloudError::new(format!(
                "query endpoint repair returned endpoint {repaired_endpoint_id}, expected owned \
                 endpoint {endpoint_id}"
            )),
        )
        .await;
    }

    let mut replacement = build_service_query_key(
        org_id,
        &new_api_key_id,
        new_key_id,
        new_key_secret,
        Some(repaired_endpoint_id.clone()),
        &old_key.service_name,
        Utc::now(),
    );
    replacement
        .pending_cleanup_api_key_ids
        .push(old_api_key_id.clone());
    if let Err(error) =
        credentials::set_service_query_key(service_id, replacement.clone(), &provisioning_lock)
    {
        return fail_after_repair_binding(
            client,
            org_id,
            service_id,
            &new_api_key_id,
            &endpoint_state,
            error,
        )
        .await;
    }

    if let Err(mut error) = client
        .delete_api_key_if_exists(org_id, &old_api_key_id)
        .await
    {
        error.message = format!(
            "the replacement query key for service {service_id} is active, but the superseded \
             API key {old_api_key_id} could not be deleted: {}. Its exact ID remains stored; \
             rerun `clickhousectl cloud service repair-query-key {service_id} --org-id {org_id}` \
             to finish cleanup",
            error.message
        );
        return Err(error);
    }

    replacement.pending_cleanup_api_key_ids.clear();
    credentials::set_service_query_key(service_id, replacement, &provisioning_lock)?;
    Ok(QueryKeyRepairResult {
        status: "repaired",
        service_id: service_id.to_string(),
        organization_id: org_id.to_string(),
        replaced_api_key_id: Some(old_api_key_id),
        api_key_id: new_api_key_id,
        endpoint_id: repaired_endpoint_id,
    })
}

async fn fail_after_key_creation<T>(
    client: &CloudClient,
    org_id: &str,
    api_key_id: &str,
    provisioning_error: CloudError,
) -> CloudResult<T> {
    match client.delete_api_key_if_exists(org_id, api_key_id).await {
        Ok(_) => Err(provisioning_error),
        Err(cleanup_error) => Err(CloudError {
            message: format!(
                "{provisioning_error}; additionally, failed to delete newly created API key \
                 {api_key_id}: {cleanup_error}"
            ),
            kind: provisioning_error.kind,
        }),
    }
}

/// The keys already bound to an existing query endpoint, taken from a
/// successful GET. The upsert replaces `openApiKeys` wholesale, so an absent
/// `openApiKeys` cannot be read as "no keys bound": merging into an empty list
/// would revoke every binding the response failed to report. An explicitly
/// empty list is a real answer and merges normally.
fn existing_open_api_keys(endpoint: ServiceQueryAPIEndpoint) -> CloudResult<Vec<String>> {
    endpoint.open_api_keys.ok_or_else(|| {
        CloudError::new(
            "the query endpoint response is missing field 'openApiKeys', so the keys currently \
             bound to the endpoint are unknown; binding a new key would revoke them",
        )
    })
}

fn endpoint_without_key(
    endpoint: ServiceQueryAPIEndpoint,
    api_key_uuid: &str,
) -> CloudResult<Option<InstanceServiceQueryApiEndpointsPostRequest>> {
    let mut open_api_keys = existing_open_api_keys(endpoint.clone())?;
    if !open_api_keys.iter().any(|key| key == api_key_uuid) {
        return Ok(None);
    }
    open_api_keys.retain(|key| key != api_key_uuid);

    Ok(Some(InstanceServiceQueryApiEndpointsPostRequest {
        allowed_origins: require_field(endpoint.allowed_origins, "allowedOrigins")?,
        open_api_keys,
        roles: require_field(endpoint.roles, "roles")?,
    }))
}

async fn unbind_query_endpoint(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_uuid: &str,
) -> CloudResult<()> {
    let Some(endpoint) = client
        .get_query_endpoint_for_binding(org_id, service_id)
        .await?
    else {
        return Ok(());
    };
    let Some(request) = endpoint_without_key(endpoint, api_key_uuid)? else {
        return Ok(());
    };
    client
        .create_query_endpoint(org_id, service_id, &request)
        .await?;
    Ok(())
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
) -> CloudResult<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
    let mut open_api_keys = match client
        .get_query_endpoint_for_binding(org_id, service_id)
        .await?
    {
        Some(endpoint) => existing_open_api_keys(endpoint)?,
        None => Vec::new(),
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

    client
        .create_query_endpoint(org_id, service_id, &endpoint_request)
        .await
}

impl CloudClient {
    async fn get_query_endpoint_for_binding(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<Option<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint>> {
        match self
            .api()
            .instance_query_endpoint_get(org_id, service_id)
            .await
        {
            Ok(response) => Self::unwrap_response(response).map(Some),
            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(None),
            Err(error) => Err(self.convert_error_for_organization(error, org_id)),
        }
    }

    async fn delete_query_endpoint_if_exists(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<()> {
        match self
            .api()
            .instance_query_endpoint_delete(org_id, service_id)
            .await
        {
            Ok(_) | Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(()),
            Err(error) => Err(self.convert_error_for_organization(error, org_id)),
        }
    }
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
            "api-key-uuid",
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
        assert!(key.pending_cleanup_api_key_ids.is_empty());
        assert_eq!(key.service_name, "demo");
        assert_eq!(key.created_at, created_at);
    }

    fn endpoint(
        open_api_keys: Option<Vec<&str>>,
    ) -> clickhouse_cloud_api::models::ServiceQueryAPIEndpoint {
        clickhouse_cloud_api::models::ServiceQueryAPIEndpoint {
            allowed_origins: None,
            id: Some("ep-1".to_string()),
            open_api_keys: open_api_keys.map(|keys| keys.into_iter().map(str::to_string).collect()),
            roles: None,
        }
    }

    #[test]
    fn existing_keys_are_returned_when_the_endpoint_reports_them() {
        assert_eq!(
            existing_open_api_keys(endpoint(Some(vec!["uuid-a", "uuid-b"]))).unwrap(),
            vec!["uuid-a".to_string(), "uuid-b".to_string()],
        );
    }

    #[test]
    fn an_explicitly_empty_key_list_is_a_real_answer() {
        assert!(
            existing_open_api_keys(endpoint(Some(vec![])))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn absent_open_api_keys_is_refused_rather_than_treated_as_empty() {
        let err = existing_open_api_keys(endpoint(None)).unwrap_err();
        assert!(
            err.to_string().contains("'openApiKeys'") && err.to_string().contains("revoke"),
            "error should name the field and the consequence: {err}",
        );
    }

    #[test]
    fn endpoint_unbind_preserves_current_settings_and_other_keys() {
        let request = endpoint_without_key(
            ServiceQueryAPIEndpoint {
                allowed_origins: Some("https://example.com".into()),
                id: Some("ep-1".into()),
                open_api_keys: Some(vec!["other-key".into(), "new-key".into()]),
                roles: Some(vec![QueryEndpointRole::SqlConsoleReadOnly]),
            },
            "new-key",
        )
        .unwrap()
        .unwrap();

        assert_eq!(request.allowed_origins, "https://example.com");
        assert_eq!(request.open_api_keys, ["other-key"]);
        assert_eq!(request.roles, [QueryEndpointRole::SqlConsoleReadOnly]);
    }

    #[test]
    fn query_api_key_request_has_service_scoped_provisioning_shape() {
        let request = build_query_api_key_request("analytics");
        assert_eq!(request.name, "clickhousectl-query-analytics");
        assert!(request.assigned_role_ids.is_empty());
        assert!(request.expire_at.is_none());
        assert!(request.hash_data.is_none());
        assert_eq!(request.state, ApiKeyPostRequestState::Enabled);
        assert_eq!(request.ip_access_list.len(), 1);
        assert_eq!(request.ip_access_list[0].source, "0.0.0.0/0");
        assert_eq!(
            request.ip_access_list[0].description.as_deref(),
            Some("clickhousectl auto-provisioned key for service analytics")
        );
        #[cfg(feature = "deprecated-fields")]
        assert!(request.roles.is_none());
    }

    #[test]
    fn repair_endpoint_request_replaces_only_the_owned_binding() {
        let state = inspect_repair_endpoint(
            Some(ServiceQueryAPIEndpoint {
                id: Some("endpoint-1".into()),
                roles: Some(vec![QueryEndpointRole::SqlConsoleReadOnly]),
                open_api_keys: Some(vec!["other-key".into(), "old-key".into()]),
                allowed_origins: Some("https://example.com".into()),
            }),
            "endpoint-1",
            "service-1",
        )
        .unwrap();

        let request = build_repair_endpoint_request(&state, "old-key", "new-key");
        assert_eq!(request.roles, vec![QueryEndpointRole::SqlConsoleReadOnly]);
        assert_eq!(request.open_api_keys, vec!["other-key", "new-key"]);
        assert_eq!(request.allowed_origins, "https://example.com");

        let RepairEndpointState::Existing {
            original_request, ..
        } = state
        else {
            panic!("expected existing endpoint");
        };
        assert_eq!(
            original_request.open_api_keys,
            vec!["other-key", "old-key"],
            "the rollback request must retain the exact original bindings"
        );
    }

    #[test]
    fn repair_endpoint_request_can_recreate_a_missing_owned_endpoint() {
        let state = inspect_repair_endpoint(None, "deleted-endpoint", "service-1").unwrap();
        let request = build_repair_endpoint_request(&state, "old-key", "new-key");
        assert_eq!(request.roles, vec![QueryEndpointRole::SqlConsoleAdmin]);
        assert_eq!(request.open_api_keys, vec!["new-key"]);
        assert_eq!(request.allowed_origins, "*");
    }

    #[test]
    fn repair_refuses_a_different_or_incomplete_endpoint() {
        let different = inspect_repair_endpoint(
            Some(ServiceQueryAPIEndpoint {
                id: Some("endpoint-2".into()),
                roles: Some(vec![]),
                open_api_keys: Some(vec![]),
                allowed_origins: Some("*".into()),
            }),
            "endpoint-1",
            "service-1",
        )
        .err()
        .unwrap();
        assert!(different.to_string().contains("refusing to modify"));

        let incomplete = inspect_repair_endpoint(
            Some(ServiceQueryAPIEndpoint {
                id: Some("endpoint-1".into()),
                roles: Some(vec![]),
                open_api_keys: None,
                allowed_origins: Some("*".into()),
            }),
            "endpoint-1",
            "service-1",
        )
        .err()
        .unwrap();
        assert!(incomplete.to_string().contains("openApiKeys"));
    }

    #[test]
    fn repair_requires_complete_ownership_metadata() {
        let key = ServiceQueryKey {
            organization_id: None,
            api_key_id: None,
            key_id: "query-key".into(),
            key_secret: "query-secret".into(),
            endpoint_id: Some("endpoint-1".into()),
            pending_cleanup_api_key_ids: vec![],
            service_name: "demo".into(),
            created_at: Utc::now(),
        };
        let error = require_owned_query_key(&key, "org-1", "service-1").unwrap_err();
        assert!(error.to_string().contains("legacy or non-owned"));
    }
}
