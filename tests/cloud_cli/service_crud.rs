use crate::support::{
    CleanupRegistry, CliRunner, FailureRecorder, StepKind, TestContext, TestResult,
    delete_service_and_confirm_gone, json_string, log_phase, log_run_header, poll_until,
    service_has_ip_access_entry, service_list_is_empty, service_name_in_list,
    service_present_in_list,
};
use serde_json::Value;

#[test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
fn cloud_service_crud_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let runner = CliRunner::new(&ctx);
    let mut cleanup = CleanupRegistry::default();

    let test_result = (|| -> TestResult<()> {
        log_run_header("cloud_service_crud_lifecycle", &ctx);
        let mut failures = FailureRecorder::default();
        let primary_ip = "203.0.113.10/32";
        let secondary_ip = "203.0.113.11/32";
        let create_options_ip = "203.0.113.12/32";
        let base_memory_gb = 8_u64;
        let scaled_memory_gb = 16_u64;
        let base_replicas = 3_u64;
        let scaled_replicas = 4_u64;

        log_phase("Org Checks");
        let org = failures
            .run(&ctx, StepKind::Blocking, "verify org access", || {
                runner.run_cloud(["org".to_string(), "get".to_string(), ctx.org_id.clone()])
            })?
            .expect("blocking steps always return a value");
        let org_id = json_string(&org.json, &["/id", "/org/id"])?;
        assert_eq!(org_id, ctx.org_id);
        let current_org_name = json_string(&org.json, &["/name", "/org/name"])?.to_string();

        let org_list = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify org list includes target org",
                || runner.run_cloud(["org".to_string(), "list".to_string()]),
            )?
            .expect("blocking steps always return a value");
        assert!(
            org_list_contains(&org_list.json, &ctx.org_id),
            "org list did not include target org {}",
            ctx.org_id
        );

        failures.run(&ctx, StepKind::NonBlocking, "idempotent org update", || {
            let updated = runner.run_cloud([
                "org".to_string(),
                "update".to_string(),
                ctx.org_id.clone(),
                "--name".to_string(),
                current_org_name.clone(),
            ])?;
            let updated_org_id = json_string(&updated.json, &["/id", "/org/id"])?;
            if updated_org_id != ctx.org_id {
                return Err(
                    format!("org update returned unexpected org id {}", updated_org_id).into(),
                );
            }
            Ok(())
        })?;

        failures.run(&ctx, StepKind::NonBlocking, "org usage", || {
            let usage = runner.run_cloud([
                "org".to_string(),
                "usage".to_string(),
                ctx.org_id.clone(),
                "--from-date".to_string(),
                "2025-01-01".to_string(),
                "--to-date".to_string(),
                "2025-01-31".to_string(),
            ])?;
            if usage.json.is_null() {
                return Err("org usage returned null JSON".into());
            }
            Ok(())
        })?;

        log_phase("Provision Service");
        let list_before = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "check for leftover tagged services",
                || runner.service_list_for_run(),
            )?
            .expect("blocking steps always return a value");
        assert!(
            service_list_is_empty(&list_before.json),
            "found an existing tagged test service for this run id before create"
        );

        let mut create_args = vec![
            "service".to_string(),
            "create".to_string(),
            "--name".to_string(),
            ctx.service_name(),
            "--provider".to_string(),
            ctx.provider.clone(),
            "--region".to_string(),
            ctx.region.clone(),
            "--min-replica-memory-gb".to_string(),
            base_memory_gb.to_string(),
            "--max-replica-memory-gb".to_string(),
            base_memory_gb.to_string(),
            "--num-replicas".to_string(),
            base_replicas.to_string(),
            "--ip-allow".to_string(),
            create_options_ip.to_string(),
            "--idle-scaling".to_string(),
            "true".to_string(),
            "--idle-timeout-minutes".to_string(),
            "5".to_string(),
            "--org-id".to_string(),
            ctx.org_id.clone(),
        ];

        for tag in ctx.run_tags() {
            create_args.push("--tag".to_string());
            create_args.push(tag);
        }

        let created = failures
            .run(&ctx, StepKind::Blocking, "create service", || {
                runner.run_cloud(create_args)
            })?
            .expect("blocking steps always return a value");
        let service_id = json_string(&created.json, &["/service/id", "/id"])?.to_string();
        let password = json_string(&created.json, &["/password", "/service/password"])?.to_string();
        eprintln!("service_id: <redacted>");
        cleanup.register_service(service_id.clone());

        let ready = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for service steady state",
                || {
                    poll_until(
                        "service steady state",
                        ctx.steady_state_timeout,
                        ctx.poll_interval,
                        || {
                            let output = runner.service_get(&service_id)?;
                            let state = json_string(&output.json, &["/service/state", "/state"])?;
                            if matches!(state, "running" | "idle") {
                                Ok(Some(output))
                            } else {
                                Ok(None)
                            }
                        },
                    )
                },
            )?
            .expect("blocking steps always return a value");
        let ready_name = json_string(&ready.json, &["/service/name", "/name"])?;
        assert_eq!(ready_name, ctx.service_name());
        assert!(
            service_has_ip_access_entry(&ready.json, create_options_ip),
            "created service did not expose expected initial ip allow entry {}",
            create_options_ip
        );
        assert_eq!(
            json_u64(
                &ready.json,
                &["/service/minReplicaMemoryGb", "/minReplicaMemoryGb"]
            ),
            Some(base_memory_gb)
        );
        assert_eq!(
            json_u64(
                &ready.json,
                &["/service/maxReplicaMemoryGb", "/maxReplicaMemoryGb"]
            ),
            Some(base_memory_gb)
        );
        assert_eq!(
            json_u64(&ready.json, &["/service/numReplicas", "/numReplicas"]),
            Some(base_replicas)
        );

        let listed = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify service is discoverable in list",
                || runner.service_list_for_run(),
            )?
            .expect("blocking steps always return a value");
        assert!(
            service_present_in_list(&listed.json, &service_id),
            "created service was not visible in service list"
        );

        failures.run(&ctx, StepKind::Blocking, "rename service", || {
            runner.run_cloud([
                "service".to_string(),
                "update".to_string(),
                service_id.clone(),
                "--name".to_string(),
                ctx.updated_service_name(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])
        })?;

        let updated = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for rename visibility in get",
                || {
                    poll_until(
                        "service rename visibility in get",
                        ctx.create_timeout,
                        ctx.poll_interval,
                        || {
                            let output = runner.service_get(&service_id)?;
                            let name = json_string(&output.json, &["/service/name", "/name"])?;
                            if name == ctx.updated_service_name() {
                                Ok(Some(output))
                            } else {
                                Ok(None)
                            }
                        },
                    )
                },
            )?
            .expect("blocking steps always return a value");
        let updated_name = json_string(&updated.json, &["/service/name", "/name"])?;
        assert_eq!(updated_name, ctx.updated_service_name());

        let renamed_list = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify rename is visible in list",
                || {
                    poll_until(
                        "service rename visibility in list",
                        ctx.create_timeout,
                        ctx.poll_interval,
                        || {
                            let output = runner.service_list_for_run()?;
                            if service_name_in_list(&output.json, &service_id)
                                .as_deref()
                                .is_some_and(|name| name == ctx.updated_service_name())
                            {
                                Ok(Some(output))
                            } else {
                                Ok(None)
                            }
                        },
                    )
                },
            )?
            .expect("blocking steps always return a value");
        assert_eq!(
            service_name_in_list(&renamed_list.json, &service_id).as_deref(),
            Some(ctx.updated_service_name().as_str())
        );

        log_phase("Capability Checks");
        failures.run(&ctx, StepKind::NonBlocking, "service prometheus", || {
            let metrics = runner.run_cloud_raw([
                "service".to_string(),
                "prometheus".to_string(),
                service_id.clone(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])?;
            if metrics.stdout.trim().is_empty() {
                return Err("service prometheus returned empty output".into());
            }
            Ok(())
        })?;

        let open_ip = "0.0.0.0/0";
        failures.run(
            &ctx,
            StepKind::Blocking,
            "open ip access for client tests",
            || {
                mutate_ip_allow_entry(&ctx, &runner, &service_id, "--add-ip-allow", open_ip)?;
                poll_for_ip_presence(&ctx, &runner, &service_id, open_ip, true)
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "cloud service client query by id",
            || {
                let output = runner.service_client_query(&service_id, &password, "SELECT 1")?;
                let trimmed = output.stdout.trim();
                if trimmed != "1" {
                    return Err(format!(
                        "expected SELECT 1 to return '1', got '{}'",
                        trimmed
                    )
                    .into());
                }
                Ok(())
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "cloud service client query by name",
            || {
                let output = runner.service_client_query_by_name(
                    &ctx.updated_service_name(),
                    &password,
                    "SELECT 'cloud_client_ok'",
                )?;
                let trimmed = output.stdout.trim();
                if trimmed != "cloud_client_ok" {
                    return Err(format!(
                        "expected 'cloud_client_ok', got '{}'",
                        trimmed
                    )
                    .into());
                }
                Ok(())
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "cloud service client with generate-password",
            || {
                let output = runner.service_client_query_generate_password(
                    &service_id,
                    "SELECT 'gen_pw_ok'",
                )?;
                let trimmed = output.stdout.trim();
                if trimmed != "gen_pw_ok" {
                    return Err(format!(
                        "expected 'gen_pw_ok', got '{}'",
                        trimmed
                    )
                    .into());
                }
                Ok(())
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "close ip access after client tests",
            || {
                mutate_ip_allow_entry(&ctx, &runner, &service_id, "--remove-ip-allow", open_ip)?;
                poll_for_ip_presence(&ctx, &runner, &service_id, open_ip, false)
            },
        )?;

        failures.run(&ctx, StepKind::NonBlocking, "idempotent rename", || {
            runner.run_cloud([
                "service".to_string(),
                "update".to_string(),
                service_id.clone(),
                "--name".to_string(),
                ctx.updated_service_name(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])?;
            Ok(())
        })?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "service update enable_core_dumps",
            || {
                let current = runner.service_get(&service_id)?;
                let current_value = json_bool(
                    &current.json,
                    &["/service/enableCoreDumps", "/enableCoreDumps"],
                )
                .unwrap_or(false);
                runner.run_cloud([
                    "service".to_string(),
                    "update".to_string(),
                    service_id.clone(),
                    "--enable-core-dumps".to_string(),
                    current_value.to_string(),
                    "--org-id".to_string(),
                    ctx.org_id.clone(),
                ])?;
                Ok(())
            },
        )?;

        failures.run(&ctx, StepKind::NonBlocking, "add service tag", || {
            runner.run_cloud([
                "service".to_string(),
                "update".to_string(),
                service_id.clone(),
                "--add-tag".to_string(),
                "phase=updated".to_string(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])?;
            Ok(())
        })?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "add first ip allow entry",
            || {
                mutate_ip_allow_entry(&ctx, &runner, &service_id, "--add-ip-allow", primary_ip)?;
                poll_for_ip_presence(&ctx, &runner, &service_id, primary_ip, true)
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "add second ip allow entry",
            || {
                mutate_ip_allow_entry(&ctx, &runner, &service_id, "--add-ip-allow", secondary_ip)?;
                poll_until(
                    "multiple ip allow visibility",
                    ctx.create_timeout,
                    ctx.poll_interval,
                    || {
                        let output = runner.service_get(&service_id)?;
                        if service_has_ip_access_entry(&output.json, primary_ip)
                            && service_has_ip_access_entry(&output.json, secondary_ip)
                        {
                            Ok(Some(()))
                        } else {
                            Ok(None)
                        }
                    },
                )?;
                Ok(())
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "remove one ip allow entry while keeping another",
            || {
                mutate_ip_allow_entry(&ctx, &runner, &service_id, "--remove-ip-allow", primary_ip)?;
                poll_until(
                    "partial ip allow removal",
                    ctx.create_timeout,
                    ctx.poll_interval,
                    || {
                        let output = runner.service_get(&service_id)?;
                        if !service_has_ip_access_entry(&output.json, primary_ip)
                            && service_has_ip_access_entry(&output.json, secondary_ip)
                        {
                            Ok(Some(()))
                        } else {
                            Ok(None)
                        }
                    },
                )?;
                Ok(())
            },
        )?;

        failures.run(
            &ctx,
            StepKind::NonBlocking,
            "remove remaining ip allow entry",
            || {
                mutate_ip_allow_entry(
                    &ctx,
                    &runner,
                    &service_id,
                    "--remove-ip-allow",
                    secondary_ip,
                )?;
                poll_for_ip_presence(&ctx, &runner, &service_id, secondary_ip, false)
            },
        )?;

        log_phase("Shutdown And Delete");
        failures.run(&ctx, StepKind::Blocking, "stop service", || {
            runner.run_cloud([
                "service".to_string(),
                "stop".to_string(),
                service_id.clone(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])?;
            poll_until(
                "service stopped",
                ctx.create_timeout,
                ctx.poll_interval,
                || {
                    let output = runner.service_get(&service_id)?;
                    let state = json_string(&output.json, &["/service/state", "/state"])?;
                    if matches!(state, "idle" | "stopped") {
                        Ok(Some(()))
                    } else {
                        Ok(None)
                    }
                },
            )?;
            Ok(())
        })?;

        failures.run(&ctx, StepKind::Blocking, "start service", || {
            runner.run_cloud([
                "service".to_string(),
                "start".to_string(),
                service_id.clone(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])?;
            poll_until(
                "service restarted",
                ctx.steady_state_timeout,
                ctx.poll_interval,
                || {
                    let output = runner.service_get(&service_id)?;
                    let state = json_string(&output.json, &["/service/state", "/state"])?;
                    if matches!(state, "running" | "idle") {
                        Ok(Some(()))
                    } else {
                        Ok(None)
                    }
                },
            )?;
            Ok(())
        })?;

        failures.run(&ctx, StepKind::Blocking, "scale out to 4 replicas", || {
            scale_service_and_wait(
                &ctx,
                &runner,
                &service_id,
                Some(base_memory_gb),
                Some(base_memory_gb),
                Some(scaled_replicas),
                "replica scale out",
            )
        })?;

        failures.run(&ctx, StepKind::Blocking, "scale back to 3 replicas", || {
            scale_service_and_wait(
                &ctx,
                &runner,
                &service_id,
                Some(base_memory_gb),
                Some(base_memory_gb),
                Some(base_replicas),
                "replica scale in",
            )
        })?;

        failures.run(&ctx, StepKind::Blocking, "scale up to 16 GB", || {
            scale_service_and_wait(
                &ctx,
                &runner,
                &service_id,
                Some(scaled_memory_gb),
                Some(scaled_memory_gb),
                Some(base_replicas),
                "vertical scale up",
            )
        })?;

        failures.run(&ctx, StepKind::Blocking, "scale back down to 8 GB", || {
            scale_service_and_wait(
                &ctx,
                &runner,
                &service_id,
                Some(base_memory_gb),
                Some(base_memory_gb),
                Some(base_replicas),
                "vertical scale down",
            )
        })?;

        failures.run(&ctx, StepKind::Blocking, "delete service", || {
            delete_service_and_confirm_gone(&runner, &service_id)
        })?;
        cleanup.unregister_service(&service_id);

        failures.finish()
    })();

    let cleanup_result = cleanup.cleanup(&runner);

    match (test_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error.into()),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}\ncleanup failed:\n{cleanup_error}").into())
        }
    }
}

fn mutate_ip_allow_entry(
    ctx: &TestContext,
    runner: &CliRunner<'_>,
    service_id: &str,
    flag: &str,
    ip: &str,
) -> TestResult<()> {
    runner.run_cloud([
        "service".to_string(),
        "update".to_string(),
        service_id.to_string(),
        flag.to_string(),
        ip.to_string(),
        "--org-id".to_string(),
        ctx.org_id.clone(),
    ])?;
    Ok(())
}

fn poll_for_ip_presence(
    ctx: &TestContext,
    runner: &CliRunner<'_>,
    service_id: &str,
    ip: &str,
    expected_present: bool,
) -> TestResult<()> {
    poll_until(
        &format!("ip visibility for {}", ip),
        ctx.create_timeout,
        ctx.poll_interval,
        || {
            let output = runner.service_get(service_id)?;
            if service_has_ip_access_entry(&output.json, ip) == expected_present {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        },
    )?;
    Ok(())
}

fn org_list_contains(value: &Value, org_id: &str) -> bool {
    value
        .pointer("/organizations")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .is_some_and(|orgs| {
            orgs.iter().any(|org| {
                org.pointer("/id")
                    .and_then(Value::as_str)
                    .is_some_and(|candidate| candidate == org_id)
            })
        })
}

fn json_u64(value: &Value, pointers: &[&str]) -> Option<u64> {
    pointers.iter().find_map(|pointer| {
        value.pointer(pointer).and_then(|candidate| {
            candidate.as_u64().or_else(|| {
                let number = candidate.as_f64()?;
                if number.fract() == 0.0 && number >= 0.0 {
                    Some(number as u64)
                } else {
                    None
                }
            })
        })
    })
}

fn json_bool(value: &Value, pointers: &[&str]) -> Option<bool> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_bool))
}

fn scale_service_and_wait(
    ctx: &TestContext,
    runner: &CliRunner<'_>,
    service_id: &str,
    min_memory_gb: Option<u64>,
    max_memory_gb: Option<u64>,
    replicas: Option<u64>,
    description: &str,
) -> TestResult<()> {
    let mut args = vec![
        "service".to_string(),
        "scale".to_string(),
        service_id.to_string(),
    ];

    if let Some(value) = min_memory_gb {
        args.push("--min-replica-memory-gb".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = max_memory_gb {
        args.push("--max-replica-memory-gb".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = replicas {
        args.push("--num-replicas".to_string());
        args.push(value.to_string());
    }
    args.push("--org-id".to_string());
    args.push(ctx.org_id.clone());

    runner.run_cloud(args)?;

    poll_until(
        &format!("{description} visibility"),
        ctx.steady_state_timeout,
        ctx.poll_interval,
        || {
            let output = runner.service_get(service_id)?;
            let current_min = json_u64(
                &output.json,
                &["/service/minReplicaMemoryGb", "/minReplicaMemoryGb"],
            );
            let current_max = json_u64(
                &output.json,
                &["/service/maxReplicaMemoryGb", "/maxReplicaMemoryGb"],
            );
            let current_replicas =
                json_u64(&output.json, &["/service/numReplicas", "/numReplicas"]);

            if min_memory_gb.is_none_or(|value| current_min == Some(value))
                && max_memory_gb.is_none_or(|value| current_max == Some(value))
                && replicas.is_none_or(|value| current_replicas == Some(value))
            {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        },
    )?;

    Ok(())
}
