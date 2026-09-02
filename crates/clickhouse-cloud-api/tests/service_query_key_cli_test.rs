//! Live coverage of rejected stored Query API key classification (#528)
//! through the real `clickhousectl` binary.
//!
//! Provisions a disposable ClickHouse Cloud service, lets `cloud service
//! query` auto-provision and store a per-service key, then disables and later
//! expires that exact key through the management API. Each time, the CLI must
//! report the state with its stable JSON code, keep the local record
//! byte-for-byte, and create nothing in the organization. Finally the explicit
//! `repair-query-key` command, the documented way forward, replaces the
//! expired key and a query succeeds again.
//!
//! Every resource is created here and torn down here: the service, the key the
//! CLI provisions, the key the repair creates, and the query endpoint binding.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use chrono::{DateTime, Utc};
use clickhouse_cloud_api::Client;
use clickhouse_cloud_api::models::*;
use common::support::*;
use serde_json::Value;

const CLICKHOUSECTL_BIN_ENV: &str = "CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN";
/// How long a management-side change (disable, expire) may take before the
/// Query API enforces it and the CLI can classify the rejection.
const DEFAULT_REJECTION_TIMEOUT_SECS: u64 = 300;
/// How far ahead to set `expireAt` when the API refuses a time in the past.
const EXPIRY_LEAD: Duration = Duration::from_secs(90);
/// Clock-skew margin waited after `expireAt` before expecting rejection.
const EXPIRY_MARGIN: Duration = Duration::from_secs(15);

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_service_query_key_disabled_and_expired_are_never_replaced() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let clickhousectl = clickhousectl_binary()?;
    let cli_workspace = tempfile::tempdir()?;
    let cli_home = cli_workspace.path().join("home");
    std::fs::create_dir(&cli_home)?;
    let cli = Cli {
        binary: clickhousectl,
        workdir: cli_workspace.path().to_path_buf(),
        home: cli_home,
        api_url: clickhouse_cloud_api_url(),
        org_id: ctx.org_id.clone(),
    };
    let rejection_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_QUERY_KEY_REJECTION_SECS",
        DEFAULT_REJECTION_TIMEOUT_SECS,
    )?;

    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();
    let service_name = format!("{}-qk", ctx.service_name());

    let test_result = async {
        log_run_header(
            "cloud_service_query_key_disabled_and_expired_are_never_replaced",
            &ctx,
        );
        let mut failures = FailureRecorder::default();

        // ── Provision ───────────────────────────────────────────────

        log_phase("Provision Service");

        let create_body = ServicePostRequest {
            name: service_name.clone(),
            provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
            region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
            min_replica_memory_gb: Some(8.0),
            max_replica_memory_gb: Some(8.0),
            num_replicas: Some(1),
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(5.0),
            tags: Some(ctx.run_tags()),
            ..Default::default()
        };
        let created = failures
            .run(&ctx, StepKind::Blocking, "create service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let body = create_body.clone();
                async move {
                    let resp = client.instance_create(&org_id, &body).await?;
                    resp.result
                        .ok_or_else(|| "service create returned no result".into())
                }
            })
            .await?
            .expect("blocking steps always return a value");
        let service = require_field(created.service, "service")?;
        let service_id = require_field(service.id, "service.id")?.to_string();
        eprintln!("service_id: <redacted>");
        cleanup.register_service(service_id.clone());

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for service steady state",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        wait_for_service_running(
                            &client,
                            &org_id,
                            &service_id,
                            ctx.steady_state_timeout,
                            ctx.poll_interval,
                        )
                        .await
                    }
                },
            )
            .await?;

        // ── First query provisions and stores a key ─────────────────

        log_phase("First query provisions a per-service key");

        let first_query = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "first query auto-provisions and stores a per-service key",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move { cli.query(&service_id, false) }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        // Register whatever the CLI provisioned *before* judging the run, so a
        // failed assertion below never leaves the key behind. The endpoint
        // binding dies with the service; the key does not.
        cleanup.register_query_endpoint(service_id.clone());
        let stored = cli.stored_key(&service_id);
        if let Ok(stored) = &stored {
            cleanup.register_api_key(stored.api_key_id.clone());
        }
        if !first_query.status.success() {
            return Err(cli_failure("service query (first use)", &first_query).into());
        }
        let stdout = String::from_utf8_lossy(&first_query.stdout);
        if stdout.trim() != "1" {
            return Err(format!("expected `SELECT 1` to print 1, got {stdout:?}").into());
        }
        // The record must carry the exact ownership metadata the classifier
        // and the repair command rely on.
        let stored = stored?;
        let api_key_id = stored.api_key_id.clone();
        let credentials_before = cli.credentials_file()?;

        // ── Disabled ────────────────────────────────────────────────

        log_phase("Disabled key");

        failures
            .run(&ctx, StepKind::Blocking, "disable the stored key", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let api_key_id = api_key_id.clone();
                async move {
                    client
                        .openapi_key_update(
                            &org_id,
                            &api_key_id,
                            &key_patch(Some(ApiKeyPatchRequestState::Disabled), None),
                        )
                        .await?;
                    Ok(())
                }
            })
            .await?;

        let disabled = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "query with a disabled key is refused as query_key_disabled",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move {
                        cli.poll_for_rejection(
                            &service_id,
                            "query_key_disabled",
                            rejection_timeout,
                            ctx.poll_interval,
                        )
                        .await
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert_eq!(disabled["error"]["api_key_id"], api_key_id);
        assert_eq!(
            disabled["error"]["command"],
            format!(
                "clickhousectl cloud service repair-query-key {service_id} --org-id {}",
                ctx.org_id
            )
        );
        let message = disabled["error"]["message"].as_str().unwrap_or_default();
        assert!(message.contains("is disabled"), "{message}");
        assert!(
            message.contains(&format!(
                "clickhousectl cloud key update {api_key_id} --state enabled"
            )),
            "{message}"
        );
        assert!(
            !message.contains(&stored.key_secret),
            "the stored secret must never be printed"
        );
        assert_eq!(
            cli.credentials_file()?,
            credentials_before,
            "a disabled key must leave the local record untouched"
        );

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "no key was created and the binding was not changed",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let api_key_id = api_key_id.clone();
                    let key_name = format!("clickhousectl-query-{service_name}");
                    async move {
                        assert_single_owned_key(
                            &client,
                            &org_id,
                            &service_id,
                            &key_name,
                            &api_key_id,
                        )
                        .await
                    }
                },
            )
            .await?;

        // ── Expired ─────────────────────────────────────────────────

        log_phase("Expired key");

        let expire_at = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "re-enable the stored key and let it expire",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let api_key_id = api_key_id.clone();
                    async move { expire_key(&client, &org_id, &api_key_id).await }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        let now = Utc::now();
        if expire_at > now {
            let wait = (expire_at - now).to_std().unwrap_or_default() + EXPIRY_MARGIN;
            eprintln!("  step: waiting {wait:?} for the key to expire");
            tokio::time::sleep(wait).await;
        }

        let expired = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "query with an expired key is refused as query_key_expired",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move {
                        cli.poll_for_rejection(
                            &service_id,
                            "query_key_expired",
                            rejection_timeout,
                            ctx.poll_interval,
                        )
                        .await
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert_eq!(expired["error"]["api_key_id"], api_key_id);
        let message = expired["error"]["message"].as_str().unwrap_or_default();
        assert!(message.contains("expired at"), "{message}");
        assert!(message.contains("repair-query-key"), "{message}");
        assert_eq!(
            cli.credentials_file()?,
            credentials_before,
            "an expired key must leave the local record untouched"
        );

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "still no key was created and the binding was not changed",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let api_key_id = api_key_id.clone();
                    let key_name = format!("clickhousectl-query-{service_name}");
                    async move {
                        assert_single_owned_key(
                            &client,
                            &org_id,
                            &service_id,
                            &key_name,
                            &api_key_id,
                        )
                        .await
                    }
                },
            )
            .await?;

        // ── Explicit repair ─────────────────────────────────────────

        log_phase("Explicit repair");

        eprintln!("  step: stored management key id before repair: {api_key_id}");
        let repaired = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "repair-query-key replaces the expired key deliberately",
                || {
                    let cli = cli.clone();
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let api_key_id = api_key_id.clone();
                    async move {
                        let mut output = cli.repair(&service_id)?;
                        if !output.status.success() {
                            eprintln!(
                                "  diag: first repair attempt failed: {}",
                                cli_failure("service repair-query-key", &output)
                            );
                            match client
                                .instance_query_endpoint_get(&org_id, &service_id)
                                .await
                            {
                                Ok(resp) => eprintln!(
                                    "  diag: endpoint binding now: {:?}",
                                    resp.result.and_then(|e| e.open_api_keys)
                                ),
                                Err(e) => eprintln!("  diag: endpoint get failed: {e}"),
                            }
                            match client.openapi_key_get(&org_id, &api_key_id).await {
                                Ok(resp) => eprintln!(
                                    "  diag: old key now: state={:?} expireAt={:?}",
                                    resp.result.as_ref().and_then(|k| k.state.clone()),
                                    resp.result.as_ref().and_then(|k| k.expire_at)
                                ),
                                Err(e) => eprintln!("  diag: old key get failed: {e}"),
                            }
                            // Observed once on 2026-09-02: the endpoint upsert
                            // inside repair answered `400 BAD_REQUEST: OpenAPI
                            // key <id> does not belong to the organization`
                            // right after the replacement key was created, and
                            // an identical run passed. The CLI itself does not
                            // retry, so the diagnostics above are printed to
                            // make a recurrence visible in CI logs; one retry
                            // keeps a transient from failing the suite.
                            eprintln!("  diag: retrying repair once after 20s");
                            tokio::time::sleep(Duration::from_secs(20)).await;
                            output = cli.repair(&service_id)?;
                        }
                        if !output.status.success() {
                            return Err(cli_failure("service repair-query-key", &output).into());
                        }
                        let result: Value = serde_json::from_slice(&output.stdout)?;
                        Ok(result)
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert_eq!(repaired["status"], "repaired", "{repaired}");
        assert_eq!(repaired["replacedApiKeyId"], api_key_id, "{repaired}");
        let new_api_key_id = repaired["apiKeyId"]
            .as_str()
            .ok_or("repair result has no apiKeyId")?
            .to_string();
        assert_ne!(new_api_key_id, api_key_id);
        cleanup.register_api_key(new_api_key_id.clone());
        let stored_after = cli.stored_key(&service_id)?;
        assert_eq!(stored_after.api_key_id, new_api_key_id);
        assert!(
            stored_after.pending_cleanup_api_key_ids.is_empty(),
            "the superseded key was deleted, so nothing is pending"
        );

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "the expired key is gone and only the replacement is bound",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let old = api_key_id.clone();
                    let new = new_api_key_id.clone();
                    let key_name = format!("clickhousectl-query-{service_name}");
                    async move {
                        match client.openapi_key_get(&org_id, &old).await {
                            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => {}
                            Ok(_) => return Err("the superseded key still exists".into()),
                            Err(error) => return Err(error.into()),
                        }
                        assert_single_owned_key(&client, &org_id, &service_id, &key_name, &new)
                            .await
                    }
                },
            )
            .await?;
        cleanup.unregister_api_key(&api_key_id);

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "a query succeeds with the replacement key",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move {
                        // A fresh binding can be rejected for a moment while it
                        // converges; the CLI does not retry on this path, so
                        // the test does.
                        poll_until(
                            "query success with the replacement key",
                            rejection_timeout,
                            ctx.poll_interval,
                            || {
                                let cli = cli.clone();
                                let service_id = service_id.clone();
                                async move {
                                    let output = cli.query(&service_id, false)?;
                                    if output.status.success() {
                                        Ok(Some(()))
                                    } else {
                                        eprintln!(
                                            "  poll: query not yet accepted: {}",
                                            first_line(&String::from_utf8_lossy(&output.stderr))
                                        );
                                        Ok(None)
                                    }
                                }
                            },
                        )
                        .await
                    }
                },
            )
            .await?;

        failures.finish()
    }
    .await;

    log_phase("Cleanup");
    let cleanup_result = cleanup
        .cleanup(
            &client,
            &ctx.org_id,
            ctx.delete_timeout,
            ctx.poll_interval,
            None,
        )
        .await;

    test_result?;
    cleanup_result.map_err(|error| error.into())
}

