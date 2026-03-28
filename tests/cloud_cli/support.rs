use serde_json::Value;
use std::env;
use std::fmt;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

const DEFAULT_CREATE_TIMEOUT_SECS: u64 = 1_800;
const DEFAULT_DELETE_TIMEOUT_SECS: u64 = 900;
const DEFAULT_STEADY_STATE_TIMEOUT_SECS: u64 = 1_800;
const DEFAULT_POLL_INTERVAL_SECS: u64 = 10;

pub struct TestContext {
    pub api_key: String,
    pub api_secret: String,
    pub org_id: String,
    pub provider: String,
    pub region: String,
    pub run_id: String,
    pub temp_home: TempDir,
    pub create_timeout: Duration,
    pub delete_timeout: Duration,
    pub steady_state_timeout: Duration,
    pub poll_interval: Duration,
    pub continue_on_non_blocking_failures: bool,
}

impl fmt::Debug for TestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestContext")
            .field("api_key", &"<redacted>")
            .field("api_secret", &"<redacted>")
            .field("org_id", &"<redacted>")
            .field("provider", &self.provider)
            .field("region", &self.region)
            .field("run_id", &self.run_id)
            .field("temp_home", &self.temp_home.path())
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

        let temp_home = tempfile::Builder::new()
            .prefix(&format!("clickhousectl-it-{run_id}-"))
            .tempdir()?;

        Ok(Self {
            api_key: required_env("CLICKHOUSE_CLOUD_API_KEY")?,
            api_secret: required_env("CLICKHOUSE_CLOUD_API_SECRET")?,
            org_id: required_env("CLICKHOUSE_CLOUD_TEST_ORG_ID")?,
            provider: required_env("CLICKHOUSE_CLOUD_TEST_PROVIDER")?,
            region: required_env("CLICKHOUSE_CLOUD_TEST_REGION")?,
            run_id,
            temp_home,
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

    pub fn temp_home_path(&self) -> &std::path::Path {
        self.temp_home.path()
    }

    pub fn updated_service_name(&self) -> String {
        format!("{}-updated", self.service_name())
    }

    pub fn run_tags(&self) -> Vec<String> {
        vec![
            "managed-by=clickhousectl-it".to_string(),
            "suite=service-crud".to_string(),
            format!("run-id={}", self.run_id),
        ]
    }
}

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
    pub fn run<T, F>(
        &mut self,
        ctx: &TestContext,
        kind: StepKind,
        step_name: &str,
        step: F,
    ) -> TestResult<Option<T>>
    where
        F: FnOnce() -> TestResult<T>,
    {
        log_step_start(kind, step_name);
        match step() {
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

pub struct CliRunner<'a> {
    ctx: &'a TestContext,
    binary_path: std::path::PathBuf,
}

impl<'a> CliRunner<'a> {
    pub fn new(ctx: &'a TestContext) -> Self {
        Self {
            ctx,
            binary_path: resolve_binary_path(),
        }
    }

    pub fn run_cloud<I, S>(&self, args: I) -> TestResult<CliOutput>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut command_args = vec!["cloud".to_string(), "--json".to_string()];
        command_args.extend(args.into_iter().map(Into::into));
        self.run(command_args)
    }

    pub fn run_cloud_raw<I, S>(&self, args: I) -> TestResult<RawCliOutput>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut command_args = vec!["cloud".to_string(), "--json".to_string()];
        command_args.extend(args.into_iter().map(Into::into));
        self.run_raw(command_args)
    }

    pub fn service_get(&self, service_id: &str) -> TestResult<CliOutput> {
        self.run_cloud([
            "service".to_string(),
            "get".to_string(),
            service_id.to_string(),
            "--org-id".to_string(),
            self.ctx.org_id.clone(),
        ])
    }

    pub fn service_list_for_run(&self) -> TestResult<CliOutput> {
        let mut args = vec![
            "service".to_string(),
            "list".to_string(),
            "--org-id".to_string(),
            self.ctx.org_id.clone(),
        ];

        for tag in self.ctx.run_tags() {
            args.push("--filter".to_string());
            args.push(format!("tag:{tag}"));
        }

        self.run_cloud(args)
    }

    pub fn service_client_query(
        &self,
        service_id: &str,
        password: &str,
        query: &str,
    ) -> TestResult<RawCliOutput> {
        let args = vec![
            "cloud".to_string(),
            "service".to_string(),
            "client".to_string(),
            "--id".to_string(),
            service_id.to_string(),
            "--password".to_string(),
            password.to_string(),
            "--org-id".to_string(),
            self.ctx.org_id.clone(),
            "-q".to_string(),
            query.to_string(),
        ];
        self.run_raw(args)
    }

    pub fn service_client_query_by_name(
        &self,
        service_name: &str,
        password: &str,
        query: &str,
    ) -> TestResult<RawCliOutput> {
        let args = vec![
            "cloud".to_string(),
            "service".to_string(),
            "client".to_string(),
            "--name".to_string(),
            service_name.to_string(),
            "--password".to_string(),
            password.to_string(),
            "--org-id".to_string(),
            self.ctx.org_id.clone(),
            "-q".to_string(),
            query.to_string(),
        ];
        self.run_raw(args)
    }

    pub fn service_stop(&self, service_id: &str) -> TestResult<CliOutput> {
        self.run_cloud([
            "service".to_string(),
            "stop".to_string(),
            service_id.to_string(),
            "--org-id".to_string(),
            self.ctx.org_id.clone(),
        ])
    }

    fn run(&self, args: Vec<String>) -> TestResult<CliOutput> {
        let raw = self.run_raw(args)?;
        CliOutput::from_raw(raw)
    }

    fn run_raw(&self, args: Vec<String>) -> TestResult<RawCliOutput> {
        let redacted_command = redact_command(&self.binary_path, &args);
        log_command_start(&redacted_command);
        let started = Instant::now();
        let output = Command::new(&self.binary_path)
            .args(&args)
            .env("HOME", self.ctx.temp_home_path())
            .env("CLICKHOUSE_CLOUD_API_KEY", &self.ctx.api_key)
            .env("CLICKHOUSE_CLOUD_API_SECRET", &self.ctx.api_secret)
            .output()?;
        let elapsed = started.elapsed();
        let raw = RawCliOutput::from_output(output, elapsed, redacted_command)?;
        log_command_ok(raw.status_code, raw.elapsed);
        Ok(raw)
    }
}

