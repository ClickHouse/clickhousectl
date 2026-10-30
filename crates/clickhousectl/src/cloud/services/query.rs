use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::{CloudErrorCode, CloudErrorDetail, eprint_line, or_absent};
use crate::cloud::services::query_key::{CallerKeyBinding, RepairVerification};
use crate::cloud::shared::resolve_org_id;
use crate::failure::{self, ApiFailure, FailureKind, FailureStage, ProvisioningState};
use clickhouse_cloud_api::models::{
    Service, ServiceEndpoint, ServiceEndpointProtocol, ServiceState,
};
use std::{io::IsTerminal, time::Duration};

#[derive(Clone, Copy)]
struct QueryEndpointReadiness {
    timeout: Duration,
    initial_backoff: Duration,
    max_backoff: Duration,
}

const QUERY_ENDPOINT_READINESS: QueryEndpointReadiness = QueryEndpointReadiness {
    timeout: Duration::from_secs(120),
    initial_backoff: Duration::from_millis(200),
    max_backoff: Duration::from_secs(5),
};

/// Resolve a service by name or ID within the given org.
/// Exactly one of `name` or `id` must be provided.
async fn resolve_service(
    client: &CloudClient,
    org_id: &str,
    name: Option<&str>,
    id: Option<&str>,
) -> CloudResult<Service> {
    match (name, id) {
        (Some(name), None) => {
            let services = client.list_services(org_id).await?;
            let id = crate::cloud::shared::select_named_id(
                "service",
                name,
                services
                    .iter()
                    .map(|service| (service.name.as_deref(), service.id.as_ref())),
            )?;
            services
                .into_iter()
                .find(|service| {
                    service
                        .id
                        .as_ref()
                        .is_some_and(|candidate| candidate.to_string() == id)
                })
                .ok_or_else(|| CloudError::new("resolved service is missing from the list"))
        }
        (None, Some(id)) => Ok(client.get_service(org_id, id).await?),
        (Some(_), Some(_)) => Err(CloudError::new(
            "supply either a positional service ID or --name",
        )),
        // Clap rejects this state; retain the check for any future non-CLI caller.
        (None, None) => Err(CloudError::new(
            "supply a positional service ID or --name to identify the service",
        )),
    }
}

pub(super) async fn service_query_key_repair(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let result =
        crate::cloud::services::query_key::repair_service_query_key(client, &org_id, service_id)
            .await?;
    // A retired key that could not be deleted is a warning, not a failure: the
    // replacement is active and its ID is stored for the retry (#527). The
    // warning goes to stderr in both modes; the JSON result carries the IDs.
    let mut result = result;
    if let Some(warning) = &result.cleanup_warning {
        eprint_line(warning);
    }
    // The repair is committed before the probe runs, so its result is printed
    // whatever the probe says (#658): a `--json` consumer must never lose the
    // IDs of a repair that happened. Only a probe that kept being rejected
    // for the whole readiness window is an error, and it is reported after
    // the result.
    let mut verification_error = None;
    if result.status == "repaired" {
        match verify_repaired_query_key(
            client,
            &org_id,
            service_id,
            &result.api_key_id,
            QUERY_ENDPOINT_READINESS,
        )
        .await
        {
            Ok(verification) => result.verification = Some(verification),
            Err(error) => {
                result.verification = Some(RepairVerification::Failed);
                verification_error = Some(error);
            }
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        match result.status {
            "already_repaired" => println!(
                "Query key for service {} was already repaired by another process (active API key: {})",
                result.service_id, result.api_key_id
            ),
            _ => {
                println!("Query key repaired for service {}", result.service_id);
                println!(
                    "  Replaced API key: {}",
                    or_absent(result.replaced_api_key_id.as_deref())
                );
                println!("  New API key: {}", result.api_key_id);
                println!("  Query endpoint: {}", result.endpoint_id);
                if !result.deleted_api_key_ids.is_empty() {
                    println!(
                        "  Deleted API keys: {}",
                        result.deleted_api_key_ids.join(", ")
                    );
                }
                if !result.pending_cleanup_api_key_ids.is_empty() {
                    println!(
                        "  API keys awaiting deletion: {}",
                        result.pending_cleanup_api_key_ids.join(", ")
                    );
                }
                if let Some(verification) = result.verification {
                    println!("  Query API check: {}", verification.as_str());
                }
            }
        }
    }
    match verification_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Confirm that the Query API accepts the key a repair just committed (#658).
///
/// The new binding reaches the Query API host a moment after the upsert; a
/// query issued in between is rejected, and the stored-key classifier (#528)
/// would then read an enabled, bound key as "IP allowlist or secret
/// mismatch", a verdict that is wrong and points nowhere useful. So repair
/// probes the endpoint with the new key the way first-use provisioning does.
///
/// The repair is committed before this runs and nothing here undoes it. The
/// outcome is reported, not acted on: `Verified` when a probe succeeded;
/// `Skipped` when the key was not probed (no Query API host configured, the
/// service not `running` or its state unreadable, the record unreadable or
/// replaced by a concurrent repair, or the probe answered idle/stopped);
/// `Failed` when the probe failed for a reason unrelated to readiness. Only a
/// key the Query API kept rejecting for the whole readiness window is an
/// error, and even then the message says the repair stands and points at
/// `cloud service query`, which classifies the rejection — never at rerunning
/// the repair, which would rotate a key that may simply not have propagated
/// yet.
async fn verify_repaired_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_id: &str,
    readiness: QueryEndpointReadiness,
) -> CloudResult<RepairVerification> {
    let stored = credentials::try_get_service_query_key(service_id);
    let (key_id, key_secret) = match repaired_query_credential(stored, service_id, api_key_id) {
        Ok(credential) => credential,
        Err(reason) => {
            return Ok(skip_repair_verification(
                org_id, service_id, api_key_id, &reason,
            ));
        }
    };
    probe_repaired_query_key(
        client,
        org_id,
        service_id,
        api_key_id,
        &key_id,
        &key_secret,
        readiness,
    )
    .await
}

/// The credential pair to probe with, if the record still names the key the
/// repair just stored. A record that vanished, that a concurrent repair
/// replaced, or that could not be read back at all is not this run's to
/// verify; the `Err` carries the reason the probe is skipped. The repair is
/// already committed, so re-reading it is never what turns the run into a
/// failure.
fn repaired_query_credential(
    stored: CloudResult<Option<credentials::ServiceQueryKey>>,
    service_id: &str,
    api_key_id: &str,
) -> Result<(String, String), String> {
    let stored = stored.map_err(|error| {
        format!("the stored query key for service {service_id} could not be read ({error})")
    })?;
    let changed = || {
        format!(
            "the stored query key for service {service_id} changed before the replacement could \
             be probed"
        )
    };
    let stored = stored.ok_or_else(changed)?;
    if stored.api_key_id.as_deref() != Some(api_key_id) {
        return Err(changed());
    }
    Ok((stored.key_id, stored.key_secret))
}

fn next_query_command(service_id: &str, org_id: &str) -> String {
    format!("clickhousectl cloud service query {service_id} --org-id {org_id} --query \"SELECT 1\"")
}

/// Print the one `Note:` line every skipped verification gets, and return
/// `Skipped`. A closed stderr must not panic: the repair is already done.
fn skip_repair_verification(
    org_id: &str,
    service_id: &str,
    api_key_id: &str,
    reason: &str,
) -> RepairVerification {
    eprint_line(format!(
        "Note: {reason}, so the repaired query key was not probed; the replacement (API key \
         {api_key_id}) is stored in .clickhouse/credentials.json and is verified by the next `{}`",
        next_query_command(service_id, org_id)
    ));
    RepairVerification::Skipped
}

#[allow(clippy::too_many_arguments)]
async fn probe_repaired_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_id: &str,
    key_id: &str,
    key_secret: &str,
    readiness: QueryEndpointReadiness,
) -> CloudResult<RepairVerification> {
    // Never probe production by accident: a control plane the query host
    // cannot be derived from (a local mock, an unusual base URL) means the
    // library would fall back to the production host.
    if client.configured_query_host().is_none() {
        return Ok(skip_repair_verification(
            org_id,
            service_id,
            api_key_id,
            &format!(
                "no Query API host is configured for {} (set CLICKHOUSE_CLOUD_QUERY_HOST)",
                client.base_url()
            ),
        ));
    }
    let state = match client.get_service(org_id, service_id).await {
        Ok(service) => service.state,
        Err(error) => {
            return Ok(skip_repair_verification(
                org_id,
                service_id,
                api_key_id,
                &format!("could not read service {service_id}: {error}"),
            ));
        }
    };
    if state != Some(ServiceState::Running) {
        // A probe would wake an idle service, and a stopped one cannot answer.
        let state = state
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "in an unknown state".to_string());
        return Ok(skip_repair_verification(
            org_id,
            service_id,
            api_key_id,
            &format!("service {service_id} is {state}"),
        ));
    }

    let outcome = wait_for_query_endpoint_readiness(readiness, || {
        client.run_service_query(
            service_id,
            key_id,
            key_secret,
            "SELECT 1",
            None,
            "TabSeparated",
            false,
        )
    })
    .await;
    let repaired = format!(
        "the query key for service {service_id} was replaced (new API key {api_key_id}) and \
         stored in .clickhouse/credentials.json"
    );
    match outcome {
        Ok(false) => Ok(RepairVerification::Verified),
        Ok(true) => Ok(skip_repair_verification(
            org_id,
            service_id,
            api_key_id,
            &format!("service {service_id} is idle"),
        )),
        Err(QueryReadinessError::Api(clickhouse_cloud_api::Error::ServiceStopped)) => {
            Ok(skip_repair_verification(
                org_id,
                service_id,
                api_key_id,
                &format!("service {service_id} is stopped"),
            ))
        }
        Err(QueryReadinessError::Api(error)) => {
            let converted = client.convert_error(error);
            eprint_line(format!(
                "Warning: {repaired}, but probing it failed: {converted}; it is verified by the \
                 next `{}`",
                next_query_command(service_id, org_id)
            ));
            Ok(RepairVerification::Failed)
        }
        Err(QueryReadinessError::TimedOut(timeout)) => {
            let command = next_query_command(service_id, org_id);
            let message = format!(
                "{repaired}, but the Query API kept rejecting it for {timeout:?}. Do not rerun \
                 repair-query-key: that would rotate a key that may only be slow to propagate. \
                 Run `{command}`, which classifies the rejection"
            );
            Err(CloudError::new(message.clone())
                .with_failure(ApiFailure::new(FailureKind::Timeout))
                .with_details(CloudErrorDetail {
                    code: CloudErrorCode::QueryKeyRepairUnverified,
                    message,
                    host: None,
                    port: None,
                    command: Some(command),
                    api_key_id: Some(api_key_id.to_string()),
                    ip_access_list: None,
                })
                .at_stage(FailureStage::QueryRequest))
        }
    }
}

