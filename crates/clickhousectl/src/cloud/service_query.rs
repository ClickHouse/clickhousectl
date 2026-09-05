//! Auto-provisioning of per-service Query API endpoints.
//!
//! Creates a dedicated API key and binds it to the service's query endpoint
//! with role `sql_console_admin`. The key's `key_id`/`key_secret` are
//! persisted in `.clickhouse/credentials.json` keyed by service id, so later
//! `cloud service query` invocations can authenticate without contacting the
//! control plane.

use crate::cloud::client::{CloudClient, CloudError, CloudErrorKind, Result as CloudResult};
use crate::cloud::credentials::{self, ServiceQueryKey};
use crate::cloud::output::{CloudErrorCode, CloudErrorDetail, eprint_line};
use crate::failure::{ApiFailure, FailureKind, FailureStage};
use chrono::{DateTime, Utc};
use clickhouse_cloud_api::models::{
    ApiKey, ApiKeyPostRequest, ApiKeyPostRequestState, ApiKeyPostResponse, ApiKeyState,
    InstanceServiceQueryApiEndpointsPostRequest, IpAccessListEntry, QueryEndpointRole,
    ServiceQueryAPIEndpoint,
};
use serde::Serialize;
use std::time::Duration;

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

/// How long a key created by this invocation may take to become visible to
/// the service behind the query-endpoint upsert (#658).
///
/// `POST /keys` and `PUT .../serviceQueryEndpoint` are answered by different
/// services, and the second can answer `400` for a key the first has just
/// created and `GET /keys/{id}` already returns. The wait is bounded and
/// exponential like the query-endpoint readiness wait in `services.rs`. Live
/// on 2026-09-02, 1 of 20 back-to-back repairs hit it and the first retry,
/// one second later, succeeded; the 30 s deadline leaves ample headroom. The
/// deadline decides whether another attempt is *started*; an attempt already
/// in flight is never cut off, so a last-attempt success is never discarded.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyPropagation {
    pub(crate) timeout: Duration,
    pub(crate) initial_backoff: Duration,
    pub(crate) max_backoff: Duration,
}

pub(crate) const KEY_PROPAGATION: KeyPropagation = KeyPropagation {
    timeout: Duration::from_secs(30),
    initial_backoff: Duration::from_secs(1),
    max_backoff: Duration::from_secs(8),
};

/// Whether an upsert failure can be the endpoint service not yet seeing a key
/// created moments ago. Structural: the status of the typed [`Error::Api`]
/// variant, never the message text (#450). The propagation failure is a
/// `400` whose body names the key ID; because that ID varies, no fixed
/// message could tell it apart honestly, so *every* `400` inside the window
/// is retried. The request body is built by this module from constants plus
/// the key list it just read, so a genuine `400` is rare, and it costs
/// exactly the window before it fails with the same rollback as before.
///
/// [`Error::Api`]: clickhouse_cloud_api::Error::Api
fn key_propagation_error(error: &clickhouse_cloud_api::Error) -> bool {
    matches!(error, clickhouse_cloud_api::Error::Api { status: 400, .. })
}

/// The upsert notice, printed once per run on the first retry. Stderr in every
/// output mode, like the readiness notice, so `--json` stdout stays one value.
const KEY_PROPAGATION_NOTICE: &str =
    "Waiting for the new API key to become visible to the Query API endpoint...";

/// Run `upsert` until it succeeds, fails for a reason other than key
/// propagation, or `propagation.timeout` elapses. Only the upsert is repeated,
/// with the same body each time; a success is never followed by another
/// attempt. Each retry is noted for telemetry so a dashboard sees that the
/// run waited on propagation, the same way the readiness loop does (#450).
async fn upsert_until_key_propagates<T, F, Fut>(
    propagation: KeyPropagation,
    mut upsert: F,
) -> Result<T, clickhouse_cloud_api::Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, clickhouse_cloud_api::Error>>,
{
    let deadline = tokio::time::Instant::now() + propagation.timeout;
    let mut backoff = propagation.initial_backoff;
    let mut waiting = false;

    loop {
        let error = match upsert().await {
            Ok(value) => return Ok(value),
            Err(error) if key_propagation_error(&error) => error,
            Err(error) => return Err(error),
        };
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(error);
        }
        if !waiting {
            // A closed stderr must not panic here: the key exists and the
            // rollback that would delete it has not run yet.
            crate::cloud::output::eprint_line(KEY_PROPAGATION_NOTICE);
            waiting = true;
        }
        crate::failure::note_retry();
        tokio::time::sleep(backoff.min(remaining)).await;
        backoff = backoff.saturating_mul(2).min(propagation.max_backoff);
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

    let key_response = client
        .create_api_key(org_id, &key_request)
        .await
        .map_err(|error| error.at_stage(FailureStage::KeyCreate))?;
    // `key_id`/`key_secret` are the credential pair used for query auth.
    // The endpoint binding's `openApiKeys` array, by contrast, references
    // API keys by their resource UUID — the same value the management
    // endpoints (GET/DELETE /v1/.../keys/{keyId}) accept. Resolve the UUID
    // first: every failure past this point deletes the key it identifies, so
    // an absent `key.id` is the only one with no cleanup available — we
    // cannot name the key we just created.
    let api_key_uuid = require_field(key_response.key.as_ref().and_then(|key| key.id), "key.id")
        .map_err(|error| error.at_stage(FailureStage::KeyCreate))?
        .to_string();

    // Every response field is `Option<T>`, and an absent credential cannot be
    // substituted with a placeholder: fail loudly instead of persisting an
    // empty key pair that every later query would reject.
    let (key_id, key_secret) = match require_credential_pair(&key_response) {
        Ok(pair) => pair,
        Err(e) => {
            let e = e.at_stage(FailureStage::KeyCreate);
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
            ..persistence_error
        });
    }

    fail_after_key_creation(client, org_id, api_key_id, persistence_error).await
}

/// Whether a repair confirmed that the Query API accepts the new key (#658).
///
/// The repair itself is committed whatever the value: the binding, the stored
/// record and the retirement of the old key do not depend on the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairVerification {
    /// A probe query with the new key succeeded.
    Verified,
    /// The key was not probed: the service is not running, its state could
    /// not be read, no Query API host is configured, or the record changed.
    /// The next query verifies the key.
    Skipped,
    /// The probe failed for a reason unrelated to the key becoming ready
    /// (transport, 5xx), or kept being rejected for the whole readiness
    /// window. The next query verifies the key.
    Failed,
}

impl RepairVerification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }
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
    /// Every retired owned key this run deleted: the superseded key and any
    /// earlier retirement whose deletion had been pending (#527).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) deleted_api_key_ids: Vec<String>,
    /// Retired owned keys whose deletion failed. Their exact IDs stay in the
    /// stored record and are retried on the next query (#527).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) pending_cleanup_api_key_ids: Vec<String>,
    /// The stderr warning that goes with `pending_cleanup_api_key_ids`. Prose
    /// for a human; the JSON result carries the IDs themselves.
    #[serde(skip)]
    pub(crate) cleanup_warning: Option<String>,
    /// Whether the Query API was confirmed to accept the new key (#658). Set
    /// by the command handler after the repair is committed; absent for an
    /// `already_repaired` result, whose key another process verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) verification: Option<RepairVerification>,
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

