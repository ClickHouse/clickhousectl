//! Query API endpoint access for `cloud service query`.
//!
//! On first use the caller's own API key is bound to the service's query
//! endpoint (#1043), which is created with role `sql_console_admin` when the
//! service has none; an existing endpoint keeps its roles. No key is created
//! and nothing is written locally. Per-service keys that earlier versions
//! created and stored in `.clickhouse/credentials.json` are no longer used
//! (#1325): see [`crate::cloud::credentials::take_legacy_service_query_key`].

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::{CloudErrorCode, CloudErrorDetail};
use crate::failure::{ApiFailure, FailureKind, FailureStage, ProvisioningState};
use clickhouse_cloud_api::models::{
    ApiKey, InstanceServiceQueryApiEndpointsPostRequest, QueryEndpointRole,
    ServiceQueryAPIEndpoint, Whoami,
};
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

/// How long a key may take to become visible to the service behind the
/// query-endpoint upsert (#658).
///
/// Keys and `PUT .../serviceQueryEndpoint` are answered by different
/// services, and the second can answer `400` for a key that was created
/// moments earlier and `GET /keys/{id}` already returns. The wait is bounded
/// and exponential like the query-endpoint readiness wait in
/// `services/query.rs`. Live on 2026-09-02, 1 of 20 back-to-back binds of a
/// just-created key hit it and the first retry, one second later, succeeded; the 30 s deadline leaves ample
/// headroom. The deadline decides whether another attempt is *started*; an
/// attempt already in flight is never cut off, so a last-attempt success is
/// never discarded.
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

/// [`KEY_PROPAGATION`], with the hidden test override of its window.
fn key_propagation() -> KeyPropagation {
    KeyPropagation {
        timeout: crate::timing::duration_override(
            crate::timing::KEY_PROPAGATION_TIMEOUT_ENV,
            KEY_PROPAGATION.timeout,
        ),
        ..KEY_PROPAGATION
    }
}

/// Whether an upsert failure can be the endpoint service not yet seeing a key
/// created moments ago. Structural: the status of the typed [`Error::Api`]
/// variant, never the message text (#450). The propagation failure is a
/// `400` whose body names the key ID; because that ID varies, no fixed
/// message could tell it apart honestly, so *every* `400` inside the window
/// is retried. The request body is built by this module from the endpoint
/// configuration it just read, so a genuine `400` is rare, and it costs
/// exactly the window before it fails as it would have without the wait.
///
/// [`Error::Api`]: clickhouse_cloud_api::Error::Api
fn key_propagation_error(error: &clickhouse_cloud_api::Error) -> bool {
    matches!(error, clickhouse_cloud_api::Error::Api { status: 400, .. })
}

/// The upsert notice, printed once per run on the first retry. Stderr in every
/// output mode, like the readiness notice, so `--json` stdout stays one value.
const KEY_PROPAGATION_NOTICE: &str =
    "Waiting for the Query API endpoint to accept the API key binding...";

/// Run `upsert` until it succeeds, fails for a reason other than key
/// propagation, or `propagation.timeout` elapses. Only the upsert is repeated,
/// with the same body each time; a success is never followed by another
/// attempt. Each retry is noted for telemetry so a dashboard sees that the
/// run waited on propagation, the same way the readiness loop does (#450).
pub(super) async fn upsert_until_key_propagates<T, F, Fut>(
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
            // A closed stderr must not panic here: the caller has yet to
            // report the upsert's outcome.
            eprintln!("{KEY_PROPAGATION_NOTICE}");
            waiting = true;
        }
        crate::failure::note_retry();
        tokio::time::sleep(backoff.min(remaining)).await;
        backoff = backoff.saturating_mul(2).min(propagation.max_backoff);
    }
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