pub(super) struct ServiceQueryOptions {
    pub(super) name: Option<String>,
    pub(super) id: Option<String>,
    pub(super) query: Option<String>,
    pub(super) queries_file: Option<String>,
    pub(super) database: Option<String>,
    pub(super) format: Option<String>,
    pub(super) json: bool,

    pub(super) no_auto_enable: bool,
}

#[allow(clippy::too_many_arguments)]
async fn run_basic_service_query(
    client: &CloudClient,
    service_id: &str,
    key_id: &str,
    key_secret: &str,
    sql: &str,
    database: Option<&str>,
    format: &str,
    service_name: &str,
    confirmed_idle: bool,
) -> Result<reqwest::Response, QueryRequestError> {
    run_service_query_with_wake(service_name, confirmed_idle, |wake| {
        client.run_service_query(service_id, key_id, key_secret, sql, database, format, wake)
    })
    .await
}

/// Preserve an explicit pre-execution idle response when the following wake
/// attempt fails. A timeout alone never permits another SQL submission.
struct QueryRequestError {
    error: clickhouse_cloud_api::Error,
    wake_requested: bool,
}

impl QueryRequestError {
    fn into_cloud_error(self, client: &CloudClient, target: QueryTarget<'_>) -> CloudError {
        convert_query_error(
            client,
            self.error,
            QueryTarget {
                may_be_waking: target.may_be_waking || self.wake_requested,
                ..target
            },
        )
    }
}

async fn run_service_query_with_wake<F, Fut>(
    service_name: &str,
    confirmed_idle: bool,
    run: F,
) -> Result<reqwest::Response, QueryRequestError>
where
    F: Fn(bool) -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, clickhouse_cloud_api::Error>>,
{
    let waking_error = |error| QueryRequestError {
        error,
        wake_requested: true,
    };
    if confirmed_idle {
        eprint_waking_service(service_name);
        failure::note_retry();
        return run(true).await.map_err(waking_error);
    }

    match run(false).await {
        Err(clickhouse_cloud_api::Error::ServiceIdle) => {
            eprint_waking_service(service_name);
            failure::note_retry();
            run(true).await.map_err(waking_error)
        }
        other => other.map_err(|error| QueryRequestError {
            error,
            wake_requested: false,
        }),
    }
}

/// Why the readiness wait ended without the endpoint accepting a probe.
///
/// The deadline is the CLI's own: no HTTP response stands behind it, so it is
/// a variant of its own rather than a synthesized `Error::Api { status: 408 }`
/// — telemetry reports `http_status` as the exact status the server sent, and
/// a made-up one would be a lie there (#450).
#[derive(Debug)]
enum QueryReadinessError {
    /// The endpoint was still rejecting probes when the deadline elapsed.
    TimedOut(Duration),
    /// A probe failed for a reason other than "not ready yet".
    Api(clickhouse_cloud_api::Error),
}

fn query_readiness_timeout_error(timeout: Duration) -> CloudError {
    CloudError::new(format!(
        "Query API endpoint did not become ready within {timeout:?}"
    ))
    .with_failure(ApiFailure::new(FailureKind::Timeout))
}

async fn wait_for_query_endpoint_readiness<T, F, Fut>(
    readiness: QueryEndpointReadiness,
    mut probe: F,
) -> Result<bool, QueryReadinessError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, clickhouse_cloud_api::Error>>,
{
    let deadline = tokio::time::Instant::now() + readiness.timeout;
    let mut backoff = readiness.initial_backoff;
    let mut waiting = false;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(QueryReadinessError::TimedOut(readiness.timeout));
        }

        match tokio::time::timeout(remaining, probe()).await {
            Ok(Ok(_)) => return Ok(false),
            Ok(Err(clickhouse_cloud_api::Error::ServiceIdle)) => return Ok(true),
            Ok(Err(error)) if query_endpoint_readiness_error(&error) => {}
            Ok(Err(error)) => return Err(QueryReadinessError::Api(error)),
            Err(_) => return Err(QueryReadinessError::TimedOut(readiness.timeout)),
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(QueryReadinessError::TimedOut(readiness.timeout));
        }
        if !waiting {
            eprint_line("Waiting for the Query API endpoint to become ready...");
            waiting = true;
        }
        // One more probe is about to be sent: the retry bucket is how a
        // dashboard sees "this run waited on propagation" (#450).
        failure::note_retry();
        tokio::time::sleep(backoff.min(remaining)).await;
        backoff = backoff.saturating_mul(2).min(readiness.max_backoff);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_just_provisioned_service_query(
    client: &CloudClient,
    key_id: &str,
    key_secret: &str,
    sql: &str,
    database: Option<&str>,
    format: &str,
    target: QueryTarget<'_>,
    readiness: QueryEndpointReadiness,
) -> CloudResult<reqwest::Response> {
    run_just_provisioned_service_query_or(
        client,
        key_id,
        key_secret,
        sql,
        database,
        format,
        target,
        readiness,
        query_readiness_timeout_error,
    )
    .await
}

/// [`run_just_provisioned_service_query`], reporting an exhausted readiness
/// window with `on_timeout` (given the window) instead of the generic
/// timeout error. Every other failure converts as usual.
#[allow(clippy::too_many_arguments)]
async fn run_just_provisioned_service_query_or(
    client: &CloudClient,
    key_id: &str,
    key_secret: &str,
    sql: &str,
    database: Option<&str>,
    format: &str,
    target: QueryTarget<'_>,
    readiness: QueryEndpointReadiness,
    on_timeout: impl FnOnce(Duration) -> CloudError,
) -> CloudResult<reqwest::Response> {
    let confirmed_idle = wait_for_query_endpoint_readiness(readiness, || {
        client.run_service_query(
            target.service_id,
            key_id,
            key_secret,
            "SELECT 1",
            None,
            "TabSeparated",
            false,
        )
    })
    .await
    .map_err(|error| match error {
        QueryReadinessError::TimedOut(timeout) => on_timeout(timeout),
        QueryReadinessError::Api(error) => convert_query_error(client, error, target),
    })?;

    run_basic_service_query(
        client,
        target.service_id,
        key_id,
        key_secret,
        sql,
        database,
        format,
        target.service_name,
        confirmed_idle,
    )
    .await
    .map_err(|error| error.into_cloud_error(client, target))
}