/// The endpoint configuration after a repair: every retired owned key (the
/// superseded one and any earlier retirement still pending) is unbound and the
/// replacement is bound; nothing else about the endpoint changes. Only IDs
/// taken from the stored record are ever removed (#527).
fn build_repair_endpoint_request(
    state: &RepairEndpointState,
    retired_api_key_ids: &[String],
    new_api_key_id: &str,
) -> InstanceServiceQueryApiEndpointsPostRequest {
    match state {
        RepairEndpointState::Existing {
            original_request, ..
        } => {
            let mut request = original_request.clone();
            request
                .open_api_keys
                .retain(|key_id| !retired_api_key_ids.contains(key_id));
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

/// Everything a failed repair needs to undo itself: the key it created, the
/// endpoint configuration it read, the record it started from and the lock
/// that record is held under.
struct RepairRollback<'a> {
    new_api_key_id: &'a str,
    endpoint_state: &'a RepairEndpointState,
    old_api_key_id: &'a str,
    lock: &'a credentials::QueryProvisioningLock,
}

async fn fail_after_repair_binding<T>(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    rollback: &RepairRollback<'_>,
    mut repair_error: CloudError,
) -> CloudResult<T> {
    let new_api_key_id = rollback.new_api_key_id;
    if let Err(rollback_error) =
        restore_repair_endpoint(client, org_id, service_id, rollback.endpoint_state).await
    {
        // The endpoint may still bind the new key, and a key deleted while
        // bound leaves a dangling UUID that can fail every later upsert, so
        // the key is kept and named rather than scheduled for deletion.
        repair_error.message = format!(
            "{repair_error}; additionally, failed to restore the original query endpoint \
             bindings: {rollback_error}. Newly created API key {new_api_key_id} was retained \
             because the endpoint may still reference it"
        );
        return Err(repair_error);
    }
    if let Err(cleanup_error) = client
        .delete_api_key_if_exists(org_id, new_api_key_id)
        .await
    {
        // The binding is back to what it was, so the new key is unbound and
        // grants nothing, but it exists. Its ID must not live only in this
        // message (#527): it is recorded on the record the repair started
        // from, guarded on that record's active key, so the next query
        // retries the deletion (#658).
        let recorded = credentials::add_pending_cleanup_if_api_key_matches(
            service_id,
            rollback.old_api_key_id,
            new_api_key_id,
            rollback.lock,
        );
        repair_error.message = match recorded {
            Ok(true) => format!(
                "{repair_error}; additionally, failed to delete newly created API key \
                 {new_api_key_id}: {cleanup_error}. Its ID was recorded under \
                 service_query_keys.{service_id}.pending_cleanup_api_key_ids in \
                 .clickhouse/credentials.json and deletion is retried automatically by the \
                 next `clickhousectl cloud service query --id {service_id} --org-id {org_id} \
                 ...`"
            ),
            Ok(false) => format!(
                "{repair_error}; additionally, failed to delete newly created API key \
                 {new_api_key_id}: {cleanup_error}. The stored record changed meanwhile, so \
                 the ID was not recorded; to delete the key now, run `clickhousectl cloud key \
                 delete {new_api_key_id} --org-id {org_id}`"
            ),
            Err(record_error) => format!(
                "{repair_error}; additionally, failed to delete newly created API key \
                 {new_api_key_id}: {cleanup_error}, and its ID could not be recorded for a \
                 later retry: {record_error}. To delete the key now, run `clickhousectl cloud \
                 key delete {new_api_key_id} --org-id {org_id}`"
            ),
        };
    }
    Err(repair_error)
}

fn repair_state_changed(expected: &ServiceQueryKey, current: &ServiceQueryKey) -> bool {
    expected.organization_id != current.organization_id
        || expected.api_key_id != current.api_key_id
        || expected.key_id != current.key_id
        || expected.key_secret != current.key_secret
        || expected.endpoint_id != current.endpoint_id
        || expected.pending_cleanup_api_key_ids != current.pending_cleanup_api_key_ids
}

pub async fn repair_service_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
) -> CloudResult<QueryKeyRepairResult> {
    // Snapshot before waiting so a concurrent repair winner can be reused
    // rather than immediately rotated again after the lock is acquired.
    let expected_stale = credentials::try_get_service_query_key(service_id)?.ok_or_else(|| {
        CloudError::new(format!(
            "no stored query key exists for service {service_id}; run a query normally before \
             requesting repair"
        ))
    })?;
    require_owned_query_key(&expected_stale, org_id, service_id)?;

    let provisioning_lock = credentials::lock_query_provisioning().await?;
    let old_key = credentials::try_get_service_query_key(service_id)?.ok_or_else(|| {
        CloudError::new(format!(
            "the stored query key for service {service_id} was removed while waiting to repair it"
        ))
    })?;
    let (old_api_key_id, expected_endpoint_id) =
        require_owned_query_key(&old_key, org_id, service_id)?;
    let old_api_key_id = old_api_key_id.to_string();
    let expected_endpoint_id = expected_endpoint_id.to_string();

    if repair_state_changed(&expected_stale, &old_key) {
        return Ok(QueryKeyRepairResult {
            status: "already_repaired",
            service_id: service_id.to_string(),
            organization_id: org_id.to_string(),
            replaced_api_key_id: None,
            api_key_id: old_api_key_id,
            endpoint_id: expected_endpoint_id,
            deleted_api_key_ids: vec![],
            pending_cleanup_api_key_ids: vec![],
            cleanup_warning: None,
            verification: None,
        });
    }

    // Every owned key this repair retires: the superseded key and any earlier
    // retirement whose deletion is still pending. All of them come from the
    // stored record read under the lock, never from a name match or a scan of
    // the organization's keys (#527).
    let mut retired_api_key_ids = old_key.pending_cleanup_api_key_ids.clone();
    if !retired_api_key_ids.contains(&old_api_key_id) {
        retired_api_key_ids.push(old_api_key_id.clone());
    }

    let endpoint = client
        .get_query_endpoint_for_binding(org_id, service_id)
        .await
        .map_err(|error| error.at_stage(FailureStage::EndpointGet))?;
    let endpoint_state = inspect_repair_endpoint(endpoint, &expected_endpoint_id, service_id)
        .map_err(|error| error.at_stage(FailureStage::EndpointGet))?;

    let key_request = build_query_api_key_request(&old_key.service_name);
    let key_response = client
        .create_api_key(org_id, &key_request)
        .await
        .map_err(|error| error.at_stage(FailureStage::KeyCreate))?;
    let new_api_key_id = require_field(key_response.key.as_ref().and_then(|key| key.id), "key.id")
        .map_err(|error| error.at_stage(FailureStage::KeyCreate))?
        .to_string();
    let (new_key_id, new_key_secret) = match require_credential_pair(&key_response) {
        Ok(pair) => pair,
        Err(error) => {
            let error = error.at_stage(FailureStage::KeyCreate);
            return fail_after_key_creation(client, org_id, &new_api_key_id, error).await;
        }
    };

    // One upsert does all the endpoint work: the retired keys leave
    // `openApiKeys` and the replacement joins it. The freshly created key may
    // not be visible to the endpoint service yet, so the upsert waits that
    // out (#658). If it still fails, the original bindings are restored and
    // nothing has been retired.
    let endpoint_request =
        build_repair_endpoint_request(&endpoint_state, &retired_api_key_ids, &new_api_key_id);
    let rollback = RepairRollback {
        new_api_key_id: &new_api_key_id,
        endpoint_state: &endpoint_state,
        old_api_key_id: &old_api_key_id,
        lock: &provisioning_lock,
    };
    let repaired_endpoint = match client
        .bind_created_query_key(org_id, service_id, &endpoint_request, KEY_PROPAGATION)
        .await
    {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return fail_after_repair_binding(
                client,
                org_id,
                service_id,
                &rollback,
                error.at_stage(FailureStage::EndpointUpsert),
            )
            .await;
        }
    };
    // The upsert answered, but unusably: both the absent id and a foreign id
    // below are failures of the `endpoint_upsert` boundary (#450).
    let repaired_endpoint_id = match require_field(repaired_endpoint.id, "query endpoint id") {
        Ok(endpoint_id) => endpoint_id,
        Err(error) => {
            return fail_after_repair_binding(
                client,
                org_id,
                service_id,
                &rollback,
                error.at_stage(FailureStage::EndpointUpsert),
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
            &rollback,
            CloudError::new(format!(
                "query endpoint repair returned endpoint {repaired_endpoint_id}, expected owned \
                 endpoint {endpoint_id}"
            ))
            .at_stage(FailureStage::EndpointUpsert),
        )
        .await;
    }

    // Commit the replacement with every retired ID still listed as pending
    // *before* deleting anything: from here on the only copy of each retired
    // ID is on disk, so a crash or a failed delete can never lose one.
    let mut replacement = build_service_query_key(
        org_id,
        &new_api_key_id,
        new_key_id,
        new_key_secret,
        Some(repaired_endpoint_id.clone()),
        &old_key.service_name,
        Utc::now(),
    );
    replacement.pending_cleanup_api_key_ids = retired_api_key_ids.clone();
    if let Err(error) =
        credentials::set_service_query_key(service_id, replacement.clone(), &provisioning_lock)
    {
        return fail_after_repair_binding(client, org_id, service_id, &rollback, error).await;
    }

    // The replacement is the credential of record and unbound keys grant
    // nothing, so deleting the retired keys is best effort: a failure keeps
    // the ID pending, is reported, and is retried by the next query (#527).
    let outcome = delete_retired_query_keys(client, org_id, &retired_api_key_ids).await;
    let pending_cleanup_api_key_ids = outcome.failed_ids();
    replacement.pending_cleanup_api_key_ids = pending_cleanup_api_key_ids.clone();
    credentials::set_service_query_key(service_id, replacement, &provisioning_lock).map_err(
        |mut error| {
            error.message = format!(
                "the query key for service {service_id} was repaired (new API key \
                 {new_api_key_id}), but the credentials file could not be updated after \
                 cleanup: {}. The retired key IDs stay listed as pending and are retried on the \
                 next query",
                error.message
            );
            error
        },
    )?;
    let cleanup_warning = (!outcome.failed.is_empty())
        .then(|| pending_cleanup_warning(service_id, org_id, &outcome.failed));
    Ok(QueryKeyRepairResult {
        status: "repaired",
        service_id: service_id.to_string(),
        organization_id: org_id.to_string(),
        replaced_api_key_id: Some(old_api_key_id),
        api_key_id: new_api_key_id,
        endpoint_id: repaired_endpoint_id,
        deleted_api_key_ids: outcome.deleted,
        pending_cleanup_api_key_ids,
        cleanup_warning,
        verification: None,
    })
}

// ── retired key cleanup (issue #527) ────────────────────────────────────────
//
// A repair retires the superseded key: the endpoint upsert unbinds it and the
// key is then deleted. Deletion is best effort. The retired ID is written to
// the record's `pending_cleanup_api_key_ids` before the delete is attempted,
// so a failure never loses the only copy of the ID; the next query for the
// service retries the deletion quietly, and `cloud service delete` deletes
// pending retirements alongside the current key. Only IDs taken from the
// stored record are ever deleted.

/// The result of one best-effort pass over retired owned key IDs.
struct RetirementOutcome {
    /// Keys that are now gone (deleted, or already absent).
    deleted: Vec<String>,
    /// Keys whose deletion failed, with the failure. Their IDs stay pending.
    failed: Vec<(String, CloudError)>,
}

impl RetirementOutcome {
    fn failed_ids(&self) -> Vec<String> {
        self.failed.iter().map(|(id, _)| id.clone()).collect()
    }
}

/// Delete each retired key, continuing past failures so one unreachable key
/// does not leave the others behind. A key that is already gone counts as
/// deleted: the point is that it no longer exists.
async fn delete_retired_query_keys(
    client: &CloudClient,
    org_id: &str,
    api_key_ids: &[String],
) -> RetirementOutcome {
    let mut outcome = RetirementOutcome {
        deleted: vec![],
        failed: vec![],
    };
    for api_key_id in api_key_ids {
        match client
            .delete_api_key_if_exists(org_id, api_key_id)
            .await
            .map_err(|error| error.at_stage(FailureStage::KeyDelete))
        {
            Ok(_) => outcome.deleted.push(api_key_id.clone()),
            Err(error) => outcome.failed.push((api_key_id.clone(), error)),
        }
    }
    outcome
}

/// The warning for retired keys that could not be deleted: names every key
/// ID and failure, where the IDs are kept, and how the deletion is retried.
fn pending_cleanup_warning(
    service_id: &str,
    org_id: &str,
    failed: &[(String, CloudError)],
) -> String {
    let failures = failed
        .iter()
        .map(|(api_key_id, error)| format!("{api_key_id}: {error}"))
        .collect::<Vec<_>>()
        .join("; ");
    let noun = if failed.len() == 1 {
        "superseded API key"
    } else {
        "superseded API keys"
    };
    format!(
        "Warning: the query key for service {service_id} is active, but the {noun} could not be \
         deleted ({failures}). The exact key IDs remain in .clickhouse/credentials.json under \
         service_query_keys.{service_id}.pending_cleanup_api_key_ids and deletion is retried \
         automatically by the next `clickhousectl cloud service query --id {service_id} --org-id \
         {org_id} ...`. To delete a key now, run `clickhousectl cloud key delete <key-id> --org-id \
         {org_id}`"
    )
}

/// Retry the pending retirements of the stored key for `service_id` before a
/// query runs (#527). Quiet on success; a failure is a warning on stderr and
/// never a query failure, and every undeleted ID stays stored. A write failure
/// on stderr is discarded, so a closed stderr cannot stop the query (#598).
///
/// The record is re-read under the provisioning lock, and the list is edited
/// only while the record still names `stored`'s active key: a concurrent
/// repair's fresh record, and its list, are not this run's to touch. The
/// active key itself is never deleted, whatever the list says.
pub(crate) async fn retry_pending_query_key_cleanup(
    client: &CloudClient,
    stored: &ServiceQueryKey,
    service_id: &str,
    org_id: &str,
) {
    if stored.pending_cleanup_api_key_ids.is_empty() {
        return;
    }
    match retire_pending_query_keys(client, stored, service_id, org_id).await {
        Ok(None) => {}
        Ok(Some(warning)) => eprint_line(warning),
        Err(error) => eprint_line(format!(
            "Warning: could not retry the deletion of superseded query API keys for service \
             {service_id}: {error}. Their IDs remain stored and the retry runs again on the \
             next query"
        )),
    }
}

async fn retire_pending_query_keys(
    client: &CloudClient,
    stored: &ServiceQueryKey,
    service_id: &str,
    org_id: &str,
) -> CloudResult<Option<String>> {
    let Some(active_api_key_id) = stored.api_key_id.as_deref() else {
        return Ok(Some(format!(
            "Warning: the stored query key for service {service_id} lists superseded API keys \
             awaiting deletion ({}) but names no active management key, so the list cannot be \
             reconciled safely and was left alone",
            stored.pending_cleanup_api_key_ids.join(", ")
        )));
    };
    // The keys live in the organization that provisioned them.
    let org_id = stored.organization_id.as_deref().unwrap_or(org_id);

    let lock = credentials::lock_query_provisioning().await?;
    let Some(current) = credentials::try_get_service_query_key(service_id)? else {
        return Ok(None);
    };
    if current.api_key_id.as_deref() != Some(active_api_key_id) {
        return Ok(None);
    }
    let retired: Vec<String> = current
        .pending_cleanup_api_key_ids
        .iter()
        .filter(|pending| pending.as_str() != active_api_key_id)
        .cloned()
        .collect();
    if retired.is_empty() {
        return Ok(None);
    }

    let outcome = delete_retired_query_keys(client, org_id, &retired).await;
    if !outcome.deleted.is_empty() {
        credentials::remove_pending_cleanup_if_api_key_matches(
            service_id,
            active_api_key_id,
            &outcome.deleted,
            &lock,
        )?;
    }
    Ok((!outcome.failed.is_empty())
        .then(|| pending_cleanup_warning(service_id, org_id, &outcome.failed)))
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
            // The cleanup failure is secondary: the classification stays the
            // one of the failure that triggered the rollback.
            ..provisioning_error
        }),
    }
}