// ── Helpers ──────────────────────────────────────────────────────────

/// The stored per-service record, as the CLI writes it to
/// `.clickhouse/credentials.json`. Only the fields this test reads.
#[derive(Debug, Clone, serde::Deserialize)]
struct StoredQueryKey {
    api_key_id: String,
    key_secret: String,
    #[serde(default)]
    pending_cleanup_api_key_ids: Vec<String>,
}

#[derive(Clone)]
struct Cli {
    binary: PathBuf,
    workdir: PathBuf,
    home: PathBuf,
    api_url: String,
    org_id: String,
}

impl Cli {
    fn command(&self, args: &[&str]) -> TestResult<Output> {
        // The CLI authenticates with the same management credentials the test
        // client uses; the environment is otherwise the test's own so that a
        // staging `CLICKHOUSE_CLOUD_QUERY_HOST`, if set, applies to both.
        Ok(Command::new(&self.binary)
            .current_dir(&self.workdir)
            .env("HOME", &self.home)
            .env("DO_NOT_TRACK", "1")
            .env(
                "CLICKHOUSE_CLOUD_API_KEY",
                required_env("CLICKHOUSE_CLOUD_API_KEY")?,
            )
            .env(
                "CLICKHOUSE_CLOUD_API_SECRET",
                required_env("CLICKHOUSE_CLOUD_API_SECRET")?,
            )
            .args(["cloud", "--url", &self.api_url])
            .args(args)
            .output()?)
    }