/// Whether an error means "the Query API endpoint is not (yet) usable by this
/// credential" rather than "the query itself failed".
///
/// The test is structural: a SQL-level rejection arrives as
/// `Error::Sql`, which proves the service ran the request, so the endpoint is
/// demonstrably usable. It used to be told apart by sniffing the formatted
/// message for a `SQL error ` prefix (#450).
fn query_endpoint_readiness_error(error: &clickhouse_cloud_api::Error) -> bool {
    matches!(
        error,
        clickhouse_cloud_api::Error::Api {
            status: 401 | 403 | 404,
            ..
        }
    )
}

fn stored_query_key_rejection_status(error: &clickhouse_cloud_api::Error) -> Option<u16> {
    match error {
        clickhouse_cloud_api::Error::Api {
            status: status @ (401 | 403),
            ..
        } => Some(*status),
        _ => None,
    }
}

/// The `--no-auto-enable` refusal: the Query API rejected the management key
/// (401/403/404) and binding that key to the endpoint would have fixed that,
/// but the caller forbade it. The message is the CLI's; the classification stays the API's actual
/// rejection, carried across from the variant rather than reset to `other`
/// (#450).
fn refused_query_provisioning_error(
    service_id: &str,
    rejection: &clickhouse_cloud_api::Error,
) -> CloudError {
    CloudError::new(format!(
        "the authenticated API key cannot use the Query API endpoint for service {service_id}, \
         and --no-auto-enable prevents binding it to the endpoint"
    ))
    .with_failure(failure::classify_api_error(rejection))
}

pub(super) async fn service_query(
    client: &CloudClient,
    options: ServiceQueryOptions,
) -> CloudResult<()> {
    // Anchor the duration bucket of any failure classification (#450) at the
    // start of the run rather than at process start, so it measures the query
    // and not the CLI's own startup.
    failure::start_span();

    let sql = read_query_sql(options.query.as_deref(), options.queries_file.as_deref())
        .map_err(|error| error.at_stage(FailureStage::SqlInput))?;
    let org_id = resolve_org_id(client)
        .await
        .map_err(|error| error.at_stage(FailureStage::OrgResolution))?;
    let service = resolve_service(
        client,
        &org_id,
        options.name.as_deref(),
        options.id.as_deref(),
    )
    .await
    .map_err(|error| error.at_stage(FailureStage::ServiceResolution))?;
    let service_id = match service.id {
        Some(id) => id.to_string(),
        None => options.id.clone().ok_or_else(|| {
            CloudError::new("the API response is missing the service id")
                .at_stage(FailureStage::ServiceResolution)
        })?,
    };
    let service_name = or_absent(service.name.as_deref());
    // Resolved once, from the `GET service` response the query already needed:
    // the gateway-timeout hint spells out a `clickhouse client` command, and
    // it must use the service's real native endpoint when the API returned
    // one (#644).
    let native = native_secure_endpoint(service.endpoints.as_deref());
    let target = QueryTarget {
        service_name: &service_name,
        service_id: &service_id,
        org_id: &org_id,
        native: native.as_ref(),
        // Use the service lookup already required for this query. This is
        // evidence of a possible wake delay, not proof the SQL never ran.
        may_be_waking: matches!(
            service.state,
            Some(ServiceState::Idle | ServiceState::Awaking)
        ),
    };

    let format = options.format.unwrap_or_else(|| {
        if options.json {
            "JSONEachRow".to_string()
        } else {
            default_query_format()
        }
    });

    // The `query_request` classifications below are a *fallback*: recording
    // is first-write-wins, so an inner boundary that knows the exact stage
    // (`whoami`, `endpoint_get`, ...) keeps its record and only failures
    // no boundary claimed -- an unusable local credential store, say -- land
    // on the coarse `query_request` stage rather than going unclassified
    // (#450).
    let response = if client.is_bearer_auth() {
        // OAuth carries no write access, so no key or endpoint provisioning is
        // possible on this path at all.
        failure::set_provisioning_state(ProvisioningState::Bearer);
        let run = |wake: bool| {
            client.run_service_query_bearer(
                &service_id,
                &sql,
                options.database.as_deref(),
                &format,
                wake,
            )
        };
        run_service_query_with_wake(&service_name, false, run)
            .await
            .map_err(|error| {
                error
                    .into_cloud_error(client, target)
                    .at_stage(FailureStage::QueryRequest)
            })?
    } else {
        let convert = |error: QueryRequestError| error.into_cloud_error(client, target);
        let result = if let Some(key) = credentials::try_get_service_query_key(&service_id)
            .map_err(|error| error.at_stage(FailureStage::QueryRequest))?
        {
            failure::set_provisioning_state(ProvisioningState::StoredKey);
            // A repair that could not delete a superseded key left its ID on
            // the record; finish that here, quietly, before the query (#527).
            // The list is empty on every ordinary run, so this costs nothing
            // unless a retirement is actually outstanding.
            crate::cloud::services::query_key::retry_pending_query_key_cleanup(
                client,
                &key,
                &service_id,
                &org_id,
            )
            .await;
            match run_basic_service_query(
                client,
                &service_id,
                &key.key_id,
                &key.key_secret,
                &sql,
                options.database.as_deref(),
                &format,
                &service_name,
                false,
            )
            .await
            {
                Err(error) => match stored_query_key_rejection_status(&error.error) {
                    // The 401/403 alone does not say whether the key is stale
                    // or was deliberately revoked; the management record
                    // decides, and nothing is replaced automatically (#528).
                    Some(status) => {
                        return Err(
                            crate::cloud::services::query_key::rejected_stored_query_key_error(
                                client,
                                &key,
                                &service_id,
                                &org_id,
                                status,
                            )
                            .await
                            .at_stage(FailureStage::QueryRequest),
                        );
                    }
                    None => Err(convert(error)),
                },
                Ok(response) => Ok(response),
            }
        } else {
            failure::set_provisioning_state(ProvisioningState::ManagementKey);
            let (key_id, key_secret) = client.basic_auth_credentials().ok_or_else(|| {
                CloudError::new("API key credentials are unavailable")
                    .at_stage(FailureStage::QueryRequest)
            })?;
            match run_basic_service_query(
                client,
                &service_id,
                key_id,
                key_secret,
                &sql,
                options.database.as_deref(),
                &format,
                &service_name,
                false,
            )
            .await
            {
                Err(error) if query_endpoint_readiness_error(&error.error) => {
                    if options.no_auto_enable {
                        failure::set_provisioning_state(ProvisioningState::Refused);
                        return Err(refused_query_provisioning_error(&service_id, &error.error)
                            .at_stage(FailureStage::QueryRequest));
                    }
                    eprint_line(format!(
                        "Binding the authenticated API key to the Query API endpoint for \
                         service '{}'...",
                        service_name
                    ));
                    // No key is created: the caller's own key is bound to the
                    // endpoint and the query re-runs with it (#1043). The bind
                    // records `BoundCallerKey` itself once it succeeds.
                    failure::set_provisioning_state(ProvisioningState::Provisioning);
                    let binding = crate::cloud::services::query_key::bind_caller_query_key(
                        client,
                        &org_id,
                        &service_id,
                    )
                    .await
                    .map_err(|error| error.at_stage(FailureStage::QueryRequest))?;
                    match binding {
                        CallerKeyBinding::Bound => {
                            run_just_provisioned_service_query(
                                client,
                                key_id,
                                key_secret,
                                &sql,
                                options.database.as_deref(),
                                &format,
                                target,
                                QUERY_ENDPOINT_READINESS,
                            )
                            .await
                        }
                        // Nothing was written. A concurrent run may have bound
                        // the key while this one waited for the lock, so wait
                        // for the endpoint as the binder does; only a key
                        // still rejected for the whole window means rebinding
                        // cannot help.
                        CallerKeyBinding::AlreadyBound { caller_uuid } => {
                            failure::set_provisioning_state(ProvisioningState::ManagementKey);
                            run_just_provisioned_service_query_or(
                                client,
                                key_id,
                                key_secret,
                                &sql,
                                options.database.as_deref(),
                                &format,
                                target,
                                QUERY_ENDPOINT_READINESS,
                                |waited| {
                                    crate::cloud::services::query_key::bound_caller_key_rejected_error(
                                        &service_id,
                                        &org_id,
                                        &caller_uuid,
                                        waited,
                                    )
                                    .at_stage(FailureStage::EndpointGet)
                                },
                            )
                            .await
                        }
                    }
                }
                other => other.map_err(convert),
            }
        };
        result.map_err(|error| error.at_stage(FailureStage::QueryRequest))?
    };

    use futures_util::StreamExt;
    use std::io::Write as _;
    let mut stream = response.bytes_stream();
    let mut handle = crate::stdout::stdout();
    let mut byte_count = 0;
    let mut last_byte = None;
    // Both failure modes of the stream are classified here, at the boundary
    // that knows which one happened (#450): a truncated body is a transport
    // failure, a refused write is local I/O.
    let stream_failure = |error: CloudError| error.at_stage(FailureStage::ResponseStream);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|error| {
            stream_failure(
                CloudError::new(format!("Failed to read query response: {error}"))
                    .with_failure(ApiFailure::new(FailureKind::Transport)),
            )
        })?;
        handle
            .write_all(&bytes)
            .map_err(|error| stream_failure(CloudError::from(error)))?;
        byte_count += bytes.len();
        if let Some(last) = bytes.last() {
            last_byte = Some(*last);
        }
    }
    match query_output_completion(&format, byte_count, last_byte) {
        QueryOutputCompletion::None => {}
        QueryOutputCompletion::Newline => handle
            .write_all(b"\n")
            .map_err(|error| stream_failure(CloudError::from(error)))?,
        QueryOutputCompletion::Acknowledge => eprint_line("OK"),
    }
    handle
        .flush()
        .map_err(|error| stream_failure(CloudError::from(error)))?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum QueryOutputCompletion {
    None,
    Newline,
    Acknowledge,
}

