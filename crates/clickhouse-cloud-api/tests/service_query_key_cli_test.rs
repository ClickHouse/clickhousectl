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
//! A second case covers the retirement lifecycle (#527): each repair deletes
//! the key it replaces and unbinds it from the endpoint, so repeated repairs do
//! not grow the organization's key inventory or the endpoint binding; a
//! retirement left pending on the local record is retried by the next query;
//! and `cloud service delete` deletes the current key and every pending
//! retirement along with the service.
//!
//! A third case stresses key propagation (#658): the endpoint upsert can refuse
//! a key created moments earlier, and the CLI waits that out. Repeated repairs,
//! each followed at once by a query, plus repeated first-use provisioning after
//! the key was deleted out from under the CLI, must all succeed first time and
//! leave exactly one owned key.
//!
//! Every resource is created here and torn down here: the service, the keys the
//! CLI provisions and the repairs create, the keys the retirement case injects,
//! and the query endpoint binding.

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
/// Repairs run back to back by the propagation stress case (#658).
const DEFAULT_REPAIR_STRESS_ITERATIONS: u64 = 6;
/// First-use provisioning rounds run by the propagation stress case (#658).
const DEFAULT_PROVISION_STRESS_ITERATIONS: u64 = 2;
/// The CLI's stderr notice while it waits for a new key to propagate (#658).
const KEY_PROPAGATION_NOTICE: &str =
    "Waiting for the new API key to become visible to the Query API endpoint...";

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

        let service_id =
            create_running_service(&ctx, &client, &mut failures, &mut cleanup, &service_name)
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
        let repaired = repair(
            &ctx,
            &cli,
            &client,
            &mut failures,
            "repair-query-key replaces the expired key deliberately",
            &service_id,
            &api_key_id,
        )
        .await?;
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

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_service_query_key_repairs_retire_keys_and_service_delete_cleans_up() -> TestResult<()>
{
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
    let service_name = format!("{}-qr", ctx.service_name());
    let key_name = format!("clickhousectl-query-{service_name}");

    let test_result = async {
        log_run_header(
            "cloud_service_query_key_repairs_retire_keys_and_service_delete_cleans_up",
            &ctx,
        );
        let mut failures = FailureRecorder::default();

        // ── Provision ───────────────────────────────────────────────

        let service_id =
            create_running_service(&ctx, &client, &mut failures, &mut cleanup, &service_name)
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
        cleanup.register_query_endpoint(service_id.clone());
        let stored = cli.stored_key(&service_id);
        if let Ok(stored) = &stored {
            cleanup.register_api_key(stored.api_key_id.clone());
        }
        if !first_query.status.success() {
            return Err(cli_failure("service query (first use)", &first_query).into());
        }
        let mut current_api_key_id = stored?.api_key_id;

        // ── Two repairs in a row ────────────────────────────────────
        //
        // The key is perfectly healthy; repair is an explicit replacement.
        // Each one must retire the key it replaces: deleted from the
        // organization and gone from the endpoint's `openApiKeys`, with
        // nothing left pending on the local record.

        for round in 1..=2 {
            log_phase(&format!("Repair {round} retires the key it replaces"));

            let repaired = repair(
                &ctx,
                &cli,
                &client,
                &mut failures,
                &format!("repair {round} replaces the current key deliberately"),
                &service_id,
                &current_api_key_id,
            )
            .await?;
            assert_eq!(repaired["status"], "repaired", "{repaired}");
            assert_eq!(
                repaired["replacedApiKeyId"], current_api_key_id,
                "{repaired}"
            );
            assert_eq!(
                repaired["deletedApiKeyIds"],
                serde_json::json!([current_api_key_id]),
                "the superseded key is reported deleted: {repaired}"
            );
            assert!(
                repaired.get("pendingCleanupApiKeyIds").is_none(),
                "nothing may be left pending after a clean repair: {repaired}"
            );
            let new_api_key_id = repaired["apiKeyId"]
                .as_str()
                .ok_or("repair result has no apiKeyId")?
                .to_string();
            assert_ne!(new_api_key_id, current_api_key_id);
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
                    "the superseded key is gone and only the replacement is bound",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let service_id = service_id.clone();
                        let old = current_api_key_id.clone();
                        let new = new_api_key_id.clone();
                        let key_name = key_name.clone();
                        async move {
                            assert_key_gone(&client, &org_id, &old).await?;
                            assert_single_owned_key(&client, &org_id, &service_id, &key_name, &new)
                                .await
                        }
                    },
                )
                .await?;
            cleanup.unregister_api_key(&current_api_key_id);
            current_api_key_id = new_api_key_id;
        }

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "a query succeeds with the latest replacement key",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move {
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

        // ── A pending retirement is retried by the next query ───────
        //
        // The API cannot be made to fail a delete on demand, so the state a
        // failed cleanup leaves behind is reproduced directly: a key the
        // test owns, listed on the local record as awaiting deletion.

        log_phase("Pending retirement retried by the next query");

        let retired_for_query = create_owned_key(
            &ctx,
            &client,
            &mut failures,
            &mut cleanup,
            &format!("{key_name}-retired-1"),
        )
        .await?;
        cli.add_pending_retirement(&service_id, &retired_for_query)?;
        let query = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "a query retries the pending deletion quietly and still runs",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move { cli.query(&service_id, false) }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        if !query.status.success() {
            return Err(cli_failure("service query (pending retirement)", &query).into());
        }
        let stderr = String::from_utf8_lossy(&query.stderr);
        assert!(
            !stderr.contains("Warning:"),
            "a successful retry prints no warning: {stderr}"
        );
        assert!(
            cli.stored_key(&service_id)?
                .pending_cleanup_api_key_ids
                .is_empty(),
            "the retried key must leave the pending list"
        );
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "the retried key is gone from the organization",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let key_id = retired_for_query.clone();
                    async move { assert_key_gone(&client, &org_id, &key_id).await }
                },
            )
            .await?;
        cleanup.unregister_api_key(&retired_for_query);

        // ── Service deletion cleans current and pending keys ────────

        log_phase("Service delete cleans the current key and a pending retirement");

        let retired_for_delete = create_owned_key(
            &ctx,
            &client,
            &mut failures,
            &mut cleanup,
            &format!("{key_name}-retired-2"),
        )
        .await?;
        cli.add_pending_retirement(&service_id, &retired_for_delete)?;

        // The CLI's own `--force` stop is exercised by its subprocess tests;
        // here the service is stopped through the API so the run measures
        // the key cleanup, not the stop.
        failures
            .run(&ctx, StepKind::Blocking, "stop the service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                async move {
                    client
                        .instance_state_update(
                            &org_id,
                            &service_id,
                            &ServiceStatePatchRequest {
                                command: Some(ServiceStatePatchRequestCommand::Stop),
                            },
                        )
                        .await?;
                    wait_for_service_state(
                        &client,
                        &org_id,
                        &service_id,
                        &["stopped"],
                        ctx.delete_timeout,
                        ctx.poll_interval,
                    )
                    .await
                }
            })
            .await?;

        let deleted = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "cloud service delete removes the service and both owned keys",
                || {
                    let cli = cli.clone();
                    let service_id = service_id.clone();
                    async move { cli.delete_service(&service_id) }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        if !deleted.status.success() {
            return Err(cli_failure("service delete", &deleted).into());
        }
        let credentials: Value = serde_json::from_slice(&cli.credentials_file()?)?;
        assert!(
            credentials["service_query_keys"].get(&service_id).is_none(),
            "the local record must be removed once every owned key is deleted: {credentials}"
        );
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "the current key and the pending retirement are gone",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let current = current_api_key_id.clone();
                    let pending = retired_for_delete.clone();
                    async move {
                        assert_key_gone(&client, &org_id, &current).await?;
                        assert_key_gone(&client, &org_id, &pending).await
                    }
                },
            )
            .await?;
        cleanup.unregister_api_key(&current_api_key_id);
        cleanup.unregister_api_key(&retired_for_delete);
        // The endpoint dies with the service; the service itself stays
        // registered so cleanup confirms it is gone.
        cleanup.unregister_query_endpoint(&service_id);

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

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_service_query_key_repair_survives_key_propagation_delay() -> TestResult<()> {
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
    let repair_iterations = count_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_REPAIR_STRESS_ITERATIONS",
        DEFAULT_REPAIR_STRESS_ITERATIONS,
    )?;
    let provision_iterations = count_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_PROVISION_STRESS_ITERATIONS",
        DEFAULT_PROVISION_STRESS_ITERATIONS,
    )?;
    let rejection_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_QUERY_KEY_REJECTION_SECS",
        DEFAULT_REJECTION_TIMEOUT_SECS,
    )?;

    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();
    let service_name = format!("{}-kp", ctx.service_name());
    let key_name = format!("clickhousectl-query-{service_name}");

    log_run_header(
        "cloud_service_query_key_repair_survives_key_propagation_delay",
        &ctx,
    );
    let inventory_before = key_inventory(&client, &ctx.org_id).await?;
    eprintln!(
        "  step: organization holds {} keys before the run",
        inventory_before.len()
    );

    let test_result = async {
        let mut failures = FailureRecorder::default();

        // ── Provision ───────────────────────────────────────────────

        let service_id =
            create_running_service(&ctx, &client, &mut failures, &mut cleanup, &service_name)
                .await?;

        log_phase("First query provisions a per-service key");

        let first_query = cli.query(&service_id, false)?;
        cleanup.register_query_endpoint(service_id.clone());
        let stored = cli.stored_key(&service_id);
        if let Ok(stored) = &stored {
            cleanup.register_api_key(stored.api_key_id.clone());
        }
        if !first_query.status.success() {
            return Err(cli_failure("service query (first use)", &first_query).into());
        }
        let mut current_api_key_id = stored?.api_key_id;
        let mut summary = StressSummary::default();
        summary.note_provision(&first_query, Duration::ZERO);

        // ── Repairs back to back, each followed at once by a query ──
        //
        // Each repair creates a key and binds it straight away, the window in
        // which the endpoint service may not know the key yet (#658). The
        // query that follows must succeed at the first attempt: the repair
        // exits 0 only once the replacement is usable.

        log_phase(&format!(
            "{repair_iterations} repairs, each followed immediately by a query"
        ));

        for round in 1..=repair_iterations {
            let started = std::time::Instant::now();
            let repaired = cli.repair(&service_id)?;
            let repair_elapsed = started.elapsed();
            let repair_stderr = String::from_utf8_lossy(&repaired.stderr).to_string();
            let waited = repair_stderr.contains(KEY_PROPAGATION_NOTICE);
            if !repaired.status.success() {
                summary.print(round);
                return Err(cli_failure("service repair-query-key", &repaired).into());
            }
            let result: Value = serde_json::from_slice(&repaired.stdout)?;
            assert_eq!(result["status"], "repaired", "{result}");
            assert_eq!(result["replacedApiKeyId"], current_api_key_id, "{result}");
            let new_api_key_id = result["apiKeyId"]
                .as_str()
                .ok_or("repair result has no apiKeyId")?
                .to_string();
            cleanup.register_api_key(new_api_key_id.clone());

            let started = std::time::Instant::now();
            let query = cli.query(&service_id, true)?;
            let query_elapsed = started.elapsed();
            let first_try = query.status.success();
            if !first_try {
                let stderr = String::from_utf8_lossy(&query.stderr);
                eprintln!(
                    "  diag: round {round}: the query right after the repair failed: {}",
                    first_line(&stderr)
                );
                // Keep the run going to measure how often this happens; the
                // summary fails the test at the end.
                poll_until(
                    "the replacement key to be accepted",
                    rejection_timeout,
                    ctx.poll_interval,
                    || {
                        let cli = cli.clone();
                        let service_id = service_id.clone();
                        async move {
                            let output = cli.query(&service_id, true)?;
                            Ok(output.status.success().then_some(()))
                        }
                    },
                )
                .await?;
            }
            summary.note_repair(round, repair_elapsed, waited, first_try, query_elapsed);

            assert_key_gone(&client, &ctx.org_id, &current_api_key_id).await?;
            assert_single_owned_key(
                &client,
                &ctx.org_id,
                &service_id,
                &key_name,
                &new_api_key_id,
            )
            .await?;
            cleanup.unregister_api_key(&current_api_key_id);
            current_api_key_id = new_api_key_id;
        }

        // ── First-use provisioning after the key was deleted ────────
        //
        // The documented recovery path: the key is deleted out from under
        // the CLI, a query classifies the rejection as `query_key_deleted`
        // and forgets the record, and the next query provisions afresh —
        // creating a key and binding it, the same create-then-bind.
        //
        // The deleting administrator here also unbinds the key from the
        // endpoint. A deleted key whose UUID is left in `openApiKeys` makes
        // every later upsert fail with the same 400 (observed live on
        // 2026-09-02); the `query_key_deleted` verdict does not remove that
        // binding yet, which is tracked as a follow-up to #658.

        log_phase(&format!(
            "{provision_iterations} first-use provisioning rounds after the key is deleted"
        ));

        for round in 1..=provision_iterations {
            unbind_key(&client, &ctx.org_id, &service_id, &current_api_key_id).await?;
            client
                .openapi_key_delete(&ctx.org_id, &current_api_key_id)
                .await?;
            cleanup.unregister_api_key(&current_api_key_id);
            let rejected = cli
                .poll_for_rejection(
                    &service_id,
                    "query_key_deleted",
                    rejection_timeout,
                    ctx.poll_interval,
                )
                .await?;
            eprintln!(
                "  step: round {round}: the deleted key was classified: {}",
                rejected["error"]["code"]
            );
            let credentials: Value = serde_json::from_slice(&cli.credentials_file()?)?;
            assert!(
                credentials["service_query_keys"].get(&service_id).is_none(),
                "the record of a deleted key is forgotten: {credentials}"
            );

            let started = std::time::Instant::now();
            let provisioned = cli.query(&service_id, false)?;
            let elapsed = started.elapsed();
            let stored = cli.stored_key(&service_id);
            if let Ok(stored) = &stored {
                cleanup.register_api_key(stored.api_key_id.clone());
            }
            if !provisioned.status.success() {
                summary.print(repair_iterations);
                return Err(cli_failure("service query (re-provisioning)", &provisioned).into());
            }
            summary.note_provision(&provisioned, elapsed);
            current_api_key_id = stored?.api_key_id;
            assert_single_owned_key(
                &client,
                &ctx.org_id,
                &service_id,
                &key_name,
                &current_api_key_id,
            )
            .await?;
        }

        summary.print(repair_iterations);
        summary.assert_every_query_succeeded_first_time()?;
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

    // The organization must hold exactly the keys it held before: nothing
    // this run created may survive it, and nothing it did not create may go.
    let inventory_after = key_inventory(&client, &ctx.org_id).await?;
    let leaked: Vec<&String> = inventory_after.difference(&inventory_before).collect();
    let missing: Vec<&String> = inventory_before.difference(&inventory_after).collect();
    eprintln!(
        "  step: organization holds {} keys after the run (leaked: {leaked:?}, missing: {missing:?})",
        inventory_after.len()
    );

    test_result?;
    cleanup_result?;
    if !leaked.is_empty() || !missing.is_empty() {
        return Err(
            format!("key inventory changed: leaked {leaked:?}, missing {missing:?}").into(),
        );
    }
    Ok(())
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

    fn delete_service(&self, service_id: &str) -> TestResult<Output> {
        self.command(&[
            "--json",
            "service",
            "delete",
            service_id,
            "--org-id",
            &self.org_id,
        ])
    }

    fn credentials_path(&self) -> PathBuf {
        self.workdir.join(".clickhouse").join("credentials.json")
    }

    /// Reproduce the state a failed retirement leaves behind: `api_key_id`
    /// listed on the service's record as awaiting deletion (#527).
    fn add_pending_retirement(&self, service_id: &str, api_key_id: &str) -> TestResult<()> {
        let mut credentials: Value = serde_json::from_slice(&self.credentials_file()?)?;
        let record = credentials["service_query_keys"]
            .get_mut(service_id)
            .ok_or("the CLI stored no per-service query key")?;
        let pending = record
            .as_object_mut()
            .ok_or("the stored record is not an object")?
            .entry("pending_cleanup_api_key_ids")
            .or_insert_with(|| Value::Array(vec![]));
        pending
            .as_array_mut()
            .ok_or("pending_cleanup_api_key_ids is not an array")?
            .push(Value::String(api_key_id.to_string()));
        std::fs::write(
            self.credentials_path(),
            serde_json::to_vec_pretty(&credentials)?,
        )?;
        Ok(())
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
        ..Default::default()
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

/// Run `repair-query-key` and return its JSON result.
///
/// The CLI waits out key propagation itself (#658): the endpoint upsert can
/// answer `400 OpenAPI key <id> does not belong to the organization` right
/// after the replacement key is created, and the CLI retries it inside a
/// bounded window. This harness no longer retries, so a transient that
/// escapes that window fails the suite; the diagnostics printed on failure
/// show what the endpoint and the old key looked like at that moment.
async fn repair(
    ctx: &TestContext,
    cli: &Cli,
    client: &Client,
    failures: &mut FailureRecorder,
    step: &str,
    service_id: &str,
    api_key_id: &str,
) -> TestResult<Value> {
    eprintln!("  step: stored management key id before repair: {api_key_id}");
    let result = failures
        .run(ctx, StepKind::Blocking, step, || {
            let cli = cli.clone();
            let client = client.clone();
            let org_id = ctx.org_id.clone();
            let service_id = service_id.to_string();
            let api_key_id = api_key_id.to_string();
            async move {
                let output = cli.repair(&service_id)?;
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains(KEY_PROPAGATION_NOTICE) {
                    eprintln!("  diag: the CLI waited for the new key to propagate");
                }
                if !output.status.success() {
                    eprintln!(
                        "  diag: repair failed: {}",
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
                    return Err(cli_failure("service repair-query-key", &output).into());
                }
                let result: Value = serde_json::from_slice(&output.stdout)?;
                Ok(result)
            }
        })
        .await?
        .expect("blocking steps always return a value");
    Ok(result)
}

/// Create a key this test owns, with no organization role (like the CLI's own
/// query keys), and register it for cleanup. Returns its management ID.
async fn create_owned_key(
    ctx: &TestContext,
    client: &Client,
    failures: &mut FailureRecorder,
    cleanup: &mut CleanupRegistry,
    name: &str,
) -> TestResult<String> {
    let key_id = failures
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
                        assigned_role_ids: vec![],
                        expire_at: None,
                        hash_data: None,
                        ip_access_list: vec![IpAccessListEntry {
                            source: "0.0.0.0/0".to_string(),
                            description: Some("clickhousectl integration test".to_string()),
                        }],
                        #[cfg(feature = "deprecated-fields")]
                        roles: None,
                        state: ApiKeyPostRequestState::Enabled,
                    };
                    let resp = client.openapi_key_create(&org_id, &body).await?;
                    let created = resp.result.ok_or("key create returned no result")?;
                    let key = created.key.ok_or("key create returned no key")?;
                    Ok(require_field(key.id, "key.id")?.to_string())
                }
            },
        )
        .await?
        .expect("blocking steps always return a value");
    cleanup.register_api_key(key_id.clone());
    Ok(key_id)
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
                allowed_origins: endpoint
                    .allowed_origins
                    .ok_or("query endpoint reports no allowedOrigins")?,
            },
        )
        .await?;
    Ok(())
}

