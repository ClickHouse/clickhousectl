use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;
use std::env;
use std::fmt;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

const DEFAULT_CREATE_TIMEOUT_SECS: u64 = 1_800;
const DEFAULT_DELETE_TIMEOUT_SECS: u64 = 900;
const DEFAULT_STEADY_STATE_TIMEOUT_SECS: u64 = 1_800;
const DEFAULT_POLL_INTERVAL_SECS: u64 = 10;

pub struct TestContext {
    pub org_id: String,
    pub provider: String,
    pub region: String,
    pub run_id: String,
    pub create_timeout: Duration,
    pub delete_timeout: Duration,
    pub steady_state_timeout: Duration,
    pub poll_interval: Duration,
    pub continue_on_non_blocking_failures: bool,
}

impl fmt::Debug for TestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestContext")
            .field("org_id", &"<redacted>")
            .field("provider", &self.provider)
            .field("region", &self.region)
            .field("run_id", &self.run_id)
            .field("create_timeout", &self.create_timeout)
            .field("delete_timeout", &self.delete_timeout)
            .field("steady_state_timeout", &self.steady_state_timeout)
            .field("poll_interval", &self.poll_interval)
            .field(
                "continue_on_non_blocking_failures",
                &self.continue_on_non_blocking_failures,
            )
            .finish()
    }
}

impl TestContext {
    pub fn from_env() -> TestResult<Self> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let github_run_id = env::var("GITHUB_RUN_ID").ok();
        let github_sha = env::var("GITHUB_SHA").ok();
        let run_id = match (github_run_id, github_sha) {
            (Some(run_id), Some(sha)) => format!("{run_id}-{}", &sha[..7.min(sha.len())]),
            (Some(run_id), None) => run_id,
            (None, Some(sha)) => format!("local-{}", &sha[..7.min(sha.len())]),
            (None, None) => format!("local-{timestamp}"),
        };

