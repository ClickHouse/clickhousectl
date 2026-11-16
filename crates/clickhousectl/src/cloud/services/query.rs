use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult, read_flag_file_bytes};
use crate::cloud::output::{CloudErrorCode, CloudErrorDetail, or_absent};
use crate::cloud::services::query_key::CallerKeyBinding;
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

/// [`QUERY_ENDPOINT_READINESS`], with the hidden test override of its timeout.
fn query_endpoint_readiness() -> QueryEndpointReadiness {
    QueryEndpointReadiness {
        timeout: crate::timing::duration_override(
            crate::timing::QUERY_READINESS_TIMEOUT_ENV,
            QUERY_ENDPOINT_READINESS.timeout,
        ),
        ..QUERY_ENDPOINT_READINESS
    }
}

/// Resolve a service by name or ID within the given org.
/// Exactly one of `name` or `id` must be provided.
pub(super) async fn resolve_service(
    client: &CloudClient,
    org_id: &str,
    name: Option<&str>,
    id: Option<&str>,
) -> CloudResult<Service> {
    match (name, id) {
        (Some(name), None) => {
            let services = client.list_services(org_id).await?;
            let id = crate::cloud::shared::select_named_id(
                crate::cloud::client::ResourceKind::Service,
                crate::cloud::client::Scope::Organization(org_id),
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
        (Some(_), Some(_)) => Err(CloudError::usage(
            "supply either a positional service ID or --name",
        )),
        // Clap rejects this state; retain the check for any future non-CLI caller.
        (None, None) => Err(CloudError::usage(
            "supply a positional service ID or --name to identify the service",
        )),
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
            eprintln!("Waiting for the Query API endpoint to become ready...");
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

/// Whether `error`, the management key's rejection of a statement, refused
/// the statement rather than the key (#1240): a role missing a grant, say.
///
/// The Query API sends both refusals as the same `403`, so the endpoint is
/// asked directly with a harmless `SELECT 1` under the same credential. If it
/// answers, the key is bound and accepted, so binding cannot fix the refusal
/// and re-running the statement cannot change it. Only a `403` is probed: a
/// `401` refuses the credential and a `404` means there is no endpoint. A
/// probe that fails for any reason proves nothing, and the caller binds as
/// before.
async fn statement_refused_by_usable_endpoint(
    client: &CloudClient,
    service_id: &str,
    key_id: &str,
    key_secret: &str,
    error: &clickhouse_cloud_api::Error,
) -> bool {
    matches!(error, clickhouse_cloud_api::Error::Api { status: 403, .. })
        && client
            .run_service_query(
                service_id,
                key_id,
                key_secret,
                "SELECT 1",
                None,
                "TabSeparated",
                false,
            )
            .await
            .is_ok()
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

    // A Query API key stored for this service by an earlier version is no
    // longer used: drop its record and say which key is left in the
    // organization (#1325). Never blocks the query.
    crate::cloud::services::query_key::retire_legacy_query_key_record(&service_id, &org_id);

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
        let result = {
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
                Err(error)
                    if query_endpoint_readiness_error(&error.error)
                        && !statement_refused_by_usable_endpoint(
                            client,
                            &service_id,
                            key_id,
                            key_secret,
                            &error.error,
                        )
                        .await =>
                {
                    if options.no_auto_enable {
                        failure::set_provisioning_state(ProvisioningState::Refused);
                        return Err(refused_query_provisioning_error(&service_id, &error.error)
                            .at_stage(FailureStage::QueryRequest));
                    }
                    eprintln!(
                        "Binding the authenticated API key to the Query API endpoint for \
                         service '{}'...",
                        service_name
                    );
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
                                query_endpoint_readiness(),
                            )
                            .await
                        }
                        // Nothing was written. A concurrent run may have just
                        // bound the key and its change may not have reached
                        // the Query API yet, so wait for the endpoint as the
                        // binder does; only a key still rejected for the whole
                        // window means rebinding cannot help.
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
                                query_endpoint_readiness(),
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
        QueryOutputCompletion::Acknowledge => eprintln!("OK"),
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
    eprintln!("Service '{service_name}' is idle; waking it (this may take a minute)...");
}

/// A service's native-protocol (`nativesecure`) endpoint.
///
/// Both halves are required. A hint that named the real host but guessed the
/// port would hand the user a command that does not connect, which is worse
/// than a command that visibly needs filling in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeEndpoint {
    pub(super) host: String,
    pub(super) port: i64,
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
pub(super) fn native_secure_endpoint(
    endpoints: Option<&[ServiceEndpoint]>,
) -> Option<NativeEndpoint> {
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

/// The `cloud service client` command that reruns the statement over the
/// native protocol, signed in as the user's Cloud identity: no password.
///
/// The SQL is always a literal placeholder: the user's statement is never
/// echoed back into an error message.
fn native_client_command(target: QueryTarget<'_>) -> String {
    format!(
        "clickhousectl cloud service client {} --org-id {} --query '<your SQL>'",
        target.service_id, target.org_id
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
        let command = native_client_command(target);
        let message = format!(
            "the query timed out at the Query API gateway, which stops waiting after about 30 \
             seconds.\nThe statement may still be running on the service or may already have \
             completed; check `SELECT query_id, elapsed FROM system.processes` and verify its \
             outcome before running it again.\n\nHint: run long statements over the native \
             protocol instead. Install ClickHouse 25.12 or newer with\n  clickhousectl local use \
             latest\nthen rerun the query with\n  {command}\nIt signs in as your Cloud user \
             through a browser; no password is needed."
        );
        CloudErrorDetail {
            code: CloudErrorCode::QueryTimeout,
            message,
            host: target.native.map(|native| native.host.clone()),
            port: target.native.map(|native| native.port),
            command: Some(command),
            api_key_id: None,
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
     <service-id>. Or read a whole statement from stdin with --queries-file -.";

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
/// uses a service-ID placeholder instead of guessing one.
fn decode_query_sql(bytes: Vec<u8>, source: &str) -> CloudResult<String> {
    String::from_utf8(bytes).map_err(|_| {
        CloudError::new(format!(
            concat!(
                "SQL from {} is not valid UTF-8. The Query API accepts only UTF-8 SQL ",
                "and text-format data; use the native protocol for binary formats such as ",
                "RowBinary or Native, and for bulk input. Install ClickHouse 25.12 or newer ",
                "with `clickhousectl local use latest`, then stream the data with:\n  ",
                "clickhousectl cloud service client <service-id> --query ",
                "'INSERT INTO <table> FORMAT RowBinary' < <data-file>\n",
                "It signs in as your Cloud user through a browser; no password is needed."
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
            return Err(CloudError::usage("--query was empty"));
        }
        if inline_query_conflicts_with_stdin(inline, stdin_input()) {
            return Err(CloudError::usage(INLINE_QUERY_WITH_STDIN_ERROR));
        }
        return Ok(query.to_string());
    }

    if let Some(path) = queries_file {
        let (bytes, source) = if path == "-" {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            (bytes, "stdin".to_string())
        } else {
            (
                read_flag_file_bytes("--queries-file", path)?,
                format!("queries file '{}'", path),
            )
        };
        let content = decode_query_sql(bytes, &source)?;
        if content.trim().is_empty() {
            return Err(CloudError::usage("queries file was empty"));
        }
        return Ok(content);
    }

    if std::io::stdin().is_terminal() {
        return Err(CloudError::usage(
            "no SQL provided. Pass --query, --queries-file, or pipe SQL on stdin.",
        ));
    }

    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes)?;
    let content = decode_query_sql(bytes, "stdin")?;
    if content.trim().is_empty() {
        return Err(CloudError::usage("no SQL received on stdin"));
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
    use crate::cloud::client::CloudErrorKind;

    #[test]
    fn empty_sql_is_a_usage_error_and_an_unreadable_file_is_io() {
        let directory = tempfile::tempdir().unwrap();
        let empty = directory.path().join("empty.sql");
        std::fs::write(&empty, " \n").unwrap();
        for error in [
            read_query_sql(Some(" "), None).unwrap_err(),
            read_query_sql(None, empty.to_str()).unwrap_err(),
        ] {
            assert_eq!(error.kind, CloudErrorKind::Usage, "{error}");
        }

        let missing = directory.path().join("missing.sql");
        let error = read_query_sql(None, missing.to_str()).unwrap_err();
        assert_eq!(error.kind, CloudErrorKind::Generic);
        assert!(error.message.contains("--queries-file"), "{error}");
        assert!(error.message.contains(missing.to_str().unwrap()), "{error}");
        assert_eq!(
            error.failure.map(|failure| failure.kind),
            Some(FailureKind::Io)
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
        // The message reports the bound that elapsed.
        assert!(
            converted.message.contains(&format!("{timeout:?}")),
            "{}",
            converted.message
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

    /// The gateway timeout warns before a rerun and hands over a
    /// `cloud service client` command for the same service and org (#644,
    /// #1154): one that clap accepts, needs no password, and never carries
    /// the statement.
    #[test]
    fn query_timeout_error_hints_cloud_service_client() {
        let native = NativeEndpoint {
            host: "demo.gcp.clickhouse.cloud".into(),
            port: 9440,
        };
        for native in [Some(&native), None] {
            let error = query_timeout_error(test_query_target(native));

            // A timeout is exit code 1 (Generic), never an auth failure, and
            // the classification is the variant's (#450).
            assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);
            assert_eq!(
                error.failure,
                Some(ApiFailure::with_status(FailureKind::Timeout, 500))
            );

            let details = error.details.as_deref().expect("json details");
            assert_eq!(details.code, CloudErrorCode::QueryTimeout);
            assert_eq!(details.message, error.message);
            // The endpoint fields are the API's, or omitted: never guessed.
            assert_eq!(details.host.as_deref(), native.map(|n| n.host.as_str()));
            assert_eq!(details.port, native.map(|n| n.port));

            let command = details.command.as_deref().expect("suggested command");
            assert!(error.message.contains(command), "{}", error.message);
            let argv = shlex::split(command).expect("shell-quoted command");
            assert_eq!(argv[0], "clickhousectl");
            let cli = <crate::cli::Cli as clap::Parser>::try_parse_from(&argv)
                .unwrap_or_else(|e| panic!("{command}: {e}"));
            let crate::cli::Commands::Cloud(cloud) = cli.command else {
                panic!("expected a cloud command: {command}");
            };
            let cloud = *cloud;
            assert_eq!(cloud.org_id.as_deref(), Some("org-1"));
            let crate::cloud::cli::CloudCommands::Service {
                command: crate::cloud::cli::ServiceCommands::Client(args),
            } = cloud.command
            else {
                panic!("expected `cloud service client`: {command}");
            };
            assert_eq!(args.service_id.id.as_deref(), Some("svc-1"));
            assert_eq!(args.query, ["<your SQL>"]);
            assert!(args.args.is_empty());
        }
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
            // Only a timeout is reworded for a waking service; anything else
            // converts exactly as it would without the wake.
            let plain = client.convert_error(clickhouse_cloud_api::Error::Api {
                status,
                message: "unrelated failure".into(),
            });
            assert_eq!(error.message, plain.message);
            assert!(error.details.is_none());
            assert_eq!(error.failure.unwrap().http_status, Some(status));
            assert_eq!(error.kind, plain.kind);
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
        // Rewritten to name the service and the command that starts it.
        assert!(stopped.message.contains("'demo'"), "{stopped}");
        assert!(
            stopped
                .message
                .contains("`clickhousectl cloud service start svc-1 --org-id org-1`"),
            "{stopped}"
        );
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
}
