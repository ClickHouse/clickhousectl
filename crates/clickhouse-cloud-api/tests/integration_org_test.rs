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
/// Test phases land in issues #153 through #157 — additional phases for
/// custom roles, activity, prometheus and private endpoint config are
/// added in their own issues.
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
        // bodies for custom roles, activity, prometheus and private
        // endpoint config are added in #154–#157.

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
                                let target = target_for_patch.clone();
                                let body = MemberPatchRequest {
                                    assigned_role_ids: Some(target.clone()),
                                    ..Default::default()
                                };
                                async move {
                                    let resp = client
                                        .member_update(&org_id, &user_id, &body)
                                        .await?;
                                    let updated = resp
                                        .result
                                        .ok_or("member update returned no result")?;
                                    let got: std::collections::HashSet<String> = updated
                                        .assigned_roles
                                        .iter()
                                        .map(|ar| ar.role_id.to_string())
                                        .collect();
                                    let want: std::collections::HashSet<String> =
                                        target.into_iter().collect();
                                    if got != want {
                                        return Err(format!(
                                            "member update echoed unexpected role ids {got:?}; \
                                             wanted {want:?}"
                                        )
                                        .into());
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
                                async move {
                                    let resp = client.member_get(&org_id, &user_id).await?;
                                    let member = resp
                                        .result
                                        .ok_or("member get returned no result")?;
                                    let got: std::collections::HashSet<String> = member
                                        .assigned_roles
                                        .iter()
                                        .map(|ar| ar.role_id.to_string())
                                        .collect();
                                    if got != want {
                                        return Err(format!(
                                            "post-flip GET returned role ids {got:?}; \
                                             wanted {want:?}"
                                        )
                                        .into());
                                    }
                                    Ok(())
                                }
                            },
                        )
                        .await?;

                    // Eager restore (best-effort). The cleanup registry
                    // still owns the safety net; this just keeps the org
                    // clean if the remaining test body needs to read the
                    // original assignments.
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
                                async move {
                                    let resp = client
                                        .member_update(&org_id, &user_id, &body)
                                        .await?;
                                    let restored = resp
                                        .result
                                        .ok_or("member restore returned no result")?;
                                    let got: std::collections::HashSet<String> = restored
                                        .assigned_roles
                                        .iter()
                                        .map(|ar| ar.role_id.to_string())
                                        .collect();
                                    let want: std::collections::HashSet<String> =
                                        want_ids.into_iter().collect();
                                    if got != want {
                                        return Err(format!(
                                            "restore echoed unexpected role ids {got:?}; \
                                             wanted {want:?}"
                                        )
                                        .into());
                                    }
                                    Ok(())
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

