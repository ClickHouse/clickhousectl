//! Privacy-safe runtime failure classification (#450).
//!
//! Anonymous telemetry used to describe every non-auth runtime failure of
//! `cloud service query` as `outcome=error, exit_code=1`, which made 6,444
//! exit-1 events indistinguishable: a syntax error, a stopped service, a
//! rate-limited provisioning burst and a broken pipe all looked the same.
//! This module adds the missing axes as **closed, definition-owned
//! categories**:
//!
//! * [`FailureStage`] — which stage of the run failed;
//! * [`FailureKind`] — what kind of failure it was;
//! * a bounded HTTP status ([`bounded_status`]);
//! * a retry-count bucket, the query-credential [`ProvisioningState`], and a
//!   duration bucket.
//!
//! # The privacy boundary
//!
//! Every category is a Rust enum defined here; every serialized value is a
//! `&'static str` returned by an `as_str` method or a `u16` that survived the
//! [`REPORTABLE_STATUSES`] allowlist. Nothing in the recorded state can hold
//! an owned `String`, so SQL text, identifiers, file paths, response bodies
//! and credentials are *structurally* unable to reach the payload — the same
//! guarantee `telemetry::capture` gives for flag names, enforced by the types
//! rather than by scrubbing.
//!
//! Categories are set **only at code-owned error boundaries**: a
//! [`FailureKind`] comes from matching the library's typed error variant
//! ([`classify_api_error`]) or from the CLI operation that failed, never from
//! parsing an error message, and a [`FailureStage`] is supplied by the call
//! site that owns the stage. [`State::record`] is first-write-wins so the
//! innermost boundary — the one that actually knows what happened — survives
//! a coarser fallback recorded by an outer wrapper.
//!
//! The recorders are compiled unconditionally so call sites in the cloud
//! command modules need no `cfg` noise; without the `telemetry` feature
//! nothing reads the state back.
#![cfg_attr(not(feature = "telemetry"), allow(dead_code))]

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Which stage of a run failed. Closed vocabulary: a stage exists only if
/// some boundary in the CLI owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStage {
    /// Reading/validating the SQL the user supplied (`--query`,
    /// `--queries-file`, stdin) — before any network access.
    SqlInput,
    /// Resolving the organization to operate in.
    OrgResolution,
    /// Resolving the target service by name or id.
    ServiceResolution,
    /// Sending a statement to the Query API (including waiting for a
    /// just-provisioned endpoint to accept it).
    QueryRequest,
    /// Creating the auto-provisioned Query API key.
    KeyCreate,
    /// Reading the service's existing query-endpoint configuration.
    EndpointGet,
    /// Writing (upserting) the query-endpoint configuration.
    EndpointUpsert,
    /// Streaming a statement's response body to stdout.
    ResponseStream,
}

impl FailureStage {
    /// The wire value. Literal strings only, so this field can never carry
    /// user data.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SqlInput => "sql_input",
            Self::OrgResolution => "org_resolution",
            Self::ServiceResolution => "service_resolution",
            Self::QueryRequest => "query_request",
            Self::KeyCreate => "key_create",
            Self::EndpointGet => "endpoint_get",
            Self::EndpointUpsert => "endpoint_upsert",
            Self::ResponseStream => "response_stream",
        }
    }

    /// Every stage, for closed-vocabulary tests.
    #[cfg(test)]
    const ALL: &'static [Self] = &[
        Self::SqlInput,
        Self::OrgResolution,
        Self::ServiceResolution,
        Self::QueryRequest,
        Self::KeyCreate,
        Self::EndpointGet,
        Self::EndpointUpsert,
        Self::ResponseStream,
    ];
}

