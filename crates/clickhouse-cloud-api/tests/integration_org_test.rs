mod common;

use clickhouse_cloud_api::models::*;
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

        // ── Members ─────────────────────────────────────────────────
        //
        // Cover `member_get_list`, `member_get`, and `member_update`
        // against the configured secondary user. The pre-test role is
        // captured and registered with the cleanup registry *before*
        // any mutating call, so teardown restores the original role
        // even if the assertions below blow up.
        //
        // `member_delete` is deliberately out of scope (would kick the
        // test fixture from the org) per #151.

        log_phase("Members");
        let secondary_user_id = ctx.secondary_user_id()?.to_string();

        let members = failures
            .run(&ctx, StepKind::NonBlocking, "list org members", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                async move {
                    let resp = client.member_get_list(&org_id).await?;
                    resp.result
                        .ok_or_else(|| "member list returned no result".into())
                }
            })
            .await?;

        if let Some(members) = members.as_ref() {
            let secondary_present = members
                .iter()
                .any(|m| m.user_id == secondary_user_id);
            assert!(
                secondary_present,
                "member list did not include configured secondary user"
            );
            // The list must surface at least two distinct users — the API
            // key's owner plus the secondary fixture. We don't pin the
            // exact primary id (varies per fixture), just that the list
            // is wider than the secondary user alone.
            let distinct_users = members
                .iter()
                .map(|m| m.user_id.as_str())
                .collect::<std::collections::HashSet<_>>();
            assert!(
                distinct_users.len() >= 2,
                "member list contained fewer than two users; expected primary + secondary"
            );
        }

        let secondary_member = failures
            .run(&ctx, StepKind::NonBlocking, "get secondary member", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let user_id = secondary_user_id.clone();
                async move {
                    let resp = client.member_get(&org_id, &user_id).await?;
                    resp.result
                        .ok_or_else(|| "member get returned no result".into())
                }
            })
            .await?;

        if let Some(secondary_member) = secondary_member {
            assert_eq!(secondary_member.user_id, secondary_user_id);
            assert!(
                !secondary_member.email.is_empty(),
                "secondary member email was empty"
            );

            // Custom Roles round-trip. Orgs that have migrated to Custom
            // Roles reject the legacy `role` enum on PATCH ("Organization
            // has migrated to Custom Roles. Use 'assignedRoleIds' instead
            // of 'role'."), so the round-trip swaps the secondary user's
            // assignedRoleIds and restores the original set.
            //
            // The teardown task is registered before the mutating call so
            // a panic between here and the next assertion can't leave the
            // fixture with the wrong assignments.

            let original_role_ids: Vec<String> = secondary_member
                .assigned_roles
                .iter()
                .map(|ar| ar.role_id.to_string())
                .collect();

            let org_roles = failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "list org roles for member round-trip",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        async move {
                            let resp = client.organization_roles_get_list(&org_id).await?;
                            resp.result.ok_or_else(|| "org roles list returned no result".into())
                        }
                    },
                )
                .await?;

            if let Some(org_roles) = org_roles {
                let originals: std::collections::HashSet<&str> =
                    original_role_ids.iter().map(|s| s.as_str()).collect();
                let candidate_extra = org_roles
                    .iter()
                    .map(|r| r.id.clone())
                    .find(|id| !originals.contains(id.as_str()));

                let target_role_ids: Option<Vec<String>> = match candidate_extra {
                    Some(extra) => Some(vec![extra]),
                    None if !original_role_ids.is_empty() => Some(Vec::new()),
                    None => None,
                };

                if let Some(target_role_ids) = target_role_ids {
                    cleanup.register_member_role_restore(
                        secondary_user_id.clone(),
                        original_role_ids.clone(),
                    );

                    eprintln!(
                        "  member assignedRoleIds round-trip: {:?} -> {:?} -> {:?}",
                        original_role_ids, target_role_ids, original_role_ids,
                    );

                    // The PATCH response body can echo the *pre-change*
                    // state (the API treats the update as eventually
                    // consistent), so we don't assert on its body — we
                    // only require it to return a result. The subsequent
                    // GET poll is the source of truth for whether the
                    // update propagated.
                    let target_for_patch = target_role_ids.clone();
                    failures
                        .run(
                            &ctx,
                            StepKind::NonBlocking,
                            "flip secondary member assignedRoleIds",
                            || {
                                let client = client.clone();
                                let org_id = ctx.org_id.clone();
                                let user_id = secondary_user_id.clone();
                                let body = MemberPatchRequest {
                                    assigned_role_ids: Some(target_for_patch.clone()),
                                    ..Default::default()
                                };
                                async move {
                                    let resp = client
                                        .member_update(&org_id, &user_id, &body)
                                        .await?;
                                    if resp.result.is_none() {
                                        return Err(
                                            "member update returned no result".into(),
                                        );
                                    }
                                    Ok(())
                                }
                            },
                        )
                        .await?;

                    let target_for_get = target_role_ids.clone();
                    failures
                        .run(
                            &ctx,
                            StepKind::NonBlocking,
                            "verify flipped assignedRoleIds via GET",
                            || {
                                let client = client.clone();
                                let org_id = ctx.org_id.clone();
                                let user_id = secondary_user_id.clone();
                                let want: std::collections::HashSet<String> =
                                    target_for_get.into_iter().collect();
                                let interval = ctx.poll_interval;
                                async move {
                                    // Poll for propagation. The PATCH is
                                    // accepted immediately but the GET
                                    // endpoint may lag by a few seconds.
                                    poll_until(
                                        "member assignedRoleIds reflects flip",
                                        std::time::Duration::from_secs(30),
                                        interval,
                                        || {
                                            let client = client.clone();
                                            let org_id = org_id.clone();
                                            let user_id = user_id.clone();
                                            let want = want.clone();
                                            async move {
                                                let resp = client
                                                    .member_get(&org_id, &user_id)
                                                    .await?;
                                                let member = resp.result.ok_or(
                                                    "member get returned no result",
                                                )?;
                                                let got: std::collections::HashSet<String> =
                                                    member
                                                        .assigned_roles
                                                        .iter()
                                                        .map(|ar| ar.role_id.to_string())
                                                        .collect();
                                                if got == want {
                                                    Ok(Some(()))
                                                } else {
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

                    // Eager restore (best-effort). The cleanup registry
                    // still owns the safety net; this just keeps the org
                    // clean if the remaining test body needs to read the
                    // original assignments. Same eventual-consistency
                    // caveat applies — assert via poll, not the PATCH
                    // response.
                    let original_for_restore = original_role_ids.clone();
                    failures
                        .run(
                            &ctx,
                            StepKind::NonBlocking,
                            "restore secondary member assignedRoleIds",
                            || {
                                let client = client.clone();
                                let org_id = ctx.org_id.clone();
                                let user_id = secondary_user_id.clone();
                                let want_ids = original_for_restore.clone();
                                let body = MemberPatchRequest {
                                    assigned_role_ids: Some(want_ids.clone()),
                                    ..Default::default()
                                };
                                let interval = ctx.poll_interval;
                                async move {
                                    let resp = client
                                        .member_update(&org_id, &user_id, &body)
                                        .await?;
                                    if resp.result.is_none() {
                                        return Err(
                                            "member restore returned no result".into(),
                                        );
                                    }
                                    let want: std::collections::HashSet<String> =
                                        want_ids.into_iter().collect();
                                    poll_until(
                                        "member assignedRoleIds reflects restore",
                                        std::time::Duration::from_secs(30),
                                        interval,
                                        || {
                                            let client = client.clone();
                                            let org_id = org_id.clone();
                                            let user_id = user_id.clone();
                                            let want = want.clone();
                                            async move {
                                                let resp = client
                                                    .member_get(&org_id, &user_id)
                                                    .await?;
                                                let member = resp.result.ok_or(
                                                    "member get returned no result",
                                                )?;
                                                let got: std::collections::HashSet<String> =
                                                    member
                                                        .assigned_roles
                                                        .iter()
                                                        .map(|ar| ar.role_id.to_string())
                                                        .collect();
                                                if got == want {
                                                    Ok(Some(()))
                                                } else {
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
                    // Eager restore succeeded — drop the registry entry so
                    // cleanup doesn't issue a redundant PATCH at teardown.
                    cleanup.unregister_member_role_restore(&secondary_user_id);
                } else {
                    eprintln!(
                        "  SKIP assignedRoleIds round-trip: org has no role distinct \
                         from the secondary user's current assignments and the user \
                         has no roles to drop"
                    );
                }
            }
        }

        // ── Invitations ─────────────────────────────────────────────
        //
        // Cover the full invitation CRUD against a synthetic recipient.
        // We address invitations to
        // `alasdair.brown+clickhousectl_{run_id}@clickhouse.com` — Gmail
        // catch-all aliasing means the message lands in a real inbox
        // but is never acted on, and the run-id keeps two CI runs from
        // colliding. The invitation is cancelled (deleted) before the
        // recipient could realistically accept; the cleanup registry
        // is the safety net if the test body fails before cancellation.
        //
        // Invitation accept is UI-only and out of scope per #151.

        log_phase("Invitations");
        let invitation_email = format!(
            "alasdair.brown+clickhousectl_{}@clickhouse.com",
            ctx.run_id
        );
        let invitation_email_for_assert = invitation_email.clone();

        let invitation = failures
            .run(&ctx, StepKind::NonBlocking, "create invitation", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let body = InvitationPostRequest {
                    email: invitation_email.clone(),
                    role: InvitationPostRequestRole::Developer,
                    assigned_role_ids: vec![],
                };
                async move {
                    let resp = client.invitation_create(&org_id, &body).await?;
                    resp.result
                        .ok_or_else(|| "invitation create returned no result".into())
                }
            })
            .await?;

        if let Some(invitation) = invitation {
            let invitation_id = invitation.id.to_string();
            cleanup.register_invitation(invitation_id.clone());
            assert_eq!(
                invitation.email, invitation_email_for_assert,
                "invitation create echoed unexpected email"
            );
            assert_eq!(
                invitation.role.to_string(),
                InvitationPostRequestRole::Developer.to_string(),
                "invitation create echoed unexpected role"
            );

            let invitation_id_for_list = invitation_id.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "list invitations includes new one",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        async move {
                            let resp = client.invitation_get_list(&org_id).await?;
                            let list = resp
                                .result
                                .ok_or("invitation list returned no result")?;
                            if !list
                                .iter()
                                .any(|inv| inv.id.to_string() == invitation_id_for_list)
                            {
                                return Err(format!(
                                    "invitation list did not include new invitation {invitation_id_for_list}"
                                )
                                .into());
                            }
                            Ok(())
                        }
                    },
                )
                .await?;

            let invitation_id_for_get = invitation_id.clone();
            let invitation_email_for_get = invitation_email_for_assert.clone();
            failures
                .run(&ctx, StepKind::NonBlocking, "get invitation", || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let id = invitation_id_for_get.clone();
                    let want_email = invitation_email_for_get.clone();
                    async move {
                        let resp = client.invitation_get(&org_id, &id).await?;
                        let fetched = resp
                            .result
                            .ok_or("invitation get returned no result")?;
                        if fetched.id.to_string() != id {
                            return Err(format!(
                                "invitation get returned wrong id {}; wanted {id}",
                                fetched.id
                            )
                            .into());
                        }
                        if fetched.email != want_email {
                            return Err(format!(
                                "invitation get returned wrong email {}; wanted {want_email}",
                                fetched.email
                            )
                            .into());
                        }
                        if fetched.role.to_string() != "developer" {
                            return Err(format!(
                                "invitation get returned wrong role {}; wanted developer",
                                fetched.role
                            )
                            .into());
                        }
                        Ok(())
                    }
                })
                .await?;

            let invitation_id_for_delete = invitation_id.clone();
            let delete_result = failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "cancel invitation before accept",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let id = invitation_id_for_delete.clone();
                        async move {
                            client.invitation_delete(&org_id, &id).await?;
                            Ok(())
                        }
                    },
                )
                .await?;
            if delete_result.is_some() {
                // Successful eager cancel — drop the registry entry so
                // teardown doesn't issue a redundant 404.
                cleanup.unregister_invitation(&invitation_id);
            }
        }

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
        // Permissions on a single policy must share a resource scope; the
        // API rejects mixed-scope policies ("All permissions in a policy
        // must target the same resource scope"). The create step uses one
        // org-scoped policy; the patch step extends to a second
        // service-scoped policy, exercising the multi-policy code path.
        let initial_org_permissions = vec!["control-plane:organization:view".to_string()];
        let patched_service_permissions = vec!["control-plane:service:view".to_string()];
        let all_patched_permissions: Vec<String> = initial_org_permissions
            .iter()
            .chain(patched_service_permissions.iter())
            .cloned()
            .collect();

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
                let permissions = initial_org_permissions.clone();
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
            let initial_org_permissions_clone = initial_org_permissions.clone();
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
                        let expected_permissions = initial_org_permissions_clone;
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

            // Patch the role: extend the permissions to a second policy
            // with a different resource scope. PATCH replaces the full
            // policy set, so we send name + actors unchanged plus two
            // policies — one org-scoped (kept from create) and one
            // service-scoped (new). Mixing scopes inside a single policy
            // is rejected by the API.
            let role_id_clone = role_id.clone();
            let role_name_clone = role_name.clone();
            let org_perms_clone = initial_org_permissions.clone();
            let service_perms_clone = patched_service_permissions.clone();
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
                        let org_perms = org_perms_clone;
                        let service_perms = service_perms_clone;
                        async move {
                            let org_resource = format!("organization/{org_id}");
                            let body = RoleUpdateRequest {
                                name,
                                actors: vec![],
                                policies: vec![
                                    RBACPolicyCreateRequest {
                                        allow_deny: RBACPolicyCreateRequestAllowdeny::ALLOW,
                                        permissions: org_perms,
                                        resources: vec![org_resource],
                                        tags: None,
                                    },
                                    RBACPolicyCreateRequest {
                                        allow_deny: RBACPolicyCreateRequestAllowdeny::ALLOW,
                                        permissions: service_perms,
                                        resources: vec!["instance/*".to_string()],
                                        tags: None,
                                    },
                                ],
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
            let expected_all_clone = all_patched_permissions.clone();
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "get custom role reflects patched permissions",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let role_id = role_id_clone;
                        let expected_permissions = expected_all_clone;
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
