// Generic integration-test infrastructure: env-driven test context, failure
// recording, polling, logging, ClickHouse Cloud provisioning, and the HTTP
// query helper. Shared by every integration binary (cloud CRUD lifecycle and
// ClickPipes E2E). ClickPipes-specific AWS/EC2/Kinesis/Redpanda helpers live
// in `tests/clickpipes/support.rs`, which re-exports this module so callers
// can pull both surfaces from a single `use crate::support::*;`.
//
// Each test binary uses a different subset — silence dead_code for the rest.
#![allow(dead_code)]

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
        let label = env::var("CLICKHOUSE_CLOUD_TEST_RUN_LABEL").ok();
        let github_run_id = env::var("GITHUB_RUN_ID").ok();
        let github_sha = env::var("GITHUB_SHA").ok();
        let sha7 = github_sha
            .as_deref()
            .map(|s| s[..7.min(s.len())].to_string());
        let run_id = match (label, github_run_id, sha7) {
            (Some(label), _, Some(sha)) => format!("{label}-{sha}"),
            (Some(label), _, None) => label,
            (None, Some(rid), Some(sha)) => format!("{rid}-{sha}"),
            (None, Some(rid), None) => rid,
            (None, None, Some(sha)) => format!("local-{sha}"),
            (None, None, None) => format!("local-{timestamp}"),
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
    /// `default.<table>` names to DROP during teardown. Only relevant when
    /// the service is NOT being deleted (attach mode); when the service is
    /// being deleted, table drops are redundant but harmless.
    tables: Vec<String>,
    api_key_ids: Vec<String>,
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

    /// Register a `default.<table>` destination table so teardown drops it.
    /// Stages call this just before pipe-create — once the table exists,
    /// ClickPipes refuses a future pipe-create against the same name unless
    /// it's empty, so test re-runs against a shared CHC service collide.
    pub fn register_table(&mut self, table: impl Into<String>) {
        self.tables.push(table.into());
    }

    /// Drain another registry's entries into this one. Used by the parallel
    /// driver to fold per-stage registries back into the parent for teardown.
    pub fn merge_from(&mut self, mut other: CleanupRegistry) {
        self.service_ids.append(&mut other.service_ids);
        self.postgres_ids.append(&mut other.postgres_ids);
        self.clickpipes.append(&mut other.clickpipes);
        self.tables.append(&mut other.tables);
        self.api_key_ids.append(&mut other.api_key_ids);
    }

    pub fn unregister_clickpipe(&mut self, service_id: &str, clickpipe_id: &str) {
        self.clickpipes
            .retain(|(svc, pipe)| !(svc == service_id && pipe == clickpipe_id));
    }

    pub fn register_api_key(&mut self, key_id: impl Into<String>) {
        self.api_key_ids.push(key_id.into());
    }

    pub fn unregister_api_key(&mut self, key_id: &str) {
        self.api_key_ids
            .retain(|registered| registered != key_id);
    }

    pub async fn cleanup(
        &mut self,
        client: &Client,
        org_id: &str,
        delete_timeout: Duration,
        poll_interval: Duration,
        ch_query: Option<&ClickHouseQuery>,
    ) -> Result<(), String> {
        let mut failures = Vec::new();

        // API keys are cleaned up first; they belong to the org, not a
        // specific service, so they outlive service deletion if leaked.
        while let Some(key_id) = self.api_key_ids.pop() {
            match client.openapi_key_delete(org_id, &key_id).await {
                Ok(_) => {}
                Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => {}
                Err(e) => failures.push(format!("api key {key_id}: {e}")),
            }
        }

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

        // Drop registered destination tables BEFORE deleting the service —
        // once the service is gone we can't query. If the service is also
        // being deleted, drops are redundant but harmless; if we're attached
        // to a shared service, drops are essential to avoid re-run collision.
        if let Some(query) = ch_query {
            while let Some(table) = self.tables.pop() {
                eprintln!("  cleanup: drop table default.{table}");
                if let Err(error) = query
                    .run_query(&format!("DROP TABLE IF EXISTS default.{table}"))
                    .await
                {
                    failures.push(format!("drop table {table}: {error}"));
                }
            }
        } else {
            // No query helper passed — clear the queue (service deletion will
            // take the tables with it).
            self.tables.clear();
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

pub fn required_env(name: &str) -> TestResult<String> {
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