/// What kind of failure it was, derived from a typed error variant or from
/// the failing operation — never from message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// Local I/O: reading a queries file, writing to stdout, the credentials
    /// file.
    Io,
    /// The request never got an HTTP response (DNS, TLS, connect, reset).
    Transport,
    /// An HTTP 4xx that is not more specifically classified below.
    Http4xx,
    /// An HTTP 5xx.
    Http5xx,
    /// The service ran the request and rejected the SQL itself.
    SqlError,
    /// The service is stopped, so it cannot answer queries at all.
    ServiceStopped,
    /// A client- or server-side timeout, including waiting for a query
    /// endpoint to become usable.
    Timeout,
    /// HTTP 429.
    RateLimited,
    /// Anything the code-owned boundaries cannot place — including a missing
    /// required response field or an unusable local state.
    Other,
}

impl FailureKind {
    /// The wire value. Literal strings only.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io",
            Self::Transport => "transport",
            Self::Http4xx => "http_4xx",
            Self::Http5xx => "http_5xx",
            Self::SqlError => "sql_error",
            Self::ServiceStopped => "service_stopped",
            Self::Timeout => "timeout",
            Self::RateLimited => "rate_limited",
            Self::Other => "other",
        }
    }

    #[cfg(test)]
    const ALL: &'static [Self] = &[
        Self::Io,
        Self::Transport,
        Self::Http4xx,
        Self::Http5xx,
        Self::SqlError,
        Self::ServiceStopped,
        Self::Timeout,
        Self::RateLimited,
        Self::Other,
    ];
}

/// How far Query API credential provisioning had got when the run failed.
/// This is the axis that separates "a query failed" from "a provisioning
/// burst failed", which is what the concurrency reproduction in #450 could
/// not tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisioningState {
    /// The caller's own OAuth bearer token was used; no key provisioning is
    /// possible on this path.
    Bearer,
    /// The per-service key from `.clickhouse/credentials.json` was used.
    StoredKey,
    /// The authenticated management API key was used directly.
    ManagementKey,
    /// Provisioning a key and endpoint was in flight.
    Provisioning,
    /// Provisioning completed during this run and the query used the new key.
    Provisioned,
    /// Provisioning was required but `--no-auto-enable` forbade it.
    Refused,
}

impl ProvisioningState {
    /// The wire value. Literal strings only.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bearer => "bearer",
            Self::StoredKey => "stored_key",
            Self::ManagementKey => "management_key",
            Self::Provisioning => "provisioning",
            Self::Provisioned => "provisioned",
            Self::Refused => "refused",
        }
    }

    #[cfg(test)]
    const ALL: &'static [Self] = &[
        Self::Bearer,
        Self::StoredKey,
        Self::ManagementKey,
        Self::Provisioning,
        Self::Provisioned,
        Self::Refused,
    ];
}

/// HTTP statuses that may be reported exactly. A status outside the list is
/// dropped rather than sent: the list is the whole vocabulary of the field,
/// so a proxy or gateway inventing a status can never widen it. The class is
/// still readable from [`FailureKind`] (`http_4xx`/`http_5xx`/`rate_limited`/
/// `timeout`), so dropping the exact value loses only precision.
const REPORTABLE_STATUSES: &[u16] = &[
    206, 400, 401, 403, 404, 405, 408, 409, 413, 422, 429, 500, 501, 502, 503, 504,
];

/// The status, if it is one this field is allowed to carry.
pub fn bounded_status(status: u16) -> Option<u16> {
    REPORTABLE_STATUSES.contains(&status).then_some(status)
}

/// A failure kind plus its bounded HTTP status, as resolved at the boundary
/// that converted a typed library error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiFailure {
    pub kind: FailureKind,
    pub http_status: Option<u16>,
}

impl ApiFailure {
    pub fn new(kind: FailureKind) -> Self {
        Self {
            kind,
            http_status: None,
        }
    }

    pub fn with_status(kind: FailureKind, status: u16) -> Self {
        Self {
            kind,
            http_status: bounded_status(status),
        }
    }
}

