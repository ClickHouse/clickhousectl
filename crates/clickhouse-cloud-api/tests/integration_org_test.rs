mod common;

use common::support::*;

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
