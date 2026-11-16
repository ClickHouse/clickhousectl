//! Live coverage of `cloud service query` Query API access through the real
//! `clickhousectl` binary.
//!
//! It covers first use (#1043): the CLI binds the caller's own API key (its
//! `whoami` key ID) to the service's endpoint, keeping the keys already bound
//! and pruning one the organization has deleted, creates no key and writes no
//! record; a second query reuses the binding. When the service has no endpoint, the bind creates it.
//!
//! Every resource is created here and torn down here: the services, the keys
//! the test creates, the query endpoints, and the caller's own binding.
//!
//! Whether a query bound a key is read from the endpoint through the API,
//! never from the CLI's stderr notice. The live harness is the API library's
//! own (`crates/clickhouse-cloud-api/tests/common/`); run this target with
//! `cargo test -p clickhousectl --test live_service_query_bind_test -- --ignored`.

#[path = "../../clickhouse-cloud-api/tests/common/mod.rs"]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::Duration;

use clickhouse_cloud_api::Client;
use clickhouse_cloud_api::models::*;
use common::support::*;
use serde_json::Value;

/// How long a management-side change (an unbinding) may take before the
/// Query API enforces it and the CLI sees the key refused.
const DEFAULT_REJECTION_TIMEOUT_SECS: u64 = 300;
/// How long the test's own endpoint upsert may retry a `400` while a key it
/// created moments earlier propagates (#658).
const SEED_PROPAGATION_TIMEOUT: Duration = Duration::from_secs(90);

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_service_query_binds_the_callers_own_key_on_first_use() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let clickhousectl = PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"));
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
    let service_name = format!("{}-qb", ctx.service_name());
    // The name earlier versions gave the key they minted on first use.
    let minted_key_name = format!("clickhousectl-query-{service_name}");

    let test_result = async {
        log_run_header(
            "cloud_service_query_binds_the_callers_own_key_on_first_use",
            &ctx,
        );
        let mut failures = FailureRecorder::default();

        // ── Provision ───────────────────────────────────────────────

        let service_id =
            create_running_service(&ctx, &client, &mut failures, &mut cleanup, &service_name)
                .await?;
        let caller = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "whoami identifies the caller's API key in this organization",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    async move { caller_api_key_id(&client, &org_id).await }
                },
            )
            .await?
            .expect("blocking steps always return a value");

        // ── An endpoint with a live and a deleted binding ───────────
        //
        // The endpoint already binds a key the test owns and one the
        // organization has since deleted. The bind must keep the first and
        // prune the second: a dangling UUID makes every upsert fail (#659).

        log_phase("Endpoint already binds a live key and a deleted one");

        let kept = create_owned_key(
            &ctx,
            &client,
            &mut failures,
            &mut cleanup,
            &format!("{service_name}-kept"),
        )
        .await?
        .id;
        let dangling = create_owned_key(
            &ctx,
            &client,
            &mut failures,
            &mut cleanup,
            &format!("{service_name}-dangling"),
        )
        .await?
        .id;
        cleanup.register_query_endpoint(service_id.clone());
        upsert_binding(
            &client,
            &ctx.org_id,
            &service_id,
            vec![kept.clone(), dangling.clone()],
        )
        .await?;
        client.openapi_key_delete(&ctx.org_id, &dangling).await?;
        cleanup.unregister_api_key(&dangling);
        assert_key_gone(&client, &ctx.org_id, &dangling).await?;
        eprintln!(
            "  diag: the deleted key's UUID is still bound: {}",
            bound_keys(&client, &ctx.org_id, &service_id)
                .await?
                .contains(&dangling)
        );

        // ── First query binds the caller's key ──────────────────────

        log_phase("First query binds the caller's own key");

        // The endpoint binds only `kept` (and the deleted key), so the
        // caller's key in the binding afterwards is what shows the bind.
        let inventory_before = key_names(&client, &ctx.org_id).await?;
        let first_query = cli.query(&service_id, false)?;
        if !first_query.status.success() {
            return Err(cli_failure("service query (first use)", &first_query).into());
        }
        let stdout = String::from_utf8_lossy(&first_query.stdout);
        if stdout.trim() != "1" {
            return Err(format!("expected `SELECT 1` to print 1, got {stdout:?}").into());
        }
        assert_no_key_minted(
            &client,
            &ctx.org_id,
            &inventory_before,
            &minted_key_name,
            &mut cleanup,
        )
        .await?;
        assert!(
            cli.stored_record(&service_id)?.is_none(),
            "the bind must not write a per-service record"
        );
        let bound = bound_key_set(&client, &ctx.org_id, &service_id).await?;
        assert_eq!(
            bound,
            [kept.clone(), caller.clone()].into_iter().collect(),
            "the endpoint must bind the kept key and the caller's key, and nothing else"
        );
        let roles = client
            .instance_query_endpoint_get(&ctx.org_id, &service_id)
            .await?
            .result
            .and_then(|endpoint| endpoint.roles)
            .unwrap_or_default();
        // The endpoint existed before the bind, so it keeps the roles the
        // test seeded it with (`sql_console_admin`): a bind never rewrites them.
        assert!(
            roles.contains(&QueryEndpointRole::SqlConsoleAdmin),
            "the endpoint must keep its sql_console_admin role: {roles:?}"
        );

        // ── Second query reuses the binding ─────────────────────────

        log_phase("Second query reuses the binding");

        // `--no-auto-enable` refuses any bind, so its success shows none was
        // needed.
        let second_query = cli.query_with(&service_id, false, &["--no-auto-enable"])?;
        if !second_query.status.success() {
            return Err(cli_failure("service query (reuse)", &second_query).into());
        }
        assert_eq!(
            bound_key_set(&client, &ctx.org_id, &service_id).await?,
            bound,
            "the second query must leave the binding unchanged"
        );
        assert!(cli.stored_record(&service_id)?.is_none());

        log_phase("Unbind the caller's key");
        unbind_key(&client, &ctx.org_id, &service_id, &caller).await?;
        assert_eq!(
            bound_keys(&client, &ctx.org_id, &service_id).await?,
            std::slice::from_ref(&kept)
        );

        // ── No endpoint: the bind creates it ────────────────────────

        log_phase("No endpoint: the bind creates one");

        client
            .instance_query_endpoint_delete(&ctx.org_id, &service_id)
            .await?;
        // The Query API may honour the old binding for a moment; wait until
        // it refuses the caller, so the next query must bind.
        cli.poll_for_refusal_without_auto_enable(&service_id, rejection_timeout, ctx.poll_interval)
            .await?;
        // With no endpoint, the endpoint binding exactly the caller's key
        // afterwards is what shows the bind created it.
        let inventory_before = key_names(&client, &ctx.org_id).await?;
        let created = cli.query(&service_id, false)?;
        if !created.status.success() {
            return Err(cli_failure("service query (no endpoint)", &created).into());
        }
        assert_eq!(String::from_utf8_lossy(&created.stdout).trim(), "1");
        assert_no_key_minted(
            &client,
            &ctx.org_id,
            &inventory_before,
            &minted_key_name,
            &mut cleanup,
        )
        .await?;
        assert!(cli.stored_record(&service_id)?.is_none());
        assert_eq!(
            bound_keys(&client, &ctx.org_id, &service_id).await?,
            std::slice::from_ref(&caller),
            "the created endpoint must bind exactly the caller's key"
        );

        // Leave the caller's key bound nowhere: the endpoint goes now, and
        // cleanup deletes it again (a 404 is fine) should anything above fail.
        log_phase("Remove the caller's binding");
        client
            .instance_query_endpoint_delete(&ctx.org_id, &service_id)
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
            .env("CLICKHOUSECTL_NO_UPDATE_CHECK", "1")
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
        self.query_with(service_id, json, &[])
    }

    /// [`Self::query`] with extra flags.
    fn query_with(&self, service_id: &str, json: bool, extra: &[&str]) -> TestResult<Output> {
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
        args.extend_from_slice(extra);
        self.command(&args)
    }

    fn credentials_path(&self) -> PathBuf {
        self.workdir.join(".clickhouse").join("credentials.json")
    }

    /// The service's stored record, or `None` when there is none (or no
    /// credentials file at all).
    fn stored_record(&self, service_id: &str) -> TestResult<Option<Value>> {
        let bytes = match std::fs::read(self.credentials_path()) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let credentials: Value = serde_json::from_slice(&bytes)?;
        Ok(credentials["service_query_keys"].get(service_id).cloned())
    }

    /// Run `--no-auto-enable --json` queries until the CLI refuses because
    /// the caller's key would need binding. A success means the Query API
    /// still honours a binding that was just removed.
    ///
    /// The refusal is recognised structurally: exit 1 and a JSON error whose
    /// code is the Query API's `http_4xx` rejection and whose message names
    /// the service and the `--no-auto-enable` flag that forbade the bind.
    async fn poll_for_refusal_without_auto_enable(
        &self,
        service_id: &str,
        timeout: Duration,
        interval: Duration,
    ) -> TestResult<()> {
        poll_until(
            "the Query API to refuse the unbound caller key",
            timeout,
            interval,
            || {
                let cli = self.clone();
                let service_id = service_id.to_string();
                async move {
                    let output = cli.query_with(&service_id, true, &["--no-auto-enable"])?;
                    if output.status.success() {
                        eprintln!("  poll: the Query API still accepts the caller key");
                        return Ok(None);
                    }
                    if is_no_auto_enable_refusal(&output, &service_id) {
                        Ok(Some(()))
                    } else {
                        Err(cli_failure("service query --no-auto-enable", &output).into())
                    }
                }
            },
        )
        .await
    }
}