pub struct RawCliOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
    pub redacted_command: String,
}

impl fmt::Debug for RawCliOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawCliOutput")
            .field("status_code", &self.status_code)
            .field("stdout", &redact_text_ids(&self.stdout))
            .field("stderr", &redact_text_ids(&self.stderr))
            .field("elapsed", &self.elapsed)
            .field("redacted_command", &self.redacted_command)
            .finish()
    }
}

impl RawCliOutput {
    fn from_output(
        output: Output,
        elapsed: Duration,
        redacted_command: String,
    ) -> TestResult<Self> {
        let status_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        let redacted_stdout = redact_text_ids(&stdout);
        let redacted_stderr = redact_text_ids(&stderr);

        if !output.status.success() {
            return Err(Box::new(CommandFailure {
                redacted_command,
                status_code,
                stdout: redacted_stdout,
                stderr: redacted_stderr,
                elapsed,
                parse_error: None,
            }));
        }

        Ok(Self {
            status_code,
            stdout,
            stderr,
            elapsed,
            redacted_command,
        })
    }
}

#[allow(dead_code)]
pub struct CliOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
    pub json: Value,
    pub redacted_command: String,
}

impl fmt::Debug for CliOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CliOutput")
            .field("status_code", &self.status_code)
            .field("stdout", &redact_text_ids(&self.stdout))
            .field("stderr", &redact_text_ids(&self.stderr))
            .field("elapsed", &self.elapsed)
            .field("json", &redact_json_ids(&self.json))
            .field("redacted_command", &self.redacted_command)
            .finish()
    }
}

