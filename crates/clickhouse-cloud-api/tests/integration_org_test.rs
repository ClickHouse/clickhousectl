mod common;

use clickhouse_cloud_api::models::*;
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

        // ── OpenAPI Key Lifecycle ────────────────────────────────────
        //
        // Covers `openapi_key_create`, `openapi_key_get`,
        // `openapi_key_get_list`, `openapi_key_update`, and
        // `openapi_key_delete` against the live API. This key is
        // independent of the service-suite key in `integration_test.rs`
        // so failures in one suite cannot poison the other.

        log_phase("OpenAPI Key Lifecycle");

        let key_name = format!("clickhousectl-org-it-{}", ctx.run_id);
        let key_name_for_create = key_name.clone();

        let created_key = failures
            .run(&ctx, StepKind::NonBlocking, "openapi_key_create", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let key_name = key_name_for_create.clone();
                async move {
                    let body = ApiKeyPostRequest {
                        name: key_name.clone(),
                        assigned_role_ids: vec![],
                        expire_at: None,
                        hash_data: None,
                        ip_access_list: vec![IpAccessListEntry {
                            source: "0.0.0.0/0".to_string(),
                            description: Some(
                                "clickhousectl org integration test key".to_string(),
                            ),
                        }],
                        roles: Some(vec!["admin".to_string()]),
                        state: ApiKeyPostRequestState::Enabled,
                    };
                    let resp = client.openapi_key_create(&org_id, &body).await?;
                    let created = resp
                        .result
                        .ok_or_else(|| "openapi_key_create returned no result".to_string())?;
                    if created.key.name != key_name {
                        return Err(format!(
                            "openapi_key_create returned name {:?}, expected {:?}",
                            created.key.name, key_name
                        )
                        .into());
                    }
                    Ok(created)
                }
            })
            .await?;

        if let Some(created_key) = created_key {
            let api_key_uuid = created_key.key.id.to_string();
            cleanup.register_api_key(api_key_uuid.clone());

            // openapi_key_get — assert fields match what create returned.
            let api_key_uuid_for_get = api_key_uuid.clone();
            let expected_name = key_name.clone();
            failures
                .run(&ctx, StepKind::NonBlocking, "openapi_key_get", || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let api_key_uuid = api_key_uuid_for_get.clone();
                    let expected_name = expected_name.clone();
                    async move {
                        let resp = client.openapi_key_get(&org_id, &api_key_uuid).await?;
                        let key = resp
                            .result
                            .ok_or_else(|| "openapi_key_get returned no result".to_string())?;
                        if key.id.to_string() != api_key_uuid {
                            return Err(format!(
                                "openapi_key_get returned id {}, expected {}",
                                key.id, api_key_uuid
                            )
                            .into());
                        }
                        if key.name != expected_name {
                            return Err(format!(
                                "openapi_key_get returned name {:?}, expected {:?}",
                                key.name, expected_name
                            )
                            .into());
                        }
                        if !matches!(key.state, ApiKeyState::Enabled) {
                            return Err(format!(
                                "openapi_key_get returned state {:?}, expected Enabled",
                                key.state
                            )
                            .into());
                        }
                        Ok(())
                    }
                })
                .await?;

            // openapi_key_get_list — assert the new key appears.
            let api_key_uuid_for_list = api_key_uuid.clone();
            failures
                .run(&ctx, StepKind::NonBlocking, "openapi_key_get_list", || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let api_key_uuid = api_key_uuid_for_list.clone();
                    async move {
                        let resp = client.openapi_key_get_list(&org_id).await?;
                        let keys = resp
                            .result
                            .ok_or_else(|| "openapi_key_get_list returned no result".to_string())?;
                        if !keys.iter().any(|k| k.id.to_string() == api_key_uuid) {
                            return Err(format!(
                                "openapi_key_get_list did not contain newly created key {api_key_uuid} (found {} keys)",
                                keys.len()
                            )
                            .into());
                        }
                        Ok(())
                    }
                })
                .await?;

            // openapi_key_update — round-trip state enabled -> disabled,
            // verify via GET, then restore to enabled and re-verify.
            let api_key_uuid_for_update = api_key_uuid.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "openapi_key_update state round-trip",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let api_key_uuid = api_key_uuid_for_update.clone();
                        async move {
                            let disable_req = ApiKeyPatchRequest {
                                state: Some(ApiKeyPatchRequestState::Disabled),
                                ..ApiKeyPatchRequest::default()
                            };
                            let patched = client
                                .openapi_key_update(&org_id, &api_key_uuid, &disable_req)
                                .await?;
                            let patched_key = patched
                                .result
                                .ok_or_else(|| "openapi_key_update returned no result".to_string())?;
                            if !matches!(patched_key.state, ApiKeyState::Disabled) {
                                return Err(format!(
                                    "openapi_key_update -> disabled returned state {:?}",
                                    patched_key.state
                                )
                                .into());
                            }

                            // Verify the disabled state via a fresh GET.
                            let resp = client.openapi_key_get(&org_id, &api_key_uuid).await?;
                            let key = resp
                                .result
                                .ok_or_else(|| "openapi_key_get after disable returned no result".to_string())?;
                            if !matches!(key.state, ApiKeyState::Disabled) {
                                return Err(format!(
                                    "openapi_key_get after disable returned state {:?}, expected Disabled",
                                    key.state
                                )
                                .into());
                            }

                            // Restore to enabled so the round-trip is symmetric.
                            let enable_req = ApiKeyPatchRequest {
                                state: Some(ApiKeyPatchRequestState::Enabled),
                                ..ApiKeyPatchRequest::default()
                            };
                            let restored = client
                                .openapi_key_update(&org_id, &api_key_uuid, &enable_req)
                                .await?;
                            let restored_key = restored
                                .result
                                .ok_or_else(|| "openapi_key_update -> enabled returned no result".to_string())?;
                            if !matches!(restored_key.state, ApiKeyState::Enabled) {
                                return Err(format!(
                                    "openapi_key_update -> enabled returned state {:?}",
                                    restored_key.state
                                )
                                .into());
                            }
                            Ok(())
                        }
                    },
                )
                .await?;

            // openapi_key_delete — end of phase. On success, drop it from
            // the cleanup registry; if delete fails (or never runs because
            // an earlier blocking step bailed) the registry teardown will
            // mop up with a 404-tolerant best-effort delete.
            let api_key_uuid_for_delete = api_key_uuid.clone();
            let deleted = failures
                .run(&ctx, StepKind::NonBlocking, "openapi_key_delete", || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let api_key_uuid = api_key_uuid_for_delete.clone();
                    async move {
                        client.openapi_key_delete(&org_id, &api_key_uuid).await?;
                        Ok(())
                    }
                })
                .await?;
            if deleted.is_some() {
                cleanup.unregister_api_key(&api_key_uuid);
            }
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