/// The configuration of the service's existing query endpoint, or `None` when
/// the service has none yet. A bind rewrites only `openApiKeys`, so `roles`
/// and `allowedOrigins` are echoed back unchanged. `roles` is required by the
/// upsert and must be known: an absent one is refused before anything is
/// written rather than replaced by a default. `allowedOrigins` is optional on
/// both sides, so an absent one is echoed back as absent. Every failure
/// belongs to the endpoint read.
async fn read_bind_endpoint(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
) -> CloudResult<Option<InstanceServiceQueryApiEndpointsPostRequest>> {
    let at_stage = |error: CloudError| error.at_stage(FailureStage::EndpointGet);
    let Some(mut endpoint) = client
        .get_query_endpoint_for_binding(org_id, service_id)
        .await
        .map_err(at_stage)?
    else {
        return Ok(None);
    };
    let roles = endpoint.roles.take();
    let allowed_origins = endpoint.allowed_origins.take();
    let open_api_keys = existing_open_api_keys(endpoint).map_err(at_stage)?;
    let roles = require_field(roles, "query endpoint roles").map_err(at_stage)?;
    Ok(Some(InstanceServiceQueryApiEndpointsPostRequest {
        roles,
        open_api_keys,
        allowed_origins,
    }))
}

/// Whether two `openApiKeys` entries name the same key: equal as UUIDs when
/// both parse (so case and hyphenation do not matter), else equal as strings.
fn same_key(a: &str, b: &str) -> bool {
    match (uuid::Uuid::parse_str(a), uuid::Uuid::parse_str(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The `openApiKeys` to upsert: the existing bindings minus those known to be
/// `gone`, without duplicates and in their original order, with `api_key_uuid`
/// appended when it is not already bound. Never drops a key it was not told is
/// gone, so the upsert cannot silently revoke another binding.
fn merged_open_api_keys(existing: Vec<String>, api_key_uuid: &str, gone: &[String]) -> Vec<String> {
    let mut merged: Vec<String> = Vec::with_capacity(existing.len() + 1);
    for key in existing {
        if !same_key(&key, api_key_uuid) && gone.contains(&key) {
            continue;
        }
        if !merged.contains(&key) {
            merged.push(key);
        }
    }
    if !merged.iter().any(|key| same_key(key, api_key_uuid)) {
        merged.push(api_key_uuid.to_string());
    }
    merged
}

/// The upsert body that binds `open_api_keys` to the endpoint. An existing
/// endpoint keeps its own `roles` and `allowedOrigins`, since roles are
/// endpoint-wide and apply to every bound key; only a new endpoint gets this
/// module's defaults.
fn build_bind_endpoint_request(
    existing: Option<&InstanceServiceQueryApiEndpointsPostRequest>,
    open_api_keys: Vec<String>,
) -> InstanceServiceQueryApiEndpointsPostRequest {
    match existing {
        Some(existing) => InstanceServiceQueryApiEndpointsPostRequest {
            roles: existing.roles.clone(),
            open_api_keys,
            allowed_origins: existing.allowed_origins.clone(),
        },
        None => InstanceServiceQueryApiEndpointsPostRequest {
            // The binding grants read/write SQL access only through this
            // service's endpoint; it does not assign an organization-level
            // role to the key.
            roles: vec![QueryEndpointRole::SqlConsoleAdmin],
            open_api_keys,
            allowed_origins: Some(ALLOWED_ORIGINS.to_string()),
        },
    }
}

/// What [`bind_caller_query_key`] did to the endpoint.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CallerKeyBinding {
    /// The caller's key was merged into the endpoint (or the endpoint was
    /// created); the binding still has to reach the Query API host.
    Bound,
    /// The endpoint already listed the caller's key and no stale binding was
    /// pruned, so nothing was written. Another process may have just bound
    /// it, so the binding may still have to reach the Query API host;
    /// otherwise rebinding cannot fix the rejection.
    AlreadyBound { caller_uuid: String },
}

/// The caller's key was already bound to the endpoint, nothing stale was
/// pruned, and the Query API kept rejecting the key for the whole readiness
/// window `waited`, so rebinding cannot help. The verdict comes from the
/// endpoint read (the caller records that stage); like any exhausted
/// readiness window it is classified as a timeout (#450).
pub(crate) fn bound_caller_key_rejected_error(
    service_id: &str,
    org_id: &str,
    caller_uuid: &str,
    waited: Duration,
) -> CloudError {
    let inspect =
        format!("clickhousectl cloud service query-endpoint get {service_id} --org-id {org_id}");
    let message = format!(
        "the authenticated API key {caller_uuid} is already bound to the Query API endpoint of \
         service {service_id}, yet the Query API still rejected it after waiting {}s. Nothing \
         was changed. Likely causes: the endpoint's roles or allowed origins, the key's IP \
         access list not covering this machine, or the key's role. Inspect the endpoint \
         with\n  {inspect}\nand the key with\n  clickhousectl cloud key get {caller_uuid} \
         --org-id {org_id}",
        waited.as_secs()
    );
    CloudError::new(message.clone())
        .with_failure(ApiFailure::new(FailureKind::Timeout))
        .with_details(CloudErrorDetail {
            code: CloudErrorCode::QueryKeyBoundRejected,
            message,
            host: None,
            port: None,
            command: Some(inspect),
            api_key_id: Some(caller_uuid.to_string()),
        })
}

/// The upsert's `403`, re-presented: a key without write access, such as a
/// read-only one, cannot bind itself to the endpoint (#1325). The status comes
/// from the classification of the typed library error, never the message, and
/// the classification and exit code (a `403` is not an auth failure) are
/// carried across; the API's own message is kept. Any other failure is
/// returned unchanged.
fn bind_forbidden_or(
    error: CloudError,
    service_id: &str,
    org_id: &str,
    caller_uuid: &str,
) -> CloudError {
    if error.failure.and_then(|failure| failure.http_status) != Some(403) {
        return error;
    }
    let inspect =
        format!("clickhousectl cloud service query-endpoint get {service_id} --org-id {org_id}");
    let message = format!(
        "the authenticated API key {caller_uuid} must be bound to the Query API endpoint of \
         service {service_id} before it can query it, and the Cloud API refused the binding \
         ({}). Nothing was changed. Binding needs write access, so bind this key with an API \
         key that has it:\n  clickhousectl cloud service query-endpoint create {service_id} \
         --role <role> --open-api-key {caller_uuid} --org-id {org_id}\n(add \
         --allowed-origins '*' when the service has no endpoint yet). --role replaces the \
         endpoint's roles for every bound key; read them first with\n  {inspect}",
        error.message
    );
    CloudError {
        message: message.clone(),
        details: Some(Box::new(CloudErrorDetail {
            code: CloudErrorCode::QueryKeyBindForbidden,
            message,
            host: None,
            port: None,
            command: Some(inspect),
            api_key_id: Some(caller_uuid.to_string()),
        })),
        ..error
    }
}

/// The resource UUID of the caller's own API key — the value `openApiKeys`
/// takes — from a `whoami` answer (#1043). Refuses anything but an API key
/// that names its UUID and belongs to `org_id`: binding a key of another
/// organization, or guessing at a user's identity, must fail before any write.
fn caller_key_uuid(identity: &Whoami, org_id: &str) -> CloudResult<String> {
    const PREFIX: &str = "cannot bind the caller's API key to the query endpoint";
    let key = match identity {
        Whoami::WhoamiApiKey(key) => key,
        Whoami::WhoamiUser(_) => {
            return Err(CloudError::new(format!(
                "{PREFIX}: the Cloud API identified the caller as a user, not an API key"
            )));
        }
        Whoami::Unknown(_) => {
            return Err(CloudError::new(format!(
                "{PREFIX}: the Cloud API identified the caller as an unrecognized actor type"
            )));
        }
    };
    let key_id = key
        .key_id
        .as_deref()
        .filter(|key_id| !key_id.is_empty())
        .ok_or_else(|| {
            CloudError::new(format!(
                "{PREFIX}: the identity response is missing field 'keyId'"
            ))
        })?;
    let key_org = key.organization_id.ok_or_else(|| {
        CloudError::new(format!(
            "{PREFIX}: the identity response is missing field 'organizationId'"
        ))
    })?;
    if uuid::Uuid::parse_str(org_id).ok() != Some(key_org) {
        return Err(CloudError::new(format!(
            "{PREFIX}: the API key belongs to organization {key_org}, not {org_id}"
        )));
    }
    Ok(key_id.to_string())
}

/// Whether a bound key's lookup proves it was deleted. Only a confirmed
/// absence counts: a lookup that failed says nothing about the key, so the
/// binding is kept rather than revoked on uncertainty.
fn bound_key_is_gone(lookup: &CloudResult<Option<ApiKey>>) -> bool {
    matches!(lookup, Ok(None))
}

/// The bound keys other than `caller_uuid` that the organization no longer
/// has. A deleted key's UUID left in `openApiKeys` makes every later upsert
/// fail with `400` (#659), so it is pruned before the caller's key is merged.
/// A failed lookup never aborts the bind and records no failure stage: the
/// key is simply kept.
async fn dangling_bound_keys(
    client: &CloudClient,
    org_id: &str,
    bound: &[String],
    caller_uuid: &str,
) -> Vec<String> {
    let mut gone: Vec<String> = Vec::new();
    for key in bound {
        if same_key(key, caller_uuid) || gone.contains(key) {
            continue;
        }
        let lookup = client.get_api_key_if_exists(org_id, key).await;
        if bound_key_is_gone(&lookup) {
            gone.push(key.clone());
        }
    }
    gone
}

/// Bind the caller's own API key to the service's query endpoint (#1043),
/// creating the endpoint when absent. Identifies the key with `whoami`,
/// refusing before any write unless it is an API key of `org_id`; then prunes
/// bindings to deleted keys and merges the caller's UUID into the rest,
/// keeping an existing endpoint's roles and allowed origins. Creates no key
/// and writes nothing locally, so a first-use query works from any directory,
/// read-only or not, and a later failure has nothing to roll back (#1450). On
/// success the run's provisioning state becomes
/// [`ProvisioningState::BoundCallerKey`].
///
/// The read-merge-upsert is not serialized across processes: concurrent binds
/// of the same key write the same list, and a bind of a different key that
/// lands in between can only drop a binding the next run of its owner
/// restores.
///
/// When the endpoint already lists the caller and nothing was pruned, an
/// upsert would change nothing, so none is made and the result is
/// [`CallerKeyBinding::AlreadyBound`]: the caller still waits for the
/// endpoint (a concurrent run may have just bound the key) and, if the Query
/// API rejects the key for the whole window, fails with
/// [`bound_caller_key_rejected_error`].
pub(crate) async fn bind_caller_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
) -> CloudResult<CallerKeyBinding> {
    let identity = client
        .get_whoami()
        .await
        .map_err(|error| error.at_stage(FailureStage::Whoami))?;
    let caller_uuid =
        caller_key_uuid(&identity, org_id).map_err(|error| error.at_stage(FailureStage::Whoami))?;

    let existing = read_bind_endpoint(client, org_id, service_id).await?;
    let bound = existing
        .as_ref()
        .map(|endpoint| endpoint.open_api_keys.clone())
        .unwrap_or_default();
    let gone = dangling_bound_keys(client, org_id, &bound, &caller_uuid).await;
    if gone.is_empty() && bound.iter().any(|key| same_key(key, &caller_uuid)) {
        return Ok(CallerKeyBinding::AlreadyBound { caller_uuid });
    }
    let request = build_bind_endpoint_request(
        existing.as_ref(),
        merged_open_api_keys(bound, &caller_uuid, &gone),
    );

    client
        .upsert_query_endpoint_binding(org_id, service_id, &request, key_propagation())
        .await
        .map_err(|error| {
            bind_forbidden_or(error, service_id, org_id, &caller_uuid)
                .at_stage(FailureStage::EndpointUpsert)
        })?;
    crate::failure::set_provisioning_state(ProvisioningState::BoundCallerKey);
    Ok(CallerKeyBinding::Bound)
}

/// Drop the Query API key record an earlier version stored for `service_id`
/// in `.clickhouse/credentials.json`, if there is one, and tell the user
/// which management key it names (#1325). That key still exists in the
/// organization, enabled and bound to the endpoint, so the notice gives the
/// command that deletes it; once deleted, the next bind prunes its binding.
///
/// Best effort: the command never depends on the record, so a store that
/// cannot be read or written leaves it for a later run without failing this
/// one. Stderr in every output mode, so `--json` stdout stays one value.
pub(crate) fn retire_legacy_query_key_record(service_id: &str, org_id: &str) {
    if let Ok(Some(record)) = credentials::take_legacy_service_query_key(service_id) {
        eprintln!("{}", legacy_query_key_notice(&record, service_id, org_id));
    }
}

/// The notice for a dropped legacy record. The record is opaque JSON from an
/// earlier version, so every field is optional; only the management key IDs
/// and names are read, never the key secret.
fn legacy_query_key_notice(record: &serde_json::Value, service_id: &str, org_id: &str) -> String {
    let org_id = record
        .get("organization_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(org_id);
    let mut api_key_ids: Vec<&str> = record
        .get("api_key_id")
        .and_then(serde_json::Value::as_str)
        .into_iter()
        .collect();
    let pending = record
        .get("pending_cleanup_api_key_ids")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str);
    for id in pending {
        if !api_key_ids.contains(&id) {
            api_key_ids.push(id);
        }
    }
    let prefix = format!(
        "Note: the Query API key that an earlier clickhousectl stored for service {service_id} \
         is no longer used, and its local record was removed. The key still exists in \
         organization {org_id};"
    );
    if api_key_ids.is_empty() {
        let name = record
            .get("service_name")
            .and_then(serde_json::Value::as_str)
            .map(|name| format!(" (named clickhousectl-query-{name})"))
            .unwrap_or_default();
        return format!(
            "{prefix} find it{name} with\n  clickhousectl cloud key list --org-id {org_id}\nand \
             delete it with `clickhousectl cloud key delete <id> --org-id {org_id}`."
        );
    }
    let commands: String = api_key_ids
        .iter()
        .map(|id| format!("\n  clickhousectl cloud key delete {id} --org-id {org_id}"))
        .collect();
    format!("{prefix} delete it with{commands}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::client::CloudErrorKind;

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
        // The paused clock only moves when the loop sleeps, so the stamps are
        // exact. `std::time::Instant` ignores the pause and would make every
        // bound below vacuous.
        let started = tokio::time::Instant::now();
        let _ = upsert_until_key_propagates(propagation, move || {
            counter.fetch_add(1, Ordering::SeqCst);
            recorder.lock().unwrap().push(started.elapsed());
            std::future::ready(Err::<(), _>(propagation_400()))
        })
        .await;
        let stamps = stamps.lock().unwrap();
        // Sleeps of 5, then 10 (the cap) until the last one is cut to the 5 ms
        // left before the deadline; the attempt at the deadline is the last.
        let expected: Vec<Duration> = [0, 5, 15, 25, 35, 40]
            .into_iter()
            .map(Duration::from_millis)
            .collect();
        assert_eq!(*stamps, expected);
        assert_eq!(attempts.load(Ordering::SeqCst), expected.len());
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
        assert!(err.to_string().contains("'openApiKeys'"), "{err}");
    }

    // ── binding the caller's own key (issue #1043) ──────────────────────

    const ORG: &str = "5fae43a3-0000-4000-8000-000000000001";

    fn whoami_key(key_id: Option<&str>, org: Option<&str>) -> Whoami {
        Whoami::WhoamiApiKey(clickhouse_cloud_api::models::WhoamiApiKey {
            actor_type: Some(clickhouse_cloud_api::models::WhoamiApiKeyActortype::ApiKey),
            key_id: key_id.map(str::to_string),
            name: Some("ci".into()),
            organization_id: org.map(|org| uuid::Uuid::parse_str(org).unwrap()),
        })
    }

    #[test]
    fn whoami_api_key_of_the_resolved_org_yields_its_resource_uuid() {
        let identity = whoami_key(Some("key-uuid-1"), Some(ORG));
        assert_eq!(caller_key_uuid(&identity, ORG).unwrap(), "key-uuid-1");
        // The resolved org is compared as a UUID, not as text.
        assert_eq!(
            caller_key_uuid(&identity, &ORG.to_uppercase()).unwrap(),
            "key-uuid-1"
        );
    }

    /// Only an API key of the resolved organization is the caller's key;
    /// every other identity is a generic error naming what was wrong.
    #[test]
    fn whoami_identities_other_than_a_key_of_the_resolved_org_are_refused() {
        let other = "5fae43a3-0000-4000-8000-000000000002";
        let cases = [
            (
                Whoami::WhoamiUser(clickhouse_cloud_api::models::WhoamiUser::default()),
                ORG,
                vec!["user"],
            ),
            (
                Whoami::Unknown(serde_json::json!({ "actorType": "robot" })),
                ORG,
                vec!["unrecognized"],
            ),
            (whoami_key(None, Some(ORG)), ORG, vec!["'keyId'"]),
            (whoami_key(Some(""), Some(ORG)), ORG, vec!["'keyId'"]),
            (
                whoami_key(Some("key-uuid-1"), None),
                ORG,
                vec!["'organizationId'"],
            ),
            (
                whoami_key(Some("key-uuid-1"), Some(other)),
                ORG,
                vec![other, ORG],
            ),
            // An org id that is not a UUID can never match.
            (whoami_key(Some("key-uuid-1"), Some(ORG)), "acme", vec![]),
        ];
        for (identity, org, needles) in cases {
            let error = caller_key_uuid(&identity, org).unwrap_err();
            assert_eq!(error.kind, CloudErrorKind::Generic, "{error}");
            for needle in needles {
                assert!(error.message.contains(needle), "{needle}: {error}");
            }
        }
    }

    #[test]
    fn only_a_confirmed_absence_prunes_a_bound_key() {
        assert!(bound_key_is_gone(&Ok(None)));
        assert!(!bound_key_is_gone(&Ok(Some(ApiKey::default()))));
        assert!(!bound_key_is_gone(&Err(CloudError::new("boom"))));
        assert!(!bound_key_is_gone(&Err(CloudError::auth("forbidden"))));
    }

    #[test]
    fn merging_drops_gone_keys_dedupes_and_appends_the_caller_once() {
        let existing = vec![
            "keep-a".to_string(),
            "gone-b".to_string(),
            "keep-a".to_string(),
            "keep-c".to_string(),
        ];
        assert_eq!(
            merged_open_api_keys(existing, "caller", &["gone-b".to_string()]),
            ["keep-a", "keep-c", "caller"]
        );
        // An already-bound caller keeps its position and is never pruned.
        assert_eq!(
            merged_open_api_keys(
                vec!["caller".to_string(), "keep-a".to_string()],
                "caller",
                &["caller".to_string()],
            ),
            ["caller", "keep-a"]
        );
    }

    #[test]
    fn bind_request_for_a_new_endpoint_binds_only_the_caller() {
        let request =
            build_bind_endpoint_request(None, merged_open_api_keys(vec![], "caller", &[]));
        assert_eq!(request.open_api_keys, ["caller"]);
        assert_eq!(request.roles, [QueryEndpointRole::SqlConsoleAdmin]);
        assert_eq!(request.allowed_origins.as_deref(), Some(ALLOWED_ORIGINS));
    }

    #[test]
    fn bind_request_for_an_existing_endpoint_keeps_live_keys_roles_and_origins() {
        let existing = InstanceServiceQueryApiEndpointsPostRequest {
            roles: vec![QueryEndpointRole::SqlConsoleReadOnly],
            open_api_keys: vec!["other-1".into(), "deleted".into(), "other-2".into()],
            allowed_origins: Some("https://app.example.com".into()),
        };
        let request = build_bind_endpoint_request(
            Some(&existing),
            merged_open_api_keys(
                existing.open_api_keys.clone(),
                "caller",
                &["deleted".to_string()],
            ),
        );
        assert_eq!(request.open_api_keys, ["other-1", "other-2", "caller"]);
        assert_eq!(request.roles, [QueryEndpointRole::SqlConsoleReadOnly]);
        assert_eq!(
            request.allowed_origins.as_deref(),
            Some("https://app.example.com")
        );
    }

    #[test]
    fn keys_compare_as_uuids_when_both_parse_and_as_strings_otherwise() {
        const LOWER: &str = "0b1c2d3e-4f50-6172-8394-a5b6c7d8e9f0";
        const UPPER: &str = "0B1C2D3E-4F50-6172-8394-A5B6C7D8E9F0";
        assert!(same_key(LOWER, UPPER));
        assert!(same_key("caller", "caller"));
        assert!(!same_key("Caller", "caller"));
        assert!(!same_key(LOWER, "0b1c2d3e-4f50-6172-8394-a5b6c7d8e9f1"));
        // An uppercase binding of the caller is the caller: kept in place,
        // never pruned, and not appended a second time.
        assert_eq!(
            merged_open_api_keys(
                vec![UPPER.to_string(), "keep-a".to_string()],
                LOWER,
                &[UPPER.to_string()],
            ),
            [UPPER, "keep-a"]
        );
    }

    #[test]
    fn an_already_bound_rejection_names_the_key_the_wait_and_where_to_look() {
        let error = bound_caller_key_rejected_error(
            "svc-1",
            "org-1",
            "key-uuid-1",
            Duration::from_secs(120),
        );
        assert_eq!(error.kind, CloudErrorKind::Generic);
        for needle in [
            "key-uuid-1",
            "after waiting 120s",
            "cloud service query-endpoint get svc-1 --org-id org-1",
            "cloud key get key-uuid-1 --org-id org-1",
        ] {
            assert!(
                error.message.contains(needle),
                "{needle}: {}",
                error.message
            );
        }
        let details = error.details.as_deref().unwrap();
        assert_eq!(details.code, CloudErrorCode::QueryKeyBoundRejected);
        assert_eq!(details.api_key_id.as_deref(), Some("key-uuid-1"));
        assert_eq!(
            details.command.as_deref(),
            Some("clickhousectl cloud service query-endpoint get svc-1 --org-id org-1")
        );
        assert_eq!(error.failure.unwrap().kind, FailureKind::Timeout);
    }

    // ── read-only keys and legacy records (issue #1325) ─────────────────

    fn converted(status: u16) -> CloudError {
        CloudError::new("API error (403): FORBIDDEN").with_failure(
            crate::failure::classify_api_error(&clickhouse_cloud_api::Error::Api {
                status,
                message: "FORBIDDEN".into(),
            }),
        )
    }

    #[test]
    fn a_forbidden_bind_names_the_key_and_both_remedies_and_keeps_its_classification() {
        let error = bind_forbidden_or(converted(403), "svc-1", "org-1", "key-uuid-1");
        assert_eq!(error.kind, CloudErrorKind::Generic);
        let failure = error.failure.unwrap();
        assert_eq!(failure.kind, FailureKind::Http4xx);
        assert_eq!(failure.http_status, Some(403));
        for needle in [
            "key-uuid-1",
            "API error (403): FORBIDDEN",
            "cloud service query-endpoint create svc-1",
            "--open-api-key key-uuid-1 --org-id org-1",
        ] {
            assert!(
                error.message.contains(needle),
                "{needle}: {}",
                error.message
            );
        }
        let details = error.details.as_deref().unwrap();
        assert_eq!(details.code, CloudErrorCode::QueryKeyBindForbidden);
        assert_eq!(details.message, error.message);
        assert_eq!(details.api_key_id.as_deref(), Some("key-uuid-1"));
        assert_eq!(
            details.command.as_deref(),
            Some("clickhousectl cloud service query-endpoint get svc-1 --org-id org-1")
        );
    }

    #[test]
    fn only_a_403_bind_failure_is_re_presented() {
        for error in [
            converted(400),
            converted(401),
            converted(500),
            CloudError::new("unclassified"),
        ] {
            let before = (error.message.clone(), error.kind, error.failure);
            let after = bind_forbidden_or(error, "svc-1", "org-1", "key-uuid-1");
            assert_eq!((after.message, after.kind, after.failure), before);
            assert!(after.details.is_none());
        }
    }

    #[test]
    fn the_legacy_notice_names_every_stored_key_and_never_the_secret() {
        let record = serde_json::json!({
            "organization_id": "org-stored",
            "api_key_id": "key-active",
            "pending_cleanup_api_key_ids": ["key-retired", "key-active"],
            "key_id": "query-key-id",
            "key_secret": "query-key-secret",
            "service_name": "demo",
        });
        let notice = legacy_query_key_notice(&record, "svc-1", "org-now");
        assert!(notice.contains("svc-1"), "{notice}");
        for id in ["key-active", "key-retired"] {
            let command = format!("clickhousectl cloud key delete {id} --org-id org-stored");
            assert_eq!(notice.matches(&command).count(), 1, "{id}: {notice}");
        }
        assert!(!notice.contains("query-key-secret"), "{notice}");
        assert!(!notice.contains("query-key-id"), "{notice}");
        assert!(!notice.contains("org-now"), "{notice}");
    }

    #[test]
    fn a_legacy_record_without_key_ids_points_to_the_key_list() {
        let record =
            serde_json::json!({"key_id": "kid", "key_secret": "sec", "service_name": "demo"});
        let notice = legacy_query_key_notice(&record, "svc-1", "org-now");
        for needle in [
            "clickhousectl cloud key list --org-id org-now",
            "clickhousectl-query-demo",
        ] {
            assert!(notice.contains(needle), "{needle}: {notice}");
        }

        // Not even the shape is trusted: anything else still yields a notice.
        let notice = legacy_query_key_notice(&serde_json::json!("garbage"), "svc-1", "org-now");
        assert!(
            notice.contains("cloud key list --org-id org-now"),
            "{notice}"
        );
    }
}