impl CliOutput {
    fn from_raw(raw: RawCliOutput) -> TestResult<Self> {
        let json = serde_json::from_str(&raw.stdout).map_err(|error| {
            Box::new(CommandFailure {
                redacted_command: raw.redacted_command.clone(),
                status_code: raw.status_code,
                stdout: raw.stdout.clone(),
                stderr: raw.stderr.clone(),
                elapsed: raw.elapsed,
                parse_error: Some(error.to_string()),
            }) as Box<dyn std::error::Error>
        })?;

        Ok(Self {
            status_code: raw.status_code,
            stdout: raw.stdout,
            stderr: raw.stderr,
            elapsed: raw.elapsed,
            json,
            redacted_command: raw.redacted_command,
        })
    }
}

#[derive(Default)]
pub struct CleanupRegistry {
    service_ids: Vec<String>,
}

impl CleanupRegistry {
    pub fn register_service(&mut self, service_id: impl Into<String>) {
        self.service_ids.push(service_id.into());
    }

    pub fn unregister_service(&mut self, service_id: &str) {
        self.service_ids
            .retain(|registered| registered != service_id);
    }

    pub fn cleanup(&mut self, runner: &CliRunner<'_>) -> Result<(), String> {
        let mut failures = Vec::new();

        while let Some(service_id) = self.service_ids.pop() {
            if let Err(error) = ensure_service_gone(runner, &service_id) {
                failures.push(format!("{service_id}: {error}"));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n"))
        }
    }
}

pub fn poll_until<F, T>(
    description: &str,
    timeout: Duration,
    interval: Duration,
    mut check: F,
) -> TestResult<T>
where
    F: FnMut() -> TestResult<Option<T>>,
{
    let started = Instant::now();
    let mut last_error: Option<String> = None;
    eprintln!("  poll: waiting for {description}");

    loop {
        match check() {
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

        thread::sleep(interval);
    }
}

pub fn json_string<'a>(value: &'a Value, pointers: &[&str]) -> TestResult<&'a str> {
    for pointer in pointers {
        if let Some(value) = value.pointer(pointer).and_then(Value::as_str) {
            return Ok(value);
        }
    }

    Err(format!("missing string at any of {:?}", pointers).into())
}

pub fn json_string_opt<'a>(value: &'a Value, pointers: &[&str]) -> Option<&'a str> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
}

pub fn service_present_in_list(value: &Value, service_id: &str) -> bool {
    if let Some(services) = value.pointer("/services").and_then(Value::as_array) {
        return services
            .iter()
            .any(|service| json_string_opt(service, &["/id"]).is_some_and(|id| id == service_id));
    }

    if let Some(services) = value.as_array() {
        return services
            .iter()
            .any(|service| json_string_opt(service, &["/id"]).is_some_and(|id| id == service_id));
    }

    false
}

pub fn service_name_in_list(value: &Value, service_id: &str) -> Option<String> {
    let services = value
        .pointer("/services")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());

    services.and_then(|services| {
        services.iter().find_map(|service| {
            let id = json_string_opt(service, &["/id"])?;
            let name = json_string_opt(service, &["/name"])?;
            (id == service_id).then(|| name.to_string())
        })
    })
}

pub fn service_list_is_empty(value: &Value) -> bool {
    if let Some(services) = value.pointer("/services").and_then(Value::as_array) {
        return services.is_empty();
    }

    if let Some(services) = value.as_array() {
        return services.is_empty();
    }

    false
}

pub fn service_has_ip_access_entry(value: &Value, source: &str) -> bool {
    let entries = value
        .pointer("/service/ipAccessList")
        .or_else(|| value.pointer("/ipAccessList"));

    entries.and_then(Value::as_array).is_some_and(|entries| {
        entries.iter().any(|entry| {
            entry
                .pointer("/source")
                .and_then(Value::as_str)
                .is_some_and(|candidate| candidate == source)
        })
    })
}

