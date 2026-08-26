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
        service_name: service_name.to_string(),
        created_at,
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
}