/// Create a disposable service tagged for this run, register it for cleanup,
/// and wait until it is running or idle. Returns its ID.
async fn create_running_service(
    ctx: &TestContext,
    client: &Client,
    failures: &mut FailureRecorder,
    cleanup: &mut CleanupRegistry,
    service_name: &str,
) -> TestResult<String> {
    log_phase("Provision Service");

    let create_body = ServicePostRequest {
        name: service_name.to_string(),
        provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
        region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
        min_replica_memory_gb: Some(8.0),
        max_replica_memory_gb: Some(8.0),
        num_replicas: Some(1),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(5.0),
        tags: Some(ctx.run_tags()),
        autoscaling_mode: None,
        backup_id: None,
        backup_encryption_config: None,
        byoc_id: None,
        compliance_type: None,
        data_warehouse_id: None,
        enable_core_dumps: None,
        endpoints: None,
        has_transparent_data_encryption: None,
        ip_access_list: Vec::new(),
        is_readonly: None,
        max_replicas: None,
        #[cfg(feature = "deprecated-fields")]
        max_total_memory_gb: None,
        min_replicas: None,
        #[cfg(feature = "deprecated-fields")]
        min_total_memory_gb: None,
        #[cfg(feature = "deprecated-fields")]
        private_endpoint_ids: None,
        private_preview_terms_checked: None,
        profile: None,
        release_channel: None,
        #[cfg(feature = "deprecated-fields")]
        tier: None,
    };
    let created = failures
        .run(ctx, StepKind::Blocking, "create service", || {
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
            ctx,
            StepKind::Blocking,
            "wait for service steady state",
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                async move {
                    wait_for_service_state(
                        &client,
                        &org_id,
                        &service_id,
                        &["running", "idle"],
                        ctx.steady_state_timeout,
                        ctx.poll_interval,
                    )
                    .await
                }
            },
        )
        .await?;
    Ok(service_id)
}