// Strict deletion is used by the test body itself to verify that the explicit delete
// command succeeded for a still-existing service. If the service is already gone here,
// the delete behavior was not actually exercised and the test should fail.
pub fn delete_service_and_confirm_gone(runner: &CliRunner<'_>, service_id: &str) -> TestResult<()> {
    eprintln!("  delete: deleting service");
    request_service_deletion(runner, service_id, false)?;
    wait_for_service_to_disappear(runner, service_id)
}

// Cleanup teardown has different semantics from the strict test assertion above:
// its job is only to ensure that no cloud resources are left behind. "Already gone"
// is therefore a successful end state and should not cause cleanup to fail.
pub fn ensure_service_gone(runner: &CliRunner<'_>, service_id: &str) -> TestResult<()> {
    eprintln!("  cleanup: deleting service");
    request_service_deletion(runner, service_id, true)?;
    wait_for_service_to_disappear(runner, service_id)
}

fn request_service_deletion(
    runner: &CliRunner<'_>,
    service_id: &str,
    allow_missing_service: bool,
) -> TestResult<()> {
    let delete_args = [
        "service".to_string(),
        "delete".to_string(),
        service_id.to_string(),
        "--org-id".to_string(),
        runner.ctx.org_id.clone(),
    ];

    match runner.run_cloud_raw(delete_args.clone()) {
        Ok(_) => {}
        Err(error) => {
            let message = error.to_string();
            if allow_missing_service && service_missing(&message) {
                return Ok(());
            }

            if message.contains("409 Conflict") && message.contains("Current state: 'running'") {
                eprintln!("  cleanup: service is running, stopping before delete");
                let _ = runner.service_stop(service_id)?;
                poll_until(
                    "service stop before deletion",
                    runner.ctx.delete_timeout,
                    runner.ctx.poll_interval,
                    || {
                        let output = runner.service_get(service_id)?;
                        let state = json_string(&output.json, &["/service/state", "/state"])?;
                        if matches!(state, "idle" | "stopped" | "degraded" | "failed") {
                            Ok(Some(()))
                        } else {
                            Ok(None)
                        }
                    },
                )?;

                match runner.run_cloud_raw(delete_args) {
                    Ok(_) => {}
                    Err(error) => {
                        let message = error.to_string();
                        if allow_missing_service && service_missing(&message) {
                            return Ok(());
                        }
                        return Err(error);
                    }
                }
            } else {
                return Err(error);
            }
        }
    }

    Ok(())
}

fn wait_for_service_to_disappear(runner: &CliRunner<'_>, service_id: &str) -> TestResult<()> {
    poll_until(
        "service deletion",
        runner.ctx.delete_timeout,
        runner.ctx.poll_interval,
        || match runner.service_get(service_id) {
            Ok(output) => {
                let state = json_string_opt(&output.json, &["/service/state", "/state"]);
                if matches!(state, Some("deleted") | Some("deleting")) {
                    return Ok(None);
                }
                Ok(None)
            }
            Err(error) => {
                let message = error.to_string();
                if message.contains("404") || message.contains("not found") {
                    Ok(Some(()))
                } else {
                    Err(error)
                }
            }
        },
    )?;

    Ok(())
}

fn service_missing(message: &str) -> bool {
    message.contains("404") || message.contains("not found")
}

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
    Ok(env::var(name).map_err(|_| format!("missing required environment variable {name}"))?)
}

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

fn log_command_start(command: &str) {
    eprintln!("  cmd: {}", command);
}

