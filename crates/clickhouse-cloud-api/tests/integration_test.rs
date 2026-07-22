mod common;

use clickhouse_cloud_api::Client;
use clickhouse_cloud_api::models::*;
use common::support::*;

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
        // The deprecated `instance_scaling_update` endpoint validates
        // `minTotalMemoryGb`/`maxTotalMemoryGb` as multiples of 12, unlike
        // the modern `instance_replica_scaling_update` endpoint which
        // accepts multiples of 4. Use a dedicated pair of values for the
        // deprecated round-trip to satisfy that constraint.
        #[cfg(feature = "deprecated-fields")]
        let deprecated_base_total_memory_gb = 12.0_f64;
        #[cfg(feature = "deprecated-fields")]
        let deprecated_scaled_total_memory_gb = 24.0_f64;
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

        // ── 2. Query API Endpoint ────────────────────────────────────
        //
        // Exercise the path that `cloud service query` uses: create a
        // dedicated API key, bind it to the service's query endpoint with
        // role `sql_console_admin`, run `SELECT 1` over HTTP via
        // queries.clickhouse.cloud, and assert the result.

        log_phase("Query API Endpoint");

        let query_key = failures
            .run(&ctx, StepKind::Blocking, "create query API key", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let key_name = format!("{}-query", ctx.service_name());
                async move {
                    let body = ApiKeyPostRequest {
                        name: key_name,
                        assigned_role_ids: vec![],
                        expire_at: None,
                        hash_data: None,
                        ip_access_list: vec![IpAccessListEntry {
                            source: "0.0.0.0/0".to_string(),
                            description: Some(
                                "clickhousectl integration test query key".to_string(),
                            ),
                        }],
                        #[cfg(feature = "deprecated-fields")]
                        roles: None,
                        state: ApiKeyPostRequestState::Enabled,
                    };
                    let resp = client.openapi_key_create(&org_id, &body).await?;
                    resp.result
                        .ok_or_else(|| "api key create returned no result".into())
                }
            })
            .await?
            .expect("blocking steps always return a value");
        // `query_key.key_id` is the credential id used for HTTP auth on the
        // query endpoint. Management endpoints (GET/DELETE /keys/{id}) and the
        // endpoint binding's `openApiKeys` array reference the API key's
        // resource UUID instead — `query_key.key.id`.
        let api_key_uuid = query_key.key.id.to_string();
        cleanup.register_api_key(api_key_uuid.clone());

        // Before binding the key to a query endpoint, calling the Query API
        // must fail. We don't pin the exact status (the control plane can
        // return 401/403/404 here depending on which check trips first); we
        // just require a 4xx so the test catches the regression where the
        // endpoint silently works without a binding.
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "query before endpoint enabled fails",
                || {
                    let client = client.clone();
                    let service_id = service_id.clone();
                    let key_id = query_key.key_id.clone();
                    let key_secret = query_key.key_secret.clone();
                    async move {
                        match client
                            .run_query(
                                &service_id,
                                &key_id,
                                &key_secret,
                                "SELECT 1",
                                None,
                                "TabSeparated",
                                false,
                            )
                            .await
                        {
                            Ok(response) => {
                                let status = response.status();
                                let body = response.text().await.unwrap_or_default();
                                Err(format!(
                                    "expected 4xx before endpoint enabled, got {status}: {}",
                                    body.trim()
                                )
                                .into())
                            }
                            Err(clickhouse_cloud_api::Error::Api { status, message })
                                if (400..500).contains(&status) =>
                            {
                                eprintln!(
                                    "  query without endpoint correctly rejected: {status}: {message}"
                                );
                                Ok(())
                            }
                            Err(e) => Err(format!(
                                "expected 4xx before endpoint enabled, got unexpected error: {e}"
                            )
                            .into()),
                        }
                    }
                },
            )
            .await?;

        let initial_endpoint = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "upsert query endpoint with admin role",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let api_key_uuid = api_key_uuid.clone();
                    async move {
                        let body = InstanceServiceQueryApiEndpointsPostRequest {
                            roles: vec!["sql_console_admin".to_string()],
                            open_api_keys: vec![api_key_uuid],
                            allowed_origins: "*".to_string(),
                        };
                        let resp = client
                            .instance_query_endpoint_upsert(&org_id, &service_id, &body)
                            .await?;
                        resp.result
                            .ok_or_else(|| "query endpoint upsert returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        // Register the binding for cleanup BEFORE doing anything else with it.
        // The registry deletes query endpoints before API keys, so a panic
        // mid-phase still leaves the org tidy.
        cleanup.register_query_endpoint(service_id.clone());

        // GET the binding back and assert it matches the upsert. Catches
        // regressions where the control plane stores something different from
        // what we sent (roles silently demoted, openApiKeys dropped, etc).
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "get query endpoint matches upsert",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let api_key_uuid = api_key_uuid.clone();
                    let initial_endpoint = initial_endpoint.clone();
                    async move {
                        let resp = client
                            .instance_query_endpoint_get(&org_id, &service_id)
                            .await?;
                        let endpoint = resp
                            .result
                            .ok_or("query endpoint get returned no result")?;
                        if endpoint.id != initial_endpoint.id {
                            return Err(format!(
                                "get returned different endpoint id: {} (upsert) vs {} (get)",
                                initial_endpoint.id, endpoint.id
                            )
                            .into());
                        }
                        if !endpoint.roles.iter().any(|r| r == "sql_console_admin") {
                            return Err(format!(
                                "get missing sql_console_admin role: {:?}",
                                endpoint.roles
                            )
                            .into());
                        }
                        if !endpoint.open_api_keys.contains(&api_key_uuid) {
                            return Err(format!(
                                "get missing our key from openApiKeys: {:?}",
                                endpoint.open_api_keys
                            )
                            .into());
                        }
                        if endpoint.allowed_origins != "*" {
                            return Err(format!(
                                "get returned unexpected allowedOrigins: {:?}",
                                endpoint.allowed_origins
                            )
                            .into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        // Endpoint propagation can lag a few seconds behind the upsert; poll
        // for the first successful query rather than asserting on the first
        // try.
        failures
            .run(&ctx, StepKind::Blocking, "run SELECT 1 via Query API", || {
                let client = client.clone();
                let service_id = service_id.clone();
                let key_id = query_key.key_id.clone();
                let key_secret = query_key.key_secret.clone();
                async move {
                    poll_until(
                        "query API SELECT 1",
                        std::time::Duration::from_secs(120),
                        std::time::Duration::from_secs(5),
                        || {
                            let client = client.clone();
                            let service_id = service_id.clone();
                            let key_id = key_id.clone();
                            let key_secret = key_secret.clone();
                            async move {
                                match client
                                    .run_query(
                                        &service_id,
                                        &key_id,
                                        &key_secret,
                                        "SELECT 1",
                                        None,
                                        "TabSeparated",
                                        false,
                                    )
                                    .await
                                {
                                    Ok(response) => {
                                        let body = response.text().await.map_err(|e| {
                                            format!("query response read failed: {e}")
                                        })?;
                                        let trimmed = body.trim();
                                        if trimmed == "1" {
                                            Ok(Some(()))
                                        } else {
                                            Err(format!(
                                                "unexpected query response: {trimmed:?}"
                                            )
                                            .into())
                                        }
                                    }
                                    Err(clickhouse_cloud_api::Error::Api {
                                        status, message,
                                    }) if status == 401 || status == 403 || status == 404 => {
                                        // Propagation delay — keep polling.
                                        eprintln!(
                                            "  query endpoint not ready yet ({status}): {message}"
                                        );
                                        Ok(None)
                                    }
                                    Err(e) => Err(e.into()),
                                }
                            }
                        },
                    )
                    .await
                }
            })
            .await?;

        // The query endpoint binding uses `sql_console_admin`, so the key
        // must be able to write — `cloud service query` is the canonical
        // path for INSERTs and DDL, not just SELECT. Walk through
        // CREATE TABLE / INSERT / SELECT to catch regressions where the
        // role on the binding is silently demoted to read-only.
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "CREATE TABLE + INSERT + SELECT via Query API",
                || {
                    let client = client.clone();
                    let service_id = service_id.clone();
                    let key_id = query_key.key_id.clone();
                    let key_secret = query_key.key_secret.clone();
                    async move {
                        async fn exec(
                            client: &clickhouse_cloud_api::Client,
                            service_id: &str,
                            key_id: &str,
                            key_secret: &str,
                            sql: &str,
                        ) -> Result<String, Box<dyn std::error::Error>> {
                            let response = client
                                .run_query(
                                    service_id,
                                    key_id,
                                    key_secret,
                                    sql,
                                    None,
                                    "TabSeparated",
                                    false,
                                )
                                .await?;
                            response
                                .text()
                                .await
                                .map_err(|e| format!("query response read failed: {e}").into())
                        }

                        exec(
                            &client,
                            &service_id,
                            &key_id,
                            &key_secret,
                            "CREATE TABLE clickhousectl_it_write (x UInt32) ENGINE = MergeTree ORDER BY x",
                        )
                        .await
                        .map_err(|e| -> Box<dyn std::error::Error> {
                            format!("CREATE TABLE failed (role may not grant writes): {e}").into()
                        })?;
                        exec(
                            &client,
                            &service_id,
                            &key_id,
                            &key_secret,
                            "INSERT INTO clickhousectl_it_write VALUES (1), (2), (3)",
                        )
                        .await
                        .map_err(|e| -> Box<dyn std::error::Error> {
                            format!("INSERT failed: {e}").into()
                        })?;
                        let body = exec(
                            &client,
                            &service_id,
                            &key_id,
                            &key_secret,
                            "SELECT sum(x) FROM clickhousectl_it_write",
                        )
                        .await?;
                        let trimmed = body.trim();
                        if trimmed != "6" {
                            return Err(format!(
                                "unexpected sum after INSERT: got {trimmed:?}, expected \"6\""
                            )
                            .into());
                        }
                        // Tidy up: the service is about to be deleted anyway,
                        // but leaving artifacts behind makes debugging harder
                        // if cleanup ever short-circuits.
                        exec(
                            &client,
                            &service_id,
                            &key_id,
                            &key_secret,
                            "DROP TABLE clickhousectl_it_write",
                        )
                        .await?;
                        Ok(())
                    }
                },
            )
            .await?;

        // Re-upserting the same endpoint must be idempotent: the resource id
        // should not change, and the existing credentials should keep working.
        // Catches regressions where the control plane rotates the binding or
        // strips `openApiKeys` on a no-op write.
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "re-upsert query endpoint is idempotent",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let api_key_uuid = api_key_uuid.clone();
                    let initial_endpoint = initial_endpoint.clone();
                    async move {
                        let body = InstanceServiceQueryApiEndpointsPostRequest {
                            roles: vec!["sql_console_admin".to_string()],
                            open_api_keys: vec![api_key_uuid.clone()],
                            allowed_origins: "*".to_string(),
                        };
                        let resp = client
                            .instance_query_endpoint_upsert(&org_id, &service_id, &body)
                            .await?;
                        let endpoint = resp
                            .result
                            .ok_or("re-upsert returned no result")?;
                        if endpoint.id != initial_endpoint.id {
                            return Err(format!(
                                "re-upsert changed endpoint id: {} -> {}",
                                initial_endpoint.id, endpoint.id
                            )
                            .into());
                        }
                        if !endpoint.open_api_keys.contains(&api_key_uuid) {
                            return Err(format!(
                                "re-upsert dropped our key from openApiKeys: {:?}",
                                endpoint.open_api_keys
                            )
                            .into());
                        }
                        if !endpoint.roles.iter().any(|r| r == "sql_console_admin") {
                            return Err(format!(
                                "re-upsert dropped sql_console_admin role: {:?}",
                                endpoint.roles
                            )
                            .into());
                        }
                        Ok(())
                    }
                },
            )
            .await?;

        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "SELECT 1 still works after re-upsert",
                || {
                    let client = client.clone();
                    let service_id = service_id.clone();
                    let key_id = query_key.key_id.clone();
                    let key_secret = query_key.key_secret.clone();
                    async move {
                        let response = client
                            .run_query(
                                &service_id,
                                &key_id,
                                &key_secret,
                                "SELECT 1",
                                None,
                                "TabSeparated",
                                false,
                            )
                            .await?;
                        let body = response
                            .text()
                            .await
                            .map_err(|e| format!("query response read failed: {e}"))?;
                        let trimmed = body.trim();
                        if trimmed == "1" {
                            Ok(())
                        } else {
                            Err(format!(
                                "unexpected query response after re-upsert: {trimmed:?}"
                            )
                            .into())
                        }
                    }
                },
            )
            .await?;

        // Delete the binding BEFORE the API key so we can distinguish
        // "binding cleanup works" from "API key was already gone."
        failures
            .run(&ctx, StepKind::NonBlocking, "delete query endpoint", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let service_id = service_id.clone();
                async move {
                    client
                        .instance_query_endpoint_delete(&org_id, &service_id)
                        .await?;
                    Ok(())
                }
            })
            .await?;
        cleanup.unregister_query_endpoint(&service_id);

        // GET must now report the binding is gone. The control plane returns
        // 404 once the binding is deleted; accept any 4xx in case the exact
        // status drifts, but require an error rather than a stale 200.
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "get query endpoint after delete returns 4xx",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        match client
                            .instance_query_endpoint_get(&org_id, &service_id)
                            .await
                        {
                            Ok(resp) => {
                                Err(format!(
                                    "expected 4xx after delete, got 200 with result: {:?}",
                                    resp.result
                                )
                                .into())
                            }
                            Err(clickhouse_cloud_api::Error::Api { status, message })
                                if (400..500).contains(&status) =>
                            {
                                eprintln!(
                                    "  query endpoint correctly absent after delete: {status}: {message}"
                                );
                                Ok(())
                            }
                            Err(e) => Err(format!(
                                "expected 4xx after delete, got unexpected error: {e}"
                            )
                            .into()),
                        }
                    }
                },
            )
            .await?;

        failures
            .run(&ctx, StepKind::Blocking, "delete query API key", || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let api_key_uuid = api_key_uuid.clone();
                async move {
                    client.openapi_key_delete(&org_id, &api_key_uuid).await?;
                    Ok(())
                }
            })
            .await?;
        cleanup.unregister_api_key(&api_key_uuid);

        // ── 3. Stop / Start ──────────────────────────────────────────

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

        // ── 4. Rename & Settings ─────────────────────────────────────

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

        // ── 5. ClickHouse Settings ───────────────────────────────────
        //
        // Round-trip a service-level ClickHouse setting through the four
        // settings endpoints (`schema`, `list`, `update`, `get`) and capture
        // the original value so cleanup can restore it.
        //
        // The schema endpoint does not expose a "restartRequired" flag, so we
        // pick from a curated allowlist of well-known runtime-changeable
        // settings (no restart required). We intersect the allowlist with the
        // schema returned at test time, so we only touch a setting the cloud
        // control plane currently advertises as configurable. If none of the
        // allowlisted settings appear in the schema this phase records a
        // non-blocking failure rather than guessing at an unknown setting.

        log_phase("ClickHouse Settings");

        let settings_schema = failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "clickhouse settings schema get",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client
                            .service_clickhouse_settings_schema_get(&org_id, &service_id)
                            .await?;
                        let schema = resp
                            .result
                            .ok_or("clickhouse settings schema returned no result")?;
                        if schema.settings.is_empty() {
                            return Err("clickhouse settings schema returned no entries".into());
                        }
                        Ok(schema)
                    }
                },
            )
            .await?;

        if let Some(schema) = settings_schema {
            // Curated allowlist of ClickHouse settings that are safe to mutate
            // on a running service and do not require a restart. All of these
            // are per-query / per-user runtime knobs — changing them does not
            // bounce the server. Ordered by preference; first match wins.
            //
            // Hardcoded because the schema endpoint does not carry a
            // restart-required marker today; using a curated list is the
            // conservative alternative to picking blindly. The cloud schema
            // exposes a curated subset of OSS settings, so we list multiple
            // alternatives — if the cloud control plane stops exposing one,
            // the next match is used.
            const NO_RESTART_CANDIDATES: &[&str] = &[
                "max_concurrent_queries_for_user",
                "max_threads",
                "max_memory_usage_for_user",
                "min_insert_block_size_rows",
                "min_insert_block_size_bytes",
                "max_insert_block_size",
                "max_partitions_per_insert_block",
                "max_block_size",
                "max_concurrent_queries",
                "max_concurrent_select_queries",
                "max_concurrent_insert_queries",
                "max_execution_time",
                "max_result_rows",
            ];

            let chosen = NO_RESTART_CANDIDATES.iter().find_map(|name| {
                schema
                    .settings
                    .iter()
                    .find(|entry| entry.name == *name)
                    .cloned()
            });

            if let Some(entry) = chosen {
                let setting_name = entry.name.clone();
                eprintln!("  chose setting: {setting_name} (type: {})", entry.r#type);

                let list_resp = failures
                    .run(
                        &ctx,
                        StepKind::NonBlocking,
                        "clickhouse settings list get",
                        || {
                            let client = client.clone();
                            let org_id = ctx.org_id.clone();
                            let service_id = service_id.clone();
                            async move {
                                let resp = client
                                    .service_clickhouse_settings_list_get(
                                        &org_id,
                                        &service_id,
                                    )
                                    .await?;
                                let list = resp.result.ok_or(
                                    "clickhouse settings list returned no result",
                                )?;
                                if list.settings.is_empty() {
                                    return Err(
                                        "clickhouse settings list returned no entries".into(),
                                    );
                                }
                                Ok(list)
                            }
                        },
                    )
                    .await?;

                let original_value = list_resp.as_ref().and_then(|list| {
                    list.settings
                        .iter()
                        .find(|s| s.name == setting_name)
                        .map(|s| s.value.clone())
                });

                if let Some(original) = original_value {
                    eprintln!("  current value: {original}");

                    // Pick a new numeric value that differs from the current
                    // one. The candidates are all integer-typed settings, so
                    // we parse the current value as an integer; if parsing
                    // fails we bail to the next pre-set safe value below.
                    let new_value = match original.parse::<u64>() {
                        Ok(0) => "1".to_string(),
                        Ok(n) => (n.saturating_add(1)).to_string(),
                        Err(_) => "1".to_string(),
                    };

                    // Register the restore BEFORE attempting the mutation so
                    // a failed mid-mutation still triggers a cleanup attempt.
                    cleanup.register_clickhouse_setting_restore(
                        service_id.clone(),
                        setting_name.clone(),
                        original.clone(),
                    );

                    // The `settings` field on the API is a JSON-encoded string
                    // (the spec example is "{\"compatibility\":\"24.8\"}"). Build
                    // it with serde_json so the inner JSON escapes correctly
                    // regardless of what the setting name/value look like.
                    let patch_body_settings = serde_json::to_string(
                        &serde_json::json!({ setting_name.clone(): new_value.clone() }),
                    )?;
                    let update_ok = failures
                        .run(
                            &ctx,
                            StepKind::NonBlocking,
                            "clickhouse settings update",
                            || {
                                let client = client.clone();
                                let org_id = ctx.org_id.clone();
                                let service_id = service_id.clone();
                                let body = ServiceClickhouseSettingsPatchRequest {
                                    settings: Some(patch_body_settings.clone()),
                                };
                                async move {
                                    let resp = client
                                        .service_clickhouse_settings_update(
                                            &org_id,
                                            &service_id,
                                            &body,
                                        )
                                        .await?;
                                    if resp.result.is_none() {
                                        return Err(
                                            "clickhouse settings update returned no result"
                                                .into(),
                                        );
                                    }
                                    Ok(())
                                }
                            },
                        )
                        .await?;

                    if update_ok.is_some() {
                        failures
                            .run(
                                &ctx,
                                StepKind::NonBlocking,
                                "clickhouse setting get reflects update",
                                || {
                                    let client = client.clone();
                                    let org_id = ctx.org_id.clone();
                                    let service_id = service_id.clone();
                                    let setting_name = setting_name.clone();
                                    let expected = new_value.clone();
                                    let interval = ctx.poll_interval;
                                    async move {
                                        // The control plane may take a few
                                        // seconds to propagate the change to
                                        // the per-setting GET endpoint, so
                                        // poll briefly rather than asserting
                                        // on the first read.
                                        poll_until(
                                            "clickhouse setting reflects update",
                                            std::time::Duration::from_secs(60),
                                            interval,
                                            || {
                                                let client = client.clone();
                                                let org_id = org_id.clone();
                                                let service_id = service_id.clone();
                                                let setting_name = setting_name.clone();
                                                let expected = expected.clone();
                                                async move {
                                                    let resp = client
                                                        .service_clickhouse_setting_get(
                                                            &org_id,
                                                            &service_id,
                                                            &setting_name,
                                                        )
                                                        .await?;
                                                    let got = resp.result.ok_or(
                                                        "clickhouse setting get returned no result",
                                                    )?;
                                                    if got.value == expected {
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
                    }
                } else {
                    failures
                        .run(
                            &ctx,
                            StepKind::NonBlocking,
                            "clickhouse settings round-trip: capture original value",
                            || {
                                let setting_name = setting_name.clone();
                                async move {
                                    let err: Box<dyn std::error::Error> = format!(
                                        "setting {setting_name} not present in settings list — \
                                         cannot capture original value for round-trip"
                                    )
                                    .into();
                                    Err::<(), _>(err)
                                }
                            },
                        )
                        .await?;
                }
            } else {
                // The schema endpoint was reachable (proven by the prior
                // step) but none of the curated no-restart-required
                // candidates are exposed. Rather than recording a hard
                // failure that would abort the run in fail-fast mode, log
                // the schema's setting names so the allowlist can be
                // updated, and skip the mutation phase. The earlier
                // `clickhouse settings schema get` step still records
                // coverage of the schema endpoint.
                let exposed: Vec<&str> =
                    schema.settings.iter().map(|s| s.name.as_str()).collect();
                eprintln!(
                    "  SKIP clickhouse settings round-trip: none of {:?} matched the \
                     {} settings the cloud schema currently exposes: {:?}",
                    NO_RESTART_CANDIDATES,
                    exposed.len(),
                    exposed,
                );
            }
        }

        // ── 6. Private Endpoints ─────────────────────────────────────
        //
        // Cover `instance_private_endpoint_create` and
        // `instance_private_endpoint_config_get` against the live service.
        //
        // `instance_private_endpoint_config_get` should always succeed: it
        // returns the service-side endpoint service id + private DNS hostname
        // and does not need a real provider-side endpoint to exist.
        //
        // `instance_private_endpoint_create` requires a `ServicPrivateEndpointePostRequest`
        // whose `id` is a provider-specific identifier (AWS `vpce-…`, GCP
        // numeric PSC id, Azure GUID). The control plane validates that id
        // against the underlying provider, so a synthetic value with no real
        // cloud resource behind it is expected to be rejected with a 4xx.
        //
        // The assertion is therefore: the call must either succeed (in which
        // case we register inline cleanup, re-read config, and remove the
        // binding) OR fail with an unambiguous 4xx structured error. Any
        // other outcome — 5xx, network error, or a 200 with malformed payload
        // — is treated as a real failure recorded via FailureRecorder. This
        // is the "assertion-on-error" fallback called out in issue #160.

        log_phase("Private Endpoints");

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "private endpoint config get",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client
                            .instance_private_endpoint_config_get(&org_id, &service_id)
                            .await?;
                        let config = resp.result.ok_or(
                            "instance_private_endpoint_config_get returned no result",
                        )?;
                        if config.endpoint_service_id.is_empty() {
                            return Err(
                                "private endpoint config returned empty endpointServiceId"
                                    .into(),
                            );
                        }
                        if config.private_dns_hostname.is_empty() {
                            return Err(
                                "private endpoint config returned empty privateDnsHostname"
                                    .into(),
                            );
                        }
                        eprintln!(
                            "  private endpoint config: endpointServiceId len={} privateDnsHostname len={}",
                            config.endpoint_service_id.len(),
                            config.private_dns_hostname.len()
                        );
                        Ok(())
                    }
                },
            )
            .await?;

        // Build a provider-shaped but synthetic endpoint id. `ctx.run_id`
        // is embedded so concurrent test runs and post-mortems can trace
        // which run wrote which (rejected) id.
        let synthetic_endpoint_id = synthetic_private_endpoint_id(&ctx);
        let create_body = ServicPrivateEndpointePostRequest {
            id: synthetic_endpoint_id.clone(),
            description: format!("clickhousectl-it private endpoint {}", ctx.run_id),
        };

        // The endpoint we just attempted to attach (only set if create
        // succeeded). Hosts the inline cleanup that fires after the
        // re-read of `private_endpoint_config_get`.
        let mut created_endpoint_id: Option<String> = None;

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "private endpoint create (synthetic id)",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let create_body = create_body.clone();
                    let synthetic_endpoint_id = synthetic_endpoint_id.clone();
                    let created_endpoint_id = &mut created_endpoint_id;
                    async move {
                        match client
                            .instance_private_endpoint_create(
                                &org_id,
                                &service_id,
                                &create_body,
                            )
                            .await
                        {
                            Ok(resp) => {
                                let endpoint = resp.result.ok_or(
                                    "instance_private_endpoint_create returned no result",
                                )?;
                                if endpoint.id != synthetic_endpoint_id {
                                    return Err(format!(
                                        "private endpoint create returned unexpected id: \
                                         got {}, expected {}",
                                        endpoint.id, synthetic_endpoint_id
                                    )
                                    .into());
                                }
                                eprintln!(
                                    "  private endpoint create unexpectedly succeeded \
                                     (provider={}, region={}); registering inline cleanup",
                                    endpoint.cloud_provider, endpoint.region
                                );
                                *created_endpoint_id = Some(endpoint.id);
                                Ok(())
                            }
                            Err(clickhouse_cloud_api::Error::Api { status, message })
                                if (400..500).contains(&status) =>
                            {
                                // Expected path: the API rejected the
                                // synthetic id because no real cloud resource
                                // backs it. The structured error shape
                                // (status + message) is what we assert on.
                                eprintln!(
                                    "  private endpoint create correctly rejected \
                                     synthetic id with {status}: {message}"
                                );
                                if message.trim().is_empty() {
                                    return Err(format!(
                                        "private endpoint create returned {status} \
                                         with empty error body"
                                    )
                                    .into());
                                }
                                Ok(())
                            }
                            Err(e) => Err(format!(
                                "private endpoint create returned unexpected error \
                                 (expected 4xx or success): {e}"
                            )
                            .into()),
                        }
                    }
                },
            )
            .await?;

        // Re-read the config endpoint after the create attempt: even when
        // the create is rejected we want a second `config_get` call in the
        // mix so transient deserialization regressions on the GET path
        // surface.
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "private endpoint config get after create attempt",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client
                            .instance_private_endpoint_config_get(&org_id, &service_id)
                            .await?;
                        resp.result
                            .ok_or("private endpoint config get returned no result")?;
                        Ok(())
                    }
                },
            )
            .await?;

        // If create succeeded, the synthetic endpoint is now associated with
        // the service via `privateEndpointIds`. Detach it inline so the
        // service can later be deleted cleanly — the service delete path
        // does not implicitly free a private endpoint binding.
        if let Some(endpoint_id) = created_endpoint_id.clone() {
            failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "detach synthetic private endpoint from service",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let service_id = service_id.clone();
                        let endpoint_id = endpoint_id.clone();
                        async move {
                            client
                                .instance_update(
                                    &org_id,
                                    &service_id,
                                    &ServicePatchRequest {
                                        private_endpoint_ids: Some(
                                            InstancePrivateEndpointsPatch {
                                                add: vec![],
                                                remove: vec![endpoint_id],
                                            },
                                        ),
                                        ..Default::default()
                                    },
                                )
                                .await?;
                            Ok(())
                        }
                    },
                )
                .await?;
        }

        // ── 7. IP Access ─────────────────────────────────────────────

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

        // ── 8. Scaling ───────────────────────────────────────────────

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

        // Vertical scaling round-trip via the deprecated
        // `instance_scaling_update` endpoint (PATCH /scaling). This is
        // distinct from `instance_replica_scaling_update` (PATCH
        // /replicaScaling) exercised above: the deprecated endpoint takes
        // `minTotalMemoryGb` / `maxTotalMemoryGb` and only the vertical
        // axis. The deprecated endpoint additionally requires the totals to
        // be multiples of 12, so we first move via the modern endpoint to
        // `deprecated_base_total_memory_gb` before the round-trip. We stay
        // at 1 replica so the total-memory body maps directly to
        // per-replica memory.
        //
        // The whole deprecated vertical-scaling phase is gated on the
        // `deprecated-fields` feature: the post-scale verification reads
        // `min/max_total_memory_gb`, which only exist with the feature on.
        #[cfg(feature = "deprecated-fields")]
        {
        failures
            .run(
                &ctx,
                StepKind::Blocking,
                "land on multiple-of-12 memory before deprecated phase",
                || {
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
                            Some(deprecated_base_total_memory_gb),
                            Some(deprecated_base_total_memory_gb),
                            Some(base_replicas),
                            "modern scale to deprecated base",
                            timeout,
                            interval,
                        )
                        .await
                    }
                },
            )
            .await?;

        let pre_vertical = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "capture pre-state for deprecated vertical scaling",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        let resp = client.instance_get(&org_id, &service_id).await?;
                        resp.result
                            .ok_or_else(|| "service get returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        // Sanity: the deprecated body's totals only equal per-replica when
        // num_replicas == 1. We rely on the previous step landing us there.
        assert_eq!(pre_vertical.num_replicas, base_replicas);
        let pre_min_total = pre_vertical.min_total_memory_gb;
        let pre_max_total = pre_vertical.max_total_memory_gb;

        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "deprecated vertical scale up to 24 GB",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.steady_state_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        scale_service_vertical_and_wait(
                            &client,
                            &org_id,
                            &service_id,
                            Some(deprecated_scaled_total_memory_gb),
                            Some(deprecated_scaled_total_memory_gb),
                            "deprecated vertical scale up",
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
                "deprecated vertical scale back to pre-state",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let timeout = ctx.steady_state_timeout;
                    let interval = ctx.poll_interval;
                    async move {
                        scale_service_vertical_and_wait(
                            &client,
                            &org_id,
                            &service_id,
                            Some(pre_min_total),
                            Some(pre_max_total),
                            "deprecated vertical scale restore",
                            timeout,
                            interval,
                        )
                        .await
                    }
                },
            )
            .await?;
        }

        // ── 9. Scaling Schedule (Beta) ───────────────────────────────
        //
        // Exercise the Beta scaling_schedule_{get,upsert,delete} trio
        // for shape coverage. Schedule entries are chosen to be inert:
        //
        //  - replica counts and memory match the current service state
        //    (1 replica, 8 GB), so even if an entry happens to be active
        //    during the test run it cannot drive any real scaling action;
        //  - the upsert window covers a single hour (1 a.m. – 2 a.m. UTC)
        //    on Sunday only, with the same inert replica config so the
        //    entry's effect is always a no-op regardless of when the
        //    suite runs.
        //
        // The pre-state (typically an empty schedule) is captured here
        // and restored as a cleanup step, not a test-body step.

        log_phase("Scaling Schedule");

        let pre_schedule = failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "scaling_schedule get pre-state",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        // A freshly-created service has no autoscaling schedule
                        // configured, and the API responds with 404 rather than
                        // an empty `ScalingSchedule`. Treat that as the canonical
                        // empty pre-state so the round-trip can still exercise
                        // upsert/replace; any other error still surfaces.
                        match client.scaling_schedule_get(&org_id, &service_id).await {
                            Ok(resp) => resp
                                .result
                                .ok_or_else(|| "scaling_schedule get returned no result".into()),
                            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => {
                                Ok(ScalingSchedule::default())
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                },
            )
            .await?;

        // Only run the round-trip if we successfully captured the
        // pre-state. If the initial GET failed, restoring afterwards
        // would risk leaving a synthetic schedule on the service.
        if let Some(pre_state) = pre_schedule {
            // Skip restore registration when pre-state is empty: the API
            // rejects upserts with an empty `entries` array, and there is
            // nothing meaningful to restore. Cleanup of synthetic entries
            // is still covered by the service-delete teardown below.
            if !pre_state.entries.is_empty() {
                cleanup
                    .register_scaling_schedule_restore(service_id.clone(), pre_state.clone());
            }
            eprintln!(
                "  captured scaling_schedule pre-state: {} entries",
                pre_state.entries.len()
            );

            // 9a. Upsert a synthetic-but-inert schedule.
            let upsert_entry = ScalingScheduleEntryRequest {
                name: "clickhousectl-it-upsert-window".to_string(),
                weekdays: vec![0], // Sunday only
                start_hour_utc: 1,
                end_hour_utc: 2,
                min_replica_memory_gb: Some(base_memory_gb),
                max_replica_memory_gb: Some(base_memory_gb),
                min_replicas: Some(base_replicas as i64),
                max_replicas: Some(base_replicas as i64),
                idle_scaling: Some(true),
                idle_timeout_minutes: Some(5),
                // Vertical entry expressed as equal min/max; the fixed-count
                // and horizontal-autoscaling forms need org-level enablement.
                num_replicas: None,
                ..Default::default()
            };

            let upserted = failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "scaling_schedule upsert inert window",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let service_id = service_id.clone();
                        let entry = upsert_entry.clone();
                        async move {
                            let body = ScalingSchedulePostRequest {
                                entries: vec![entry],
                            };
                            let resp = client
                                .scaling_schedule_upsert(&org_id, &service_id, &body)
                                .await?;
                            resp.result.ok_or_else(|| {
                                "scaling_schedule upsert returned no result".into()
                            })
                        }
                    },
                )
                .await?;

            // 9b. GET and confirm the upsert is visible.
            if upserted.is_some() {
                failures
                    .run(
                        &ctx,
                        StepKind::NonBlocking,
                        "scaling_schedule get reflects upsert",
                        || {
                            let client = client.clone();
                            let org_id = ctx.org_id.clone();
                            let service_id = service_id.clone();
                            let expected_name = upsert_entry.name.clone();
                            async move {
                                let resp = client
                                    .scaling_schedule_get(&org_id, &service_id)
                                    .await?;
                                let schedule = resp.result.ok_or(
                                    "scaling_schedule get returned no result after upsert",
                                )?;
                                if schedule.entries.len() != 1 {
                                    return Err(format!(
                                        "expected 1 entry after upsert, got {}",
                                        schedule.entries.len()
                                    )
                                    .into());
                                }
                                let entry = &schedule.entries[0];
                                if entry.name != expected_name {
                                    return Err(format!(
                                        "upserted entry name mismatch: got {:?}, expected {:?}",
                                        entry.name, expected_name
                                    )
                                    .into());
                                }
                                if entry.start_hour_utc != 1 || entry.end_hour_utc != 2 {
                                    return Err(format!(
                                        "upserted entry window mismatch: got {}-{} UTC, expected 1-2",
                                        entry.start_hour_utc, entry.end_hour_utc
                                    )
                                    .into());
                                }
                                Ok(())
                            }
                        },
                    )
                    .await?;
            }

            // 9c. Delete the schedule.
            let deleted = failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "scaling_schedule delete",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let service_id = service_id.clone();
                        async move {
                            client
                                .scaling_schedule_delete(&org_id, &service_id)
                                .await?;
                            Ok(())
                        }
                    },
                )
                .await?;

            // 9d. GET should now return 404 (no schedule configured).
            if deleted.is_some() {
                failures
                    .run(
                        &ctx,
                        StepKind::NonBlocking,
                        "scaling_schedule get returns 404 after delete",
                        || {
                            let client = client.clone();
                            let org_id = ctx.org_id.clone();
                            let service_id = service_id.clone();
                            async move {
                                match client
                                    .scaling_schedule_get(&org_id, &service_id)
                                    .await
                                {
                                    Err(clickhouse_cloud_api::Error::Api {
                                        status: 404,
                                        ..
                                    }) => Ok(()),
                                    Ok(_) => Err(
                                        "scaling_schedule get returned a schedule after delete"
                                            .into(),
                                    ),
                                    Err(e) => Err(e.into()),
                                }
                            }
                        },
                    )
                    .await?;
            }
        }

        // ── 10. Upgrade Window ───────────────────────────────────────
        //
        // The upgrade window controls when ClickHouse Cloud is allowed to
        // upgrade the service. A freshly-created service typically has no
        // window configured (GET returns 404). We exercise the full
        // GET/PUT/GET/DELETE/GET round-trip, with `weekday: 0`,
        // `startHourUtc: 0` (Sunday 00:00 UTC — the spec restricts
        // `startHourUtc` to {0, 6, 12, 18} and `weekday` to 0..=6).
        //
        // If the service did already have a window, we snapshot it for
        // restore on cleanup so the post-test state equals the pre-test
        // state on the off chance that the service survives teardown.

        log_phase("Upgrade Window");

        let pre_window = failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "upgrade_window get pre-state",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    async move {
                        match client.upgrade_window_get(&org_id, &service_id).await {
                            Ok(resp) => resp
                                .result
                                .map(Some)
                                .ok_or_else(|| "upgrade_window get returned no result".into()),
                            // Freshly-provisioned services usually 404; treat
                            // that as the canonical empty pre-state.
                            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => {
                                Ok(None)
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                },
            )
            .await?;

        if let Some(pre_state) = pre_window {
            // Only register a restore if the service had a real pre-existing
            // window. If the pre-state was 404, the DELETE step in this phase
            // returns the service to its original empty state on its own.
            if let Some(window) = pre_state {
                eprintln!(
                    "  captured upgrade_window pre-state: weekday={}, startHourUtc={}",
                    window.weekday, window.start_hour_utc,
                );
                cleanup.register_upgrade_window_restore(service_id.clone(), window);
            } else {
                eprintln!("  captured upgrade_window pre-state: (none)");
            }

            // 10a. PUT a known-valid window.
            let put_body = UpgradeWindowPutRequest {
                weekday: 0,
                start_hour_utc: 0,
            };
            let put_window = failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "upgrade_window put",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let service_id = service_id.clone();
                        let body = put_body.clone();
                        async move {
                            let resp = client
                                .upgrade_window_update(&org_id, &service_id, &body)
                                .await?;
                            resp.result
                                .ok_or_else(|| "upgrade_window put returned no result".into())
                        }
                    },
                )
                .await?;

            // 10b. GET reflects upsert.
            if put_window.is_some() {
                failures
                    .run(
                        &ctx,
                        StepKind::NonBlocking,
                        "upgrade_window get reflects upsert",
                        || {
                            let client = client.clone();
                            let org_id = ctx.org_id.clone();
                            let service_id = service_id.clone();
                            let expected_weekday = put_body.weekday;
                            let expected_start = put_body.start_hour_utc;
                            async move {
                                let resp = client
                                    .upgrade_window_get(&org_id, &service_id)
                                    .await?;
                                let window = resp
                                    .result
                                    .ok_or("upgrade_window get returned no result")?;
                                if window.weekday != expected_weekday
                                    || window.start_hour_utc != expected_start
                                {
                                    return Err(format!(
                                        "upgrade_window get mismatch: expected weekday={expected_weekday} startHourUtc={expected_start}, got weekday={got_w} startHourUtc={got_h}",
                                        got_w = window.weekday,
                                        got_h = window.start_hour_utc,
                                    )
                                    .into());
                                }
                                Ok(())
                            }
                        },
                    )
                    .await?;
            }

            // 10c. DELETE.
            let deleted = failures
                .run(
                    &ctx,
                    StepKind::NonBlocking,
                    "upgrade_window delete",
                    || {
                        let client = client.clone();
                        let org_id = ctx.org_id.clone();
                        let service_id = service_id.clone();
                        async move {
                            client.upgrade_window_delete(&org_id, &service_id).await?;
                            Ok(())
                        }
                    },
                )
                .await?;

            // 10d. GET should now return 404 (no window configured).
            if deleted.is_some() {
                failures
                    .run(
                        &ctx,
                        StepKind::NonBlocking,
                        "upgrade_window get returns 404 after delete",
                        || {
                            let client = client.clone();
                            let org_id = ctx.org_id.clone();
                            let service_id = service_id.clone();
                            async move {
                                match client
                                    .upgrade_window_get(&org_id, &service_id)
                                    .await
                                {
                                    Err(clickhouse_cloud_api::Error::Api {
                                        status: 404,
                                        ..
                                    }) => Ok(()),
                                    Ok(_) => Err(
                                        "upgrade_window get returned a window after delete"
                                            .into(),
                                    ),
                                    Err(e) => Err(e.into()),
                                }
                            }
                        },
                    )
                    .await?;
            }
        }

        // ── 11. Password ─────────────────────────────────────────────
        //
        // `instance_password_update` rotates the service password. The
        // query path used by the rest of the suite is openapi-key-based,
        // so the rotated password is not consumed anywhere — the pass
        // condition is just a successful response that surfaces a fresh
        // password. We pass an empty body so the server generates a new
        // password and returns it; no re-rotation is needed because the
        // service is about to be deleted.

        log_phase("Password");
        failures
            .run(
                &ctx,
                StepKind::NonBlocking,
                "rotate service password",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let service_id = service_id.clone();
                    let run_id = ctx.run_id.clone();
                    async move {
                        let resp = client
                            .instance_password_update(
                                &org_id,
                                &service_id,
                                &ServicePasswordPatchRequest::default(),
                            )
                            .await?;
                        let result = resp
                            .result
                            .ok_or("password update returned no result")?;
                        if result.password.is_empty() {
                            return Err("password update response had empty password".into());
                        }
                        eprintln!(
                            "  password rotated (length={}, run_id={})",
                            result.password.len(),
                            run_id
                        );
                        Ok(())
                    }
                },
            )
            .await?;

        // ── 12. Delete ───────────────────────────────────────────────

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
        cleanup.unregister_scaling_schedule_restore(&service_id);
        cleanup.unregister_service(&service_id);

        failures.finish()
    }
    .await;

    let cleanup_result = cleanup
        .cleanup(
            &client,
            &ctx.org_id,
            ctx.delete_timeout,
            ctx.poll_interval,
            None,
        )
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