fn query_output_completion(
    format: &str,
    byte_count: usize,
    last_byte: Option<u8>,
) -> QueryOutputCompletion {
    if byte_count == 0 {
        return QueryOutputCompletion::Acknowledge;
    }
    if query_format_uses_text_lines(format) && last_byte != Some(b'\n') {
        QueryOutputCompletion::Newline
    } else {
        QueryOutputCompletion::None
    }
}

fn query_format_uses_text_lines(format: &str) -> bool {
    matches!(
        format.to_ascii_lowercase().as_str(),
        "jsoneachrow" | "prettycompact" | "tabseparated"
    )
}

fn eprint_waking_service(service_name: &str) {
    eprint_line(format!(
        "Service '{service_name}' is idle; waking it (this may take a minute)..."
    ));
}

/// A service's native-protocol (`nativesecure`) endpoint.
///
/// Both halves are required. A hint that named the real host but guessed the
/// port would hand the user a command that does not connect, which is worse
/// than a command that visibly needs filling in.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeEndpoint {
    host: String,
    port: i64,
}

/// What the query path knows about its target service when it has to render
/// an error: enough to name the service, to point at `service start`, and to
/// spell out the native-protocol command that routes around the Query API
/// gateway timeout (#644).
#[derive(Debug, Clone, Copy)]
struct QueryTarget<'a> {
    service_name: &'a str,
    service_id: &'a str,
    org_id: &'a str,
    /// The service's `nativesecure` endpoint, when the `GET service`
    /// response carried a complete one.
    native: Option<&'a NativeEndpoint>,
    /// An idle/awaking service lookup, or an explicit pre-execution idle
    /// response followed by a wake request. Neither proves the SQL did not run.
    may_be_waking: bool,
}

/// The service's `nativesecure` endpoint, when the API returned a complete
/// one.
///
/// Every field of a response model is optional, so a partial endpoint (a host
/// with no port, or the reverse) counts as absent here rather than being
/// half-rendered into a command.
fn native_secure_endpoint(endpoints: Option<&[ServiceEndpoint]>) -> Option<NativeEndpoint> {
    endpoints?.iter().find_map(|endpoint| {
        if endpoint.protocol != Some(ServiceEndpointProtocol::Nativesecure) {
            return None;
        }
        Some(NativeEndpoint {
            host: endpoint.host.clone()?,
            port: endpoint.port?,
        })
    })
}

/// Stand-in for a host the API did not return. A placeholder, never a guess.
const NATIVE_HOST_PLACEHOLDER: &str = "<host>";

/// The standard native-protocol TLS port. Only used in the placeholder
/// command, where there is no endpoint to read a real port from.
const DEFAULT_NATIVE_SECURE_PORT: i64 = 9440;

/// The `clickhouse client` command that reruns the statement over the native
/// protocol.
///
/// The SQL and the password are always literal placeholders: the user's
/// statement is never echoed back into an error message, and no credential is
/// ever rendered.
fn native_client_command(native: Option<&NativeEndpoint>) -> String {
    let (host, port) = match native {
        Some(endpoint) => (endpoint.host.as_str(), endpoint.port),
        None => (NATIVE_HOST_PLACEHOLDER, DEFAULT_NATIVE_SECURE_PORT),
    };
    format!(
        "clickhouse client --host {host} --secure --port {port} --user default \
         --password '<password>' --query '<your SQL>'"
    )
}

/// Diagnose a typed gateway timeout using the state observed before the
/// request (#826). A possible wake delay does not prove the statement was
/// never executed, so neither diagnosis permits an automatic retry.
fn query_timeout_error(target: QueryTarget<'_>) -> CloudError {
    let details = if target.may_be_waking {
        let command = format!(
            "clickhousectl cloud service get {} --org-id {}",
            target.service_id, target.org_id
        );
        let message = format!(
            "the query timed out and service '{}' may still be waking; it was idle or awaking \
             before the query attempt.\nThe statement may already have executed or may still be \
             running; it was not retried after the timeout.\nCheck the service state with \
             `{command}`. Once it is running, verify the statement's outcome before deciding \
             whether it is safe to run again.",
            target.service_name
        );
        CloudErrorDetail {
            command: Some(command),
            ..CloudErrorDetail::new(CloudErrorCode::QueryTimeout, message)
        }
    } else {
        let command = native_client_command(target.native);
        let endpoint_guidance = if target.native.is_some() {
            String::new()
        } else {
            format!(
                "\nThe API response carried no native endpoint for this service; read the host and \
                 port from `clickhousectl cloud service get {} --org-id {}`.",
                target.service_id, target.org_id
            )
        };
        let message = format!(
            "the query timed out at the Query API gateway, which stops waiting after about 30 \
             seconds.\nThe statement may still be running on the service or may already have \
             completed; check `SELECT query_id, elapsed FROM system.processes` and verify its \
             outcome before running it again.\n\nHint: run long statements over the native \
             protocol instead. Install the client with\n  clickhousectl local use latest\nthen \
             rerun the query with\n  {command}{endpoint_guidance}\nThe password is the one shown \
             when the service was created; `clickhousectl cloud service reset-password {}` \
             issues a new one.",
            target.service_id
        );
        CloudErrorDetail {
            code: CloudErrorCode::QueryTimeout,
            message,
            host: target.native.map(|native| native.host.clone()),
            port: target.native.map(|native| native.port),
            command: Some(command),
            api_key_id: None,
            ip_access_list: None,
        }
    };

    CloudError::new(details.message.clone())
        // Both diagnoses retain the typed timeout classification (#450).
        .with_failure(failure::classify_api_error(
            &clickhouse_cloud_api::Error::QueryTimeout,
        ))
        .with_details(details)
}

