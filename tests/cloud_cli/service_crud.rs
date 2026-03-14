use crate::support::{
    CleanupRegistry, CliRunner, TestContext, TestResult, json_string, poll_until,
    service_list_is_empty, service_present_in_list,
};

#[test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
fn cloud_service_crud_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let runner = CliRunner::new(&ctx);
    let mut cleanup = CleanupRegistry::default();

    let test_result = (|| -> TestResult<()> {
        eprintln!("[service-crud] run_id={}", ctx.run_id);
        eprintln!("[service-crud] verifying org access");
        let org = runner.run_cloud(["org".to_string(), "get".to_string(), ctx.org_id.clone()])?;
        let org_id = json_string(&org.json, &["/id", "/org/id"])?;
        assert_eq!(org_id, ctx.org_id);

        eprintln!("[service-crud] checking for leftover tagged services");
        let list_before = runner.service_list_for_run()?;
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
            "--org-id".to_string(),
            ctx.org_id.clone(),
        ];

        for tag in ctx.run_tags() {
            create_args.push("--tag".to_string());
            create_args.push(tag);
        }

        eprintln!("[service-crud] creating service {}", ctx.service_name());
        let created = runner.run_cloud(create_args)?;
        let service_id = json_string(&created.json, &["/service/id", "/id"])?.to_string();
        let _password = json_string(&created.json, &["/password", "/service/password"])?;
        eprintln!("[service-crud] created service_id={service_id}");
        cleanup.register_service(service_id.clone());

        eprintln!("[service-crud] waiting for steady state");
        let ready = poll_until(
            &format!("service {service_id} steady state"),
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
        )?;

        let ready_name = json_string(&ready.json, &["/service/name", "/name"])?;
        assert_eq!(ready_name, ctx.service_name());

        eprintln!("[service-crud] verifying service is discoverable in list");
        let listed = runner.service_list_for_run()?;
        assert!(
            service_present_in_list(&listed.json, &service_id),
            "created service {service_id} was not visible in service list"
        );

        eprintln!(
            "[service-crud] renaming service to {}",
            ctx.updated_service_name()
        );
        runner.run_cloud([
            "service".to_string(),
            "update".to_string(),
            service_id.clone(),
            "--name".to_string(),
            ctx.updated_service_name(),
            "--org-id".to_string(),
            ctx.org_id.clone(),
        ])?;

        eprintln!("[service-crud] waiting for rename to become visible");
        let updated = poll_until(
            &format!("service {service_id} update visibility"),
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
        )?;

        let updated_name = json_string(&updated.json, &["/service/name", "/name"])?;
        assert_eq!(updated_name, ctx.updated_service_name());

        eprintln!("[service-crud] running cleanup");
        cleanup.cleanup(&runner).map_err(|error| error.into())
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