// ── rejected stored key classification (issue #528) ─────────────────────────
//
// A Query API 401/403 for a stored per-service key does not say *why* the key
// was rejected. The local secret may be stale (the key was deleted), but an
// administrator may equally have disabled the key, let it expire, unbound it
// from the endpoint, or narrowed its IP access list. Replacing the key would
// undo every one of those decisions, so before anything is touched the key's
// management record and the endpoint binding are read and the rejection is
// classified. Only a key that no longer exists makes the local record
// disposable; every other verdict keeps the record, changes nothing, and
// names the explicit `repair-query-key` command as the deliberate way to
// replace the credential.

/// The explicit, deliberate replacement command for one service credential.
fn repair_command(service_id: &str, org_id: &str) -> String {
    format!("clickhousectl cloud service repair-query-key {service_id} --org-id {org_id}")
}

/// What the management API says about the stored key itself.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyRecordState {
    /// `GET /keys/{id}` returned 404: the key is gone. The stored secret can
    /// no longer work, and the key's UUID is still listed on the endpoint;
    /// `repair-query-key` replaces the one and drops the other.
    Deleted,
    /// The key exists with `state: disabled`.
    Disabled,
    /// The key exists and its `expireAt` has passed.
    Expired { expired_at: DateTime<Utc> },
    /// Enabled and unexpired; whether it is still bound and allowed is the
    /// endpoint's and the allowlist's business. Carries the CIDRs (never a
    /// secret) for the eventual message.
    Active { ip_access_list: Vec<String> },
}