/// A key the test created: its management ID.
struct CreatedKey {
    id: String,
}

/// Create a key this test owns, with no organization role, and register it
/// for cleanup.
async fn create_owned_key(
    ctx: &TestContext,
    client: &Client,
    failures: &mut FailureRecorder,
    cleanup: &mut CleanupRegistry,
    name: &str,
) -> TestResult<CreatedKey> {
    let key = failures
        .run(
            ctx,
            StepKind::Blocking,
            &format!("create the test-owned key {name}"),
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let name = name.to_string();
                async move {
                    let body = ApiKeyPostRequest {
                        name,
                        assigned_role_ids: Some(vec![]),
                        expire_at: None,
                        hash_data: None,
                        ip_access_list: Some(vec![IpAccessListEntry {
                            source: "0.0.0.0/0".to_string(),
                            description: Some("clickhousectl integration test".to_string()),
                        }]),
                        #[cfg(feature = "deprecated-fields")]
                        roles: None,
                        state: Some(ApiKeyPostRequestState::Enabled),
                    };
                    let resp = client.openapi_key_create(&org_id, &body).await?;
                    let created = resp.result.ok_or("key create returned no result")?;
                    let key = created.key.ok_or("key create returned no key")?;
                    Ok(CreatedKey {
                        id: require_field(key.id, "key.id")?.to_string(),
                    })
                }
            },
        )
        .await?
        .expect("blocking steps always return a value");
    cleanup.register_api_key(key.id.clone());
    Ok(key)
}

