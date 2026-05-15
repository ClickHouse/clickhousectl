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

        // ── Custom Roles CRUD ───────────────────────────────────────
        //
        // Exercise the custom org role lifecycle without touching the
        // secondary user fixture: create a synthetic role whose name
        // embeds `ctx.run_id` so concurrent CI runs don't collide,
        // register it for teardown immediately, then verify it appears
        // in list/get, patch a benign field, and finally delete it.
        // Every step is NonBlocking — no later phase depends on the
        // role existing.

        log_phase("Custom Roles CRUD");

        let role_name = format!("clickhousectl-it-role-{}", ctx.run_id);
        let initial_permissions = vec!["control-plane:organization:view".to_string()];
        let updated_permissions = vec![
            "control-plane:organization:view".to_string(),
            "control-plane:service:view".to_string(),
        ];

        // List roles before creation so we can sanity-check that the
        // created role is genuinely new (id absent from the pre-state).
        let pre_role_ids = failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "list roles (pre-create)",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    async move {
                        let resp = client.organization_roles_get_list(&org_id).await?;
                        let roles = resp
                            .result
                            .ok_or("roles list returned no result")?;
                        Ok(roles.into_iter().map(|r| r.id).collect::<Vec<_>>())
                    }
                },
            )
            .await?;

        let created_role = failures
            .run(&ctx, StepKind::NonBlocking, "create custom role", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let role_name = role_name.clone();
                let permissions = initial_permissions.clone();
                async move {
                    // Org-scoped resources must reference the literal org
                    // id; the API rejects `organization/*` ("Organization *
                    // must match the role's organization") even though
                    // `instance/*` is a valid wildcard form.
                    let resource = format!("organization/{org_id}");
                    let body = RoleCreateRequest {
                        name: role_name.clone(),
                        actors: vec![],
                        policies: vec![RBACPolicyCreateRequest {
                            allow_deny: RBACPolicyCreateRequestAllowdeny::ALLOW,
                            permissions,
                            resources: vec![resource],
                            tags: None,
                        }],
                    };
                    let resp = client.organization_role_post(&org_id, &body).await?;
                    let role = resp
                        .result
                        .ok_or("role create returned no result")?;
                    if role.name != role_name {
                        return Err(format!(
                            "created role name mismatch: expected {role_name}, got {}",
                            role.name
                        )
                        .into());
                    }
                    Ok(role)
                }
            })
            .await?;

        // Register for cleanup before any further mutation so a failure
        // mid-phase still reclaims the resource via teardown.
        let role_id = if let Some(role) = created_role.as_ref() {
            cleanup.register_role(role.id.clone());
            Some(role.id.clone())
        } else {
            None
        };

        if let (Some(role_id), Some(pre_ids)) = (role_id.as_ref(), pre_role_ids.as_ref()) {
            let role_id_clone = role_id.clone();
            let pre_ids_clone = pre_ids.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "list roles (post-create) contains new role",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let role_id = role_id_clone;
                        let pre_ids = pre_ids_clone;
                        async move {
                            let resp = client.organization_roles_get_list(&org_id).await?;
                            let roles = resp
                                .result
                                .ok_or("roles list returned no result")?;
                            if !roles.iter().any(|r| r.id == role_id) {
                                return Err(format!(
                                    "created role {role_id} not visible in roles list"
                                )
                                .into());
                            }
                            if pre_ids.iter().any(|id| id == &role_id) {
                                return Err(format!(
                                    "role id {role_id} unexpectedly present before creation"
                                )
                                .into());
                            }
                            Ok(())
                        }
                    },
                )
                .await?;

            let role_id_clone = role_id.clone();
            let role_name_clone = role_name.clone();
            let initial_permissions_clone = initial_permissions.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "get custom role returns expected fields",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let role_id = role_id_clone;
                        let expected_name = role_name_clone;
                        let expected_permissions = initial_permissions_clone;
                        async move {
                            let resp = client.organization_role_get(&org_id, &role_id).await?;
                            let role = resp
                                .result
                                .ok_or("role get returned no result")?;
                            if role.id != role_id {
                                return Err(format!(
                                    "role id mismatch: expected {role_id}, got {}",
                                    role.id
                                )
                                .into());
                            }
                            if role.name != expected_name {
                                return Err(format!(
                                    "role name mismatch: expected {expected_name}, got {}",
                                    role.name
                                )
                                .into());
                            }
                            if !matches!(role.r#type, RBACRoleType::Custom) {
                                return Err(format!(
                                    "expected custom role type, got {:?}",
                                    role.r#type
                                )
                                .into());
                            }
                            let actual_permissions: Vec<String> = role
                                .policies
                                .iter()
                                .flat_map(|p| p.permissions.iter().cloned())
                                .collect();
                            for expected in &expected_permissions {
                                if !actual_permissions.iter().any(|p| p == expected) {
                                    return Err(format!(
                                        "role get is missing expected permission {expected}; \
                                         got {actual_permissions:?}"
                                    )
                                    .into());
                                }
                            }
                            Ok(())
                        }
                    },
                )
                .await?;

            // Patch the role: extend the permissions list. PATCH on this
            // endpoint replaces the full set, so we send name + actors
            // unchanged plus a single policy with the new permissions.
            let role_id_clone = role_id.clone();
            let role_name_clone = role_name.clone();
            let updated_permissions_clone = updated_permissions.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "patch custom role permissions",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let role_id = role_id_clone;
                        let name = role_name_clone;
                        let permissions = updated_permissions_clone;
                        async move {
                            let resource = format!("organization/{org_id}");
                            let body = RoleUpdateRequest {
                                name,
                                actors: vec![],
                                policies: vec![RBACPolicyCreateRequest {
                                    allow_deny: RBACPolicyCreateRequestAllowdeny::ALLOW,
                                    permissions,
                                    resources: vec![resource],
                                    tags: None,
                                }],
                            };
                            client
                                .organization_role_patch(&org_id, &role_id, &body)
                                .await?;
                            Ok(())
                        }
                    },
                )
                .await?;

            // Verify the patch via GET — the API may return the new
            // permissions in the PATCH response, but a follow-up GET is
            // what real callers will observe.
            let role_id_clone = role_id.clone();
            let updated_permissions_clone = updated_permissions.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "get custom role reflects patched permissions",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let role_id = role_id_clone;
                        let expected_permissions = updated_permissions_clone;
                        async move {
                            let resp = client.organization_role_get(&org_id, &role_id).await?;
                            let role = resp
                                .result
                                .ok_or("role get returned no result")?;
                            let actual_permissions: Vec<String> = role
                                .policies
                                .iter()
                                .flat_map(|p| p.permissions.iter().cloned())
                                .collect();
                            for expected in &expected_permissions {
                                if !actual_permissions.iter().any(|p| p == expected) {
                                    return Err(format!(
                                        "patched role is missing permission {expected}; \
                                         got {actual_permissions:?}"
                                    )
                                    .into());
                                }
                            }
                            Ok(())
                        }
                    },
                )
                .await?;

            // Delete the role and confirm via GET → 404. Unregister
            // cleanup on success; leave the registration in place on
            // failure so teardown can still try.
            let role_id_clone = role_id.clone();
            let delete_ok = failures
                .run(&ctx, StepKind::NonBlocking, "delete custom role", || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let role_id = role_id_clone;
                    async move {
                        client.organization_role_delete(&org_id, &role_id).await?;
                        Ok(())
                    }
                })
                .await?
                .is_some();

            if delete_ok {
                cleanup.unregister_role(role_id);

                let role_id_clone = role_id.clone();
                failures
                    .run(
                        &ctx,
                        StepKind::NonBlocking,
                        "get deleted role returns 404",
                        || {
                            let client = client.clone();
                            let org_id = ctx.org_id.clone();
                            let role_id = role_id_clone;
                            async move {
                                match client.organization_role_get(&org_id, &role_id).await {
                                    Ok(_) => Err(format!(
                                        "expected 404 after deleting role {role_id}, got success"
                                    )
                                    .into()),
                                    Err(clickhouse_cloud_api::Error::Api {
                                        status: 404, ..
                                    }) => Ok(()),
                                    Err(e) => Err(format!(
                                        "expected 404 after deleting role {role_id}, got {e}"
                                    )
                                    .into()),
                                }
                            }
                        },
                    )
                    .await?;
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