fn classify_key_record(key: Option<&ApiKey>, now: DateTime<Utc>) -> KeyRecordState {
    let Some(key) = key else {
        return KeyRecordState::Deleted;
    };
    if matches!(key.state, Some(ApiKeyState::Disabled)) {
        return KeyRecordState::Disabled;
    }
    if let Some(expired_at) = key.expire_at
        && expired_at <= now
    {
        return KeyRecordState::Expired { expired_at };
    }
    KeyRecordState::Active {
        ip_access_list: key
            .ip_access_list
            .iter()
            .flatten()
            .filter_map(|entry| entry.source.clone())
            .collect(),
    }
}

/// The verdict on a rejected stored key.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RejectedKeyReason {
    /// The organization no longer has the key. Like every other verdict this
    /// changes nothing: the record stays, and `repair-query-key` is the one
    /// path that replaces the key and drops its stale endpoint binding.
    Deleted,
    Disabled,
    Expired {
        expired_at: DateTime<Utc>,
    },
    /// Enabled, but the endpoint's `openApiKeys` does not list the key.
    /// `endpoint_id` is `None` when the service has no query endpoint at all.
    Unbound {
        endpoint_id: Option<String>,
    },
    /// Enabled, unexpired and bound, yet rejected: the IP access list or the
    /// local secret is the likely cause. Neither can be told apart from here.
    Rejected {
        ip_access_list: Vec<String>,
    },
}

/// Which lookup failed when the rejection could not be classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lookup {
    Key,
    Endpoint,
}

fn classify_binding(
    endpoint: Option<&ServiceQueryAPIEndpoint>,
    api_key_id: &str,
    ip_access_list: Vec<String>,
) -> CloudResult<RejectedKeyReason> {
    let Some(endpoint) = endpoint else {
        return Ok(RejectedKeyReason::Unbound { endpoint_id: None });
    };
    // An absent `openApiKeys` is not an empty one: whether the key is bound is
    // simply unknown, and the verdict must say so rather than guess.
    let bound = endpoint.open_api_keys.as_ref().ok_or_else(|| {
        CloudError::new(
            "the query endpoint response is missing field 'openApiKeys', so whether the key is \
             still bound is unknown",
        )
    })?;
    if bound.iter().any(|key_id| key_id == api_key_id) {
        Ok(RejectedKeyReason::Rejected { ip_access_list })
    } else {
        Ok(RejectedKeyReason::Unbound {
            endpoint_id: endpoint.id.clone(),
        })
    }
}

/// Everything a rejection message names: resource IDs and the HTTP status,
/// never a credential.
#[derive(Debug, Clone, Copy)]
struct RejectedKeyContext<'a> {
    service_id: &'a str,
    org_id: &'a str,
    api_key_id: &'a str,
    status: u16,
}

impl RejectedKeyContext<'_> {
    fn prefix(&self) -> String {
        format!(
            "the stored Query API key for service {} was rejected with HTTP {}",
            self.service_id, self.status
        )
    }

    fn repair(&self) -> String {
        repair_command(self.service_id, self.org_id)
    }

    /// The failure this error stands for is the Query API's own 401/403; the
    /// management lookups that produced the verdict succeeded.
    fn failure(&self) -> ApiFailure {
        ApiFailure::with_status(FailureKind::Http4xx, self.status)
    }

    fn detail(
        &self,
        code: CloudErrorCode,
        message: &str,
        command: Option<String>,
        ip_access_list: Option<Vec<String>>,
    ) -> CloudErrorDetail {
        CloudErrorDetail {
            code,
            message: message.to_string(),
            host: None,
            port: None,
            command,
            api_key_id: Some(self.api_key_id.to_string()),
            ip_access_list,
        }
    }
}

