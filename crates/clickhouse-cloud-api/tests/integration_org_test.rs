mod common;

use clickhouse_cloud_api::models::Activity;
use common::support::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Org-scoped live integration suite.
///
/// Single `#[tokio::test]` lifecycle that exercises org-level endpoints
/// (members, invitations, custom roles, activity, prometheus, private
/// endpoint config, openapi keys) without provisioning a ClickHouse or
/// Postgres service. The shape mirrors `integration_test.rs` and
/// `integration_postgres_test.rs`:
///
/// - `TestContext::from_env()` builds the shared run config.
/// - `FailureRecorder` accumulates non-blocking failures so one CI run
///   reports every broken endpoint, not just the first.
/// - `CleanupRegistry` records every created resource so teardown runs
///   even if the test body panics.
///
/// Test phases land in issues #153 through #157 — this file currently
/// holds only the lifecycle scaffold and an org access sanity check.
#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a secondary user fixture"]
async fn cloud_org_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();

    let test_result = async {
        log_run_header("cloud_org_lifecycle", &ctx);
        let mut failures = FailureRecorder::default();

        // ── Org Access ──────────────────────────────────────────────
        //
        // Confirm the API key can reach the configured org before any
        // downstream phase tries to mutate org-scoped resources. Phase
        // bodies for members, invitations, roles, activity, prometheus
        // and private endpoint config are added in #153–#157.

        log_phase("Org Access");
        let org = failures
            .run(&ctx, StepKind::Blocking, "verify org access", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                async move {
                    let resp = client.organization_get(&org_id).await?;
                    resp.result
                        .ok_or_else(|| "org get returned no result".into())
                }
            })
            .await?
            .expect("blocking steps always return a value");
        assert_eq!(org.id.to_string(), ctx.org_id);

        // ── Org Observability ───────────────────────────────────────
        //
        // Read-only checks against org-scoped endpoints that don't
        // require any fixture beyond the org itself. Both steps are
        // NonBlocking — they exist purely to detect live API drift.

        log_phase("Org Observability");

        failures
            .run(&ctx, StepKind::NonBlocking, "organization prometheus", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                async move {
                    // The org-level prometheus exporter returns empty
                    // output when no service in the org is emitting metrics
                    // at request time. This suite deliberately does not
                    // provision a service, so empty output is a valid
                    // response — coverage here is that the call succeeds.
                    // The service-level prometheus endpoint is covered with
                    // a non-empty assertion in integration_test.rs.
                    let _metrics = client.organization_prometheus_get(&org_id, None).await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "organization private endpoint config list",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let cloud_provider = ctx.provider.clone();
                    let region_id = ctx.region.clone();
                    async move {
                        // Deprecated endpoint. The API requires an existing
                        // instance in the requested provider+region before
                        // it returns a config, but this suite is
                        // deliberately service-less. Treat the
                        // "no created instances" 400 as the expected
                        // response: it still proves auth, routing and the
                        // 400 deserialization path. Any other response
                        // (including a 200) is fine too — the integration
                        // service suite covers the populated path with a
                        // real instance.
                        #[allow(deprecated)]
                        let result = client
                            .organization_private_endpoint_config_get_list(
                                &org_id,
                                &cloud_provider,
                                &region_id,
                            )
                            .await;
                        match result {
                            Ok(_) => Ok(()),
                            Err(clickhouse_cloud_api::Error::Api { status: 400, message })
                                if message.contains("no created instances") =>
                            {
                                eprintln!(
                                    "  expected 400 (no instances in region) — endpoint reachable"
                                );
                                Ok(())
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                },
            )
            .await?;

        // ── Activity Log ────────────────────────────────────────────
        //
        // Exercises `activity_get_list` and `activity_get`.
        //
        // Dependency note: per issue #151, this phase is intended to run
        // *after* the Custom Roles phase (#154) so there is a known recent
        // event (role create + delete) attributable to this test run.
        // Until #154 lands, the phase still runs but degrades to "any
        // entry within the test window" — see the polling check below.
        // When #154 is added, its phase block should be inserted directly
        // above this one to preserve the ordering.
        //
        // Activity log writes are eventually consistent. We poll
        // `activity_get_list` for up to ~30s for an entry whose
        // `created_at` falls within the current test window. On miss the
        // step records a NonBlocking FailureRecorder entry rather than
        // failing the run — other org activity may not always be
        // present in short windows on a quiet org.

        log_phase("Activity Log");

        // Capture the test window start before the first list call so any
        // role create/delete events from #154 (or other concurrent org
        // activity) recorded above are eligible. Subtract a small slack
        // window to absorb minor clock skew between the test host and the
        // control plane.
        let window_start_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(600);
        let activity_poll_budget = Duration::from_secs(30);
        let activity_poll_interval = Duration::from_secs(3);

        let recent_activity: Option<Activity> = failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "activity_get_list returns a recent entry (poll up to 30s)",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    async move {
                        // Poll with a Some/None pattern via poll_until; on
                        // timeout we map the polling error into a clean
                        // NonBlocking failure message so the FailureRecorder
                        // summary is readable.
                        match poll_until(
                            "activity entry within test window",
                            activity_poll_budget,
                            activity_poll_interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                async move {
                                    let resp = client
                                        .activity_get_list(&org_id, None, None)
                                        .await?;
                                    let entries = resp.result.unwrap_or_default();
                                    if entries.is_empty() {
                                        return Ok(None);
                                    }
                                    let hit = entries.into_iter().find(|a| {
                                        a.created_at.timestamp() as u64 >= window_start_secs
                                    });
                                    Ok(hit)
                                }
                            },
                        )
                        .await
                        {
                            Ok(activity) => Ok(activity),
                            Err(e) => Err(format!(
                                "no activity entry observed within {:?} budget: {e}",
                                activity_poll_budget
                            )
                            .into()),
                        }
                    }
                },
            )
            .await?;

        if let Some(activity) = recent_activity {
            let activity_id = activity.id.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "activity_get returns the entry with populated fields",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let activity_id = activity_id.clone();
                        async move {
                            let resp = client.activity_get(&org_id, &activity_id).await?;
                            let fetched = resp.result.ok_or_else(|| {
                                "activity_get returned no result".to_string()
                            })?;
                            if fetched.id != activity_id {
                                return Err(format!(
                                    "activity_get returned id {} but requested {}",
                                    fetched.id, activity_id
                                )
                                .into());
                            }
                            if fetched.organization_id.is_empty() {
                                return Err(
                                    "activity_get returned empty organizationId".into()
                                );
                            }
                            if fetched.created_at.timestamp() == 0 {
                                return Err(
                                    "activity_get returned zero createdAt".into()
                                );
                            }
                            Ok(())
                        }
                    },
                )
                .await?;
        }

        failures.finish()
    }
    .await;

    let cleanup_result = cleanup
        .cleanup(&client, &ctx.org_id, ctx.delete_timeout, ctx.poll_interval, None)
        .await;

    match (test_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error.into()),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}\ncleanup failed:\n{cleanup_error}").into())
        }
    }
}