    /// `SELECT 1` against the service. The output format is always pinned:
    /// the CLI switches to JSON on its own when it detects a coding agent in
    /// the environment, and this harness may well run under one.
    fn query(&self, service_id: &str, json: bool) -> TestResult<Output> {
        let mut args = vec![
            "service",
            "query",
            "--id",
            service_id,
            "--org-id",
            &self.org_id,
            "--query",
            "SELECT 1",
        ];
        if json {
            args.push("--json");
        } else {
            args.extend_from_slice(&["--format", "TabSeparated"]);
        }
        self.command(&args)
    }

    fn repair(&self, service_id: &str) -> TestResult<Output> {
        self.command(&[
            "--json",
            "service",
            "repair-query-key",
            service_id,
            "--org-id",
            &self.org_id,
        ])
    }

    fn credentials_path(&self) -> PathBuf {
        self.workdir.join(".clickhouse").join("credentials.json")
    }

    fn credentials_file(&self) -> TestResult<Vec<u8>> {
        Ok(std::fs::read(self.credentials_path())?)
    }

    fn stored_key(&self, service_id: &str) -> TestResult<StoredQueryKey> {
        let credentials: Value = serde_json::from_slice(&self.credentials_file()?)?;
        let record = credentials["service_query_keys"]
            .get(service_id)
            .cloned()
            .ok_or("the CLI stored no per-service query key")?;
        Ok(serde_json::from_value(record)?)
    }

