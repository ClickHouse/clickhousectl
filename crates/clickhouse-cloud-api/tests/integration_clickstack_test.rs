mod integration;

use clickhouse_cloud_api::models::*;
use integration::support::*;

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_clickstack_crud_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();

    let test_result = async {
        log_run_header("cloud_clickstack_crud_lifecycle", &ctx);
        let mut failures = FailureRecorder::default();

        // ── Provision service ────────────────────────────────────────

        log_phase("Provision Service");

        let create_body = ServicePostRequest {
            name: ctx.service_name(),
            provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
            region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
            min_replica_memory_gb: Some(8.0_f64),
            max_replica_memory_gb: Some(8.0_f64),
            num_replicas: Some(1.0_f64),
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

        let service_id = created.service.id.to_string();
        eprintln!("service_id: <redacted>");
        cleanup.register_service(service_id.clone());

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "wait for service steady state",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.steady_state_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        poll_until("service steady state", timeout, interval, || {
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
                },
            )
            .await?;

        // ── Sources & Webhooks (read-only) ──────────────────────────
        //
        // These are read-only views into ClickStack state. Treated as
        // NonBlocking so an unconfigured org doesn't block the CRUD
        // checks that follow.

        log_phase("Sources");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "list clickstack sources",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client
                            .click_stack_list_sources(&org_id, &service_id)
                            .await?;
                        let sources = resp.result.unwrap_or_default();
                        eprintln!("found {} clickstack source(s)", sources.len());
                        Ok(())
                    }
                },
            )
            .await?;

        log_phase("Webhooks");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "list clickstack webhooks",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client
                            .click_stack_list_webhooks(&org_id, &service_id)
                            .await?;
                        let webhooks = resp.result.unwrap_or_default();
                        eprintln!("found {} clickstack webhook(s)", webhooks.len());
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Dashboard CRUD ──────────────────────────────────────────

        log_phase("Dashboard CRUD");

        let initial_dashboard_name = format!("ctl-it-dash-{}", ctx.run_id);
        let renamed_dashboard_name = format!("{initial_dashboard_name}-renamed");

        let tile = ClickStackTileInput {
            h: 4,
            w: 4,
            x: 0,
            y: 0,
            name: "Welcome tile".to_string(),
            config: Some(ClickStackTileConfig::ClickStackMarkdownChartConfig(
                ClickStackMarkdownChartConfig {
                    display_type: ClickStackMarkdownChartConfigDisplaytype::Markdown,
                    markdown: Some("# integration test dashboard".to_string()),
                },
            )),
            ..Default::default()
        };

        let dashboard_create_body = ClickStackCreateDashboardRequest {
            name: initial_dashboard_name.clone(),
            tiles: vec![tile.clone()],
            tags: Some(vec!["clickhousectl-it".to_string()]),
            ..Default::default()
        };

        let dashboard = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "create clickstack dashboard",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let body = dashboard_create_body.clone();
                    async move {
                        let resp = client
                            .click_stack_create_dashboard(&org_id, &service_id, &body)
                            .await?;
                        resp.result
                            .ok_or_else(|| "dashboard create returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");

        assert_eq!(dashboard.name, initial_dashboard_name);
        assert!(
            !dashboard.id.is_empty(),
            "dashboard create returned empty id"
        );
        let dashboard_id = dashboard.id.clone();
        let first_tile_id = dashboard
            .tiles
            .first()
            .map(|t| t.id.clone())
            .filter(|id| !id.is_empty());

        failures
            .run(&ctx, StepKind::Blocking, "get clickstack dashboard", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let dashboard_id = dashboard_id.clone();
                let expected_name = initial_dashboard_name.clone();
                async move {
                    let resp = client
                        .click_stack_get_dashboard(&org_id, &service_id, &dashboard_id)
                        .await?;
                    let dash = resp.result.ok_or("dashboard get returned no result")?;
                    if dash.id != dashboard_id {
                        return Err(format!(
                            "dashboard get returned id {} but expected {dashboard_id}",
                            dash.id
                        )
                        .into());
                    }
                    if dash.name != expected_name {
                        return Err(format!(
                            "dashboard get returned name {} but expected {expected_name}",
                            dash.name
                        )
                        .into());
                    }
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "list dashboards includes created",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let dashboard_id = dashboard_id.clone();
                    async move {
                        let resp = client
                            .click_stack_list_dashboards(&org_id, &service_id)
                            .await?;
                        let list = resp.result.ok_or("dashboard list returned no result")?;
                        if !list.iter().any(|d| d.id == dashboard_id) {
                            return Err(
                                "created dashboard was not visible in dashboard list".into()
                            );
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        let update_body = ClickStackUpdateDashboardRequest {
            name: renamed_dashboard_name.clone(),
            tiles: vec![tile.clone()],
            tags: Some(vec!["clickhousectl-it".to_string()]),
            ..Default::default()
        };

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "update clickstack dashboard (rename)",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let dashboard_id = dashboard_id.clone();
                    let body = update_body.clone();
                    let expected_name = renamed_dashboard_name.clone();
                    async move {
                        let resp = client
                            .click_stack_update_dashboard(
                                &org_id,
                                &service_id,
                                &dashboard_id,
                                &body,
                            )
                            .await?;
                        let updated = resp.result.ok_or("dashboard update returned no result")?;
                        if updated.name != expected_name {
                            return Err(format!(
                                "dashboard update returned name {} but expected {expected_name}",
                                updated.name
                            )
                            .into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Alert CRUD ──────────────────────────────────────────────

        log_phase("Alert CRUD");

        let alert_name = format!("ctl-it-alert-{}", ctx.run_id);
        let alert_create_body = ClickStackCreateAlertRequest {
            name: Some(alert_name.clone()),
            threshold: 1.0,
            threshold_type: ClickStackCreateAlertRequestThresholdtype::Above,
            interval: ClickStackCreateAlertRequestInterval::_5m,
            source: ClickStackCreateAlertRequestSource::Tile,
            dashboard_id: Some(dashboard_id.clone()),
            tile_id: first_tile_id.clone(),
            channel: ClickStackAlertChannel::ClickStackAlertChannelEmail(
                ClickStackAlertChannelEmail {
                    email_recipients: vec!["clickhousectl-it@example.com".to_string()],
                    r#type: ClickStackAlertChannelEmailType::Email,
                },
            ),
            ..Default::default()
        };

        let alert = failures
            .run(&ctx, StepKind::Blocking, "create clickstack alert", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let body = alert_create_body.clone();
                async move {
                    let resp = client
                        .click_stack_create_alert(&org_id, &service_id, &body)
                        .await?;
                    resp.result
                        .ok_or_else(|| "alert create returned no result".into())
                }
            })
            .await?
            .expect("blocking steps always return a value");

        assert!(!alert.id.is_empty(), "alert create returned empty id");
        let alert_id = alert.id.clone();

        failures
            .run(&ctx, StepKind::Blocking, "get clickstack alert", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let alert_id = alert_id.clone();
                async move {
                    let resp = client
                        .click_stack_get_alert(&org_id, &service_id, &alert_id)
                        .await?;
                    let got = resp.result.ok_or("alert get returned no result")?;
                    if got.id != alert_id {
                        return Err(format!(
                            "alert get returned id {} but expected {alert_id}",
                            got.id
                        )
                        .into());
                    }
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "list alerts includes created",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let alert_id = alert_id.clone();
                    async move {
                        let resp = client.click_stack_list_alerts(&org_id, &service_id).await?;
                        let list = resp.result.ok_or("alert list returned no result")?;
                        if !list.iter().any(|a| a.id == alert_id) {
                            return Err("created alert was not visible in alert list".into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        let alert_update_body = ClickStackUpdateAlertRequest {
            name: Some(alert_name.clone()),
            threshold: 5.0,
            threshold_type: ClickStackUpdateAlertRequestThresholdtype::Above,
            interval: ClickStackUpdateAlertRequestInterval::_5m,
            source: ClickStackUpdateAlertRequestSource::Tile,
            dashboard_id: Some(dashboard_id.clone()),
            tile_id: first_tile_id.clone(),
            channel: ClickStackAlertChannel::ClickStackAlertChannelEmail(
                ClickStackAlertChannelEmail {
                    email_recipients: vec!["clickhousectl-it@example.com".to_string()],
                    r#type: ClickStackAlertChannelEmailType::Email,
                },
            ),
            ..Default::default()
        };

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "update clickstack alert (raise threshold)",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let alert_id = alert_id.clone();
                    let body = alert_update_body.clone();
                    async move {
                        let resp = client
                            .click_stack_update_alert(&org_id, &service_id, &alert_id, &body)
                            .await?;
                        let updated = resp.result.ok_or("alert update returned no result")?;
                        if updated.threshold != 5.0 {
                            return Err(format!(
                                "alert update returned threshold {} but expected 5.0",
                                updated.threshold
                            )
                            .into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        failures
            .run(&ctx, StepKind::Blocking, "delete clickstack alert", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                let alert_id = alert_id.clone();
                async move {
                    client
                        .click_stack_delete_alert(&org_id, &service_id, &alert_id)
                        .await?;
                    Ok(())
                }
            })
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "delete clickstack dashboard",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let dashboard_id = dashboard_id.clone();
                    async move {
                        client
                            .click_stack_delete_dashboard(&org_id, &service_id, &dashboard_id)
                            .await?;
                        Ok(())
                    }
                },
            )
            .await?;

        // ── Teardown service ────────────────────────────────────────

        log_phase("Delete");
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "stop service before delete",
                || {
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
                },
            )
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
