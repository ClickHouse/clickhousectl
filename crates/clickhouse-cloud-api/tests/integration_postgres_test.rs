mod common;

use clickhouse_cloud_api::models::*;
use common::support::*;

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_postgres_crud_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();

    let test_result = async {
        log_run_header("cloud_postgres_crud_lifecycle", &ctx);
        let mut failures = FailureRecorder::default();
        let size = PgSize::R8gd_medium;

        // ── Preflight ───────────────────────────────────────────────

        log_phase("Preflight");
        let list_before = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "check for leftover tagged postgres services",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let filters = ctx.postgres_run_tag_filters();
                    async move {
                        let resp = client.postgres_service_get_list(&org_id).await?;
                        let services = resp
                            .result
                            .ok_or("postgres list returned no result")?;
                        let leftover: Vec<_> = services
                            .into_iter()
                            .filter(|s| filters_match_tags(&filters, &s.tags))
                            .collect();
                        Ok(leftover)
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            list_before.is_empty(),
            "found an existing tagged postgres service for this run id before create"
        );

        // ── Provision ───────────────────────────────────────────────

        log_phase("Provision Postgres Service");

        let create_body = PostgresServicePostRequest {
            name: ctx.postgres_service_name(),
            provider: PgProvider::Unknown(ctx.provider.clone()),
            region: ctx.region.clone(),
            size: size.clone(),
            tags: Some(ctx.postgres_run_tags()),
            ..Default::default()
        };

        let created = failures
            .run(&ctx, StepKind::Blocking, "create postgres service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let body = create_body.clone();
                async move {
                    let resp = client.postgres_service_create(&org_id, &body).await?;
                    resp.result
                        .ok_or_else(|| "postgres create returned no result".into())
                }
            })
            .await?
            .expect("blocking steps always return a value");

        let postgres_id = created.id.to_string();
        eprintln!("postgres_id: <redacted>");
        cleanup.register_postgres(postgres_id.clone());

        let ready = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for postgres service running",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    async move {
                        poll_until(
                            "postgres running state",
                            ctx.steady_state_timeout,
                            ctx.poll_interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                let postgres_id = postgres_id.clone();
                                async move {
                                    let resp = client
                                        .postgres_service_get(&org_id, &postgres_id)
                                        .await?;
                                    let svc = resp
                                        .result
                                        .ok_or("postgres get returned no result")?;
                                    if svc.state.to_string() == "running" {
                                        Ok(Some(svc))
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
            .await?
            .expect("blocking steps always return a value");

        assert_eq!(ready.name, ctx.postgres_service_name());
        assert_eq!(ready.size.to_string(), size.to_string());
        assert_eq!(ready.region, ctx.region);
        assert_eq!(ready.provider.to_string(), ctx.provider);
        assert!(
            !ready.hostname.is_empty(),
            "running postgres service returned empty hostname"
        );
        assert!(
            !ready.connection_string.is_empty(),
            "running postgres service returned empty connection string"
        );

        let listed = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify postgres service is discoverable in list",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    async move {
                        let resp = client.postgres_service_get_list(&org_id).await?;
                        resp.result
                            .ok_or_else(|| "postgres list returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            listed.iter().any(|s| s.id.to_string() == postgres_id),
            "created postgres service was not visible in list"
        );

        // ── Certificates ────────────────────────────────────────────

        log_phase("Certificates");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "fetch postgres CA certificates",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    async move {
                        let pem = client
                            .postgres_service_certs_get(&org_id, &postgres_id)
                            .await?;
                        if !pem.contains("BEGIN CERTIFICATE") {
                            return Err(format!(
                                "cert response did not look like a PEM bundle: {} bytes",
                                pem.len()
                            )
                            .into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Runtime Config ──────────────────────────────────────────
        //
        // PATCH is intentionally not exercised end-to-end here: the generated
        // PgConfig struct has non-Option `serde_json::Value` fields that
        // serialize as `null`, which the live API rejects with
        // `Validation failed for following fields: pg_config.*`. Once the
        // OpenAPI spec marks these fields as optional (or the generator
        // emits Option<Value>) we can extend this phase to round-trip a
        // change to max_connections and verify via GET.

        log_phase("Runtime Config");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "get postgres runtime config",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    async move {
                        let resp = client
                            .postgres_instance_config_get(&org_id, &postgres_id)
                            .await?;
                        if resp.result.is_none() {
                            return Err("postgres config get returned no result".into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Patch (tags) ────────────────────────────────────────────
        //
        // We exercise PATCH by updating `tags` rather than `name`. The beta
        // Postgres PATCH endpoint rejects `name` values that the CREATE and
        // Service PATCH endpoints accept (e.g. hyphens, plain alphanumerics)
        // with "request body property can't be validated: name" — likely a
        // server-side validation bug. Switch this phase back to name once
        // the endpoint exits beta and accepts the same grammar as CREATE.

        log_phase("Patch (tags)");
        let mut new_tags = ctx.postgres_run_tags();
        new_tags.push(ResourceTagsV1 {
            key: "phase".to_string(),
            value: Some("patched".to_string()),
        });
        failures
            .run(&ctx, StepKind::Blocking, "patch postgres tags", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let postgres_id = postgres_id.clone();
                let tags = new_tags.clone();
                async move {
                    let body = PostgresServicePatchRequest {
                        tags: Some(tags),
                        ..Default::default()
                    };
                    client
                        .postgres_service_patch(&org_id, &postgres_id, &body)
                        .await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify tag patch visible in get",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    async move {
                        let resp = client
                            .postgres_service_get(&org_id, &postgres_id)
                            .await?;
                        let svc = resp.result.ok_or("postgres get returned no result")?;
                        let has_phase_tag = svc.tags.iter().any(|t| {
                            t.key == "phase" && t.value.as_deref() == Some("patched")
                        });
                        if !has_phase_tag {
                            return Err("patched `phase=patched` tag not present on service after PATCH".into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Password ────────────────────────────────────────────────

        // Per OpenAPI spec, PostgresServicePasswordResource.password is only
        // populated when the request omits `password` (server-generated path).
        // Because `PostgresServiceSetPassword.password` is a required String
        // in the generated model, we exercise the user-supplied path here and
        // treat a successful 200 as the pass condition — the response will
        // correctly contain an empty/absent password in that case.
        log_phase("Password");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "reset postgres superuser password",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    let new_password = format!("ItPw-{}-Xx!9", ctx.run_id);
                    async move {
                        let body = PostgresServiceSetPassword {
                            password: new_password,
                        };
                        client
                            .postgres_service_set_password(&org_id, &postgres_id, &body)
                            .await?;
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Restart ─────────────────────────────────────────────────

        log_phase("Restart");
        failures
            .run(&ctx, StepKind::Blocking, "restart postgres service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let postgres_id = postgres_id.clone();
                let timeout = ctx.create_timeout;
                let interval = ctx.poll_interval;
                async move {
                    client
                        .postgres_service_patch_state(
                            &org_id,
                            &postgres_id,
                            &PostgresServiceSetState {
                                command: PostgresServiceSetStateCommand::Restart,
                            },
                        )
                        .await?;
                    poll_until("postgres running after restart", timeout, interval, || {
                        let client = client.clone();
                        let org_id = org_id.clone();
                        let postgres_id = postgres_id.clone();
                        async move {
                            let resp = client
                                .postgres_service_get(&org_id, &postgres_id)
                                .await?;
                            let svc = resp
                                .result
                                .ok_or("postgres get returned no result")?;
                            if svc.state.to_string() == "running" {
                                Ok(Some(()))
                            } else {
                                Ok(None)
                            }
                        }
                    })
                    .await?;
                    Ok(())
                }
            })
            .await?;

        // ── Read Replica ────────────────────────────────────────────
        //
        // Cleanup-order note: the live API refuses to delete a primary while
        // any read replica still references it, so the replica MUST be torn
        // down BEFORE the primary. We rely on two complementary mechanisms:
        //
        //   1. `CleanupRegistry::register_postgres_replica` below — invoked
        //      *immediately* after create returns the id. The cleanup phase
        //      deletes registered replicas before registered primaries, so
        //      a mid-test panic between create and the in-body teardown
        //      cannot leak the replica and brick the primary delete.
        //   2. An explicit in-body teardown of the replica that runs before
        //      the primary Delete phase below, plus
        //      `unregister_postgres_replica` to keep the registry tidy on
        //      the happy path.

        log_phase("Read Replica");

        let replica_tags = {
            let mut t = ctx.postgres_run_tags();
            t.push(ResourceTagsV1 {
                key: "phase".to_string(),
                value: Some("read_replica".to_string()),
            });
            t
        };

        let replica = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "create postgres read replica",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    let body = PostgresServiceReadReplicaRequest {
                        name: ctx.postgres_replica_name(),
                        tags: Some(replica_tags.clone()),
                        ..Default::default()
                    };
                    async move {
                        let resp = client
                            .postgres_instance_create_read_replica(
                                &org_id,
                                &postgres_id,
                                &body,
                            )
                            .await?;
                        resp.result.ok_or_else(|| {
                            "postgres read replica create returned no result".into()
                        })
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");

        let replica_id = replica.id.to_string();
        eprintln!("postgres_replica_id: <redacted>");
        // Register before any further interaction so a panic in a later
        // step cannot leak the replica.
        cleanup.register_postgres_replica(replica_id.clone());

        // The create response should mark the new service as a replica.
        // Soft-assert via the FailureRecorder so spec drift on this single
        // field doesn't take the whole run down.
        let replica_was_primary_on_create = replica.is_primary;
        let replica_name_on_create = replica.name.clone();
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "verify replica create response marks is_primary=false",
                || async move {
                    if replica_was_primary_on_create {
                        return Err(format!(
                            "expected replica `{replica_name_on_create}` to have isPrimary=false on create response"
                        )
                        .into());
                    }
                    Ok(())
                },
            )
            .await?;

        let replica_ready = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for postgres read replica running",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let replica_id = replica_id.clone();
                    async move {
                        poll_until(
                            "postgres replica running state",
                            ctx.steady_state_timeout,
                            ctx.poll_interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                let replica_id = replica_id.clone();
                                async move {
                                    let resp = client
                                        .postgres_service_get(&org_id, &replica_id)
                                        .await?;
                                    let svc = resp
                                        .result
                                        .ok_or("postgres replica get returned no result")?;
                                    if svc.state.to_string() == "running" {
                                        Ok(Some(svc))
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
            .await?
            .expect("blocking steps always return a value");

        assert_eq!(replica_ready.name, ctx.postgres_replica_name());
        assert!(
            !replica_ready.is_primary,
            "running read replica reported isPrimary=true"
        );
        assert!(
            !replica_ready.hostname.is_empty(),
            "running postgres replica returned empty hostname"
        );
        assert!(
            !replica_ready.connection_string.is_empty(),
            "running postgres replica returned empty connection string"
        );

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "verify replica appears in postgres_service_get_list alongside primary",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let primary_id = postgres_id.clone();
                    let replica_id = replica_id.clone();
                    async move {
                        let resp = client.postgres_service_get_list(&org_id).await?;
                        let services = resp
                            .result
                            .ok_or("postgres list returned no result")?;
                        let primary_seen =
                            services.iter().any(|s| s.id.to_string() == primary_id);
                        let replica_entry = services
                            .iter()
                            .find(|s| s.id.to_string() == replica_id);
                        if !primary_seen {
                            return Err(
                                "primary postgres service no longer visible in list after replica create"
                                    .into(),
                            );
                        }
                        let Some(replica_entry) = replica_entry else {
                            return Err(
                                "created read replica was not visible in postgres_service_get_list"
                                    .into(),
                            );
                        };
                        if replica_entry.is_primary {
                            return Err(
                                "replica entry in list reported isPrimary=true".into(),
                            );
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "verify postgres_service_get on replica returns expected primary reference",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let replica_id = replica_id.clone();
                    let expected_provider = ctx.provider.clone();
                    let expected_region = ctx.region.clone();
                    async move {
                        let resp = client
                            .postgres_service_get(&org_id, &replica_id)
                            .await?;
                        let svc = resp
                            .result
                            .ok_or("postgres replica get returned no result")?;
                        if svc.is_primary {
                            return Err(
                                "GET on replica id returned isPrimary=true".into(),
                            );
                        }
                        // A read replica inherits provider+region from its
                        // primary. These are the closest "primary reference"
                        // signals the current API surface exposes on the
                        // PostgresService model.
                        if svc.provider.to_string() != expected_provider {
                            return Err(format!(
                                "replica provider `{}` did not match primary `{}`",
                                svc.provider, expected_provider
                            )
                            .into());
                        }
                        if svc.region != expected_region {
                            return Err(format!(
                                "replica region `{}` did not match primary `{}`",
                                svc.region, expected_region
                            )
                            .into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        // Explicit replica teardown BEFORE the primary Delete phase. This is
        // Blocking: if it fails the primary delete will fail too, which is
        // a cleanup-order failure that would leak resources. The replica is
        // also tracked by the cleanup registry as a safety net.
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "delete postgres read replica before primary",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let replica_id = replica_id.clone();
                    async move {
                        client
                            .postgres_service_delete(&org_id, &replica_id)
                            .await?;
                        Ok(())
                    }
                },
            )
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "confirm postgres read replica is gone after delete",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let replica_id = replica_id.clone();
                    let timeout = ctx.delete_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        poll_until("postgres replica deletion", timeout, interval, || {
                            let client = client.clone();
                            let org_id = org_id.clone();
                            let replica_id = replica_id.clone();
                            async move {
                                match client
                                    .postgres_service_get(&org_id, &replica_id)
                                    .await
                                {
                                    Ok(_) => Ok(None),
                                    Err(clickhouse_cloud_api::Error::Api {
                                        status: 404, ..
                                    }) => Ok(Some(())),
                                    Err(e) => {
                                        let message = e.to_string();
                                        if message.contains("404")
                                            || message.contains("not found")
                                        {
                                            Ok(Some(()))
                                        } else {
                                            Err(e.into())
                                        }
                                    }
                                }
                            }
                        })
                        .await?;
                        Ok(())
                    }
                },
            )
            .await?;
        cleanup.unregister_postgres_replica(&replica_id);

        // ── Delete ──────────────────────────────────────────────────

        log_phase("Delete");
        failures
            .run(&ctx, StepKind::Blocking, "delete postgres service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let postgres_id = postgres_id.clone();
                async move {
                    client.postgres_service_delete(&org_id, &postgres_id).await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "confirm postgres service is gone after delete",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    let timeout = ctx.delete_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        poll_until("postgres deletion", timeout, interval, || {
                            let client = client.clone();
                            let org_id = org_id.clone();
                            let postgres_id = postgres_id.clone();
                            async move {
                                match client
                                    .postgres_service_get(&org_id, &postgres_id)
                                    .await
                                {
                                    Ok(_) => Ok(None),
                                    Err(clickhouse_cloud_api::Error::Api {
                                        status: 404, ..
                                    }) => Ok(Some(())),
                                    Err(e) => {
                                        let message = e.to_string();
                                        if message.contains("404")
                                            || message.contains("not found")
                                        {
                                            Ok(Some(()))
                                        } else {
                                            Err(e.into())
                                        }
                                    }
                                }
                            }
                        })
                        .await?;
                        Ok(())
                    }
                },
            )
            .await?;
        cleanup.unregister_postgres(&postgres_id);

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

fn filters_match_tags(filters: &[String], tags: &[ResourceTagsV1]) -> bool {
    filters.iter().all(|filter| {
        let Some(expr) = filter.strip_prefix("tag:") else {
            return true;
        };
        let Some((key, value)) = expr.split_once('=') else {
            return tags.iter().any(|t| t.key == expr);
        };
        tags.iter()
            .any(|t| t.key == key && t.value.as_deref() == Some(value))
    })
}