    /// Run `--json` queries until the CLI refuses with `expected_code`. A
    /// success means the management-side change has not reached the Query API
    /// yet; any other failure is reported at once.
    async fn poll_for_rejection(
        &self,
        service_id: &str,
        expected_code: &str,
        timeout: Duration,
        interval: Duration,
    ) -> TestResult<Value> {
        poll_until(
            &format!("the CLI to refuse the query with {expected_code}"),
            timeout,
            interval,
            || {
                let cli = self.clone();
                let service_id = service_id.to_string();
                let expected_code = expected_code.to_string();
                async move {
                    let output = cli.query(&service_id, true)?;
                    if output.status.success() {
                        eprintln!("  poll: the Query API still accepts the key");
                        return Ok(None);
                    }
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error: Value = serde_json::from_str(stderr.trim()).map_err(|e| {
                        format!("stderr is not one JSON error object ({e}): {stderr}")
                    })?;
                    if error["error"]["code"] != expected_code {
                        return Err(
                            format!("expected error code {expected_code}, got: {error}").into()
                        );
                    }
                    if output.status.code() != Some(1) {
                        return Err(format!(
                            "expected exit code 1, got {:?}",
                            output.status.code()
                        )
                        .into());
                    }
                    Ok(Some(error))
                }
            },
        )
        .await
    }
}

