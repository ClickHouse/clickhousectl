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
                key: "managed-by".to_string(),
                value: Some("clickhousectl-it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("service-crud".to_string()),
            },
            ResourceTagsV1 {
                key: "run-id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed-by=clickhousectl-it".to_string(),
            "tag:suite=service-crud".to_string(),
            format!("tag:run-id={}", self.run_id),
        ]
    }

    pub fn postgres_service_name(&self) -> String {
        format!("clickhousectl-it-pg-{}", self.run_id)
    }

    pub fn postgres_run_tags(&self) -> Vec<ResourceTagsV1> {
        vec![
            ResourceTagsV1 {
                key: "managed-by".to_string(),
                value: Some("clickhousectl-it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("postgres-crud".to_string()),
            },
            ResourceTagsV1 {
                key: "run-id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn postgres_run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed-by=clickhousectl-it".to_string(),
            "tag:suite=postgres-crud".to_string(),
            format!("tag:run-id={}", self.run_id),
        ]
    }
}

pub fn create_client() -> TestResult<Client> {
    let key = required_env("CLICKHOUSE_CLOUD_API_KEY")?;
    let secret = required_env("CLICKHOUSE_CLOUD_API_SECRET")?;
    Ok(Client::new(key, secret))
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

    pub async fn cleanup(&mut self, client: &Client, org_id: &str, delete_timeout: Duration, poll_interval: Duration) -> Result<(), String> {
        let mut failures = Vec::new();

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