fn rejected_query_key_error(reason: &RejectedKeyReason, ctx: RejectedKeyContext<'_>) -> CloudError {
    let prefix = ctx.prefix();
    let key = ctx.api_key_id;
    let repair = ctx.repair();
    let (code, message, command, ip_access_list) = match reason {
        RejectedKeyReason::Deleted => (
            CloudErrorCode::QueryKeyDeleted,
            format!(
                "{prefix}: management API key {key} no longer exists in organization {}, so \
                 the stored credential can no longer work. It was kept and no replacement was \
                 created; the key's UUID is also still listed on the service's Query API \
                 endpoint. Nothing was changed. Replace the key and clean up that binding \
                 with\n  {repair}\nthen rerun the query",
                ctx.org_id
            ),
            repair,
            None,
        ),
        RejectedKeyReason::Disabled => (
            CloudErrorCode::QueryKeyDisabled,
            format!(
                "{prefix}: management API key {key} is disabled. Disabling a key is an \
                 access-control decision, so the stored credential was kept and no replacement \
                 was created. Re-enable the key with\n  clickhousectl cloud key update {key} \
                 --state enabled --org-id {}\nor replace it deliberately with\n  {repair}",
                ctx.org_id
            ),
            repair,
            None,
        ),
        RejectedKeyReason::Expired { expired_at } => (
            CloudErrorCode::QueryKeyExpired,
            format!(
                "{prefix}: management API key {key} expired at {}. An expired key is never \
                 replaced automatically, so the stored credential was kept and no replacement \
                 was created. Replace it deliberately with\n  {repair}",
                expired_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            ),
            repair,
            None,
        ),
        RejectedKeyReason::Unbound { endpoint_id } => {
            let binding = match endpoint_id {
                Some(endpoint_id) => format!(
                    "is enabled but is not bound to Query API endpoint {endpoint_id} of this \
                     service: its openApiKeys does not list the key"
                ),
                None => "is enabled but this service has no Query API endpoint, so nothing is \
                         bound to it"
                    .to_string(),
            };
            (
                CloudErrorCode::QueryKeyUnbound,
                format!(
                    "{prefix}: management API key {key} {binding}. Unbinding a key is an \
                     access-control decision, so the stored credential was kept and no \
                     replacement was created. Inspect the endpoint with\n  clickhousectl cloud \
                     service query-endpoint get {} --org-id {}\nor replace the key (and \
                     recreate its binding) deliberately with\n  {repair}",
                    ctx.service_id, ctx.org_id
                ),
                repair,
                None,
            )
        }
        RejectedKeyReason::Rejected { ip_access_list } => {
            let allowlist = if ip_access_list.is_empty() {
                "empty".to_string()
            } else {
                ip_access_list.join(", ")
            };
            (
                CloudErrorCode::QueryKeyRejected,
                format!(
                    "{prefix}: management API key {key} is enabled, unexpired and bound to the \
                     Query API endpoint, yet the Query API still rejects it. Either the key's IP \
                     access list ({allowlist}) does not cover this machine, or the stored \
                     secret no longer matches the key. Nothing was changed. Allow this machine \
                     in the key's IP access list (`clickhousectl cloud key update {key} \
                     --ip-allow <cidr> --org-id {}` replaces the whole list), or, if the stored \
                     secret is wrong, replace the key deliberately with\n  {repair}",
                    ctx.org_id
                ),
                repair,
                Some(ip_access_list.clone()),
            )
        }
    };
    CloudError::new(message.clone())
        .with_failure(ctx.failure())
        .with_details(ctx.detail(code, &message, Some(command), ip_access_list))
}

/// The lookup that would have classified the rejection failed, so the
/// rejection stays ambiguous. Nothing local or remote is touched. The
/// classification is the lookup failure's own (a 5xx, a transport error, a
/// management-side 403), carried across the rewrite (#450); the exit code is
/// the ordinary `1`, because the remedy is not to re-authenticate the CLI.
fn unverified_query_key_error(
    ctx: RejectedKeyContext<'_>,
    lookup: Lookup,
    cause: CloudError,
) -> CloudError {
    let what = match lookup {
        Lookup::Key => format!("management API key {}", ctx.api_key_id),
        Lookup::Endpoint => format!(
            "the Query API endpoint binding of service {}",
            ctx.service_id
        ),
    };
    let repair = ctx.repair();
    let message = format!(
        "{}, and {what} could not be read to tell a stale credential from an intentional \
         revocation: {cause}. Nothing was changed. Retry the query once the management API is \
         reachable; to replace the key regardless, run\n  {repair}",
        ctx.prefix()
    );
    CloudError {
        message: message.clone(),
        kind: CloudErrorKind::Generic,
        details: Some(Box::new(ctx.detail(
            CloudErrorCode::QueryKeyUnverified,
            &message,
            None,
            None,
        ))),
        ..cause
    }
}

/// A record written before key-ownership metadata existed names no
/// management key, so there is nothing to look up and nothing safe to remove.
fn legacy_query_key_error(service_id: &str, status: u16) -> CloudError {
    let message = format!(
        "the stored Query API key for service {service_id} was rejected with HTTP {status}, and \
         the stored record predates key-ownership metadata, so the key cannot be identified or \
         verified. Nothing was changed. Remove the `service_query_keys.{service_id}` entry from \
         .clickhouse/credentials.json to provision a fresh key on the next query, and delete the \
         old key in the ClickHouse Cloud console if it still exists"
    );
    CloudError::new(message.clone())
        .with_failure(ApiFailure::with_status(FailureKind::Http4xx, status))
        .with_details(CloudErrorDetail {
            code: CloudErrorCode::QueryKeyUnverified,
            message,
            host: None,
            port: None,
            command: None,
            api_key_id: None,
            ip_access_list: None,
        })
}

/// The error to report for a stored key the Query API rejected with `status`
/// (401/403), after classifying the rejection against the key's management
/// record and the endpoint binding.
///
/// No path here mutates anything: every verdict, and every lookup failure,
/// leaves the credentials file and the organization exactly as they were. The
/// management lookups exist only so the error can say what happened and name
/// the one deliberate way forward, `repair-query-key`.
pub(crate) async fn rejected_stored_query_key_error(
    client: &CloudClient,
    stored: &ServiceQueryKey,
    service_id: &str,
    org_id: &str,
    status: u16,
) -> CloudError {
    let Some(api_key_id) = stored.api_key_id.as_deref() else {
        return legacy_query_key_error(service_id, status);
    };
    // The key lives in the organization that provisioned it, which is the
    // stored one whenever the record has it.
    let org_id = stored.organization_id.as_deref().unwrap_or(org_id);
    let ctx = RejectedKeyContext {
        service_id,
        org_id,
        api_key_id,
        status,
    };

    let key = match client.get_api_key_if_exists(org_id, api_key_id).await {
        Ok(key) => key,
        Err(cause) => {
            return unverified_query_key_error(ctx, Lookup::Key, cause)
                .at_stage(FailureStage::KeyGet);
        }
    };
    let reason = match classify_key_record(key.as_ref(), Utc::now()) {
        KeyRecordState::Deleted => RejectedKeyReason::Deleted,
        KeyRecordState::Disabled => RejectedKeyReason::Disabled,
        KeyRecordState::Expired { expired_at } => RejectedKeyReason::Expired { expired_at },
        KeyRecordState::Active { ip_access_list } => {
            let endpoint = match client
                .get_query_endpoint_for_binding(org_id, service_id)
                .await
            {
                Ok(endpoint) => endpoint,
                Err(cause) => {
                    return unverified_query_key_error(ctx, Lookup::Endpoint, cause)
                        .at_stage(FailureStage::EndpointGet);
                }
            };
            match classify_binding(endpoint.as_ref(), api_key_id, ip_access_list) {
                Ok(reason) => reason,
                Err(cause) => {
                    return unverified_query_key_error(ctx, Lookup::Endpoint, cause)
                        .at_stage(FailureStage::EndpointGet);
                }
            }
        }
    };
    rejected_query_key_error(&reason, ctx)
}

