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

    pub fn clickpipe_s3_service_name(&self) -> String {
        format!("clickhousectl-it-cp-s3-{}", self.run_id)
    }

    pub fn clickpipe_s3_run_tags(&self) -> Vec<ResourceTagsV1> {
        vec![
            ResourceTagsV1 {
                key: "managed_by".to_string(),
                value: Some("clickhousectl_it".to_string()),
            },
            ResourceTagsV1 {
                key: "suite".to_string(),
                value: Some("clickpipe_s3".to_string()),
            },
            ResourceTagsV1 {
                key: "run_id".to_string(),
                value: Some(self.run_id.clone()),
            },
        ]
    }

    pub fn clickpipe_s3_run_tag_filters(&self) -> Vec<String> {
        vec![
            "tag:managed_by=clickhousectl_it".to_string(),
            "tag:suite=clickpipe_s3".to_string(),
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

// ── AWS Cleanup Registry ─────────────────────────────────────────────
//
// Tracks AWS-side resources (S3 buckets, IAM roles) created by E2E tests
// against integration sources, so a single teardown call removes them
// regardless of which test step failed.

#[derive(Default)]
pub struct AwsCleanupRegistry {
    s3_buckets: Vec<(aws_sdk_s3::config::Region, String)>,
    iam_roles: Vec<String>,
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

    pub async fn cleanup(
        &mut self,
        aws_config: &aws_config::SdkConfig,
        iam_client: &aws_sdk_iam::Client,
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
