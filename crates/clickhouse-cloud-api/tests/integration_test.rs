mod integration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;
use integration::support::*;

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_service_crud_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();

    let test_result = async {
        log_run_header("cloud_service_crud_lifecycle", &ctx);
        let mut failures = FailureRecorder::default();
        let base_memory_gb = 8.0_f64;
        let scaled_memory_gb = 16.0_f64;
        let base_replicas = 1.0_f64;
        let scaled_replicas = 3.0_f64;
        let primary_ip = "203.0.113.10/32";
        let secondary_ip = "203.0.113.11/32";

        // ── Org Checks ──────────────────────────────────────────────

        log_phase("Org Checks");
        let org = failures
            .run(&ctx, StepKind::Blocking, "verify org access", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                async move {
                    let resp = client.organization_get(&org_id).await?;
                    resp.result.ok_or_else(|| "org get returned no result".into())
                }
            })
            .await?
            .expect("blocking steps always return a value");
        assert_eq!(org.id.to_string(), ctx.org_id);
        let current_org_name = org.name.clone();

        let org_list = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify org list includes target org",
                || {
                    let client = client.clone();
                    async move {
                        let resp = client.organization_get_list().await?;
                        resp.result.ok_or_else(|| "org list returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            org_list
                .iter()
                .any(|o| o.id.to_string() == ctx.org_id),
            "org list did not include target org {}",
            ctx.org_id
        );

        failures
            .run(&ctx, StepKind::NonBlocking, "idempotent org update", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let name = current_org_name.clone();
                async move {
                    let resp = client
                        .organization_update(
                            &org_id,
                            &OrganizationPatchRequest {
                                name: Some(name),
                                ..Default::default()
                            },
                        )
                        .await?;
                    let updated = resp.result.ok_or("org update returned no result")?;
                    let updated_id = updated.id.to_string();
                    if updated_id != org_id {
                        return Err(
                            format!("org update returned unexpected org id {updated_id}").into()
                        );
                    }
                    Ok(())
                }
            })
            .await?;

        failures
            .run(&ctx, StepKind::NonBlocking, "org usage", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                async move {
                    let resp = client
                        .usage_cost_get(&org_id, "2025-01-01", "2025-01-31", &[])
                        .await?;
                    if resp.result.is_none() {
                        return Err("org usage returned no result".into());
                    }
                    Ok(())
                }
            })
            .await?;

        // ── 1. Provision ─────────────────────────────────────────────

        log_phase("Provision Service");

        let list_before = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "check for leftover tagged services",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let filters = ctx.run_tag_filters();
                    async move {
                        let filter_refs: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
                        let resp = client.instance_get_list(&org_id, &filter_refs).await?;
                        resp.result
                            .ok_or_else(|| "service list returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            list_before.is_empty(),
            "found an existing tagged test service for this run id before create"
        );

        let create_body = ServicePostRequest {
            name: ctx.service_name(),
            provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
            region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
            min_replica_memory_gb: Some(base_memory_gb),
            max_replica_memory_gb: Some(base_memory_gb),
            num_replicas: Some(base_replicas),
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(5.0),
            tags: Some(ctx.run_tags()),
            ..Default::default()
        };

        let created = failures
            .run(&ctx, StepKind::Blocking, "create service", || {
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

        let service = &created.service;
        let service_id = service.id.to_string();
        let _password = created.password.clone();
        eprintln!("service_id: <redacted>");
        cleanup.register_service(service_id.clone());

        let ready = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for service steady state",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        poll_until(
                            "service steady state",
                            ctx.steady_state_timeout,
                            ctx.poll_interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                let service_id = service_id.clone();
                                async move {
                                    let resp =
                                        client.instance_get(&org_id, &service_id).await?;
                                    let svc = resp.result.ok_or("service get returned no result")?;
                                    let state = svc.state.to_string();
                                    if matches!(state.as_str(), "running" | "idle") {
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

        assert_eq!(ready.name, ctx.service_name());
        assert_eq!(ready.min_replica_memory_gb, base_memory_gb);
        assert_eq!(ready.max_replica_memory_gb, base_memory_gb);
        assert_eq!(ready.num_replicas, base_replicas);

        let listed = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify service is discoverable in list",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let filters = ctx.run_tag_filters();
                    async move {
                        let filter_refs: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
                        let resp = client.instance_get_list(&org_id, &filter_refs).await?;
                        resp.result
                            .ok_or_else(|| "service list returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            listed
                .iter()
                .any(|s| s.id.to_string() == service_id),
            "created service was not visible in service list"
        );

        // ── 2. Stop / Start ──────────────────────────────────────────

        log_phase("Stop And Start");
        failures
            .run(&ctx, StepKind::Blocking, "stop service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.create_timeout;
                let interval = ctx.poll_interval;
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
                    poll_until("service stopped", timeout, interval, || {
                        let client = client.clone();
                        let org_id = org_id.clone();
                        let service_id = service_id.clone();
                        async move {
                            let resp = client.instance_get(&org_id, &service_id).await?;
                            let svc = resp.result.ok_or("service get returned no result")?;
                            let state = svc.state.to_string();
                            if matches!(state.as_str(), "idle" | "stopped") {
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

        failures
            .run(&ctx, StepKind::Blocking, "start service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.steady_state_timeout;
                let interval = ctx.poll_interval;
                async move {
                    client
                        .instance_state_update(
                            &org_id,
                            &service_id,
                            &ServiceStatePatchRequest {
                                command: Some(ServiceStatePatchRequestCommand::Start),
                            },
                        )
                        .await?;
                    poll_until("service restarted", timeout, interval, || {
                        let client = client.clone();
                        let org_id = org_id.clone();
                        let service_id = service_id.clone();
                        async move {
                            let resp = client.instance_get(&org_id, &service_id).await?;
                            let svc = resp.result.ok_or("service get returned no result")?;
                            let state = svc.state.to_string();
                            if matches!(state.as_str(), "running" | "idle") {
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

        // ── 3. Rename & Settings ─────────────────────────────────────

        log_phase("Rename And Settings");
        failures
            .run(&ctx, StepKind::Blocking, "rename service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let new_name = ctx.updated_service_name();
                async move {
                    client
                        .instance_update(
                            &org_id,
                            &service_id,
                            &ServicePatchRequest {
                                name: Some(new_name),
                                ..Default::default()
                            },
                        )
                        .await?;
                    Ok(())
                }
            })
            .await?;

        let updated = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for rename visibility in get",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let expected_name = ctx.updated_service_name();
                    let timeout = ctx.create_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        poll_until("service rename visibility in get", timeout, interval, || {
                            let client = client.clone();
                            let org_id = org_id.clone();
                            let service_id = service_id.clone();
                            let expected_name = expected_name.clone();
                            async move {
                                let resp = client.instance_get(&org_id, &service_id).await?;
                                let svc =
                                    resp.result.ok_or("service get returned no result")?;
                                if svc.name == expected_name {
                                    Ok(Some(svc))
                                } else {
                                    Ok(None)
                                }
                            }
                        })
                        .await
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert_eq!(updated.name, ctx.updated_service_name());

        let renamed_list = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "verify rename is visible in list",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let expected_name = ctx.updated_service_name();
                    let filters = ctx.run_tag_filters();
                    let timeout = ctx.create_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        poll_until("service rename visibility in list", timeout, interval, || {
                            let client = client.clone();
                            let org_id = org_id.clone();
                            let service_id = service_id.clone();
                            let expected_name = expected_name.clone();
                            let filters = filters.clone();
                            async move {
                                let filter_refs: Vec<&str> =
                                    filters.iter().map(|s| s.as_str()).collect();
                                let resp =
                                    client.instance_get_list(&org_id, &filter_refs).await?;
                                let services = resp
                                    .result
                                    .ok_or("service list returned no result")?;
                                let found = services.iter().find(|s| {
                                    s.id.to_string() == service_id
                                });
                                if found.is_some_and(|s| s.name == expected_name) {
                                    Ok(Some(services))
                                } else {
                                    Ok(None)
                                }
                            }
                        })
                        .await
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        let renamed_svc = renamed_list
            .iter()
            .find(|s| s.id.to_string() == service_id);
        assert_eq!(
            renamed_svc.map(|s| s.name.as_str()),
            Some(ctx.updated_service_name().as_str())
        );

        failures
            .run(&ctx, StepKind::NonBlocking, "idempotent rename", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let name = ctx.updated_service_name();
                async move {
                    client
                        .instance_update(
                            &org_id,
                            &service_id,
                            &ServicePatchRequest {
                                name: Some(name),
                                ..Default::default()
                            },
                        )
                        .await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "service update enable_core_dumps",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client.instance_get(&org_id, &service_id).await?;
                        let svc = resp.result.ok_or("service get returned no result")?;
                        let current_value = svc.enable_core_dumps;
                        client
                            .instance_update(
                                &org_id,
                                &service_id,
                                &ServicePatchRequest {
                                    enable_core_dumps: Some(current_value),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        Ok(())
                    }
                },
            )
            .await?;

        failures
            .run(&ctx, StepKind::NonBlocking, "add service tag", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                async move {
                    client
                        .instance_update(
                            &org_id,
                            &service_id,
                            &ServicePatchRequest {
                                tags: Some(InstanceTagsPatch {
                                    add: vec![ResourceTagsV1 {
                                        key: "phase".to_string(),
                                        value: Some("updated".to_string()),
                                    }],
                                    remove: vec![],
                                }),
                                ..Default::default()
                            },
                        )
                        .await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(&ctx, StepKind::NonBlocking, "service prometheus", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                async move {
                    let metrics = client
                        .instance_prometheus_get(&org_id, &service_id, None)
                        .await?;
                    if metrics.trim().is_empty() {
                        return Err("service prometheus returned empty output".into());
                    }
                    Ok(())
                }
            })
            .await?;

        // ── 4. IP Access ─────────────────────────────────────────────

        log_phase("IP Access");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "add first ip allow entry",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.create_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        client
                            .instance_update(
                                &org_id,
                                &service_id,
                                &ServicePatchRequest {
                                    ip_access_list: Some(IpAccessListPatch {
                                        add: vec![IpAccessListEntry {
                                            source: primary_ip.to_string(),
                                            description: Some("test primary".to_string()),
                                        }],
                                        remove: vec![],
                                    }),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        poll_for_ip_presence(
                            &client,
                            &org_id,
                            &service_id,
                            primary_ip,
                            true,
                            timeout,
                            interval,
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
                "add second ip allow entry",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.create_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        client
                            .instance_update(
                                &org_id,
                                &service_id,
                                &ServicePatchRequest {
                                    ip_access_list: Some(IpAccessListPatch {
                                        add: vec![IpAccessListEntry {
                                            source: secondary_ip.to_string(),
                                            description: Some("test secondary".to_string()),
                                        }],
                                        remove: vec![],
                                    }),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        poll_until(
                            "multiple ip allow visibility",
                            timeout,
                            interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                let service_id = service_id.clone();
                                async move {
                                    let resp =
                                        client.instance_get(&org_id, &service_id).await?;
                                    let svc =
                                        resp.result.ok_or("service get returned no result")?;
                                    if has_ip_entry(&svc, primary_ip)
                                        && has_ip_entry(&svc, secondary_ip)
                                    {
                                        Ok(Some(()))
                                    } else {
                                        Ok(None)
                                    }
                                }
                            },
                        )
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
                "remove one ip allow entry while keeping another",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.create_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        client
                            .instance_update(
                                &org_id,
                                &service_id,
                                &ServicePatchRequest {
                                    ip_access_list: Some(IpAccessListPatch {
                                        add: vec![],
                                        remove: vec![IpAccessListEntry {
                                            source: primary_ip.to_string(),
                                            description: None,
                                        }],
                                    }),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        poll_until(
                            "partial ip allow removal",
                            timeout,
                            interval,
                            || {
                                let client = client.clone();
                                let org_id = org_id.clone();
                                let service_id = service_id.clone();
                                async move {
                                    let resp =
                                        client.instance_get(&org_id, &service_id).await?;
                                    let svc =
                                        resp.result.ok_or("service get returned no result")?;
                                    if !has_ip_entry(&svc, primary_ip)
                                        && has_ip_entry(&svc, secondary_ip)
                                    {
                                        Ok(Some(()))
                                    } else {
                                        Ok(None)
                                    }
                                }
                            },
                        )
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
                "remove remaining ip allow entry",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.create_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        client
                            .instance_update(
                                &org_id,
                                &service_id,
                                &ServicePatchRequest {
                                    ip_access_list: Some(IpAccessListPatch {
                                        add: vec![],
                                        remove: vec![IpAccessListEntry {
                                            source: secondary_ip.to_string(),
                                            description: None,
                                        }],
                                    }),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        poll_for_ip_presence(
                            &client,
                            &org_id,
                            &service_id,
                            secondary_ip,
                            false,
                            timeout,
                            interval,
                        )
                        .await
                    }
                },
            )
            .await?;

        // ── 5. Scaling ───────────────────────────────────────────────

        log_phase("Scaling");
        failures
            .run(&ctx, StepKind::Blocking, "scale out to 3 replicas", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.steady_state_timeout;
                let interval = ctx.poll_interval;
                async move {
                    scale_service_and_wait(
                        &client,
                        &org_id,
                        &service_id,
                        Some(base_memory_gb),
                        Some(base_memory_gb),
                        Some(scaled_replicas),
                        "replica scale out",
                        timeout,
                        interval,
                    )
                    .await
                }
            })
            .await?;

        failures
            .run(&ctx, StepKind::Blocking, "scale up to 16 GB", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.steady_state_timeout;
                let interval = ctx.poll_interval;
                async move {
                    scale_service_and_wait(
                        &client,
                        &org_id,
                        &service_id,
                        Some(scaled_memory_gb),
                        Some(scaled_memory_gb),
                        Some(scaled_replicas),
                        "vertical scale up",
                        timeout,
                        interval,
                    )
                    .await
                }
            })
            .await?;

        failures
            .run(&ctx, StepKind::Blocking, "scale down to 8 GB", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.steady_state_timeout;
                let interval = ctx.poll_interval;
                async move {
                    scale_service_and_wait(
                        &client,
                        &org_id,
                        &service_id,
                        Some(base_memory_gb),
                        Some(base_memory_gb),
                        Some(scaled_replicas),
                        "vertical scale down",
                        timeout,
                        interval,
                    )
                    .await
                }
            })
            .await?;

        failures
            .run(&ctx, StepKind::Blocking, "scale in to 1 replica", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.steady_state_timeout;
                let interval = ctx.poll_interval;
                async move {
                    scale_service_and_wait(
                        &client,
                        &org_id,
                        &service_id,
                        Some(base_memory_gb),
                        Some(base_memory_gb),
                        Some(base_replicas),
                        "replica scale in",
                        timeout,
                        interval,
                    )
                    .await
                }
            })
            .await?;

        // ── 6. Delete ────────────────────────────────────────────────

        log_phase("Delete");

        // Stop service before delete (library has no --force equivalent)
        failures
            .run(&ctx, StepKind::Blocking, "stop service before delete", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let timeout = ctx.create_timeout;
                let interval = ctx.poll_interval;
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
                    poll_until("service stopped for delete", timeout, interval, || {
                        let client = client.clone();
                        let org_id = org_id.clone();
                        let service_id = service_id.clone();
                        async move {
                            let resp = client.instance_get(&org_id, &service_id).await?;
                            let svc = resp.result.ok_or("service get returned no result")?;
                            let state = svc.state.to_string();
                            if matches!(state.as_str(), "idle" | "stopped") {
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

        failures
            .run(&ctx, StepKind::Blocking, "delete service", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                async move {
                    client.instance_delete(&org_id, &service_id).await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "confirm service is gone after delete",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.delete_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        poll_until("service deletion", timeout, interval, || {
                            let client = client.clone();
                            let org_id = org_id.clone();
                            let service_id = service_id.clone();
                            async move {
                                match client.instance_get(&org_id, &service_id).await {
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
        cleanup.unregister_service(&service_id);

        failures.finish()
    }
    .await;

    let cleanup_result = cleanup
        .cleanup(&client, &ctx.org_id, ctx.delete_timeout, ctx.poll_interval)
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

fn has_ip_entry(svc: &Service, source: &str) -> bool {
    svc.ip_access_list.iter().any(|e| e.source == source)
}

async fn poll_for_ip_presence(
    client: &Client,
    org_id: &str,
    service_id: &str,
    ip: &str,
    expected_present: bool,
    timeout: std::time::Duration,
    interval: std::time::Duration,
) -> TestResult<()> {
    poll_until(
        &format!("ip visibility for {ip}"),
        timeout,
        interval,
        || {
            let client = client.clone();
            let org_id = org_id.to_string();
            let service_id = service_id.to_string();
            let ip = ip.to_string();
            async move {
                let resp = client.instance_get(&org_id, &service_id).await?;
                let svc = resp.result.ok_or("service get returned no result")?;
                if has_ip_entry(&svc, &ip) == expected_present {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await?;
    Ok(())
}

async fn scale_service_and_wait(
    client: &Client,
    org_id: &str,
    service_id: &str,
    min_memory_gb: Option<f64>,
    max_memory_gb: Option<f64>,
    replicas: Option<f64>,
    description: &str,
    timeout: std::time::Duration,
    interval: std::time::Duration,
) -> TestResult<()> {
    client
        .instance_replica_scaling_update(
            org_id,
            service_id,
            &ServiceReplicaScalingPatchRequest {
                min_replica_memory_gb: min_memory_gb,
                max_replica_memory_gb: max_memory_gb,
                num_replicas: replicas,
                ..Default::default()
            },
        )
        .await?;

    poll_until(
        &format!("{description} visibility"),
        timeout,
        interval,
        || {
            let client = client.clone();
            let org_id = org_id.to_string();
            let service_id = service_id.to_string();
            async move {
                let resp = client.instance_get(&org_id, &service_id).await?;
                let svc = resp.result.ok_or("service get returned no result")?;
                if min_memory_gb.is_none_or(|v| svc.min_replica_memory_gb == v)
                    && max_memory_gb.is_none_or(|v| svc.max_replica_memory_gb == v)
                    && replicas.is_none_or(|v| svc.num_replicas == v)
                {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await?;

    Ok(())
}
