use crate::support::{
    CleanupRegistry, CliRunner, FailureRecorder, StepKind, TestContext, TestResult, json_string,
    log_phase, log_run_header, poll_until,
};

#[test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
fn cloud_service_force_delete() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let runner = CliRunner::new(&ctx);
    let mut cleanup = CleanupRegistry::default();

    let test_result = (|| -> TestResult<()> {
        log_run_header("cloud_service_force_delete", &ctx);
        let mut failures = FailureRecorder::default();
        let service_name = format!("{}-force-del", ctx.service_name());

        log_phase("Create Single-Node Service");
        let mut create_args = vec![
            "service".to_string(),
            "create".to_string(),
            "--name".to_string(),
            service_name.clone(),
            "--provider".to_string(),
            ctx.provider.clone(),
            "--region".to_string(),
            ctx.region.clone(),
            "--num-replicas".to_string(),
            "1".to_string(),
            "--min-replica-memory-gb".to_string(),
            "8".to_string(),
            "--max-replica-memory-gb".to_string(),
            "8".to_string(),
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
        eprintln!("service_id: <redacted>");
        cleanup.register_service(service_id.clone());

        failures
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

        log_phase("Force Delete Running Service");
        failures.run(
            &ctx,
            StepKind::Blocking,
            "delete without --force fails on running service",
            || {
                let result = runner.run_cloud_raw([
                    "service".to_string(),
                    "delete".to_string(),
                    service_id.clone(),
                    "--org-id".to_string(),
                    ctx.org_id.clone(),
                ]);
                match result {
                    Err(_) => Ok(()),
                    Ok(_) => Err("expected delete without --force to fail on a running service".into()),
                }
            },
        )?;

        failures.run(&ctx, StepKind::Blocking, "force delete service", || {
            runner.run_cloud([
                "service".to_string(),
                "delete".to_string(),
                service_id.clone(),
                "--force".to_string(),
                "--org-id".to_string(),
                ctx.org_id.clone(),
            ])
        })?;
        cleanup.unregister_service(&service_id);

        failures.run(
            &ctx,
            StepKind::Blocking,
            "confirm service is gone",
            || {
                poll_until(
                    "service deletion",
                    ctx.delete_timeout,
                    ctx.poll_interval,
                    || match runner.service_get(&service_id) {
                        Ok(output) => {
                            let state =
                                json_string(&output.json, &["/service/state", "/state"])?;
                            if matches!(state, "deleted" | "deleting") {
                                Ok(None)
                            } else {
                                Ok(None)
                            }
                        }
                        Err(error) => {
                            let message = error.to_string();
                            if message.contains("404") || message.contains("not found") {
                                Ok(Some(()))
                            } else {
                                Err(error)
                            }
                        }
                    },
                )
            },
        )?;

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