fn convert_query_error(
    client: &CloudClient,
    error: clickhouse_cloud_api::Error,
    target: QueryTarget<'_>,
) -> CloudError {
    match error {
        clickhouse_cloud_api::Error::ServiceStopped => CloudError::new(format!(
            "service '{}' is stopped; start it with `clickhousectl cloud service start {} --org-id {}` and retry",
            target.service_name, target.service_id, target.org_id
        ))
        // Rewriting the message must not lose the classification the variant
        // already established (#450).
        .with_failure(ApiFailure::new(FailureKind::ServiceStopped)),
        clickhouse_cloud_api::Error::QueryTimeout => query_timeout_error(target),
        other => client.convert_error(other),
    }
}

/// What stdin holds when `--query` is validated (#641). Derived from the file
/// descriptor only, never from anything the user typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StdinInput {
    /// An interactive terminal: nothing was piped or redirected in, and stdin
    /// must not be read at all — a read would block on the human.
    Terminal,
    /// Not a terminal, and no byte arrived within
    /// [`STDIN_FIRST_BYTE_TIMEOUT_MS`]: `< /dev/null`, a closed pipe, or a CI
    /// runner or coding agent that has no tty and piped nothing.
    Empty,
    /// Not a terminal and at least one byte is available to read.
    Pending,
}

/// The message for `--query` plus data on stdin. The Query API takes a single
/// request body, so a statement and a separate data stream can never both be
/// sent; the message therefore leads with what does work.
const INLINE_QUERY_WITH_STDIN_ERROR: &str = "--query cannot be combined with SQL or data on stdin. \
     The Query API sends one request body, so redirected data is never read. Pipe the statement \
     and its data together on stdin instead: \
     printf 'INSERT INTO t FORMAT CSV\\n' | cat - data.csv | clickhousectl cloud service query \
     --id <id>. Or read a whole statement from stdin with --queries-file -.";

/// Whether `--query` conflicts with what is on stdin. Pure, so the decision is
/// unit-tested without a real file descriptor. Only bytes that are actually
/// waiting count: a non-terminal but empty stdin is the normal shape for CI
/// and for coding agents, so it must stay a valid way to run `--query`.
fn inline_query_conflicts_with_stdin(inline: Option<&str>, stdin: StdinInput) -> bool {
    inline.is_some() && matches!(stdin, StdinInput::Pending)
}

/// How long to wait for the first byte before deciding stdin carries nothing.
///
/// A zero timeout would reintroduce #641 as a race: in `cat data.csv |
/// clickhousectl ... --query ...` the shell starts both processes at once, so
/// the CLI can reach `poll` before `cat` has written anything and would go on
/// to send the empty INSERT. A bounded wait closes that window. Nothing that
/// already knows its answer pays it: a regular file, `/dev/null`, a closed
/// pipe (`POLLHUP`) and a pipe that already holds data all make `poll` return
/// at once. The only invocation that waits is `--query` whose stdin is an open
/// pipe with nothing written yet — the harness case that must never hang — and
/// it waits this long rather than forever.
const STDIN_FIRST_BYTE_TIMEOUT_MS: libc::c_int = 250;

/// Classify stdin without consuming it and without blocking indefinitely.
///
/// `poll` answers "would a read block?" without reading; only once it says a
/// read is ready do we `fill_buf`, which performs one syscall and leaves the
/// bytes in the shared stdin buffer, so the other SQL sources are unaffected.
/// A hangup without readable data (`POLLHUP` alone) is an empty stdin, not
/// input. The deliberate residual is now narrow: a producer that has not
/// written its first byte within [`STDIN_FIRST_BYTE_TIMEOUT_MS`] still reads
/// as no input, which is the price of never hanging on a pipe nobody is
/// writing to.
fn stdin_input() -> StdinInput {
    use std::io::BufRead as _;

    if std::io::stdin().is_terminal() {
        return StdinInput::Terminal;
    }

    if !fd_becomes_readable(libc::STDIN_FILENO, STDIN_FIRST_BYTE_TIMEOUT_MS) {
        return StdinInput::Empty;
    }

    match std::io::stdin().lock().fill_buf() {
        Ok(buffered) if !buffered.is_empty() => StdinInput::Pending,
        _ => StdinInput::Empty,
    }
}

/// Whether a read on `fd` would find something within `timeout_ms`. A hangup
/// with nothing readable (`POLLHUP` alone, a closed pipe) reports `false`, and
/// so does a `poll` that is interrupted or errors: the CLI fails open to "no
/// input" rather than refusing a run it cannot justify refusing. Split out
/// from [`stdin_input`] so the timeout is testable against a real pipe without
/// touching the process's own stdin.
fn fd_becomes_readable(fd: libc::c_int, timeout_ms: libc::c_int) -> bool {
    let mut poll_fd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: a single initialized `pollfd` is passed with a matching length,
    // and the bounded timeout keeps the call from borrowing anything beyond
    // its own return.
    let ready = unsafe { libc::poll(&mut poll_fd, 1, timeout_ms) };
    ready > 0 && poll_fd.revents & libc::POLLIN != 0
}

/// Decode SQL at the Query API boundary without replacing invalid bytes.
///
/// This happens before service lookup, so the native fallback deliberately
/// uses placeholders instead of guessing an endpoint or handling credentials.
fn decode_query_sql(bytes: Vec<u8>, source: &str) -> CloudResult<String> {
    String::from_utf8(bytes).map_err(|_| {
        CloudError::new(format!(
            concat!(
                "SQL from {} is not valid UTF-8. The Query API accepts only UTF-8 SQL ",
                "and text-format data; use the native protocol for binary formats such as ",
                "RowBinary or Native, and for bulk input. Install the client with ",
                "`clickhousectl local use latest`, then stream the statement and data with:\n  ",
                "printf 'INSERT INTO <table> FORMAT RowBinary\\n' | cat - <data-file> | ",
                "clickhousectl local client --host <nativesecure-host> --port 9440 -- --secure ",
                "--user default --password '<password>'\nFind the native host and port with ",
                "`clickhousectl cloud service get <service-id>`."
            ),
            source
        ))
        .with_failure(ApiFailure::new(FailureKind::Io))
    })
}

fn read_query_sql(inline: Option<&str>, queries_file: Option<&str>) -> CloudResult<String> {
    use std::io::Read as _;

    if let Some(query) = inline {
        if query.trim().is_empty() {
            return Err(CloudError::new("--query was empty"));
        }
        if inline_query_conflicts_with_stdin(inline, stdin_input()) {
            return Err(CloudError::new(INLINE_QUERY_WITH_STDIN_ERROR));
        }
        return Ok(query.to_string());
    }

    if let Some(path) = queries_file {
        let (bytes, source) = if path == "-" {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            (bytes, "stdin".to_string())
        } else {
            (std::fs::read(path)?, format!("queries file '{}'", path))
        };
        let content = decode_query_sql(bytes, &source)?;
        if content.trim().is_empty() {
            return Err(CloudError::new("queries file was empty"));
        }
        return Ok(content);
    }

    if std::io::stdin().is_terminal() {
        return Err(CloudError::new(
            "no SQL provided. Pass --query, --queries-file, or pipe SQL on stdin.",
        ));
    }

    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes)?;
    let content = decode_query_sql(bytes, "stdin")?;
    if content.trim().is_empty() {
        return Err(CloudError::new("no SQL received on stdin"));
    }
    Ok(content)
}