/// The management IDs of every key in the organization.
async fn key_inventory(
    client: &Client,
    org_id: &str,
) -> TestResult<std::collections::BTreeSet<String>> {
    let keys = client
        .openapi_key_get_list(org_id)
        .await?
        .result
        .ok_or("key list returned no result")?;
    Ok(keys.iter().map(|key| field_string(key.id)).collect())
}

/// Per-round record of the propagation stress case, printed as one line per
/// round so a CI log shows how often the wait kicked in and for how long.
#[derive(Default)]
struct StressSummary {
    repairs: Vec<(u64, Duration, bool, bool, Duration)>,
    provisions: Vec<(bool, Duration)>,
}

impl StressSummary {
    fn note_repair(
        &mut self,
        round: u64,
        repair_elapsed: Duration,
        waited: bool,
        query_first_try: bool,
        query_elapsed: Duration,
    ) {
        self.repairs.push((
            round,
            repair_elapsed,
            waited,
            query_first_try,
            query_elapsed,
        ));
    }

    fn note_provision(&mut self, output: &Output, elapsed: Duration) {
        let waited = String::from_utf8_lossy(&output.stderr).contains(KEY_PROPAGATION_NOTICE);
        self.provisions.push((waited, elapsed));
    }

    fn print(&self, planned_repairs: u64) {
        eprintln!(
            "  summary: {} of {planned_repairs} repairs ran",
            self.repairs.len()
        );
        for (round, repair, waited, first_try, query) in &self.repairs {
            eprintln!(
                "  summary: repair {round}: {:.1}s{}; query right after: {} in {:.1}s",
                repair.as_secs_f64(),
                if *waited {
                    " (waited on key propagation)"
                } else {
                    ""
                },
                if *first_try {
                    "ok first time"
                } else {
                    "REJECTED first time"
                },
                query.as_secs_f64(),
            );
        }
        let waited = self.repairs.iter().filter(|r| r.2).count();
        let rejected = self.repairs.iter().filter(|r| !r.3).count();
        eprintln!(
            "  summary: {waited} repairs waited on key propagation; {rejected} follow-up queries were rejected first time"
        );
        for (index, (waited, elapsed)) in self.provisions.iter().enumerate() {
            eprintln!(
                "  summary: provision {}: {:.1}s{}",
                index + 1,
                elapsed.as_secs_f64(),
                if *waited {
                    " (waited on key propagation)"
                } else {
                    ""
                },
            );
        }
    }

    fn assert_every_query_succeeded_first_time(&self) -> TestResult<()> {
        let rejected: Vec<u64> = self.repairs.iter().filter(|r| !r.3).map(|r| r.0).collect();
        if rejected.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "the query right after a successful repair was rejected in rounds {rejected:?}"
            )
            .into())
        }
    }
}

fn count_from_env_or(name: &str, default: u64) -> TestResult<u64> {
    match std::env::var(name) {
        Ok(value) => Ok(value.parse()?),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(Box::new(error)),
    }
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