/// Upsert the service's endpoint to bind exactly `open_api_keys` as
/// `sql_console_admin`, creating it if absent. A key created moments ago may
/// not be visible to the endpoint service yet and the upsert answers `400`
/// until it is (#658), so a `400` is retried for a bounded time.
async fn upsert_binding(
    client: &Client,
    org_id: &str,
    service_id: &str,
    open_api_keys: Vec<String>,
) -> TestResult<ServiceQueryAPIEndpoint> {
    let request = InstanceServiceQueryApiEndpointsPostRequest {
        roles: vec![QueryEndpointRole::SqlConsoleAdmin],
        open_api_keys,
        allowed_origins: Some("*".to_string()),
    };
    let response = retry_api_call(
        "query endpoint upsert",
        SEED_PROPAGATION_TIMEOUT,
        Duration::from_secs(3),
        || client.instance_query_endpoint_upsert(org_id, service_id, &request),
        |error| matches!(error, clickhouse_cloud_api::Error::Api { status: 400, .. }),
    )
    .await?;
    Ok(response
        .result
        .ok_or("query endpoint upsert returned no result")?)
}

/// The resource UUID of the caller's own API key, from `whoami` — the value
/// the CLI binds on first use (#1043).
async fn caller_api_key_id(client: &Client, org_id: &str) -> TestResult<String> {
    let identity = client
        .whoami_get()
        .await?
        .result
        .ok_or("whoami returned no result")?;
    let Whoami::WhoamiApiKey(key) = identity else {
        return Err(format!("whoami did not identify an API key: {identity:?}").into());
    };
    let organization = require_field(key.organization_id, "organizationId")?;
    // Compared as UUIDs, so the configured org id's case does not matter.
    if uuid::Uuid::parse_str(org_id).ok() != Some(organization) {
        return Err("the test API key belongs to another organization".into());
    }
    require_field(key.key_id, "keyId")
}

/// `GET /keys/{id}` answers 404.
async fn assert_key_gone(client: &Client, org_id: &str, api_key_id: &str) -> TestResult<()> {
    match client.openapi_key_get(org_id, api_key_id).await {
        Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(()),
        Ok(_) => Err(format!("key {api_key_id} still exists").into()),
        Err(error) => Err(error.into()),
    }
}