        Ok(Self {
            org_id: required_env("CLICKHOUSE_CLOUD_TEST_ORG_ID")?,
            provider: required_env("CLICKHOUSE_CLOUD_TEST_PROVIDER")?,
            region: required_env("CLICKHOUSE_CLOUD_TEST_REGION")?,
            run_id,
            create_timeout: duration_from_env(
                "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CREATE_SECS",
                DEFAULT_CREATE_TIMEOUT_SECS,
            )?,
            delete_timeout: duration_from_env(
                "CLICKHOUSE_CLOUD_TEST_TIMEOUT_DELETE_SECS",
                DEFAULT_DELETE_TIMEOUT_SECS,
            )?,
            steady_state_timeout: duration_from_env(
                "CLICKHOUSE_CLOUD_TEST_TIMEOUT_STEADY_STATE_SECS",
                DEFAULT_STEADY_STATE_TIMEOUT_SECS,
            )?,
            poll_interval: duration_from_env(
                "CLICKHOUSE_CLOUD_TEST_POLL_INTERVAL_SECS",
                DEFAULT_POLL_INTERVAL_SECS,
            )?,
            continue_on_non_blocking_failures: bool_from_env("CONTINUE_ON_NON_BLOCKING_FAILURES")?,
        })
    }

    pub fn service_name(&self) -> String {
        format!("clickhousectl-it-{}", self.run_id)
    }

    pub fn updated_service_name(&self) -> String {
        format!("{}-updated", self.service_name())
    }

    pub fn run_tags(&self) -> Vec<ResourceTagsV1> {
        vec![
            ResourceTagsV1 {
                key: "managed_by".to_string(),
                value: Some("clickhousectl_it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("service_crud".to_string()),
            },
            ResourceTagsV1 {
                key: "run_id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed_by=clickhousectl_it".to_string(),
            "tag:suite=service_crud".to_string(),
            format!("tag:run_id={}", self.run_id),
        ]
    }

    pub fn postgres_service_name(&self) -> String {
        format!("clickhousectl-it-pg-{}", self.run_id)
    }

    pub fn postgres_run_tags(&self) -> Vec<ResourceTagsV1> {
        vec![
            ResourceTagsV1 {
                key: "managed_by".to_string(),
                value: Some("clickhousectl_it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("postgres_crud".to_string()),
            },
            ResourceTagsV1 {
                key: "run_id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn postgres_run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed_by=clickhousectl_it".to_string(),
            "tag:suite=postgres_crud".to_string(),
            format!("tag:run_id={}", self.run_id),
        ]
    }

    pub fn clickpipe_service_name(&self) -> String {
        format!("clickhousectl-it-cp-{}", self.run_id)
    }

    pub fn clickpipe_postgres_service_name(&self) -> String {
        format!("clickhousectl-it-cp-pg-{}", self.run_id)
    }

    pub fn clickpipe_run_tags(&self) -> Vec<ResourceTagsV1> {
        vec![
            ResourceTagsV1 {
                key: "managed_by".to_string(),
                value: Some("clickhousectl_it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("clickpipe_postgres_cdc".to_string()),
            },
            ResourceTagsV1 {
                key: "run_id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn clickpipe_run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed_by=clickhousectl_it".to_string(),
            "tag:suite=clickpipe_postgres_cdc".to_string(),
            format!("tag:run_id={}", self.run_id),
        ]
    }

    /// Shared service name for the multi-source E2E driver — one ClickHouse
    /// service hosts all per-source stages in a run.
    pub fn clickpipe_e2e_service_name(&self) -> String {
        format!("clickhousectl-it-cp-e2e-{}", self.run_id)
    }

    pub fn clickpipe_e2e_run_tags(&self) -> Vec<ResourceTagsV1> {
        vec![
            ResourceTagsV1 {
                key: "managed_by".to_string(),
                value: Some("clickhousectl_it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("clickpipe_e2e".to_string()),
            },
            ResourceTagsV1 {
                key: "run_id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn clickpipe_e2e_run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed_by=clickhousectl_it".to_string(),
            "tag:suite=clickpipe_e2e".to_string(),
            format!("tag:run_id={}", self.run_id),
        ]
    }

    /// S3 bucket names must be globally unique, 3–63 chars, lowercase letters,
    /// digits, hyphens. `run_id` is already constrained to safe chars.
    pub fn aws_s3_bucket_name(&self) -> String {
        let raw = format!("clickhousectl-e2e-s3-{}", self.run_id);
        // S3 forbids underscores; substitute for safety even if run_id is clean today.
        raw.replace('_', "-").to_ascii_lowercase()
    }

    pub fn aws_iam_role_name(&self) -> String {
        format!("clickhousectl-e2e-s3-{}", self.run_id)
    }

    /// Kinesis stream names: 1–128 chars, `[A-Za-z0-9_.-]`. `run_id` is already
    /// safe but keep this distinct from the S3 role/bucket names for clarity.
    pub fn aws_kinesis_stream_name(&self) -> String {
        format!("clickhousectl-e2e-kinesis-{}", self.run_id)
    }

    /// IAM role name dedicated to the Kinesis stage (trust scoped to the
    /// per-test CHC service principal).
    pub fn aws_kinesis_iam_role_name(&self) -> String {
        format!("clickhousectl-e2e-kinesis-{}", self.run_id)
    }
}

pub fn create_client() -> TestResult<Client> {
    let key = required_env("CLICKHOUSE_CLOUD_API_KEY")?;
    let secret = required_env("CLICKHOUSE_CLOUD_API_SECRET")?;
    match env::var("CLICKHOUSE_CLOUD_API_BASE_URL").ok().filter(|s| !s.is_empty()) {
        Some(base_url) => Ok(Client::with_base_url(base_url, key, secret)),
        None => Ok(Client::new(key, secret)),
    }
}

// ── Failure Recorder ─────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepKind {
    Blocking,
    NonBlocking,
}

impl StepKind {
    pub fn label(self) -> &'static str {
        match self {
            StepKind::Blocking => "BLOCKING",
            StepKind::NonBlocking => "NON-BLOCKING",
        }
    }
}

#[derive(Debug)]
pub struct RecordedFailure {
    pub step_name: String,
    pub kind: StepKind,
    pub error: String,
}

#[derive(Default)]
pub struct FailureRecorder {
    failures: Vec<RecordedFailure>,
}

impl FailureRecorder {
    pub async fn run<T, F, Fut>(
        &mut self,
        ctx: &TestContext,
        kind: StepKind,
        step_name: &str,
        step: F,
    ) -> TestResult<Option<T>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = TestResult<T>>,
    {
        log_step_start(kind, step_name);
        match step().await {
            Ok(value) => {
                log_step_ok(kind, step_name);
                Ok(Some(value))
            }
            Err(error) => {
                if kind == StepKind::NonBlocking && ctx.continue_on_non_blocking_failures {
                    let rendered = error.to_string();
                    log_step_continue(step_name, &rendered);
                    self.failures.push(RecordedFailure {
                        step_name: step_name.to_string(),
                        kind,
                        error: rendered,
                    });
                    Ok(None)
                } else {
                    log_step_fail(kind, step_name, &error.to_string());
                    Err(error)
                }
            }
        }
    }

    pub fn finish(self) -> TestResult<()> {
        if self.failures.is_empty() {
            eprintln!("== Result ==");
            eprintln!("PASS: all recorded steps completed successfully in this test run");
            return Ok(());
        }

        eprintln!("== Result ==");
        eprintln!(
            "FAIL: {} non-blocking step failure(s) recorded in this single test run",
            self.failures.len()
        );
        for failure in &self.failures {
            eprintln!("  - [{}] {}", failure.kind.label(), failure.step_name);
        }

        eprintln!("\n== Failure Details ==");
        for (index, failure) in self.failures.iter().enumerate() {
            eprintln!(
                "{}. [{}] {}",
                index + 1,
                failure.kind.label(),
                failure.step_name
            );
            for line in failure.error.lines() {
                eprintln!("   {}", line);
            }
            if index + 1 != self.failures.len() {
                eprintln!();
            }
        }

        Err(format!(
            "{} non-blocking step failure(s) recorded; see summary above",
            self.failures.len()
        )
        .into())
    }
}

// ── Cleanup Registry ─────────────────────────────────────────────────

#[derive(Default)]
pub struct CleanupRegistry {
    service_ids: Vec<String>,
    postgres_ids: Vec<String>,
    clickpipes: Vec<(String, String)>,
}

impl CleanupRegistry {
    pub fn register_service(&mut self, service_id: impl Into<String>) {
        self.service_ids.push(service_id.into());
    }

    pub fn unregister_service(&mut self, service_id: &str) {
        self.service_ids
            .retain(|registered| registered != service_id);
    }

    pub fn register_postgres(&mut self, postgres_id: impl Into<String>) {
        self.postgres_ids.push(postgres_id.into());
    }

    pub fn unregister_postgres(&mut self, postgres_id: &str) {
        self.postgres_ids
            .retain(|registered| registered != postgres_id);
    }

    pub fn register_clickpipe(
        &mut self,
        service_id: impl Into<String>,
        clickpipe_id: impl Into<String>,
    ) {
        self.clickpipes.push((service_id.into(), clickpipe_id.into()));
    }

    /// Drain another registry's entries into this one. Used by the parallel
    /// driver to fold per-stage registries back into the parent for teardown.
    pub fn merge_from(&mut self, mut other: CleanupRegistry) {
        self.service_ids.append(&mut other.service_ids);
        self.postgres_ids.append(&mut other.postgres_ids);
        self.clickpipes.append(&mut other.clickpipes);
    }

    pub fn unregister_clickpipe(&mut self, service_id: &str, clickpipe_id: &str) {
        self.clickpipes
            .retain(|(svc, pipe)| !(svc == service_id && pipe == clickpipe_id));
    }

    pub async fn cleanup(&mut self, client: &Client, org_id: &str, delete_timeout: Duration, poll_interval: Duration) -> Result<(), String> {
        let mut failures = Vec::new();

        // Drain ClickPipes first so their parent service can be torn down cleanly.
        while let Some((service_id, clickpipe_id)) = self.clickpipes.pop() {
            if let Err(error) = ensure_clickpipe_gone(
                client,
                org_id,
                &service_id,
                &clickpipe_id,
                delete_timeout,
                poll_interval,
            )
            .await
            {
                failures.push(format!("clickpipe {service_id}/{clickpipe_id}: {error}"));
            }
        }

        while let Some(service_id) = self.service_ids.pop() {
            if let Err(error) = ensure_service_gone(client, org_id, &service_id, delete_timeout, poll_interval).await {
                failures.push(format!("{service_id}: {error}"));
            }
        }

        while let Some(postgres_id) = self.postgres_ids.pop() {
            if let Err(error) = ensure_postgres_gone(client, org_id, &postgres_id, delete_timeout, poll_interval).await {
                failures.push(format!("postgres {postgres_id}: {error}"));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n"))
        }
    }
}

async fn ensure_service_gone(
    client: &Client,
    org_id: &str,
    service_id: &str,
    delete_timeout: Duration,
    poll_interval: Duration,
) -> TestResult<()> {
    eprintln!("  cleanup: ensuring service is gone");

    // Try to stop the service first if it's running
    match client.instance_get(org_id, service_id).await {
        Ok(resp) => {
            if let Some(svc) = resp.result {
                let state = svc.state.to_string();
                if matches!(state.as_str(), "running" | "idle" | "starting" | "awaking") {
                    eprintln!("  cleanup: stopping service before delete");
                    let _ = client
                        .instance_state_update(
                            org_id,
                            service_id,
                            &ServiceStatePatchRequest {
                                command: Some(ServiceStatePatchRequestCommand::Stop),
                            },
                        )
                        .await;
                    // Wait for stop
                    let _ = poll_until("service stop for cleanup", delete_timeout, poll_interval, || {
                        let client = client.clone();
                        let org_id = org_id.to_string();
                        let service_id = service_id.to_string();
                        async move {
                            let resp = client.instance_get(&org_id, &service_id).await?;
                            let state = resp
                                .result
                                .as_ref()
                                .map(|s| s.state.to_string())
                                .unwrap_or_default();
                            if matches!(state.as_str(), "stopped" | "idle" | "degraded" | "failed") {
                                Ok(Some(()))
                            } else {
                                Ok(None)
                            }
                        }
                    })
                    .await;
                }
            }
        }
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => return Ok(()),
        Err(_) => {}
    }

    // Delete
    match client.instance_delete(org_id, service_id).await {
        Ok(_) => {}
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => return Ok(()),
        Err(e) => return Err(e.into()),
    }

    // Wait for deletion
    poll_until("service deletion", delete_timeout, poll_interval, || {
        let client = client.clone();
        let org_id = org_id.to_string();
        let service_id = service_id.to_string();
        async move {
            match client.instance_get(&org_id, &service_id).await {
                Ok(_) => Ok(None),
                Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(Some(())),
                Err(e) => Err(e.into()),
            }
        }
    })
    .await?;

    Ok(())
}

async fn ensure_clickpipe_gone(
    client: &Client,
    org_id: &str,
    service_id: &str,
    clickpipe_id: &str,
    delete_timeout: Duration,
    poll_interval: Duration,
) -> TestResult<()> {
    eprintln!("  cleanup: ensuring clickpipe is gone");

    match client.click_pipe_get(org_id, service_id, clickpipe_id).await {
        Ok(_) => {}
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => return Ok(()),
        Err(_) => {}
    }

    match client.click_pipe_delete(org_id, service_id, clickpipe_id).await {
        Ok(_) => {}
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => return Ok(()),
        Err(e) => return Err(e.into()),
    }

    poll_until("clickpipe deletion", delete_timeout, poll_interval, || {
        let client = client.clone();
        let org_id = org_id.to_string();
        let service_id = service_id.to_string();
        let clickpipe_id = clickpipe_id.to_string();
        async move {
            match client.click_pipe_get(&org_id, &service_id, &clickpipe_id).await {
                Ok(_) => Ok(None),
                Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(Some(())),
                Err(e) => Err(e.into()),
            }
        }
    })
    .await?;

    Ok(())
}

async fn ensure_postgres_gone(
    client: &Client,
    org_id: &str,
    postgres_id: &str,
    delete_timeout: Duration,
    poll_interval: Duration,
) -> TestResult<()> {
    eprintln!("  cleanup: ensuring postgres service is gone");

    match client.postgres_service_get(org_id, postgres_id).await {
        Ok(_) => {}
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => return Ok(()),
        Err(_) => {}
    }

    match client.postgres_service_delete(org_id, postgres_id).await {
        Ok(_) => {}
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => return Ok(()),
        Err(e) => return Err(e.into()),
    }

    poll_until("postgres deletion", delete_timeout, poll_interval, || {
        let client = client.clone();
        let org_id = org_id.to_string();
        let postgres_id = postgres_id.to_string();
        async move {
            match client.postgres_service_get(&org_id, &postgres_id).await {
                Ok(_) => Ok(None),
                Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(Some(())),
                Err(e) => {
                    let message = e.to_string();
                    if message.contains("404") || message.contains("not found") {
                        Ok(Some(()))
                    } else {
                        Err(e.into())
                    }
                }
            }
        }
    })
    .await?;

    Ok(())
}

// ── Polling ──────────────────────────────────────────────────────────

pub async fn poll_until<F, Fut, T>(
    description: &str,
    timeout: Duration,
    interval: Duration,
    mut check: F,
) -> TestResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = TestResult<Option<T>>>,
{
    let started = Instant::now();
    let mut last_error: Option<String> = None;
    eprintln!("  poll: waiting for {description}");

    loop {
        match check().await {
            Ok(Some(value)) => {
                eprintln!("  poll: complete after {:?}", started.elapsed());
                return Ok(value);
            }
            Ok(None) => {}
            Err(error) => last_error = Some(error.to_string()),
        }

        if started.elapsed() >= timeout {
            let mut message = format!(
                "timed out after {:?} waiting for {description}",
                started.elapsed()
            );
            if let Some(error) = last_error {
                message.push_str(&format!("; last error: {error}"));
            }
            return Err(message.into());
        }

        tokio::time::sleep(interval).await;
    }
}

// ── Logging ──────────────────────────────────────────────────────────

pub fn log_run_header(test_name: &str, ctx: &TestContext) {
    eprintln!("== {} ==", test_name);
    eprintln!("run_id: {}", ctx.run_id);
    eprintln!("org_id: <redacted>");
    eprintln!("provider: {}", ctx.provider);
    eprintln!("region: {}", ctx.region);
    eprintln!(
        "mode: {}",
        if ctx.continue_on_non_blocking_failures {
            "continue-on-non-blocking-failures"
        } else {
            "strict"
        }
    );
}

pub fn log_phase(name: &str) {
    eprintln!("\n== {} ==", name);
}

fn log_step_start(kind: StepKind, step_name: &str) {
    eprintln!("START [{}] {}", kind.label(), step_name);
}

fn log_step_ok(kind: StepKind, step_name: &str) {
    eprintln!("PASS  [{}] {}", kind.label(), step_name);
}

fn log_step_fail(kind: StepKind, step_name: &str, error: &str) {
    eprintln!("FAIL  [{}] {}", kind.label(), step_name);
    eprintln!("  error: {}", first_line(error));
}

fn log_step_continue(step_name: &str, error: &str) {
    eprintln!("WARN  [NON-BLOCKING] {}", step_name);
    eprintln!("  continuing: {}", first_line(error));
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

// ── Env helpers ──────────────────────────────────────────────────────

fn duration_from_env(name: &str, default_secs: u64) -> TestResult<Duration> {
    match env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}

fn bool_from_env(name: &str) -> TestResult<bool> {
    match env::var(name) {
        Ok(value) => match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" | "" => Ok(false),
            _ => Err(format!("invalid boolean value for {name}: {value}").into()),
        },
        Err(env::VarError::NotPresent) => Ok(false),
        Err(error) => Err(Box::new(error)),
    }
}

fn required_env(name: &str) -> TestResult<String> {
    let value =
        env::var(name).map_err(|_| format!("missing required environment variable {name}"))?;
    if value.is_empty() {
        return Err(format!("{name} is set but empty (secrets unavailable in fork PRs?)").into());
    }
    Ok(value)
}

// ── ClickHouse Service Provisioning ──────────────────────────────────
//
// Reusable across E2E stages — a single provisioned service can host
// multiple per-source stages back-to-back, amortising the ~1 min
// provisioning + ~3 min teardown over N source tests.

pub struct ProvisionedClickHouse {
    pub service_id: String,
    pub password: String,
    /// The CHC service's ClickPipes IAM principal — put this in the trust
    /// policy of any IAM role that ClickPipes will assume for this service.
    pub iam_role: String,
    pub https_endpoint: ServiceEndpoint,
    pub username: String,
    pub query: ClickHouseQuery,
}

/// Create a ClickHouse Cloud service, wait for steady state, and bundle the
/// metadata that source stages need to operate against it. Registers the
/// service with `cleanup` so teardown is automatic regardless of which step
/// later fails.
pub async fn provision_clickhouse(
    client: &Client,
    ctx: &TestContext,
    cleanup: &mut CleanupRegistry,
    service_name: &str,
    tags: Vec<ResourceTagsV1>,
) -> TestResult<ProvisionedClickHouse> {
    let body = ServicePostRequest {
        name: service_name.to_string(),
        provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
        region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
        min_replica_memory_gb: Some(8.0),
        max_replica_memory_gb: Some(8.0),
        num_replicas: Some(1.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(5.0),
        ip_access_list: vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("clickpipe integration test".to_string()),
        }],
        tags: Some(tags),
        ..Default::default()
    };

    let created = client
        .instance_create(&ctx.org_id, &body)
        .await?
        .result
        .ok_or("service create returned no result")?;
    let service_id = created.service.id.to_string();
    let password = created.password.clone();
    cleanup.register_service(service_id.clone());
    eprintln!("  provisioned clickhouse id <redacted>");

    let svc = poll_until(
        "clickhouse steady state",
        ctx.steady_state_timeout,
        ctx.poll_interval,
        || {
            let client = client.clone();
            let org_id = ctx.org_id.clone();
            let service_id = service_id.clone();
            async move {
                let resp = client.instance_get(&org_id, &service_id).await?;
                let svc = resp.result.ok_or("service get returned no result")?;
                let state = svc.state.to_string();
                if matches!(state.as_str(), "running" | "idle") {
                    Ok(Some(svc))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await?;

    if svc.iam_role.is_empty() {
        return Err(
            "provisioned service has no iamRole populated — cannot establish ClickPipes trust".into(),
        );
    }

    let https_endpoint = svc
        .endpoints
        .iter()
        .find(|e| matches!(e.protocol, ServiceEndpointProtocol::Https))
        .ok_or("ClickHouse service has no https endpoint")?
        .clone();
    let username = https_endpoint
        .username
        .clone()
        .unwrap_or_else(|| "default".to_string());

    let query = ClickHouseQuery::new(
        &https_endpoint.host,
        https_endpoint.port as u16,
        &username,
        &password,
    );

    Ok(ProvisionedClickHouse {
        service_id,
        password,
        iam_role: svc.iam_role,
        https_endpoint,
        username,
        query,
    })
}

/// Attach to an externally-provisioned CHC service rather than creating one.
/// Used by the harness when `CLICKHOUSE_CLOUD_TEST_SERVICE_ID` is set so
/// multiple per-source test runs can share a single long-lived service.
/// Does NOT register the service with cleanup — caller manages teardown.
pub async fn attach_clickhouse(
    client: &Client,
    org_id: &str,
    service_id: &str,
    password: &str,
) -> TestResult<ProvisionedClickHouse> {
    let resp = client.instance_get(org_id, service_id).await?;
    let mut svc = resp.result.ok_or("service get returned no result")?;

    // ClickPipes' pipe-create endpoint rejects idle services even though the
    // service can serve queries from idle. The PATCH /state `start` command
    // is a no-op for idle services (it's intended for `stopped` services);
    // a query is what actually triggers the wake. Send `SELECT 1` and poll
    // until the state field flips to `running` so subsequent pipe-creates
    // succeed.
    let state = svc.state.to_string();
    if state == "idle" {
        eprintln!("  service is idle — sending wake query");
        let https = svc
            .endpoints
            .iter()
            .find(|e| matches!(e.protocol, ServiceEndpointProtocol::Https))
            .ok_or("service has no https endpoint to wake")?;
        let username = https.username.clone().unwrap_or_else(|| "default".to_string());
        let wake_query = ClickHouseQuery::new(&https.host, https.port as u16, &username, password);
        let _ = wake_query.run_query("SELECT 1 FORMAT TabSeparated").await?;

        svc = poll_until(
            "clickhouse wake from idle",
            Duration::from_secs(300),
            Duration::from_secs(5),
            || {
                let client = client.clone();
                let org_id = org_id.to_string();
                let service_id = service_id.to_string();
                async move {
                    let resp = client.instance_get(&org_id, &service_id).await?;
                    let svc = resp.result.ok_or("service get returned no result")?;
                    if svc.state.to_string() == "running" {
                        Ok(Some(svc))
                    } else {
                        Ok(None)
                    }
                }
            },
        )
        .await?;
    } else if state != "running" {
        return Err(format!(
            "service {service_id} is in state {state}, expected running or idle"
        )
        .into());
    }
    if svc.iam_role.is_empty() {
        return Err(
            "attached service has no iamRole populated — cannot establish ClickPipes trust".into(),
        );
    }

    let https_endpoint = svc
        .endpoints
        .iter()
        .find(|e| matches!(e.protocol, ServiceEndpointProtocol::Https))
        .ok_or("ClickHouse service has no https endpoint")?
        .clone();
    let username = https_endpoint
        .username
        .clone()
        .unwrap_or_else(|| "default".to_string());

    let query = ClickHouseQuery::new(
        &https_endpoint.host,
        https_endpoint.port as u16,
        &username,
        password,
    );

    eprintln!("  attached to existing clickhouse service <redacted>");
    Ok(ProvisionedClickHouse {
        service_id: service_id.to_string(),
        password: password.to_string(),
        iam_role: svc.iam_role,
        https_endpoint,
        username,
        query,
    })
}

// ── ClickHouse HTTP Query Helper ─────────────────────────────────────
//
// Thin wrapper around the service's HTTPS endpoint for verifying that
// rows arrived. The PG CDC test has its own inline copy; we'll leave
// that alone but new tests should use this shared one.

#[derive(Clone)]
pub struct ClickHouseQuery {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl ClickHouseQuery {
    pub fn new(host: &str, port: u16, username: &str, password: &str) -> Self {
        Self {
            base_url: format!("https://{host}:{port}"),
            username: username.to_string(),
            password: password.to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    pub async fn run_query(&self, query: &str) -> TestResult<String> {
        let resp = self
            .http
            .post(&self.base_url)
            .basic_auth(&self.username, Some(&self.password))
            .body(query.to_string())
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(format!("ClickHouse query failed ({status}): {body}").into());
        }
        Ok(body)
    }

    pub async fn count_rows(&self, table: &str) -> TestResult<i64> {
        let body = self
            .run_query(&format!(
                "SELECT count() FROM default.{table} FORMAT TabSeparated"
            ))
            .await?;
        Ok(body.trim().parse::<i64>()?)
    }

    pub async fn scalar_string(&self, query: &str) -> TestResult<Option<String>> {
        let body = self
            .run_query(&format!("{query} FORMAT TabSeparated"))
            .await?;
        let value = body.trim();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.to_string()))
        }
    }
}

// ── AWS Cleanup Registry ─────────────────────────────────────────────
//
// Tracks AWS-side resources (S3 buckets, IAM roles) created by E2E tests
// against integration sources, so a single teardown call removes them
// regardless of which test step failed.

#[derive(Default)]
pub struct AwsCleanupRegistry {
    s3_buckets: Vec<(aws_sdk_s3::config::Region, String)>,
    iam_roles: Vec<String>,
    ec2_instances: Vec<String>,
    ec2_security_groups: Vec<String>,
    ec2_elastic_ips: Vec<String>,
    /// `(region, stream_name)` — the region is required so cleanup can build a
    /// regional Kinesis client even if the parent SDK config lives elsewhere.
    kinesis_streams: Vec<(String, String)>,
}

impl AwsCleanupRegistry {
    pub fn register_s3_bucket(
        &mut self,
        region: aws_sdk_s3::config::Region,
        bucket: impl Into<String>,
    ) {
        self.s3_buckets.push((region, bucket.into()));
    }

    pub fn register_iam_role(&mut self, role_name: impl Into<String>) {
        self.iam_roles.push(role_name.into());
    }

    pub fn register_ec2_instance(&mut self, instance_id: impl Into<String>) {
        self.ec2_instances.push(instance_id.into());
    }

    pub fn register_ec2_security_group(&mut self, sg_id: impl Into<String>) {
        self.ec2_security_groups.push(sg_id.into());
    }

    pub fn register_ec2_elastic_ip(&mut self, allocation_id: impl Into<String>) {
        self.ec2_elastic_ips.push(allocation_id.into());
    }

    pub fn register_kinesis_stream(
        &mut self,
        region: impl Into<String>,
        stream_name: impl Into<String>,
    ) {
        self.kinesis_streams.push((region.into(), stream_name.into()));
    }

    pub fn merge_from(&mut self, mut other: AwsCleanupRegistry) {
        self.s3_buckets.append(&mut other.s3_buckets);
        self.iam_roles.append(&mut other.iam_roles);
        self.ec2_instances.append(&mut other.ec2_instances);
        self.ec2_security_groups.append(&mut other.ec2_security_groups);
        self.ec2_elastic_ips.append(&mut other.ec2_elastic_ips);
        self.kinesis_streams.append(&mut other.kinesis_streams);
    }

    pub async fn cleanup(
        &mut self,
        aws_config: &aws_config::SdkConfig,
        iam_client: &aws_sdk_iam::Client,
        ec2_client: &aws_sdk_ec2::Client,
    ) -> Result<(), String> {
        let mut failures = Vec::new();

        while let Some((region, bucket)) = self.s3_buckets.pop() {
            // S3 calls must hit the bucket's region — re-build the client per bucket.
            let s3_config = aws_sdk_s3::config::Builder::from(aws_config)
                .region(region)
                .build();
            let s3 = aws_sdk_s3::Client::from_conf(s3_config);
            if let Err(error) = empty_and_delete_bucket(&s3, &bucket).await {
                failures.push(format!("s3 bucket {bucket}: {error}"));
            }
        }

        while let Some(role) = self.iam_roles.pop() {
            if let Err(error) = delete_iam_role(iam_client, &role).await {
                failures.push(format!("iam role {role}: {error}"));
            }
        }

        // Terminate instances first so they release their ENIs, then drop SGs.
        if !self.ec2_instances.is_empty() {
            let ids: Vec<String> = self.ec2_instances.drain(..).collect();
            if let Err(error) = terminate_and_wait(ec2_client, &ids).await {
                failures.push(format!("ec2 instances {ids:?}: {error}"));
            }
        }

        while let Some(sg_id) = self.ec2_security_groups.pop() {
            if let Err(error) = ec2_client.delete_security_group().group_id(&sg_id).send().await {
                let msg = error.to_string();
                if !msg.contains("InvalidGroup.NotFound") {
                    failures.push(format!("ec2 sg {sg_id}: {msg}"));
                }
            }
        }

        // Elastic IPs are auto-disassociated when their instance is terminated,
        // so by this point release is unconditional.
        while let Some(allocation_id) = self.ec2_elastic_ips.pop() {
            if let Err(error) = ec2_client
                .release_address()
                .allocation_id(&allocation_id)
                .send()
                .await
            {
                let msg = error.to_string();
                if !msg.contains("InvalidAllocationID.NotFound") {
                    failures.push(format!("ec2 eip {allocation_id}: {msg}"));
                }
            }
        }

        // Kinesis streams: build a regional client per entry so cleanup works
        // even if the parent aws_config is in a different region.
        while let Some((region, stream_name)) = self.kinesis_streams.pop() {
            let kinesis_config = aws_sdk_kinesis::config::Builder::from(aws_config)
                .region(aws_sdk_kinesis::config::Region::new(region.clone()))
                .build();
            let kinesis = aws_sdk_kinesis::Client::from_conf(kinesis_config);
            if let Err(error) = delete_kinesis_stream(&kinesis, &stream_name).await {
                failures.push(format!("kinesis stream {stream_name}: {error}"));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n"))
        }
    }
}

async fn empty_and_delete_bucket(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
) -> TestResult<()> {
    eprintln!("  cleanup: emptying s3 bucket");

    let mut continuation: Option<String> = None;
    loop {
        let mut req = s3.list_objects_v2().bucket(bucket);
        if let Some(token) = &continuation {
            req = req.continuation_token(token);
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) if e.to_string().contains("NoSuchBucket") => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        let keys: Vec<aws_sdk_s3::types::ObjectIdentifier> = resp
            .contents()
            .iter()
            .filter_map(|o| {
                o.key().and_then(|k| {
                    aws_sdk_s3::types::ObjectIdentifier::builder()
                        .key(k)
                        .build()
                        .ok()
                })
            })
            .collect();

        if !keys.is_empty() {
            let delete = aws_sdk_s3::types::Delete::builder()
                .set_objects(Some(keys))
                .quiet(true)
                .build()?;
            s3.delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await?;
        }

        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(|s| s.to_string());
        } else {
            break;
        }
    }

    eprintln!("  cleanup: deleting s3 bucket");
    match s3.delete_bucket().bucket(bucket).send().await {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("NoSuchBucket") => Ok(()),
        Err(e) => Err(e.into()),
    }
}

async fn delete_iam_role(
    iam: &aws_sdk_iam::Client,
    role_name: &str,
) -> TestResult<()> {
    eprintln!("  cleanup: detaching inline policies on iam role");

    // Inline policies first.
    let policies = match iam.list_role_policies().role_name(role_name).send().await {
        Ok(r) => r.policy_names,
        Err(e) if e.to_string().contains("NoSuchEntity") => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for name in policies {
        let _ = iam
            .delete_role_policy()
            .role_name(role_name)
            .policy_name(&name)
            .send()
            .await;
    }

    // Managed policy attachments — none expected for our tests, but be defensive.
    if let Ok(resp) = iam.list_attached_role_policies().role_name(role_name).send().await {
        for p in resp.attached_policies() {
            if let Some(arn) = p.policy_arn() {
                let _ = iam
                    .detach_role_policy()
                    .role_name(role_name)
                    .policy_arn(arn)
                    .send()
                    .await;
            }
        }
    }

    eprintln!("  cleanup: deleting iam role");
    match iam.delete_role().role_name(role_name).send().await {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("NoSuchEntity") => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// ── AWS Provisioning Helpers ─────────────────────────────────────────

/// Create a private S3 bucket with the given name in the given region, blocked
/// from public access, with `BucketOwnerEnforced` ACL semantics.
pub async fn create_private_bucket(
    s3: &aws_sdk_s3::Client,
    region: &str,
    bucket: &str,
    tags: &[(String, String)],
) -> TestResult<()> {
    use aws_sdk_s3::types::{
        BucketCannedAcl, BucketLocationConstraint, CreateBucketConfiguration,
        ObjectOwnership, OwnershipControls, OwnershipControlsRule, PublicAccessBlockConfiguration,
        Tag, Tagging,
    };

    let mut req = s3.create_bucket().bucket(bucket).acl(BucketCannedAcl::Private);
    // us-east-1 must NOT have a LocationConstraint; every other region must.
    if region != "us-east-1" {
        let cfg = CreateBucketConfiguration::builder()
            .location_constraint(BucketLocationConstraint::from(region))
            .build();
        req = req.create_bucket_configuration(cfg);
    }
    req.send().await?;

    s3.put_public_access_block()
        .bucket(bucket)
        .public_access_block_configuration(
            PublicAccessBlockConfiguration::builder()
                .block_public_acls(true)
                .ignore_public_acls(true)
                .block_public_policy(true)
                .restrict_public_buckets(true)
                .build(),
        )
        .send()
        .await?;

    s3.put_bucket_ownership_controls()
        .bucket(bucket)
        .ownership_controls(
            OwnershipControls::builder()
                .rules(
                    OwnershipControlsRule::builder()
                        .object_ownership(ObjectOwnership::BucketOwnerEnforced)
                        .build()?,
                )
                .build()?,
        )
        .send()
        .await?;

    if !tags.is_empty() {
        let aws_tags: Vec<Tag> = tags
            .iter()
            .map(|(k, v)| Tag::builder().key(k).value(v).build())
            .collect::<Result<_, _>>()?;
        s3.put_bucket_tagging()
            .bucket(bucket)
            .tagging(Tagging::builder().set_tag_set(Some(aws_tags)).build()?)
            .send()
            .await?;
    }

    Ok(())
}

pub async fn put_object_bytes(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    body: Vec<u8>,
    content_type: &str,
) -> TestResult<()> {
    s3.put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .content_type(content_type)
        .send()
        .await?;
    Ok(())
}

// ── TLS Cert Generation (rcgen) ──────────────────────────────────────
//
// Self-signed CA + server cert (IP SAN) + client cert/key bundle, used by
// the TLS Kafka stages. All PEM-encoded.

pub struct RedpandaCerts {
    pub ca_pem: String,
    pub server_cert_pem: String,
    pub server_key_pem: String,
    pub client_cert_pem: String,
    pub client_key_pem: String,
    /// The CN we put on the client cert — Redpanda derives the mTLS user
    /// identity from this string, so ACLs must be granted to `User:{client_cn}`.
    pub client_cn: String,
}

pub fn generate_redpanda_certs(broker_ip: &str, client_cn: &str) -> TestResult<RedpandaCerts> {
    use rcgen::{
        BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, SanType,
    };

    let parsed_ip: std::net::IpAddr = broker_ip
        .parse()
        .map_err(|e| format!("invalid broker ip {broker_ip}: {e}"))?;

    // CA — used to sign both the server cert (broker presents) and the client
    // cert (ClickPipes presents for mTLS).
    let ca_key = KeyPair::generate()?;
    let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "clickhousectl-e2e-test-ca");
    let ca_cert = ca_params.clone().self_signed(&ca_key)?;
    let ca_pem = ca_cert.pem();
    let issuer = Issuer::new(ca_params, ca_key);

    // Server cert: SAN = broker_ip, so ClickPipes' TLS validation matches.
    let server_key = KeyPair::generate()?;
    let mut server_params = CertificateParams::new(Vec::<String>::new())?;
    server_params
        .subject_alt_names
        .push(SanType::IpAddress(parsed_ip));
    server_params
        .distinguished_name
        .push(DnType::CommonName, "redpanda-broker");
    let server_cert = server_params.signed_by(&server_key, &issuer)?;

    // Client cert: identity comes from CN.
    let client_key = KeyPair::generate()?;
    let mut client_params = CertificateParams::new(Vec::<String>::new())?;
    client_params
        .distinguished_name
        .push(DnType::CommonName, client_cn);
    let client_cert = client_params.signed_by(&client_key, &issuer)?;

    Ok(RedpandaCerts {
        ca_pem,
        server_cert_pem: server_cert.pem(),
        server_key_pem: server_key.serialize_pem(),
        client_cert_pem: client_cert.pem(),
        client_key_pem: client_key.serialize_pem(),
        client_cn: client_cn.to_string(),
    })
}

// ── EC2 Helpers ──────────────────────────────────────────────────────

/// Resolve the default VPC for the configured region. Both us-east-2 and
/// every Integrations_Tester region we use today have one — fail loudly if
/// not.
pub async fn default_vpc_id(ec2: &aws_sdk_ec2::Client) -> TestResult<String> {
    use aws_sdk_ec2::types::Filter;

    let resp = ec2
        .describe_vpcs()
        .filters(Filter::builder().name("is-default").values("true").build())
        .send()
        .await?;
    resp.vpcs()
        .iter()
        .find_map(|v| v.vpc_id().map(|s| s.to_string()))
        .ok_or_else(|| "no default VPC found in region".into())
}

pub async fn first_subnet_in_vpc(
    ec2: &aws_sdk_ec2::Client,
    vpc_id: &str,
) -> TestResult<String> {
    use aws_sdk_ec2::types::Filter;

    let resp = ec2
        .describe_subnets()
        .filters(Filter::builder().name("vpc-id").values(vpc_id).build())
        .send()
        .await?;
    resp.subnets()
        .iter()
        .find_map(|s| s.subnet_id().map(|s| s.to_string()))
        .ok_or_else(|| format!("no subnets in vpc {vpc_id}").into())
}

/// Find the most recent Canonical-published Ubuntu 24.04 LTS amd64 AMI.
pub async fn latest_ubuntu_noble_amd64_ami(
    ec2: &aws_sdk_ec2::Client,
) -> TestResult<String> {
    use aws_sdk_ec2::types::Filter;

    let resp = ec2
        .describe_images()
        .owners("099720109477") // Canonical
        .filters(
            Filter::builder()
                .name("name")
                .values("ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*")
                .build(),
        )
        .filters(
            Filter::builder()
                .name("virtualization-type")
                .values("hvm")
                .build(),
        )
        .send()
        .await?;

    let mut images: Vec<_> = resp.images().to_vec();
    images.sort_by(|a, b| a.creation_date().cmp(&b.creation_date()));
    images
        .last()
        .and_then(|i| i.image_id().map(|s| s.to_string()))
        .ok_or_else(|| "no Ubuntu Noble AMIs found".into())
}

/// Create a single-ingress-rule security group exposing one TCP port from any
/// source. Returns the SG id. Caller must register it with `AwsCleanupRegistry`.
pub async fn create_open_security_group(
    ec2: &aws_sdk_ec2::Client,
    vpc_id: &str,
    name: &str,
    ingress_ports: &[i32],
) -> TestResult<String> {
    use aws_sdk_ec2::types::{IpPermission, IpRange};

    let sg = ec2
        .create_security_group()
        .vpc_id(vpc_id)
        .group_name(name)
        .description(format!("clickhousectl e2e ({name})"))
        .send()
        .await?;
    let sg_id = sg.group_id().ok_or("CreateSecurityGroup returned no id")?.to_string();

    let permissions: Vec<IpPermission> = ingress_ports
        .iter()
        .map(|port| {
            IpPermission::builder()
                .ip_protocol("tcp")
                .from_port(*port)
                .to_port(*port)
                .ip_ranges(IpRange::builder().cidr_ip("0.0.0.0/0").build())
                .build()
        })
        .collect();

    ec2.authorize_security_group_ingress()
        .group_id(&sg_id)
        .set_ip_permissions(Some(permissions))
        .send()
        .await?;

    Ok(sg_id)
}

/// Launch a single Ubuntu EC2 instance with the given user_data script,
/// associated public IP, and security group. Returns `(instance_id, public_ip)`
/// once the instance is in `running` state. Caller must register the instance
/// in `AwsCleanupRegistry`.
pub async fn launch_ec2_instance(
    ec2: &aws_sdk_ec2::Client,
    ami_id: &str,
    subnet_id: &str,
    sg_id: &str,
    instance_type: &str,
    user_data: &str,
    name_tag: &str,
) -> TestResult<(String, String)> {
    use aws_sdk_ec2::types::{
        BlockDeviceMapping, EbsBlockDevice, InstanceNetworkInterfaceSpecification,
        InstanceType, ResourceType, Tag, TagSpecification, VolumeType,
    };
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;

    let ud_b64 = STANDARD.encode(user_data.as_bytes());

    let nic = InstanceNetworkInterfaceSpecification::builder()
        .device_index(0)
        .associate_public_ip_address(true)
        .subnet_id(subnet_id)
        .groups(sg_id)
        .build();

    let root_volume = BlockDeviceMapping::builder()
        .device_name("/dev/sda1")
        .ebs(
            EbsBlockDevice::builder()
                .volume_size(20)
                .volume_type(VolumeType::Gp3)
                .delete_on_termination(true)
                .build(),
        )
        .build();

    let tag_spec = TagSpecification::builder()
        .resource_type(ResourceType::Instance)
        .tags(Tag::builder().key("Name").value(name_tag).build())
        .tags(
            Tag::builder()
                .key("managed_by")
                .value("clickhousectl_e2e")
                .build(),
        )
        .build();

    let resp = ec2
        .run_instances()
        .image_id(ami_id)
        .instance_type(InstanceType::from(instance_type))
        .min_count(1)
        .max_count(1)
        .user_data(ud_b64)
        .network_interfaces(nic)
        .block_device_mappings(root_volume)
        .tag_specifications(tag_spec)
        .send()
        .await?;

    let instance = resp
        .instances()
        .first()
        .ok_or("RunInstances returned no instances")?;
    let instance_id = instance
        .instance_id()
        .ok_or("RunInstances returned instance without id")?
        .to_string();

    // Poll until running + public IP allocated.
    eprintln!("  waiting for ec2 instance to enter running state");
    let public_ip = poll_until(
        "ec2 instance running with public ip",
        Duration::from_secs(300),
        Duration::from_secs(5),
        || {
            let ec2 = ec2.clone();
            let instance_id = instance_id.clone();
            async move {
                let resp = ec2
                    .describe_instances()
                    .instance_ids(instance_id)
                    .send()
                    .await?;
                let inst = resp
                    .reservations()
                    .iter()
                    .flat_map(|r| r.instances())
                    .next();
                match inst {
                    None => Ok(None),
                    Some(i) => {
                        let state = i
                            .state()
                            .and_then(|s| s.name())
                            .map(|n| n.as_str().to_string())
                            .unwrap_or_default();
                        if state != "running" {
                            return Ok(None);
                        }
                        Ok(i.public_ip_address().map(|s| s.to_string()))
                    }
                }
            }
        },
    )
    .await?;

    Ok((instance_id, public_ip))
}

/// Allocate an Elastic IP. Returns `(public_ip, allocation_id)`. Caller is
/// responsible for `register_ec2_elastic_ip(allocation_id)` and for calling
/// `associate_elastic_ip` once an instance exists.
pub async fn allocate_elastic_ip(
    ec2: &aws_sdk_ec2::Client,
) -> TestResult<(String, String)> {
    use aws_sdk_ec2::types::DomainType;

    let resp = ec2
        .allocate_address()
        .domain(DomainType::Vpc)
        .send()
        .await?;
    let ip = resp.public_ip().ok_or("AllocateAddress returned no public_ip")?.to_string();
    let alloc = resp
        .allocation_id()
        .ok_or("AllocateAddress returned no allocation_id")?
        .to_string();
    Ok((ip, alloc))
}

pub async fn associate_elastic_ip(
    ec2: &aws_sdk_ec2::Client,
    allocation_id: &str,
    instance_id: &str,
) -> TestResult<()> {
    ec2.associate_address()
        .allocation_id(allocation_id)
        .instance_id(instance_id)
        .send()
        .await?;
    Ok(())
}

/// Poll Redpanda's admin API for an HTTP user-lookup endpoint to confirm a
/// SCRAM user has been provisioned. Lets us replace fixed-time sleeps in
/// kafka stages with a real readiness check.
pub async fn wait_for_redpanda_scram_user(
    host: &str,
    admin_port: u16,
    username: &str,
    timeout: Duration,
) -> TestResult<()> {
    let target = format!("http://{host}:{admin_port}/v1/security/users");
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    poll_until(
        &format!("redpanda scram user {username}"),
        timeout,
        Duration::from_secs(3),
        || {
            let http = http.clone();
            let target = target.clone();
            let username = username.to_string();
            async move {
                let resp = match http.get(&target).send().await {
                    Ok(r) => r,
                    Err(_) => return Ok(None),
                };
                if !resp.status().is_success() {
                    return Ok(None);
                }
                let body: serde_json::Value = match resp.json().await {
                    Ok(b) => b,
                    Err(_) => return Ok(None),
                };
                let found = body
                    .as_array()
                    .map(|users| users.iter().any(|u| u.as_str() == Some(&username)))
                    .unwrap_or(false);
                if found { Ok(Some(())) } else { Ok(None) }
            }
        },
    )
    .await
}

/// Stable TCP-port probe: succeed only when the port has been open for
/// `required_consecutive` checks in a row (5 s apart). Catches "port opens,
/// then closes briefly during a service restart, then opens again" patterns
/// that would slip past a single-success probe (e.g. MongoDB's bootstrap
/// restarts mongod with auth enabled mid-script).
pub async fn wait_for_stable_tcp_port(
    host: &str,
    port: u16,
    required_consecutive: u32,
    total_timeout: Duration,
) -> TestResult<()> {
    let target = format!("{host}:{port}");
    let consecutive = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    poll_until(
        &format!("stable tcp port {host}:{port} ({required_consecutive}× in a row)"),
        total_timeout,
        Duration::from_secs(5),
        || {
            let target = target.clone();
            let consecutive = consecutive.clone();
            async move {
                match tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::net::TcpStream::connect(&target),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        let n = consecutive
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                            + 1;
                        if n >= required_consecutive {
                            Ok(Some(()))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => {
                        consecutive.store(0, std::sync::atomic::Ordering::Relaxed);
                        Ok(None)
                    }
                }
            }
        },
    )
    .await
}

/// Best-effort TCP connect probe — used to wait for a service (Redpanda, etc.)
/// to start listening after `user_data` finishes.
pub async fn wait_for_tcp_port(host: &str, port: u16, timeout: Duration) -> TestResult<()> {
    let target = format!("{host}:{port}");
    poll_until(
        &format!("tcp port {host}:{port}"),
        timeout,
        Duration::from_secs(5),
        || {
            let target = target.clone();
            async move {
                match tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::net::TcpStream::connect(&target),
                )
                .await
                {
                    Ok(Ok(_)) => Ok(Some(())),
                    _ => Ok(None),
                }
            }
        },
    )
    .await
}

async fn terminate_and_wait(
    ec2: &aws_sdk_ec2::Client,
    instance_ids: &[String],
) -> TestResult<()> {
    if instance_ids.is_empty() {
        return Ok(());
    }
    eprintln!("  cleanup: terminating ec2 instances");
    let _ = ec2
        .terminate_instances()
        .set_instance_ids(Some(instance_ids.to_vec()))
        .send()
        .await?;

    // Poll until all are in 'terminated'. AWS deletes them lazily after that.
    poll_until(
        "ec2 instances terminated",
        Duration::from_secs(300),
        Duration::from_secs(10),
        || {
            let ec2 = ec2.clone();
            let ids = instance_ids.to_vec();
            async move {
                let resp = ec2
                    .describe_instances()
                    .set_instance_ids(Some(ids))
                    .send()
                    .await?;
                let all_terminated = resp
                    .reservations()
                    .iter()
                    .flat_map(|r| r.instances())
                    .all(|i| {
                        i.state()
                            .and_then(|s| s.name())
                            .map(|n| n.as_str() == "terminated")
                            .unwrap_or(false)
                    });
                if all_terminated { Ok(Some(())) } else { Ok(None) }
            }
        },
    )
    .await
}

/// Create an IAM role whose only purpose is to be assumed by a ClickPipes
/// service principal. The trust policy targets `service_principal_arn` exactly
/// — no wildcards — so the role can only be assumed by this one CHC service.
///
/// Tags are attached best-effort after creation; the `Integrations_Tester` SSO
/// role can `CreateRole` but is denied `iam:TagRole`, so tagging at create-time
/// would fail the whole call. Cleanup is tracked via `AwsCleanupRegistry`, so
/// missing tags don't affect resource lifecycle.
pub async fn create_clickpipes_iam_role(
    iam: &aws_sdk_iam::Client,
    role_name: &str,
    service_principal_arn: &str,
    inline_policy_doc: &str,
    tags: &[(String, String)],
) -> TestResult<String> {
    use aws_sdk_iam::types::Tag;

    let trust_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Effect": "Allow",
            "Principal": { "AWS": service_principal_arn },
            "Action": "sts:AssumeRole"
        }]
    })
    .to_string();

    let resp = iam
        .create_role()
        .role_name(role_name)
        .assume_role_policy_document(trust_policy)
        .send()
        .await?;
    let role_arn = resp
        .role()
        .map(|r| r.arn().to_string())
        .ok_or("CreateRole returned no role")?;

    if !tags.is_empty() {
        let aws_tags: Vec<Tag> = tags
            .iter()
            .map(|(k, v)| Tag::builder().key(k).value(v).build())
            .collect::<Result<_, _>>()?;
        if let Err(e) = iam.tag_role().role_name(role_name).set_tags(Some(aws_tags)).send().await {
            eprintln!("  warn: failed to tag iam role (continuing): {e}");
        }
    }

    iam.put_role_policy()
        .role_name(role_name)
        .policy_name(format!("{role_name}-inline"))
        .policy_document(inline_policy_doc)
        .send()
        .await?;

    Ok(role_arn)
}

// ── Kinesis Helpers ──────────────────────────────────────────────────

/// Create an on-demand Kinesis data stream with a single shard, wait for it to
/// enter `ACTIVE`, and tag it. Returns the stream's ARN.
pub async fn create_kinesis_stream(
    kinesis: &aws_sdk_kinesis::Client,
    stream_name: &str,
    tags: &[(String, String)],
) -> TestResult<String> {
    use aws_sdk_kinesis::types::{StreamMode, StreamModeDetails};

    kinesis
        .create_stream()
        .stream_name(stream_name)
        .stream_mode_details(
            StreamModeDetails::builder()
                .stream_mode(StreamMode::OnDemand)
                .build()?,
        )
        .send()
        .await?;

    // Wait for ACTIVE — on-demand streams typically reach ACTIVE within ~30s.
    let arn = poll_until(
        &format!("kinesis stream {stream_name} ACTIVE"),
        Duration::from_secs(300),
        Duration::from_secs(5),
        || {
            let kinesis = kinesis.clone();
            let stream_name = stream_name.to_string();
            async move {
                let resp = kinesis
                    .describe_stream_summary()
                    .stream_name(&stream_name)
                    .send()
                    .await?;
                let desc = resp
                    .stream_description_summary()
                    .ok_or("DescribeStreamSummary returned no summary")?;
                let status = desc.stream_status();
                if status.as_str() == "ACTIVE" {
                    Ok(Some(desc.stream_arn().to_string()))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await?;

    if !tags.is_empty() {
        let mut req = kinesis.add_tags_to_stream().stream_name(stream_name);
        for (k, v) in tags {
            req = req.tags(k, v);
        }
        if let Err(e) = req.send().await {
            eprintln!("  warn: failed to tag kinesis stream (continuing): {e}");
        }
    }

    Ok(arn)
}

/// Put a single JSON-encoded record onto the stream. Partition key is the
/// caller's choice — any string is fine for a 1-shard stream.
pub async fn put_kinesis_record(
    kinesis: &aws_sdk_kinesis::Client,
    stream_name: &str,
    partition_key: &str,
    body: &[u8],
) -> TestResult<()> {
    kinesis
        .put_record()
        .stream_name(stream_name)
        .partition_key(partition_key)
        .data(aws_sdk_kinesis::primitives::Blob::new(body.to_vec()))
        .send()
        .await?;
    Ok(())
}

pub async fn delete_kinesis_stream(
    kinesis: &aws_sdk_kinesis::Client,
    stream_name: &str,
) -> TestResult<()> {
    eprintln!("  cleanup: deleting kinesis stream");
    match kinesis
        .delete_stream()
        .stream_name(stream_name)
        .enforce_consumer_deletion(true)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("ResourceNotFoundException") => Ok(()),
        Err(e) => Err(e.into()),
    }
}