fn log_command_ok(status_code: i32, elapsed: Duration) {
    eprintln!("  cmd result: status={} elapsed={:?}", status_code, elapsed);
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

fn resolve_binary_path() -> std::path::PathBuf {
    if let Ok(path) = env::var("CLICKHOUSECTL_BIN") {
        return std::path::PathBuf::from(path);
    }

    if let Some(path) = option_env!("CARGO_BIN_EXE_clickhousectl") {
        return std::path::PathBuf::from(path);
    }

    std::path::PathBuf::from("target/debug/clickhousectl")
}

fn redact_command(binary_path: &std::path::PathBuf, args: &[String]) -> String {
    let mut rendered = vec![binary_path.display().to_string()];
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            rendered.push("<redacted>".to_string());
            redact_next = false;
            continue;
        }

        if arg == "--org-id" {
            rendered.push(arg.clone());
            redact_next = true;
            continue;
        }

        if looks_like_uuid(arg) {
            rendered.push("<redacted-id>".to_string());
            continue;
        }

        rendered.push(arg.clone());
    }

    rendered.join(" ")
}

fn looks_like_uuid(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 5 {
        return false;
    }

    let expected = [8, 4, 4, 4, 12];
    parts
        .iter()
        .zip(expected.iter())
        .all(|(part, len)| part.len() == *len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

fn redact_text_ids(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut token = String::new();

    for ch in input.chars() {
        if ch.is_ascii_hexdigit() || ch == '-' {
            token.push(ch);
            continue;
        }

        flush_redacted_token(&mut output, &mut token);
        output.push(ch);
    }

    flush_redacted_token(&mut output, &mut token);
    output
}

fn flush_redacted_token(output: &mut String, token: &mut String) {
    if token.is_empty() {
        return;
    }

    if looks_like_uuid(token) {
        output.push_str("<redacted-id>");
    } else {
        output.push_str(token);
    }

    token.clear();
}

fn redact_json_ids(value: &Value) -> Value {
    match value {
        Value::String(text) if looks_like_uuid(text) => Value::String("<redacted-id>".to_string()),
        Value::Array(items) => Value::Array(items.iter().map(redact_json_ids).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_json_ids(value)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[derive(Debug)]
struct CommandFailure {
    redacted_command: String,
    status_code: i32,
    stdout: String,
    stderr: String,
    elapsed: Duration,
    parse_error: Option<String>,
}

impl fmt::Display for CommandFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "command failed: {}\nstatus: {}\nelapsed: {:?}\nstdout:\n{}\nstderr:\n{}",
            self.redacted_command, self.status_code, self.elapsed, self.stdout, self.stderr
        )?;

        if let Some(parse_error) = &self.parse_error {
            write!(f, "\njson parse error: {parse_error}")?;
        }

        Ok(())
    }
}

impl std::error::Error for CommandFailure {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatusExt::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatusExt::from_raw(code as u32)
    }

    #[test]
    fn cli_output_preserves_uuid_values_for_assertions() {
        let raw = RawCliOutput::from_output(
            Output {
                status: exit_status(0),
                stdout: br#"{"id":"5fae43a3-8a6e-49c4-b317-6e139718b9a3"}"#.to_vec(),
                stderr: Vec::new(),
            },
            Duration::from_secs(1),
            "clickhousectl cloud --json org get <redacted-id>".to_string(),
        )
        .expect("successful command output");

        let output = CliOutput::from_raw(raw).expect("valid json");

        assert_eq!(
            json_string(&output.json, &["/id"]).expect("json id"),
            "5fae43a3-8a6e-49c4-b317-6e139718b9a3"
        );
        assert!(format!("{output:?}").contains("<redacted-id>"));
    }

    #[test]
    fn command_failure_redacts_uuid_values_in_output() {
        let error = RawCliOutput::from_output(
            Output {
                status: exit_status(1),
                stdout: br#"{"id":"5fae43a3-8a6e-49c4-b317-6e139718b9a3"}"#.to_vec(),
                stderr: b"service 5fae43a3-8a6e-49c4-b317-6e139718b9a3 failed".to_vec(),
            },
            Duration::from_secs(1),
            "clickhousectl cloud --json org get <redacted-id>".to_string(),
        )
        .expect_err("failing command should return an error");

        let rendered = error.to_string();
        assert!(rendered.contains("<redacted-id>"));
        assert!(!rendered.contains("5fae43a3-8a6e-49c4-b317-6e139718b9a3"));
    }
}
