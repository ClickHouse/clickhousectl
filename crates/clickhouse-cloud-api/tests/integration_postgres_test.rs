mod common;

use std::env;

use clickhouse_cloud_api::models::*;
use common::support::*;
use serde_json::json;

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

        log_phase("Runtime Config");
        let baseline = failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "get postgres runtime config baseline",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let postgres_id = postgres_id.clone();
                    async move {
                        let resp = client
                            .postgres_instance_config_get(&org_id, &postgres_id)
                            .await?;
                        resp.result
                            .ok_or_else(|| "postgres config get returned no result".into())
                    }
                },
            )
            .await?;

        // Behaviour-matrix probe — gated on env var so it doesn't run on
        // every integration test invocation. Captures the 6 × 2 scenarios
        // from #163's follow-up comment for the upstream spec issue.
        if env::var("CLICKHOUSE_CLOUD_POSTGRES_CONFIG_PROBE")
            .ok()
            .filter(|s| !s.is_empty())
            .is_some()
        {
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "capture pgConfig behaviour matrix",
                    || {
                        let org_id = ctx.org_id.clone();
                        let postgres_id = postgres_id.clone();
                        async move { run_pg_config_probe(&org_id, &postgres_id).await }
                    },
                )
                .await?;
        }

        // Round-trip pgConfig fields: PATCH max_connections and
        // autovacuum_max_workers to new values, poll-until GET reflects
        // them, then PATCH back. GET
        // returns numeric pgConfig values wrapped in JSON strings (the spec
        // types them string-or-number), so extract via pg_config_value_as_i64
        // and compare tolerantly.
        if let Some(baseline) = baseline {
            let baseline_max = baseline
                .pg_config
                .max_connections
                .as_ref()
                .and_then(pg_config_value_as_i64)
                .unwrap_or(100);
            let target = baseline_max + 7;
            // Postgres defaults autovacuum_max_workers to 3; the per-run
            // service is deleted at the end, so drift on reset is harmless.
            let baseline_autovacuum = baseline
                .pg_config
                .autovacuum_max_workers
                .as_ref()
                .and_then(pg_config_value_as_i64)
                .unwrap_or(3);
            let autovacuum_target = baseline_autovacuum + 1;

            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "patch pgConfig.max_connections/autovacuum_max_workers",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let postgres_id = postgres_id.clone();
                        async move {
                            let body = PostgresInstanceConfig {
                                pg_config: PgConfig {
                                    max_connections: Some(serde_json::json!(target)),
                                    autovacuum_max_workers: Some(serde_json::json!(
                                        autovacuum_target
                                    )),
                                    ..Default::default()
                                },
                                pg_bouncer_config: PgBouncerConfig::default(),
                            };
                            client
                                .postgres_instance_config_patch(&org_id, &postgres_id, &body)
                                .await?;
                            Ok(())
                        }
                    },
                )
                .await?;

            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "verify pgConfig.max_connections patch visible",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let postgres_id = postgres_id.clone();
                        let timeout = ctx.steady_state_timeout;
                        let interval = ctx.poll_interval;
                        async move {
                            poll_until(
                                "pg_config.max_connections/autovacuum_max_workers == targets",
                                timeout,
                                interval,
                                || {
                                    let client = client.clone();
                                    let org_id = org_id.clone();
                                    let postgres_id = postgres_id.clone();
                                    async move {
                                        let resp = client
                                            .postgres_instance_config_get(&org_id, &postgres_id)
                                            .await?;
                                        let pg_config =
                                            resp.result.map(|r| r.pg_config).unwrap_or_default();
                                        let observed = pg_config
                                            .max_connections
                                            .as_ref()
                                            .and_then(pg_config_value_as_i64);
                                        let observed_autovacuum = pg_config
                                            .autovacuum_max_workers
                                            .as_ref()
                                            .and_then(pg_config_value_as_i64);
                                        if observed == Some(target)
                                            && observed_autovacuum == Some(autovacuum_target)
                                        {
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

            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "reset pgConfig.max_connections to baseline",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let postgres_id = postgres_id.clone();
                        async move {
                            let body = PostgresInstanceConfig {
                                pg_config: PgConfig {
                                    max_connections: Some(serde_json::json!(baseline_max)),
                                    autovacuum_max_workers: Some(serde_json::json!(
                                        baseline_autovacuum
                                    )),
                                    ..Default::default()
                                },
                                pg_bouncer_config: PgBouncerConfig::default(),
                            };
                            client
                                .postgres_instance_config_patch(&org_id, &postgres_id, &body)
                                .await?;
                            Ok(())
                        }
                    },
                )
                .await?;
        }

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
        // The replica create is retried for up to the steady-state timeout:
        // the live API rejects a read-replica create against a primary that
        // hasn't taken its first backup yet (`400 ... no backups, yet.` /
        // `... not ready for read replicas`), and that backup lands some
        // time after the primary reaches `running`. Without the retry the
        // flake just moves from the wait-for-running step to the create
        // step.
        //
        // The test does NOT poll the replica to `running`. Provisioning a
        // replica can take longer than the steady-state timeout (observed
        // 30+ min when the API accepts a create against a backup-less
        // primary), and the test only needs to prove the API surface works,
        // not that provisioning completes. So we assert the create response
        // shape, that the replica is visible in list/get (in any state),
        // and then tear it down — deleting a still-provisioning replica is
        // proven safe (prior failed runs deleted stuck replicas in ~11s).
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
                    let timeout = ctx.steady_state_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        let resp = retry_api_call(
                            "create postgres read replica",
                            timeout,
                            interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                let postgres_id = postgres_id.clone();
                                let body = body.clone();
                                async move {
                                    client
                                        .postgres_instance_create_read_replica(
                                            &org_id,
                                            &postgres_id,
                                            &body,
                                        )
                                        .await
                                }
                            },
                            is_no_backups_yet_error,
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

        // The create response should mark the new service as a replica with
        // the requested name and a non-empty state. Soft-assert via the
        // FailureRecorder so spec drift on a single field doesn't take the
        // whole run down. We deliberately do NOT wait for `running`: see the
        // phase header comment.
        let replica_is_primary_on_create = replica.is_primary;
        let replica_name_on_create = replica.name.clone();
        let replica_state_on_create = replica.state.to_string();
        let expected_replica_name = ctx.postgres_replica_name();
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "verify replica create response shape",
                || async move {
                    if replica_is_primary_on_create {
                        return Err(format!(
                            "expected replica `{replica_name_on_create}` to have isPrimary=false on create response"
                        )
                        .into());
                    }
                    if replica_name_on_create != expected_replica_name {
                        return Err(format!(
                            "replica name `{replica_name_on_create}` did not match expected `{expected_replica_name}`"
                        )
                        .into());
                    }
                    if replica_state_on_create.is_empty() {
                        return Err(
                            "replica create response returned empty state".into(),
                        );
                    }
                    Ok(())
                },
            )
            .await?;

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
                        // The replica may still be provisioning at this point —
                        // we deliberately don't wait for `running` (see the
                        // phase header). provider/region are inherited from the
                        // primary and present regardless of state; isPrimary
                        // is set at create time.
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
        // also tracked by the cleanup registry as a safety net. Deleting a
        // still-provisioning replica is safe (prior failed runs deleted
        // stuck replicas in ~11s); we don't need it to reach `running`
        // first.
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

// pgConfig numeric values come back from GET wrapped in JSON strings (e.g.
// `"max_connections": "100"`), while PATCH accepts plain numbers. The spec
// types these fields as string-or-number, so extract an i64 from either
// representation.
fn pg_config_value_as_i64(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
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

/// Predicate for [`retry_api_call`]: is this the "primary has no backups yet"
/// 400 that blocks read-replica creation?
///
/// The live API rejects a read-replica create against a primary that hasn't
/// taken its first backup with a 400 whose message contains one of:
///   - `no backups`        — `"Parent server is not ready for read replicas.
///                             There are no backups, yet."`
///   - `not ready for read replicas` — same response, alternate phrasing.
///
/// Matching on substrings (rather than `status == 400` alone) avoids masking
/// unrelated validation 400s that should fail the test, not retry.
fn is_no_backups_yet_error(error: &clickhouse_cloud_api::Error) -> bool {
    match error {
        clickhouse_cloud_api::Error::Api { status: 400, message } => {
            let lower = message.to_ascii_lowercase();
            lower.contains("no backups") || lower.contains("not ready for read replicas")
        }
        _ => false,
    }
}

// Sends the 6 body shapes from #163's follow-up comment to both PATCH and
// POST `/v1/organizations/{org}/postgres/{id}/config` using raw reqwest, so
// shapes the typed `PostgresInstanceConfig` cannot represent (e.g. omitted
// `pgBouncerConfig`, explicit nulls) are sent verbatim. Prints a markdown
// table on stderr for direct copy into the upstream spec issue.
async fn run_pg_config_probe(org_id: &str, postgres_id: &str) -> TestResult<()> {
    let key = required_env("CLICKHOUSE_CLOUD_API_KEY")?;
    let secret = required_env("CLICKHOUSE_CLOUD_API_SECRET")?;
    let base_url = env::var("CLICKHOUSE_CLOUD_API_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.clickhouse.cloud".to_string())
        .trim_end_matches('/')
        .to_string();
    let url = format!("{base_url}/v1/organizations/{org_id}/postgres/{postgres_id}/config");

    let valid_pg_config = json!({ "max_connections": 200 });
    let scenarios: Vec<(&str, serde_json::Value)> = vec![
        (
            "1: pgConfig only (omit pgBouncerConfig)",
            json!({ "pgConfig": valid_pg_config }),
        ),
        (
            "2: pgConfig + pgBouncerConfig: {}",
            json!({ "pgConfig": valid_pg_config, "pgBouncerConfig": {} }),
        ),
        (
            "3: pgConfig + pgBouncerConfig: null",
            json!({ "pgConfig": valid_pg_config, "pgBouncerConfig": null }),
        ),
        (
            "4: pgBouncerConfig only (omit pgConfig)",
            json!({ "pgBouncerConfig": { "default_pool_size": "10" } }),
        ),
        (
            "5: pgConfig single-field partial",
            json!({ "pgConfig": { "max_connections": 200 } }),
        ),
        (
            "6: pgConfig single-field explicit null",
            json!({ "pgConfig": { "max_connections": null } }),
        ),
    ];

    let http = reqwest::Client::new();
    let mut rows: Vec<(String, String, u16, String)> = Vec::new();

    for method in [reqwest::Method::PATCH, reqwest::Method::POST] {
        for (label, body) in &scenarios {
            let resp = http
                .request(method.clone(), &url)
                .basic_auth(&key, Some(&secret))
                .json(body)
                .send()
                .await
                .map_err(|e| format!("{method} {label}: {e}"))?;
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let snippet: String = text.chars().take(180).collect::<String>().replace('\n', " ");
            rows.push((method.to_string(), label.to_string(), status, snippet));
        }
    }

    eprintln!("\n## Postgres config behaviour matrix\n");
    eprintln!("| Method | Body shape | Status | Response (≤180 chars) |");
    eprintln!("|--------|------------|--------|------------------------|");
    for (method, label, status, snippet) in &rows {
        eprintln!("| {method} | {label} | {status} | `{snippet}` |");
    }
    eprintln!();
    Ok(())
}