fn default_query_format() -> String {
    if std::io::stdout().is_terminal() {
        "PrettyCompact".to_string()
    } else {
        "TabSeparated".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_401_or_403_counts_as_a_stored_query_key_rejection() {
        for status in [401, 403] {
            let error = clickhouse_cloud_api::Error::Api {
                status,
                message: "rejected".into(),
            };
            assert_eq!(stored_query_key_rejection_status(&error), Some(status));
        }
        assert_eq!(
            stored_query_key_rejection_status(&clickhouse_cloud_api::Error::Api {
                status: 404,
                message: "missing".into(),
            }),
            None
        );
    }

    #[test]
    fn query_output_completion_acknowledges_empty_responses() {
        assert_eq!(
            query_output_completion("TabSeparated", 0, None),
            QueryOutputCompletion::Acknowledge
        );
        assert_eq!(
            query_output_completion("RowBinary", 0, None),
            QueryOutputCompletion::Acknowledge
        );
    }

    #[test]
    fn query_output_completion_adds_only_a_missing_text_newline() {
        for format in ["TabSeparated", "PrettyCompact", "JSONEachRow"] {
            assert_eq!(
                query_output_completion(format, 2, Some(b'K')),
                QueryOutputCompletion::Newline
            );
        }
        assert_eq!(
            query_output_completion("JSONEachRow", 2, Some(b'\n')),
            QueryOutputCompletion::None
        );
    }

    #[test]
    fn query_output_completion_preserves_exact_and_binary_bodies() {
        for format in [
            "Template",
            "Buffers",
            "BSONEachRow",
            "RowBinary",
            "Native",
            "Parquet",
            "ArrowStream",
            "MsgPack",
        ] {
            assert_eq!(
                query_output_completion(format, 3, Some(0)),
                QueryOutputCompletion::None
            );
        }
    }

    #[test]
    fn query_readiness_is_only_auth_or_generic_not_found() {
        for status in [401, 403, 404] {
            assert!(query_endpoint_readiness_error(
                &clickhouse_cloud_api::Error::Api {
                    status,
                    message: "rejected".into(),
                }
            ));
        }
        for status in [400, 408, 429, 500] {
            assert!(!query_endpoint_readiness_error(
                &clickhouse_cloud_api::Error::Api {
                    status,
                    message: "query failed".into(),
                }
            ));
        }
        // A SQL-level rejection proves the endpoint *is* usable, and it is
        // recognized by its variant rather than by its message text.
        assert!(!query_endpoint_readiness_error(
            &clickhouse_cloud_api::Error::Sql {
                status: 404,
                code: "60".into(),
                details: "Unknown table".into(),
            }
        ));
        assert!(!query_endpoint_readiness_error(
            &clickhouse_cloud_api::Error::ServiceStopped
        ));
    }

    #[tokio::test]
    async fn query_does_not_provision_or_retry_transport_errors() {
        let transport_error = reqwest::Client::new()
            .get("not a url")
            .send()
            .await
            .unwrap_err();
        assert!(!query_endpoint_readiness_error(
            &clickhouse_cloud_api::Error::Http(transport_error)
        ));
    }

    #[tokio::test]
    async fn query_readiness_deadline_bounds_a_stalled_attempt() {
        let readiness = QueryEndpointReadiness {
            timeout: Duration::from_millis(10),
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        };
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            wait_for_query_endpoint_readiness(readiness, || {
                std::future::pending::<Result<(), clickhouse_cloud_api::Error>>()
            }),
        )
        .await
        .expect("stalled attempt exceeded the test guard");

        assert_query_readiness_timeout(result, readiness.timeout);
    }

    // ── post-repair verification (issue #658) ───────────────────────────

    /// The service under repair; a UUID because the service model's `id` is one.
    const VERIFIED_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";

    const FAST_READINESS: QueryEndpointReadiness = QueryEndpointReadiness {
        timeout: Duration::from_millis(60),
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(5),
    };

    async fn mount_running_service(control: &wiremock::MockServer) {
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(format!(
                "/v1/organizations/org-1/services/{VERIFIED_SERVICE_ID}"
            )))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": { "id": VERIFIED_SERVICE_ID, "name": "demo", "state": "running" },
                    "status": 200,
                    "requestId": "stub-service-get"
                })),
            )
            .mount(control)
            .await;
    }

    fn probes_received(requests: &[wiremock::Request]) -> usize {
        requests
            .iter()
            .filter(|request| request.url.path() == format!("/service/{VERIFIED_SERVICE_ID}/run"))
            .count()
    }

    #[tokio::test]
    async fn a_repair_verification_timeout_is_a_generic_error_with_its_own_code() {
        let control = wiremock::MockServer::start().await;
        mount_running_service(&control).await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(format!(
                "/service/{VERIFIED_SERVICE_ID}/run"
            )))
            .respond_with(
                wiremock::ResponseTemplate::new(401).set_body_string("API key is not authorized"),
            )
            .mount(&control)
            .await;
        let client = CloudClient::for_tests(&control.uri(), Some(&control.uri()));

        let error = probe_repaired_query_key(
            &client,
            "org-1",
            VERIFIED_SERVICE_ID,
            "new-key-uuid",
            "kid",
            "sec",
            FAST_READINESS,
        )
        .await
        .expect_err("a key rejected for the whole window is an error");

        // Not an auth failure of the command: the management credentials
        // worked, and exit code 4 would send the user to `cloud auth`.
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);
        assert!(
            error.message.starts_with(&format!(
                "the query key for service {VERIFIED_SERVICE_ID} was replaced (new API key \
                 new-key-uuid) and stored in .clickhouse/credentials.json, but the Query API \
                 kept rejecting it"
            )),
            "{}",
            error.message
        );
        assert!(
            error.message.contains("Do not rerun repair-query-key"),
            "{}",
            error.message
        );
        assert_eq!(
            error.failure.map(|failure| failure.kind),
            Some(FailureKind::Timeout)
        );
        let details = error.details.expect("the timeout has a structured form");
        assert_eq!(details.code, CloudErrorCode::QueryKeyRepairUnverified);
        assert_eq!(details.api_key_id.as_deref(), Some("new-key-uuid"));
        assert_eq!(
            details.command.as_deref(),
            Some(
                format!(
                    "clickhousectl cloud service query {VERIFIED_SERVICE_ID} --org-id org-1 \
                     --query \"SELECT 1\""
                )
                .as_str()
            )
        );
        assert_eq!(details.message, error.message);
        assert!(probes_received(&control.received_requests().await.unwrap()) >= 2);
    }

    #[tokio::test]
    async fn a_repair_verification_succeeds_once_the_probe_is_accepted() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let control = wiremock::MockServer::start().await;
        mount_running_service(&control).await;
        let attempts = Arc::new(AtomicUsize::new(0));
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(format!(
                "/service/{VERIFIED_SERVICE_ID}/run"
            )))
            .respond_with(move |_: &wiremock::Request| {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    wiremock::ResponseTemplate::new(401).set_body_string("not yet")
                } else {
                    wiremock::ResponseTemplate::new(200).set_body_string("1\n")
                }
            })
            .mount(&control)
            .await;
        let client = CloudClient::for_tests(&control.uri(), Some(&control.uri()));

        let verification = probe_repaired_query_key(
            &client,
            "org-1",
            VERIFIED_SERVICE_ID,
            "new-key-uuid",
            "kid",
            "sec",
            FAST_READINESS,
        )
        .await
        .unwrap();
        assert_eq!(verification, RepairVerification::Verified);
        assert_eq!(
            probes_received(&control.received_requests().await.unwrap()),
            2
        );
    }

    #[tokio::test]
    async fn a_repair_verification_without_a_configured_query_host_skips_without_probing() {
        let control = wiremock::MockServer::start().await;
        mount_running_service(&control).await;
        // A loopback base URL derives no query host, and no override is given.
        let client = CloudClient::for_tests(&control.uri(), None);
        let verification = probe_repaired_query_key(
            &client,
            "org-1",
            VERIFIED_SERVICE_ID,
            "new-key-uuid",
            "kid",
            "sec",
            FAST_READINESS,
        )
        .await
        .unwrap();
        assert_eq!(verification, RepairVerification::Skipped);
        assert!(control.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_repair_verification_skips_without_probing_when_the_record_is_gone() {
        // No credentials file names this service, so there is nothing to
        // probe with; the mock sees no request at all.
        let control = wiremock::MockServer::start().await;
        mount_running_service(&control).await;
        let client = CloudClient::for_tests(&control.uri(), Some(&control.uri()));
        let verification = verify_repaired_query_key(
            &client,
            "org-1",
            "svc-1-with-no-stored-record",
            "new-key-uuid",
            FAST_READINESS,
        )
        .await
        .unwrap();
        assert_eq!(verification, RepairVerification::Skipped);
        assert!(control.received_requests().await.unwrap().is_empty());
    }

    #[test]
    fn the_repaired_credential_is_probed_only_while_the_record_names_the_new_key() {
        let stored = |api_key_id: &str| credentials::ServiceQueryKey {
            organization_id: Some("org-1".into()),
            api_key_id: Some(api_key_id.into()),
            key_id: "kid".into(),
            key_secret: "sec".into(),
            endpoint_id: Some("ep-1".into()),
            pending_cleanup_api_key_ids: vec![],
            service_name: "demo".into(),
            created_at: chrono::Utc::now(),
        };
        assert_eq!(
            repaired_query_credential(Ok(Some(stored("new"))), "svc-1", "new"),
            Ok(("kid".to_string(), "sec".to_string()))
        );
        // A concurrent repair replaced the key, or the record vanished.
        let changed = "the stored query key for service svc-1 changed before the replacement \
                       could be probed";
        assert_eq!(
            repaired_query_credential(Ok(Some(stored("newer"))), "svc-1", "new"),
            Err(changed.to_string())
        );
        assert_eq!(
            repaired_query_credential(Ok(None), "svc-1", "new"),
            Err(changed.to_string())
        );
    }

    #[test]
    fn a_credentials_read_error_after_a_committed_repair_skips_rather_than_fails() {
        // The repair is committed by the time the record is re-read, so an
        // unreadable or unparsable credentials file skips the probe like every
        // other "not this run's to verify" case. Only a full-window Query API
        // rejection may exit 1.
        let reason = repaired_query_credential(
            Err(CloudError::new("credentials.json is not valid JSON")),
            "svc-1",
            "new",
        )
        .unwrap_err();
        assert_eq!(
            reason,
            "the stored query key for service svc-1 could not be read (credentials.json is not \
             valid JSON)"
        );
    }

    #[tokio::test]
    async fn query_readiness_exhaustion_is_not_an_auth_error() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let readiness = QueryEndpointReadiness {
            timeout: Duration::from_millis(20),
            initial_backoff: Duration::from_millis(20),
            max_backoff: Duration::from_millis(20),
        };
        let attempts = Arc::new(AtomicUsize::new(0));
        let probe_attempts = Arc::clone(&attempts);
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            wait_for_query_endpoint_readiness(readiness, move || {
                probe_attempts.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Err::<(), _>(clickhouse_cloud_api::Error::Api {
                    status: 401,
                    message: "query endpoint is not ready".into(),
                }))
            }),
        )
        .await
        .expect("readiness retries exceeded the test guard");

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_query_readiness_timeout(result, readiness.timeout);
    }

    fn assert_query_readiness_timeout<T>(
        result: Result<T, QueryReadinessError>,
        timeout: Duration,
    ) {
        let elapsed = match result {
            Err(QueryReadinessError::TimedOut(elapsed)) => elapsed,
            Err(QueryReadinessError::Api(error)) => panic!("expected the deadline, got {error}"),
            Ok(_) => panic!("readiness should time out"),
        };
        assert_eq!(elapsed, timeout);
        let converted = query_readiness_timeout_error(elapsed);

        assert_eq!(
            converted.kind,
            crate::cloud::client::CloudErrorKind::Generic
        );
        assert_eq!(
            converted.message,
            format!("Query API endpoint did not become ready within {timeout:?}")
        );
        // The readiness deadline is reported as a timeout (#450), not as a
        // generic 4xx: the CLI, not the API, gave up — so there is no HTTP
        // status to report either.
        assert_eq!(
            converted.failure,
            Some(ApiFailure::new(FailureKind::Timeout))
        );
    }

    /// A query target for the error-rendering tests. `native` decides whether
    /// the timeout hint has a real endpoint to name.
    fn test_query_target(native: Option<&NativeEndpoint>) -> QueryTarget<'_> {
        QueryTarget {
            service_name: "demo",
            service_id: "svc-1",
            org_id: "org-1",
            native,
            may_be_waking: false,
        }
    }

    #[test]
    fn native_secure_endpoint_picks_the_native_protocol_endpoint() {
        let endpoints = vec![
            ServiceEndpoint {
                host: Some("demo.gcp.clickhouse.cloud".into()),
                port: Some(8443),
                protocol: Some(ServiceEndpointProtocol::Https),
                username: None,
            },
            ServiceEndpoint {
                host: Some("demo.gcp.clickhouse.cloud".into()),
                port: Some(9440),
                protocol: Some(ServiceEndpointProtocol::Nativesecure),
                username: Some("default".into()),
            },
        ];
        assert_eq!(
            native_secure_endpoint(Some(&endpoints)),
            Some(NativeEndpoint {
                host: "demo.gcp.clickhouse.cloud".into(),
                port: 9440,
            })
        );
    }

    #[test]
    fn native_secure_endpoint_is_absent_when_the_response_is_incomplete() {
        // No endpoints at all, and an empty list.
        assert_eq!(native_secure_endpoint(None), None);
        assert_eq!(native_secure_endpoint(Some(&[])), None);
        // Only the HTTPS endpoint.
        assert_eq!(
            native_secure_endpoint(Some(&[ServiceEndpoint {
                host: Some("demo.clickhouse.cloud".into()),
                port: Some(8443),
                protocol: Some(ServiceEndpointProtocol::Https),
                username: None,
            }])),
            None
        );
        // Every response field is optional, so a half-returned endpoint is
        // absent rather than half-rendered into a command.
        for partial in [
            ServiceEndpoint {
                host: Some("demo.clickhouse.cloud".into()),
                port: None,
                protocol: Some(ServiceEndpointProtocol::Nativesecure),
                username: None,
            },
            ServiceEndpoint {
                host: None,
                port: Some(9440),
                protocol: Some(ServiceEndpointProtocol::Nativesecure),
                username: None,
            },
            ServiceEndpoint {
                host: Some("demo.clickhouse.cloud".into()),
                port: Some(9440),
                protocol: None,
                username: None,
            },
        ] {
            assert_eq!(
                native_secure_endpoint(Some(std::slice::from_ref(&partial))),
                None
            );
        }
    }

    /// The gateway timeout names the limit, warns before a rerun, and hands
    /// over a native-protocol command built from the service's own endpoint
    /// (#644).
    #[test]
    fn query_timeout_error_hints_the_native_endpoint() {
        let native = NativeEndpoint {
            host: "demo.gcp.clickhouse.cloud".into(),
            port: 9440,
        };
        let error = query_timeout_error(test_query_target(Some(&native)));

        assert!(
            error.message.contains("about 30 seconds"),
            "{}",
            error.message
        );
        assert!(
            error
                .message
                .contains("SELECT query_id, elapsed FROM system.processes"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains("clickhousectl local use latest"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains(
                "clickhouse client --host demo.gcp.clickhouse.cloud --secure --port 9440 \
                 --user default --password '<password>' --query '<your SQL>'"
            ),
            "{}",
            error.message
        );
        assert!(
            error
                .message
                .contains("clickhousectl cloud service reset-password svc-1"),
            "{}",
            error.message
        );
        // With a real endpoint there is nothing to look up.
        assert!(
            !error.message.contains("cloud service get"),
            "{}",
            error.message
        );
        assert!(!error.message.contains("<host>"), "{}", error.message);

        // A timeout is exit code 1 (Generic), never an auth failure, and the
        // classification is the variant's (#450).
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);
        assert_eq!(
            error.failure,
            Some(ApiFailure::with_status(FailureKind::Timeout, 500))
        );

        let details = error.details.as_deref().expect("json details");
        assert_eq!(details.code, CloudErrorCode::QueryTimeout);
        assert_eq!(details.message, error.message);
        assert_eq!(details.host.as_deref(), Some("demo.gcp.clickhouse.cloud"));
        assert_eq!(details.port, Some(9440));
        assert_eq!(
            details.command.as_deref(),
            Some(
                "clickhouse client --host demo.gcp.clickhouse.cloud --secure --port 9440 --user \
                 default --password '<password>' --query '<your SQL>'"
            )
        );
    }

    #[test]
    fn query_timeout_after_wake_retains_structural_classification() {
        let client = CloudClient::new(
            Some("test-key"),
            Some("test-secret"),
            Some("https://api.example.com/v1"),
        )
        .unwrap();
        let error = QueryRequestError {
            error: clickhouse_cloud_api::Error::QueryTimeout,
            wake_requested: true,
        }
        .into_cloud_error(&client, test_query_target(None));
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);
        assert_eq!(
            error.failure,
            Some(ApiFailure::with_status(FailureKind::Timeout, 500))
        );
        let details = error.details.as_deref().unwrap();
        assert_eq!(details.code, CloudErrorCode::QueryTimeout);
        assert_eq!(details.message, error.message);
        assert_eq!(
            details.command.as_deref(),
            Some("clickhousectl cloud service get svc-1 --org-id org-1")
        );
        assert!(details.host.is_none());
        assert!(details.port.is_none());

        for status in [401, 403, 500] {
            let error = QueryRequestError {
                error: clickhouse_cloud_api::Error::Api {
                    status,
                    message: "unrelated failure".into(),
                },
                wake_requested: true,
            }
            .into_cloud_error(&client, test_query_target(None));
            assert!(!error.message.contains("waking"));
            assert_eq!(error.failure.unwrap().http_status, Some(status));
            assert_eq!(
                error.kind == crate::cloud::client::CloudErrorKind::Auth,
                status == 401 || status == 403
            );
        }
    }

    #[test]
    fn query_timeout_error_falls_back_to_a_host_placeholder() {
        let error = query_timeout_error(test_query_target(None));

        assert!(
            error.message.contains(
                "clickhouse client --host <host> --secure --port 9440 --user default \
                 --password '<password>' --query '<your SQL>'"
            ),
            "{}",
            error.message
        );
        // No endpoint in the response, so the message says where to get one.
        assert!(
            error
                .message
                .contains("clickhousectl cloud service get svc-1 --org-id org-1"),
            "{}",
            error.message
        );

        let details = error.details.as_deref().expect("json details");
        // Absent means omitted: no fabricated host or port.
        assert_eq!(details.host, None);
        assert_eq!(details.port, None);
        assert!(
            details
                .command
                .as_deref()
                .is_some_and(|command| command.contains("--host <host>")),
            "{details:?}"
        );
        let json = serde_json::to_value(details).expect("details serialize");
        assert_eq!(json["code"], "query_timeout");
        assert!(json.get("host").is_none(), "{json}");
        assert!(json.get("port").is_none(), "{json}");
    }

    /// The message is rendered from the target and fixed placeholders only,
    /// so neither the statement nor any credential can reach it.
    #[test]
    fn query_timeout_error_never_echoes_sql_or_credentials() {
        let native = NativeEndpoint {
            host: "demo.gcp.clickhouse.cloud".into(),
            port: 9440,
        };
        for target in [test_query_target(Some(&native)), test_query_target(None)] {
            let error = query_timeout_error(target);
            assert!(error.message.contains("'<your SQL>'"), "{}", error.message);
            assert!(
                error.message.contains("--password '<password>'"),
                "{}",
                error.message
            );
        }
    }

    /// Every query-path error a boundary rewrites keeps a classification
    /// derived from what actually failed, not from its message (#450).
    #[test]
    fn query_errors_carry_their_structural_classification() {
        let client = CloudClient::new(
            Some("test-key"),
            Some("test-secret"),
            Some("https://api.example.com/v1"),
        )
        .unwrap();

        // A stopped service gets a rewritten, actionable message; the
        // classification survives the rewrite.
        let stopped = convert_query_error(
            &client,
            clickhouse_cloud_api::Error::ServiceStopped,
            test_query_target(None),
        );
        assert!(stopped.message.contains("is stopped"));
        assert_eq!(
            stopped.failure,
            Some(ApiFailure::new(FailureKind::ServiceStopped))
        );

        let sql = convert_query_error(
            &client,
            clickhouse_cloud_api::Error::Sql {
                status: 400,
                code: "62".into(),
                details: "Syntax error".into(),
            },
            test_query_target(None),
        );
        assert_eq!(
            sql.failure,
            Some(ApiFailure::with_status(FailureKind::SqlError, 400))
        );

        // `--no-auto-enable` rewrites the message but the failure is still
        // the Query API's own rejection: its class and status survive.
        for status in [401, 403, 404] {
            let refused = refused_query_provisioning_error(
                "svc-1",
                &clickhouse_cloud_api::Error::Api {
                    status,
                    message: "rejected".into(),
                },
            );
            assert!(refused.message.contains("--no-auto-enable"), "{refused}");
            assert_eq!(refused.kind, crate::cloud::client::CloudErrorKind::Generic);
            assert_eq!(
                refused.failure,
                Some(ApiFailure::with_status(FailureKind::Http4xx, status)),
                "status {status}"
            );
        }
    }

    #[test]
    fn inline_query_only_conflicts_with_bytes_waiting_on_stdin() {
        // The whole matrix of stdin states crossed with --query being present
        // (#641). Only "not a terminal and data is already waiting" conflicts:
        // a terminal must never be read, and a non-terminal empty stdin is the
        // normal shape for CI and for coding agents.
        assert!(!inline_query_conflicts_with_stdin(
            Some("SELECT 1"),
            StdinInput::Terminal
        ));
        assert!(!inline_query_conflicts_with_stdin(
            Some("SELECT 1"),
            StdinInput::Empty
        ));
        assert!(inline_query_conflicts_with_stdin(
            Some("SELECT 1"),
            StdinInput::Pending
        ));

        assert!(!inline_query_conflicts_with_stdin(
            None,
            StdinInput::Terminal
        ));
        assert!(!inline_query_conflicts_with_stdin(None, StdinInput::Empty));
        // Without --query, piped bytes are the SQL itself, not a conflict.
        assert!(!inline_query_conflicts_with_stdin(
            None,
            StdinInput::Pending
        ));
    }

    /// A real unnamed pipe as `(read_fd, write_fd)`.
    fn test_pipe() -> (libc::c_int, libc::c_int) {
        let mut fds = [0 as libc::c_int; 2];
        // SAFETY: `fds` is a two-element array, which is what `pipe` writes.
        let created = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(created, 0, "pipe() failed");
        (fds[0], fds[1])
    }

    fn write_byte(fd: libc::c_int) {
        // SAFETY: one byte is written from a live local buffer.
        let written = unsafe { libc::write(fd, b"x".as_ptr().cast(), 1) };
        assert_eq!(written, 1, "write() failed");
    }

    fn close_fd(fd: libc::c_int) {
        // SAFETY: each fd is closed exactly once by its owning test.
        unsafe { libc::close(fd) };
    }

    #[test]
    fn readiness_waits_for_a_producer_that_has_not_written_its_first_byte() {
        // The #641 race: `cat data.csv | clickhousectl ... --query ...` starts
        // both processes at once, so the CLI can look at stdin before the
        // producer has written. A zero timeout calls that "no input" and sends
        // the empty INSERT, which is the whole bug; the bounded wait sees the
        // byte. This is asserted on a real pipe rather than through a
        // subprocess so the interleaving is not left to process startup.
        let (read_fd, write_fd) = test_pipe();
        assert!(
            !fd_becomes_readable(read_fd, 0),
            "nothing has been written yet"
        );

        let producer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            write_byte(write_fd);
            close_fd(write_fd);
        });

        let started = std::time::Instant::now();
        assert!(fd_becomes_readable(read_fd, STDIN_FIRST_BYTE_TIMEOUT_MS));
        assert!(
            started.elapsed() < Duration::from_millis(STDIN_FIRST_BYTE_TIMEOUT_MS as u64 + 500),
            "the wait must stay bounded"
        );

        producer.join().unwrap();
        close_fd(read_fd);
    }

    #[test]
    fn readiness_gives_up_on_a_pipe_nobody_writes_to() {
        // A harness that wired up stdin and produced nothing must not hang the
        // command: the wait expires and the run continues.
        let (read_fd, write_fd) = test_pipe();

        let started = std::time::Instant::now();
        assert!(!fd_becomes_readable(read_fd, STDIN_FIRST_BYTE_TIMEOUT_MS));
        let waited = started.elapsed();
        assert!(
            waited < Duration::from_millis(STDIN_FIRST_BYTE_TIMEOUT_MS as u64 + 500),
            "waited {waited:?}"
        );

        close_fd(write_fd);
        close_fd(read_fd);
    }

    #[test]
    fn readiness_is_immediate_for_data_that_is_already_waiting() {
        let (read_fd, write_fd) = test_pipe();
        write_byte(write_fd);

        assert!(fd_becomes_readable(read_fd, 0));

        close_fd(write_fd);
        close_fd(read_fd);
    }

    #[test]
    fn the_stdin_conflict_message_says_what_does_work() {
        assert!(
            INLINE_QUERY_WITH_STDIN_ERROR
                .starts_with("--query cannot be combined with SQL or data on stdin.")
        );
        assert!(INLINE_QUERY_WITH_STDIN_ERROR.contains("cat - data.csv"));
        assert!(INLINE_QUERY_WITH_STDIN_ERROR.contains("--queries-file -"));
    }
}