#[allow(clippy::too_many_arguments)]
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

/// Builds a provider-shaped but synthetic private endpoint id that embeds
/// `ctx.run_id`. The format is plausible enough for the control plane to
/// accept syntactically but the underlying cloud resource does not exist,
/// so the create call is expected to be rejected with a 4xx. The run id is
/// embedded so a leaked id in API logs can be traced back to a specific
/// test run.
fn synthetic_private_endpoint_id(ctx: &TestContext) -> String {
    // Reduce the run id to a hex-safe slug capped at 16 chars to fit inside
    // provider id formats.
    let slug: String = ctx
        .run_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .take(16)
        .collect();
    let padded = format!("{slug:0<16}");
    match ctx.provider.as_str() {
        // AWS VPC endpoint ids look like `vpce-0123456789abcdef0` (17 hex
        // chars after the prefix).
        "aws" => format!("vpce-{}0", &padded[..16]),
        // GCP Private Service Connect endpoint ids are decimal strings. We
        // pick a 19-digit value seeded by the run id hash so different runs
        // collide neither with each other nor with real PSC endpoints.
        "gcp" => {
            let hash: u64 = ctx
                .run_id
                .bytes()
                .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
            format!("{:019}", hash % 10u64.pow(19))
        }
        // Azure private endpoint resource ids are GUIDs. Synthesize one
        // from the run id slug.
        "azure" => {
            let s = format!("{padded:0<32}");
            format!(
                "{}-{}-{}-{}-{}",
                &s[..8],
                &s[8..12],
                &s[12..16],
                &s[16..20],
                &s[20..32]
            )
        }
        // Unknown providers: pick something that contains the run id so it
        // is traceable. The API will reject it; that is the assertion.
        _ => format!("clickhousectl-it-{}", ctx.run_id),
    }
}

#[cfg(feature = "deprecated-fields")]
#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
async fn scale_service_vertical_and_wait(
    client: &Client,
    org_id: &str,
    service_id: &str,
    min_total_memory_gb: Option<f64>,
    max_total_memory_gb: Option<f64>,
    description: &str,
    timeout: std::time::Duration,
    interval: std::time::Duration,
) -> TestResult<()> {
    client
        .instance_scaling_update(
            org_id,
            service_id,
            &ServiceScalingPatchRequest {
                min_total_memory_gb,
                max_total_memory_gb,
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
                if min_total_memory_gb.is_none_or(|v| svc.min_total_memory_gb == v)
                    && max_total_memory_gb.is_none_or(|v| svc.max_total_memory_gb == v)
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