/// Classify a library error from its **variant**, not its message.
///
/// This is the single place a `clickhouse_cloud_api::Error` becomes a
/// category, so every cloud command inherits the same mapping by converting
/// its errors through `CloudClient::convert_error`.
pub fn classify_api_error(error: &clickhouse_cloud_api::Error) -> ApiFailure {
    use clickhouse_cloud_api::Error as E;
    match error {
        E::Api { status, .. } => ApiFailure::with_status(status_kind(*status), *status),
        E::Sql { status, .. } => ApiFailure::with_status(FailureKind::SqlError, *status),
        E::Http(error) => ApiFailure::new(if error.is_timeout() {
            FailureKind::Timeout
        } else {
            FailureKind::Transport
        }),
        E::ServiceStopped => ApiFailure::new(FailureKind::ServiceStopped),
        // An idle service is normally handled by re-sending with the wake
        // confirmation; if the error escapes anyway it is a state problem,
        // not a transport or SQL one.
        E::ServiceIdle => ApiFailure::new(FailureKind::Other),
        E::Json(_) | E::AuthMismatch(_) => ApiFailure::new(FailureKind::Other),
        // The library error is `#[non_exhaustive]`: a variant this build does
        // not know is unclassified, never inspected through its message.
        _ => ApiFailure::new(FailureKind::Other),
    }
}

/// Kind for a plain HTTP status. `408`/`504` are timeouts and `429` is rate
/// limiting; everything else collapses to its class.
fn status_kind(status: u16) -> FailureKind {
    match status {
        429 => FailureKind::RateLimited,
        408 | 504 => FailureKind::Timeout,
        400..=499 => FailureKind::Http4xx,
        500..=599 => FailureKind::Http5xx,
        _ => FailureKind::Other,
    }
}

/// The first classified failure of the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Record {
    stage: FailureStage,
    kind: FailureKind,
    http_status: Option<u16>,
}

/// Everything the recorders accumulate for one process. Held behind a mutex
/// in [`STATE`]; the methods are pure so they can be unit-tested on a local
/// value with injected time, without touching the global.
#[derive(Debug, Default)]
struct State {
    record: Option<Record>,
    provisioning: Option<ProvisioningState>,
    retries: u32,
    span_start: Option<Instant>,
}

impl State {
    /// First write wins: an inner boundary that knows the exact stage keeps
    /// its classification even when an outer wrapper records a fallback.
    fn record(&mut self, stage: FailureStage, failure: ApiFailure) {
        if self.record.is_none() {
            self.record = Some(Record {
                stage,
                kind: failure.kind,
                http_status: failure.http_status,
            });
        }
    }

    /// Latest wins: provisioning is a progression, so the state the run
    /// reached is the interesting one.
    fn set_provisioning(&mut self, state: ProvisioningState) {
        self.provisioning = Some(state);
    }

    fn note_retry(&mut self) {
        self.retries = self.retries.saturating_add(1);
    }

    fn start_span(&mut self, now: Instant) {
        self.span_start = Some(now);
    }

    /// The wire view, or `None` when no failure was classified. Buckets are
    /// computed here so the state never holds a formatted value.
    fn snapshot(&self, now: Instant) -> Option<Snapshot> {
        let record = self.record?;
        Some(Snapshot {
            failure_stage: record.stage.as_str(),
            failure_kind: record.kind.as_str(),
            http_status: record.http_status,
            retry_bucket: retry_bucket(self.retries),
            provisioning_state: self.provisioning.map(ProvisioningState::as_str),
            duration_bucket: self
                .span_start
                .map(|start| duration_bucket(now.saturating_duration_since(start))),
        })
    }
}

/// The classified failure as it goes on the wire: `&'static str` categories
/// and a bounded status, nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub failure_stage: &'static str,
    pub failure_kind: &'static str,
    pub http_status: Option<u16>,
    pub retry_bucket: &'static str,
    pub provisioning_state: Option<&'static str>,
    pub duration_bucket: Option<&'static str>,
}

/// Retry attempts as a bucket, never an exact count: an exact count of a
/// long-running loop is a fingerprint of one run, a bucket is a shape.
fn retry_bucket(retries: u32) -> &'static str {
    match retries {
        0 => "0",
        1 => "1",
        2 => "2",
        3..=5 => "3_5",
        6..=10 => "6_10",
        _ => "gt_10",
    }
}