async fn wait_for_service_state(
    client: &Client,
    org_id: &str,
    service_id: &str,
    states: &[&str],
    timeout: Duration,
    interval: Duration,
) -> TestResult<()> {
    let wanted = states.join("|");
    poll_until(
        &format!("service state {wanted}"),
        timeout,
        interval,
        || {
            let client = client.clone();
            let org_id = org_id.to_string();
            let service_id = service_id.to_string();
            let states: Vec<String> = states.iter().map(|s| s.to_string()).collect();
            async move {
                let resp = client.instance_get(&org_id, &service_id).await?;
                let svc = resp.result.ok_or("service get returned no result")?;
                if states.contains(&service_state(&svc)) {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await
}

/// Remove `api_key_id` from the service's endpoint binding, the way an
/// administrator retiring a key by hand would, leaving everything else about
/// the endpoint as it is.
async fn unbind_key(
    client: &Client,
    org_id: &str,
    service_id: &str,
    api_key_id: &str,
) -> TestResult<()> {
    let endpoint = client
        .instance_query_endpoint_get(org_id, service_id)
        .await?
        .result
        .ok_or("query endpoint get returned no result")?;
    let mut open_api_keys = endpoint
        .open_api_keys
        .ok_or("query endpoint reports no openApiKeys")?;
    open_api_keys.retain(|key| key != api_key_id);
    client
        .instance_query_endpoint_upsert(
            org_id,
            service_id,
            &InstanceServiceQueryApiEndpointsPostRequest {
                roles: endpoint.roles.ok_or("query endpoint reports no roles")?,
                open_api_keys,
                allowed_origins: Some(
                    endpoint
                        .allowed_origins
                        .ok_or("query endpoint reports no allowedOrigins")?,
                ),
            },
        )
        .await?;
    Ok(())
}

/// The keys the service's query endpoint currently binds, as a set.
async fn bound_key_set(
    client: &Client,
    org_id: &str,
    service_id: &str,
) -> TestResult<BTreeSet<String>> {
    Ok(bound_keys(client, org_id, service_id)
        .await?
        .into_iter()
        .collect())
}

/// The keys the service's query endpoint currently binds.
async fn bound_keys(client: &Client, org_id: &str, service_id: &str) -> TestResult<Vec<String>> {
    let endpoint = client
        .instance_query_endpoint_get(org_id, service_id)
        .await?
        .result
        .ok_or("query endpoint get returned no result")?;
    Ok(endpoint.open_api_keys.unwrap_or_default())
}

/// Every key in the organization, management ID to name.
async fn key_names(client: &Client, org_id: &str) -> TestResult<BTreeMap<String, String>> {
    let keys = list_all_api_keys(client, org_id).await?;
    Ok(keys
        .iter()
        .map(|key| (field_string(key.id), key.name.clone().unwrap_or_default()))
        .collect())
}

/// No key appeared since `before` under the name earlier CLI versions gave
/// the key they minted on first use. Other new keys are logged: the other
/// cases run concurrently in this organization and create keys of their own.
/// A minted key is registered for cleanup before the assertion fails.
async fn assert_no_key_minted(
    client: &Client,
    org_id: &str,
    before: &BTreeMap<String, String>,
    minted_key_name: &str,
    cleanup: &mut CleanupRegistry,
) -> TestResult<()> {
    let after = key_names(client, org_id).await?;
    let new: Vec<(&String, &String)> = after
        .iter()
        .filter(|(id, _)| !before.contains_key(*id))
        .collect();
    eprintln!(
        "  step: {} keys before, {} after; new keys: {new:?}",
        before.len(),
        after.len()
    );
    let minted: Vec<String> = after
        .iter()
        .filter(|(_, name)| name.starts_with(minted_key_name))
        .map(|(id, _)| id.clone())
        .collect();
    for id in &minted {
        cleanup.register_api_key(id.clone());
    }
    if minted.is_empty() {
        Ok(())
    } else {
        Err(format!("the CLI minted keys named {minted_key_name}: {minted:?}").into())
    }
}

fn duration_from_env_or(name: &str, default_secs: u64) -> TestResult<Duration> {
    match std::env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}

/// The `--json` refusal `--no-auto-enable` produces when the caller's key
/// would need binding: exit 1, one `{"error": {...}}` object on stderr with
/// the `http_4xx` code (the Query API's 401/403/404 rejection, carried
/// across), and a message carrying the service ID and the flag.
fn is_no_auto_enable_refusal(output: &Output, service_id: &str) -> bool {
    if output.status.code() != Some(1) {
        return false;
    }
    let Ok(envelope) = serde_json::from_slice::<Value>(&output.stderr) else {
        return false;
    };
    let error = &envelope["error"];
    let message = error["message"].as_str().unwrap_or_default();
    error["code"] == "http_4xx"
        && message.contains(service_id)
        && message.contains("--no-auto-enable")
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