/// The keys already bound to an existing query endpoint, taken from a
/// successful GET. The upsert replaces `openApiKeys` wholesale, so an absent
/// `openApiKeys` cannot be read as "no keys bound": merging into an empty list
/// would revoke every binding the response failed to report. An explicitly
/// empty list is a real answer and merges normally.
pub(crate) fn existing_open_api_keys(
    endpoint: ServiceQueryAPIEndpoint,
) -> CloudResult<Vec<String>> {
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
        .await
        .map_err(|error| error.at_stage(FailureStage::EndpointGet))?
    {
        Some(endpoint) => existing_open_api_keys(endpoint)
            .map_err(|error| error.at_stage(FailureStage::EndpointGet))?,
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
        .bind_created_query_key(org_id, service_id, &endpoint_request, KEY_PROPAGATION)
        .await
        .map_err(|error| error.at_stage(FailureStage::EndpointUpsert))
}

impl CloudClient {
    /// Upsert the endpoint configuration that binds a key created by this
    /// invocation, waiting out key propagation (#658). The caller owns the
    /// key it just created; everything else about the request is its business.
    /// Rollbacks and unbinds use [`Self::create_query_endpoint`] directly:
    /// they bind no fresh key, so there is nothing to wait for.
    pub(crate) async fn bind_created_query_key(
        &self,
        org_id: &str,
        service_id: &str,
        request: &InstanceServiceQueryApiEndpointsPostRequest,
        propagation: KeyPropagation,
    ) -> CloudResult<ServiceQueryAPIEndpoint> {
        let response = upsert_until_key_propagates(propagation, || {
            self.api()
                .instance_query_endpoint_upsert(org_id, service_id, request)
        })
        .await
        .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    /// The management record of one API key, or `None` when the organization
    /// no longer has it. Only a 404 means "gone": any other failure is
    /// reported, because it says nothing about the key.
    async fn get_api_key_if_exists(
        &self,
        org_id: &str,
        api_key_id: &str,
    ) -> CloudResult<Option<ApiKey>> {
        match self.api().openapi_key_get(org_id, api_key_id).await {
            Ok(response) => Self::unwrap_response(response).map(Some),
            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(None),
            Err(error) => Err(self.convert_error_for_organization(error, org_id)),
        }
    }

    pub(crate) async fn get_query_endpoint_for_binding(
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

    // ── key propagation wait (issue #658) ───────────────────────────────

    /// A policy that finishes within a test: real sleeps of a few
    /// milliseconds, so the loop's own timing is what is exercised.
    const FAST_PROPAGATION: KeyPropagation = KeyPropagation {
        timeout: Duration::from_millis(60),
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(4),
    };

    fn propagation_400() -> clickhouse_cloud_api::Error {
        clickhouse_cloud_api::Error::Api {
            status: 400,
            message: "OpenAPI key aaaa does not belong to the organization".into(),
        }
    }

    #[test]
    fn only_a_400_counts_as_key_propagation() {
        assert!(key_propagation_error(&propagation_400()));
        // Any 400 qualifies: the ID in the message varies, so the message is
        // not what is matched.
        assert!(key_propagation_error(&clickhouse_cloud_api::Error::Api {
            status: 400,
            message: "invalid roles".into(),
        }));
        for status in [401, 403, 404, 409, 422, 429, 500, 503] {
            assert!(
                !key_propagation_error(&clickhouse_cloud_api::Error::Api {
                    status,
                    message: "OpenAPI key aaaa does not belong to the organization".into(),
                }),
                "{status} is not a propagation failure"
            );
        }
        assert!(!key_propagation_error(
            &clickhouse_cloud_api::Error::AuthMismatch("read-only".into())
        ));
        assert!(!key_propagation_error(&clickhouse_cloud_api::Error::Sql {
            status: 400,
            code: "62".into(),
            details: "syntax".into(),
        }));
    }

    #[tokio::test]
    async fn the_upsert_is_retried_on_400_and_stops_at_the_first_success() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&attempts);
        let result = upsert_until_key_propagates(FAST_PROPAGATION, move || {
            let attempt = counter.fetch_add(1, Ordering::SeqCst);
            std::future::ready(if attempt < 2 {
                Err(propagation_400())
            } else {
                Ok("bound")
            })
        })
        .await;
        assert_eq!(result.unwrap(), "bound");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_non_400_failure_is_not_retried() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        for status in [401, 403, 404, 500] {
            let attempts = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&attempts);
            let result = upsert_until_key_propagates(FAST_PROPAGATION, move || {
                counter.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Err::<(), _>(clickhouse_cloud_api::Error::Api {
                    status,
                    message: "no".into(),
                }))
            })
            .await;
            assert!(
                matches!(result, Err(clickhouse_cloud_api::Error::Api { status: s, .. }) if s == status)
            );
            assert_eq!(attempts.load(Ordering::SeqCst), 1, "{status} was retried");
        }
    }

    #[tokio::test]
    async fn a_persisting_400_fails_with_the_last_error_once_the_deadline_passes() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&attempts);
        let started = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            upsert_until_key_propagates(FAST_PROPAGATION, move || {
                let attempt = counter.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Err::<(), _>(clickhouse_cloud_api::Error::Api {
                    status: 400,
                    message: format!("attempt {attempt}"),
                }))
            }),
        )
        .await
        .expect("the deadline must bound the wait");
        let attempts = attempts.load(Ordering::SeqCst);
        assert!(attempts >= 2, "at least one retry before the deadline");
        assert!(started.elapsed() >= FAST_PROPAGATION.timeout);
        // The error is the last attempt's, not a made-up timeout: the upsert
        // itself said no, and that is what the user must see.
        match result {
            Err(clickhouse_cloud_api::Error::Api { status, message }) => {
                assert_eq!(status, 400);
                assert_eq!(message, format!("attempt {}", attempts - 1));
            }
            other => panic!("expected the last 400, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn the_backoff_is_capped_and_never_sleeps_past_the_deadline() {
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        };
        let propagation = KeyPropagation {
            timeout: Duration::from_millis(40),
            initial_backoff: Duration::from_millis(5),
            max_backoff: Duration::from_millis(10),
        };
        let attempts = Arc::new(AtomicUsize::new(0));
        let stamps = Arc::new(Mutex::new(Vec::new()));
        let counter = Arc::clone(&attempts);
        let recorder = Arc::clone(&stamps);
        let started = std::time::Instant::now();
        let _ = upsert_until_key_propagates(propagation, move || {
            counter.fetch_add(1, Ordering::SeqCst);
            recorder.lock().unwrap().push(started.elapsed());
            std::future::ready(Err::<(), _>(propagation_400()))
        })
        .await;
        let stamps = stamps.lock().unwrap();
        // Sleeps of 5, 10, 10, 10, ... ms: at most one attempt per capped
        // backoff, and the last one lands at or before the deadline.
        assert!(stamps.len() >= 3, "{stamps:?}");
        assert!(
            stamps.last().unwrap() <= &(propagation.timeout + Duration::from_millis(20)),
            "{stamps:?}"
        );
        for pair in stamps.windows(2) {
            assert!(
                pair[1] - pair[0] <= propagation.max_backoff + Duration::from_millis(20),
                "{stamps:?}"
            );
        }
    }

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

        let request = build_repair_endpoint_request(&state, &["old-key".to_string()], "new-key");
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
        let request = build_repair_endpoint_request(&state, &["old-key".to_string()], "new-key");
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

    #[test]
    fn repair_detects_a_concurrent_winner() {
        let expected = ServiceQueryKey {
            organization_id: Some("org-1".into()),
            api_key_id: Some("stale-key".into()),
            key_id: "stale-id".into(),
            key_secret: "stale-secret".into(),
            endpoint_id: Some("endpoint-1".into()),
            pending_cleanup_api_key_ids: vec![],
            service_name: "demo".into(),
            created_at: Utc::now(),
        };
        let mut winner = expected.clone();
        winner.api_key_id = Some("winner-key".into());
        winner.key_id = "winner-id".into();
        winner.key_secret = "winner-secret".into();

        assert!(repair_state_changed(&expected, &winner));
        assert!(!repair_state_changed(&winner, &winner));
    }

    // ── rejected stored key classification (issue #528) ─────────────

    use clickhouse_cloud_api::models::IpAccessListEntryResponse;

    fn at(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn api_key(
        state: Option<ApiKeyState>,
        expire_at: Option<DateTime<Utc>>,
        cidrs: Option<&[&str]>,
    ) -> ApiKey {
        ApiKey {
            state,
            expire_at,
            ip_access_list: cidrs.map(|cidrs| {
                cidrs
                    .iter()
                    .map(|cidr| IpAccessListEntryResponse {
                        source: Some(cidr.to_string()),
                        description: None,
                    })
                    .collect()
            }),
            ..ApiKey::default()
        }
    }

    const NOW: &str = "2026-09-02T12:00:00Z";

    fn ctx() -> RejectedKeyContext<'static> {
        RejectedKeyContext {
            service_id: "svc-1",
            org_id: "org-1",
            api_key_id: "key-1",
            status: 401,
        }
    }

    #[test]
    fn a_missing_key_record_is_deleted() {
        assert_eq!(classify_key_record(None, at(NOW)), KeyRecordState::Deleted);
    }

    #[test]
    fn a_disabled_key_is_disabled_even_when_it_has_also_expired() {
        let key = api_key(
            Some(ApiKeyState::Disabled),
            Some(at("2026-01-01T00:00:00Z")),
            Some(&["0.0.0.0/0"]),
        );
        assert_eq!(
            classify_key_record(Some(&key), at(NOW)),
            KeyRecordState::Disabled
        );
    }

    #[test]
    fn an_expiry_at_or_before_now_is_expired_and_a_later_one_is_not() {
        let expired = api_key(Some(ApiKeyState::Enabled), Some(at(NOW)), None);
        assert_eq!(
            classify_key_record(Some(&expired), at(NOW)),
            KeyRecordState::Expired {
                expired_at: at(NOW)
            }
        );
        let live = api_key(
            Some(ApiKeyState::Enabled),
            Some(at("2027-01-01T00:00:00Z")),
            Some(&["10.0.0.0/8"]),
        );
        assert_eq!(
            classify_key_record(Some(&live), at(NOW)),
            KeyRecordState::Active {
                ip_access_list: vec!["10.0.0.0/8".into()]
            }
        );
    }

    #[test]
    fn an_active_key_keeps_only_the_cidrs_and_tolerates_absent_fields() {
        // Absent `state`, absent `expireAt`, absent `ipAccessList`: nothing is
        // fabricated, and none of it makes the key look revoked.
        let bare = api_key(None, None, None);
        assert_eq!(
            classify_key_record(Some(&bare), at(NOW)),
            KeyRecordState::Active {
                ip_access_list: vec![]
            }
        );
        // An entry without a `source` is skipped rather than rendered empty.
        let mut partial = api_key(Some(ApiKeyState::Enabled), None, Some(&["1.2.3.4/32"]));
        partial
            .ip_access_list
            .get_or_insert_with(Vec::new)
            .push(IpAccessListEntryResponse {
                source: None,
                description: Some("no source".into()),
            });
        assert_eq!(
            classify_key_record(Some(&partial), at(NOW)),
            KeyRecordState::Active {
                ip_access_list: vec!["1.2.3.4/32".into()]
            }
        );
    }

    #[test]
    fn a_service_without_an_endpoint_leaves_the_key_unbound() {
        assert_eq!(
            classify_binding(None, "key-1", vec![]).unwrap(),
            RejectedKeyReason::Unbound { endpoint_id: None }
        );
    }

    #[test]
    fn an_endpoint_that_omits_the_key_is_unbound_and_names_the_endpoint() {
        let endpoint = ServiceQueryAPIEndpoint {
            id: Some("ep-1".into()),
            open_api_keys: Some(vec!["other-key".into()]),
            roles: None,
            allowed_origins: None,
        };
        assert_eq!(
            classify_binding(Some(&endpoint), "key-1", vec![]).unwrap(),
            RejectedKeyReason::Unbound {
                endpoint_id: Some("ep-1".into())
            }
        );
    }

    #[test]
    fn an_endpoint_that_lists_the_key_is_rejected_with_the_allowlist() {
        let endpoint = ServiceQueryAPIEndpoint {
            id: Some("ep-1".into()),
            open_api_keys: Some(vec!["other-key".into(), "key-1".into()]),
            roles: None,
            allowed_origins: None,
        };
        assert_eq!(
            classify_binding(Some(&endpoint), "key-1", vec!["203.0.113.0/24".into()]).unwrap(),
            RejectedKeyReason::Rejected {
                ip_access_list: vec!["203.0.113.0/24".into()]
            }
        );
    }

    #[test]
    fn an_endpoint_without_open_api_keys_cannot_classify_the_binding() {
        let endpoint = ServiceQueryAPIEndpoint {
            id: Some("ep-1".into()),
            open_api_keys: None,
            roles: None,
            allowed_origins: None,
        };
        let error = classify_binding(Some(&endpoint), "key-1", vec![]).unwrap_err();
        assert!(error.to_string().contains("'openApiKeys'"), "{error}");
    }

    #[test]
    fn every_verdict_has_a_stable_code_and_the_deliberate_repair_path() {
        let cases = [
            (
                RejectedKeyReason::Deleted,
                CloudErrorCode::QueryKeyDeleted,
                "no longer exists in organization org-1",
            ),
            (
                RejectedKeyReason::Disabled,
                CloudErrorCode::QueryKeyDisabled,
                "clickhousectl cloud key update key-1 --state enabled --org-id org-1",
            ),
            (
                RejectedKeyReason::Expired {
                    expired_at: at("2026-08-01T00:00:00Z"),
                },
                CloudErrorCode::QueryKeyExpired,
                "expired at 2026-08-01T00:00:00Z",
            ),
            (
                RejectedKeyReason::Unbound {
                    endpoint_id: Some("ep-1".into()),
                },
                CloudErrorCode::QueryKeyUnbound,
                "not bound to Query API endpoint ep-1",
            ),
            (
                RejectedKeyReason::Unbound { endpoint_id: None },
                CloudErrorCode::QueryKeyUnbound,
                "has no Query API endpoint",
            ),
            (
                RejectedKeyReason::Rejected {
                    ip_access_list: vec!["203.0.113.0/24".into(), "198.51.100.7/32".into()],
                },
                CloudErrorCode::QueryKeyRejected,
                "IP access list (203.0.113.0/24, 198.51.100.7/32)",
            ),
            (
                RejectedKeyReason::Rejected {
                    ip_access_list: vec![],
                },
                CloudErrorCode::QueryKeyRejected,
                "IP access list (empty)",
            ),
        ];
        for (reason, code, needle) in cases {
            let error = rejected_query_key_error(&reason, ctx());
            let details = error.details.as_deref().expect("json details");
            assert_eq!(details.code, code, "{reason:?}");
            assert_eq!(details.message, error.message, "{reason:?}");
            assert_eq!(details.api_key_id.as_deref(), Some("key-1"), "{reason:?}");
            assert_eq!(
                details.command.as_deref(),
                Some("clickhousectl cloud service repair-query-key svc-1 --org-id org-1"),
                "{reason:?}"
            );
            assert!(
                error.message.contains(needle),
                "{reason:?}: {}",
                error.message
            );
            assert!(
                error.message.starts_with(
                    "the stored Query API key for service svc-1 was rejected with HTTP 401"
                ),
                "{}",
                error.message
            );
            assert!(
                error.message.contains("management API key key-1"),
                "{reason:?}: {}",
                error.message
            );
            // Every verdict says out loud that nothing was replaced.
            assert!(
                error.message.contains("no replacement was created")
                    || error.message.contains("Nothing was changed"),
                "{reason:?}: {}",
                error.message
            );
            // The failure is the Query API's own rejection (#450), and a
            // rejected *stored* key is never an auth-required exit.
            assert_eq!(
                error.failure,
                Some(ApiFailure::with_status(FailureKind::Http4xx, 401))
            );
            assert_eq!(error.kind, CloudErrorKind::Generic);
            let is_rejected = matches!(reason, RejectedKeyReason::Rejected { .. });
            assert_eq!(details.ip_access_list.is_some(), is_rejected, "{reason:?}");
        }
    }

    #[test]
    fn a_deleted_key_keeps_the_record_and_points_at_repair() {
        // The stored credential is kept even though it can never work again:
        // the record is what lets `repair-query-key` find the key's UUID and
        // drop it from the endpoint binding along with binding the
        // replacement. Rerunning the query is not the way forward, so it is
        // not the suggested command.
        let error = rejected_query_key_error(&RejectedKeyReason::Deleted, ctx());
        let details = error.details.as_deref().unwrap();
        assert_eq!(details.code, CloudErrorCode::QueryKeyDeleted);
        assert!(
            error
                .message
                .contains("It was kept and no replacement was created"),
            "{}",
            error.message
        );
        assert!(
            error
                .message
                .contains("still listed on the service's Query API endpoint"),
            "{}",
            error.message
        );
        assert!(
            !error.message.contains("removed") && !error.message.contains("Rerun the query"),
            "{}",
            error.message
        );
        assert_eq!(
            details.command.as_deref(),
            Some("clickhousectl cloud service repair-query-key svc-1 --org-id org-1")
        );
    }

    #[test]
    fn an_unverifiable_rejection_carries_the_lookup_failure_and_exits_generic() {
        let cause = CloudError::auth("FORBIDDEN")
            .with_failure(ApiFailure::with_status(FailureKind::Http4xx, 403));
        let error = unverified_query_key_error(ctx(), Lookup::Key, cause);
        assert_eq!(error.kind, CloudErrorKind::Generic);
        assert_eq!(
            error.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 403)),
            "the classification is the lookup's, carried across the rewrite"
        );
        let details = error.details.as_deref().unwrap();
        assert_eq!(details.code, CloudErrorCode::QueryKeyUnverified);
        assert_eq!(details.api_key_id.as_deref(), Some("key-1"));
        assert!(
            details.command.is_none(),
            "an ambiguous verdict pushes no write"
        );
        assert!(
            error
                .message
                .contains("management API key key-1 could not be read"),
            "{}",
            error.message
        );
        assert!(error.message.contains("FORBIDDEN"), "{}", error.message);
        assert!(
            error.message.contains("Nothing was changed"),
            "{}",
            error.message
        );

        let endpoint = unverified_query_key_error(
            ctx(),
            Lookup::Endpoint,
            CloudError::new("upstream unavailable")
                .with_failure(ApiFailure::with_status(FailureKind::Http5xx, 503)),
        );
        assert!(
            endpoint
                .message
                .contains("the Query API endpoint binding of service svc-1 could not be read"),
            "{}",
            endpoint.message
        );
        assert_eq!(
            endpoint.failure,
            Some(ApiFailure::with_status(FailureKind::Http5xx, 503))
        );
    }

    #[test]
    fn a_legacy_record_is_unverifiable_and_names_no_key() {
        let error = legacy_query_key_error("svc-1", 403);
        let details = error.details.as_deref().unwrap();
        assert_eq!(details.code, CloudErrorCode::QueryKeyUnverified);
        assert!(details.api_key_id.is_none());
        assert!(details.command.is_none());
        assert_eq!(
            error.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 403))
        );
        assert!(
            error.message.contains("service_query_keys.svc-1"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains("Nothing was changed"),
            "{}",
            error.message
        );
    }

    // ── retired key cleanup (issue #527) ─────────────────────────────

    #[test]
    fn repair_endpoint_request_unbinds_every_retired_key_and_keeps_the_rest() {
        let state = inspect_repair_endpoint(
            Some(ServiceQueryAPIEndpoint {
                id: Some("endpoint-1".into()),
                roles: Some(vec![QueryEndpointRole::SqlConsoleAdmin]),
                open_api_keys: Some(vec![
                    "other-key".into(),
                    "pending-key".into(),
                    "old-key".into(),
                ]),
                allowed_origins: Some("*".into()),
            }),
            "endpoint-1",
            "service-1",
        )
        .unwrap();

        // The superseded key and an earlier retirement still pending both go;
        // a key the record does not own stays.
        let retired = ["pending-key".to_string(), "old-key".to_string()];
        let request = build_repair_endpoint_request(&state, &retired, "new-key");
        assert_eq!(request.open_api_keys, vec!["other-key", "new-key"]);

        // A retired key the endpoint no longer lists is simply not there.
        let retired = ["absent-key".to_string(), "old-key".to_string()];
        let request = build_repair_endpoint_request(&state, &retired, "new-key");
        assert_eq!(
            request.open_api_keys,
            vec!["other-key", "pending-key", "new-key"]
        );
    }

    #[test]
    fn the_pending_cleanup_warning_names_every_key_the_record_and_the_retry() {
        let failed = vec![
            ("key-a".to_string(), CloudError::new("HTTP 500")),
            ("key-b".to_string(), CloudError::new("timed out")),
        ];
        let warning = pending_cleanup_warning("service-1", "org-1", &failed);
        assert!(warning.starts_with("Warning:"), "{warning}");
        assert!(
            warning.contains(
                "the superseded API keys could not be deleted (key-a: HTTP 500; key-b: timed out)"
            ),
            "{warning}"
        );
        assert!(
            warning.contains("service_query_keys.service-1.pending_cleanup_api_key_ids"),
            "{warning}"
        );
        assert!(
            warning.contains("clickhousectl cloud service query --id service-1 --org-id org-1"),
            "{warning}"
        );
        assert!(
            warning.contains("clickhousectl cloud key delete <key-id> --org-id org-1"),
            "{warning}"
        );

        let single = pending_cleanup_warning("service-1", "org-1", &failed[..1]);
        assert!(
            single.contains("the superseded API key could not be deleted (key-a: HTTP 500)"),
            "{single}"
        );
    }

    #[test]
    fn a_retirement_outcome_keeps_only_the_failed_ids_pending() {
        let outcome = RetirementOutcome {
            deleted: vec!["gone".into()],
            failed: vec![("stuck".into(), CloudError::new("boom"))],
        };
        assert_eq!(outcome.failed_ids(), ["stuck"]);
    }

    #[test]
    fn the_repair_result_serializes_the_ids_and_omits_the_prose_warning() {
        let result = QueryKeyRepairResult {
            status: "repaired",
            service_id: "service-1".into(),
            organization_id: "org-1".into(),
            replaced_api_key_id: Some("old".into()),
            api_key_id: "new".into(),
            endpoint_id: "ep".into(),
            deleted_api_key_ids: vec![],
            pending_cleanup_api_key_ids: vec!["old".into()],
            cleanup_warning: Some("Warning: prose".into()),
            verification: Some(RepairVerification::Verified),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["pendingCleanupApiKeyIds"], serde_json::json!(["old"]));
        assert_eq!(json["verification"], "verified");
        assert!(json.get("deletedApiKeyIds").is_none(), "{json}");
        assert!(json.get("cleanupWarning").is_none(), "{json}");
    }
}