/// How long the run took before it failed, bucketed. Distinguishes a fast
/// rejection from a slow provisioning wait without timing any individual
/// request.
fn duration_bucket(elapsed: Duration) -> &'static str {
    match elapsed.as_millis() {
        0..250 => "lt_250ms",
        250..1_000 => "lt_1s",
        1_000..5_000 => "lt_5s",
        5_000..30_000 => "lt_30s",
        30_000..120_000 => "lt_2m",
        _ => "ge_2m",
    }
}

/// Process-wide recorded state. A `Mutex` rather than atomics so the record
/// and its buckets stay one consistent value; the critical sections are a
/// handful of field writes on an error path.
static STATE: Mutex<State> = Mutex::new(State {
    record: None,
    provisioning: None,
    retries: 0,
    span_start: None,
});

/// Run `f` against the global state. A poisoned mutex is recovered from
/// rather than propagated: telemetry classification must never turn a command
/// failure into a panic.
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

/// Start the timing span the duration bucket is measured against. Called at
/// the top of the operation being classified; before it is called, no
/// duration is reported at all.
pub fn start_span() {
    with_state(|state| state.start_span(Instant::now()));
}

/// Record a classified failure at its owning boundary. First write wins.
pub fn record(stage: FailureStage, failure: ApiFailure) {
    with_state(|state| state.record(stage, failure));
}

/// Note that the run reached a new provisioning state. Latest wins.
pub fn set_provisioning_state(state: ProvisioningState) {
    with_state(|current| current.set_provisioning(state));
}

/// Note one retry attempt (a wake re-send, a readiness probe).
pub fn note_retry() {
    with_state(State::note_retry);
}