fn key_patch(
    state: Option<ApiKeyPatchRequestState>,
    expire_at: Option<DateTime<Utc>>,
) -> ApiKeyPatchRequest {
    ApiKeyPatchRequest {
        assigned_role_ids: None,
        expire_at,
        ip_access_list: None,
        name: None,
        #[cfg(feature = "deprecated-fields")]
        roles: None,
        state,
    }
}

/// Re-enable the key and give it an `expireAt`. A time in the past is tried
/// first; if the API refuses it, a near-future time is set and returned so the
/// caller can wait for it.
async fn expire_key(client: &Client, org_id: &str, api_key_id: &str) -> TestResult<DateTime<Utc>> {
    let past = Utc::now() - chrono::Duration::minutes(1);
    match client
        .openapi_key_update(
            org_id,
            api_key_id,
            &key_patch(Some(ApiKeyPatchRequestState::Enabled), Some(past)),
        )
        .await
    {
        Ok(_) => {
            eprintln!("  step: the API accepted an expireAt in the past");
            Ok(past)
        }
        Err(clickhouse_cloud_api::Error::Api { status, message })
            if (400..500).contains(&status) =>
        {
            eprintln!(
                "  step: the API refused a past expireAt (HTTP {status}: {}); using a near-future one",
                first_line(&message)
            );
            let soon = Utc::now() + chrono::Duration::from_std(EXPIRY_LEAD)?;
            client
                .openapi_key_update(
                    org_id,
                    api_key_id,
                    &key_patch(Some(ApiKeyPatchRequestState::Enabled), Some(soon)),
                )
                .await?;
            Ok(soon)
        }
        Err(error) => Err(error.into()),
    }
}

/// Exactly one clickhousectl-owned key named `key_name` exists, it is
/// `api_key_id`, and the service's endpoint binds it and nothing else.
async fn assert_single_owned_key(
    client: &Client,
    org_id: &str,
    service_id: &str,
    key_name: &str,
    api_key_id: &str,
) -> TestResult<()> {
    let keys = client
        .openapi_key_get_list(org_id)
        .await?
        .result
        .ok_or("key list returned no result")?;
    let owned: Vec<String> = keys
        .iter()
        .filter(|key| key.name.as_deref() == Some(key_name))
        .map(|key| field_string(key.id))
        .collect();
    if owned != [api_key_id.to_string()] {
        return Err(format!(
            "expected exactly one owned key {api_key_id} named {key_name}, found {owned:?}"
        )
        .into());
    }
    let endpoint = client
        .instance_query_endpoint_get(org_id, service_id)
        .await?
        .result
        .ok_or("query endpoint get returned no result")?;
    let bound = endpoint.open_api_keys.unwrap_or_default();
    if bound != [api_key_id.to_string()] {
        return Err(
            format!("expected the endpoint to bind exactly {api_key_id}, found {bound:?}").into(),
        );
    }
    Ok(())
}

async fn wait_for_service_running(
    client: &Client,
    org_id: &str,
    service_id: &str,
    timeout: Duration,
    interval: Duration,
) -> TestResult<()> {
    poll_until("service steady state", timeout, interval, || {
        let client = client.clone();
        let org_id = org_id.to_string();
        let service_id = service_id.to_string();
        async move {
            let resp = client.instance_get(&org_id, &service_id).await?;
            let svc = resp.result.ok_or("service get returned no result")?;
            if matches!(service_state(&svc).as_str(), "running" | "idle") {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        }
    })
    .await
}

fn duration_from_env_or(name: &str, default_secs: u64) -> TestResult<Duration> {
    match std::env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}

fn clickhousectl_binary() -> TestResult<PathBuf> {
    let path = std::env::var_os(CLICKHOUSECTL_BIN_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{CLICKHOUSECTL_BIN_ENV} must point to the built clickhousectl"))?;
    if !path.is_file() {
        return Err(format!(
            "{CLICKHOUSECTL_BIN_ENV} does not point to a file: {}",
            path.display()
        )
        .into());
    }
    Ok(path)
}

fn clickhouse_cloud_api_url() -> String {
    std::env::var("CLICKHOUSE_CLOUD_API_BASE_URL")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.clickhouse.cloud".to_string())
}

fn cli_failure(action: &str, output: &Output) -> String {
    format!(
        "clickhousectl {action} exited {}\nstderr:\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

#[allow(dead_code)]
fn _path_helpers_are_used(_: &Path) {}