/// The classified failure for this process, if any was recorded.
#[cfg(feature = "telemetry")]
pub fn snapshot() -> Option<Snapshot> {
    let now = Instant::now();
    with_state(|state| state.snapshot(now))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wire vocabularies are closed sets of literals, and each variant maps
    /// to a distinct one — a dashboard can enumerate them.
    #[test]
    fn category_vocabularies_are_closed_and_distinct() {
        let stages: Vec<&str> = FailureStage::ALL.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            stages,
            [
                "sql_input",
                "org_resolution",
                "service_resolution",
                "query_request",
                "key_create",
                "endpoint_get",
                "endpoint_upsert",
                "response_stream",
            ]
        );
        let kinds: Vec<&str> = FailureKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            kinds,
            [
                "io",
                "transport",
                "http_4xx",
                "http_5xx",
                "sql_error",
                "service_stopped",
                "timeout",
                "rate_limited",
                "other",
            ]
        );
        let provisioning: Vec<&str> = ProvisioningState::ALL.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            provisioning,
            [
                "bearer",
                "stored_key",
                "management_key",
                "provisioning",
                "provisioned",
                "refused",
            ]
        );

        for values in [stages, kinds, provisioning] {
            let unique: std::collections::BTreeSet<&str> = values.iter().copied().collect();
            assert_eq!(
                unique.len(),
                values.len(),
                "duplicate wire value in {values:?}"
            );
        }
    }

    #[test]
    fn api_errors_are_classified_from_the_variant_not_the_message() {
        use clickhouse_cloud_api::Error as E;

        let cases = [
            (
                E::Api {
                    status: 429,
                    message: "TOO_MANY_REQUESTS".into(),
                },
                FailureKind::RateLimited,
                Some(429),
            ),
            (
                E::Api {
                    status: 401,
                    message: "unauthorized".into(),
                },
                FailureKind::Http4xx,
                Some(401),
            ),
            (
                E::Api {
                    status: 408,
                    message: "Request Timeout".into(),
                },
                FailureKind::Timeout,
                Some(408),
            ),
            (
                E::Api {
                    status: 504,
                    message: "gateway timeout".into(),
                },
                FailureKind::Timeout,
                Some(504),
            ),
            (
                E::Api {
                    status: 503,
                    message: "unavailable".into(),
                },
                FailureKind::Http5xx,
                Some(503),
            ),
            // A message that looks like a SQL error but arrives in the plain
            // API variant is classified by the variant, not by the text.
            (
                E::Api {
                    status: 404,
                    message: "SQL error 60: Unknown table".into(),
                },
                FailureKind::Http4xx,
                Some(404),
            ),
            (
                E::Sql {
                    status: 400,
                    code: "62".into(),
                    details: "Syntax error".into(),
                },
                FailureKind::SqlError,
                Some(400),
            ),
            (E::ServiceStopped, FailureKind::ServiceStopped, None),
            (E::ServiceIdle, FailureKind::Other, None),
            (E::AuthMismatch("nope".into()), FailureKind::Other, None),
            // A status outside the allowlist keeps its class and drops the
            // exact value.
            (
                E::Api {
                    status: 418,
                    message: "teapot".into(),
                },
                FailureKind::Http4xx,
                None,
            ),
            (
                E::Api {
                    status: 0,
                    message: "no status".into(),
                },
                FailureKind::Other,
                None,
            ),
        ];

        for (error, kind, status) in cases {
            let failure = classify_api_error(&error);
            assert_eq!(failure.kind, kind, "wrong kind for {error:?}");
            assert_eq!(failure.http_status, status, "wrong status for {error:?}");
        }
    }

    #[tokio::test]
    async fn transport_errors_are_transport_and_timeouts_are_timeouts() {
        let transport = reqwest::Client::new()
            .get("http://127.0.0.1:1/never")
            .send()
            .await
            .unwrap_err();
        assert_eq!(
            classify_api_error(&clickhouse_cloud_api::Error::Http(transport)).kind,
            FailureKind::Transport
        );

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_delay(Duration::from_millis(200)),
            )
            .mount(&server)
            .await;
        let timeout = reqwest::Client::builder()
            .timeout(Duration::from_millis(20))
            .build()
            .unwrap()
            .get(server.uri())
            .send()
            .await
            .unwrap_err();
        assert!(timeout.is_timeout(), "expected a timeout error: {timeout}");
        assert_eq!(
            classify_api_error(&clickhouse_cloud_api::Error::Http(timeout)).kind,
            FailureKind::Timeout
        );
    }

    #[test]
    fn only_allowlisted_statuses_are_reportable() {
        for status in REPORTABLE_STATUSES {
            assert_eq!(bounded_status(*status), Some(*status));
        }
        for status in [0, 100, 200, 302, 418, 451, 599, 999] {
            assert_eq!(
                bounded_status(status),
                None,
                "status {status} must not be reportable"
            );
        }
    }

    #[test]
    fn buckets_are_bounded_labels() {
        assert_eq!(retry_bucket(0), "0");
        assert_eq!(retry_bucket(1), "1");
        assert_eq!(retry_bucket(2), "2");
        assert_eq!(retry_bucket(3), "3_5");
        assert_eq!(retry_bucket(5), "3_5");
        assert_eq!(retry_bucket(6), "6_10");
        assert_eq!(retry_bucket(10), "6_10");
        assert_eq!(retry_bucket(11), "gt_10");
        assert_eq!(retry_bucket(u32::MAX), "gt_10");

        assert_eq!(duration_bucket(Duration::ZERO), "lt_250ms");
        assert_eq!(duration_bucket(Duration::from_millis(249)), "lt_250ms");
        assert_eq!(duration_bucket(Duration::from_millis(250)), "lt_1s");
        assert_eq!(duration_bucket(Duration::from_millis(999)), "lt_1s");
        assert_eq!(duration_bucket(Duration::from_secs(1)), "lt_5s");
        assert_eq!(duration_bucket(Duration::from_secs(4)), "lt_5s");
        assert_eq!(duration_bucket(Duration::from_secs(5)), "lt_30s");
        assert_eq!(duration_bucket(Duration::from_secs(29)), "lt_30s");
        assert_eq!(duration_bucket(Duration::from_secs(30)), "lt_2m");
        assert_eq!(duration_bucket(Duration::from_secs(119)), "lt_2m");
        assert_eq!(duration_bucket(Duration::from_secs(120)), "ge_2m");
        assert_eq!(duration_bucket(Duration::from_secs(86_400)), "ge_2m");
    }

    #[test]
    fn no_recorded_failure_means_no_snapshot() {
        let mut state = State::default();
        let now = Instant::now();
        state.start_span(now);
        state.note_retry();
        state.set_provisioning(ProvisioningState::StoredKey);
        assert_eq!(
            state.snapshot(now),
            None,
            "buckets alone must not produce a snapshot: only a classified failure does"
        );
    }

    #[test]
    fn the_innermost_boundary_keeps_the_classification() {
        let mut state = State::default();
        state.record(
            FailureStage::EndpointUpsert,
            ApiFailure::with_status(FailureKind::Http5xx, 503),
        );
        // An outer wrapper records a coarse fallback for the same failure.
        state.record(
            FailureStage::QueryRequest,
            ApiFailure::new(FailureKind::Other),
        );
        let snapshot = state.snapshot(Instant::now()).unwrap();
        assert_eq!(snapshot.failure_stage, "endpoint_upsert");
        assert_eq!(snapshot.failure_kind, "http_5xx");
        assert_eq!(snapshot.http_status, Some(503));
    }

    #[test]
    fn snapshot_carries_the_buckets_and_the_latest_provisioning_state() {
        let mut state = State::default();
        let start = Instant::now();
        state.start_span(start);
        state.set_provisioning(ProvisioningState::ManagementKey);
        state.set_provisioning(ProvisioningState::Provisioning);
        for _ in 0..4 {
            state.note_retry();
        }
        state.record(
            FailureStage::KeyCreate,
            ApiFailure::with_status(FailureKind::RateLimited, 429),
        );

        let snapshot = state
            .snapshot(start + Duration::from_secs(7))
            .expect("a recorded failure produces a snapshot");
        assert_eq!(
            snapshot,
            Snapshot {
                failure_stage: "key_create",
                failure_kind: "rate_limited",
                http_status: Some(429),
                retry_bucket: "3_5",
                provisioning_state: Some("provisioning"),
                duration_bucket: Some("lt_30s"),
            }
        );
    }

    #[test]
    fn duration_is_absent_until_a_span_starts() {
        let mut state = State::default();
        state.record_for_test(FailureStage::SqlInput, FailureKind::Other);
        let snapshot = state.snapshot(Instant::now()).unwrap();
        assert_eq!(snapshot.duration_bucket, None);
        assert_eq!(snapshot.provisioning_state, None);
        assert_eq!(snapshot.http_status, None);
        assert_eq!(snapshot.retry_bucket, "0");
    }

    impl State {
        fn record_for_test(&mut self, stage: FailureStage, kind: FailureKind) {
            self.record(stage, ApiFailure::new(kind));
        }
    }

    /// The global recorders are thin wrappers over the same state; prove the
    /// wiring reaches a snapshot whose values stay inside the closed
    /// vocabularies. Exact values are asserted on a local `State` above (the
    /// semantics) and end-to-end in `telemetry_test.rs` (the wire), because
    /// the global is process-wide and any other test in this binary may have
    /// recorded first.
    #[test]
    #[cfg(feature = "telemetry")]
    fn the_global_recorders_reach_the_snapshot() {
        start_span();
        set_provisioning_state(ProvisioningState::Bearer);
        note_retry();
        record(
            FailureStage::QueryRequest,
            ApiFailure::with_status(FailureKind::SqlError, 400),
        );
        let snapshot = snapshot().expect("a recorded failure is visible globally");
        assert!(
            FailureStage::ALL
                .iter()
                .any(|stage| stage.as_str() == snapshot.failure_stage),
            "unexpected stage on the wire: {snapshot:?}"
        );
        assert!(
            FailureKind::ALL
                .iter()
                .any(|kind| kind.as_str() == snapshot.failure_kind),
            "unexpected kind on the wire: {snapshot:?}"
        );
        assert!(
            snapshot
                .http_status
                .is_none_or(|status| bounded_status(status) == Some(status))
        );
        assert_eq!(snapshot.duration_bucket, Some("lt_250ms"));
    }
}
