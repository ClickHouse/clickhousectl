//! Multi-source ClickPipes end-to-end driver.
//!
//! Provisions a single ClickHouse Cloud service, then runs each per-source
//! stage against it **in parallel**. Stages own their cleanup registries; the
//! driver merges them back after `tokio::join!` resolves so a single teardown
//! pass handles everything. Stages fail independently — one bad source is
//! recorded but doesn't abort the rest of the run.
//!
//! Add a new source by writing `run_<source>_stage` in [`integration::stages`]
//! (taking `StageCtx` by value, returning `StageOutcome`) and plugging it into
//! the `tokio::join!` below.

mod integration;

use integration::stages::*;
use integration::support::*;

const DEFAULT_AWS_REGION: &str = "eu-west-1";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires live ClickHouse Cloud + AWS credentials and provisions real resources"]
async fn cloud_clickpipe_e2e_all_sources() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ctx = TestContext::from_env()?;
    let client = create_client()?;

    // Co-locate AWS infra with the CHC service region by default, so EIP
    // quotas in one region (us-east-2 in our sandbox account) don't block us.
    let aws_region: String = std::env::var("CLICKHOUSE_CLOUD_TEST_AWS_REGION")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_AWS_REGION.to_string());

    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(aws_region.clone()))
        .load()
        .await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);
    let iam = aws_sdk_iam::Client::new(&aws_config);
    let ec2 = aws_sdk_ec2::Client::new(&aws_config);

    let mut cleanup = CleanupRegistry::default();
    let mut aws_cleanup = AwsCleanupRegistry::default();

    let run_result = async {
        log_run_header("cloud_clickpipe_e2e_all_sources", &ctx);

        // Provision once (blocking — no stage can run without the service).
        log_phase("Provision ClickHouse");
        let ch = provision_clickhouse(
            &client,
            &ctx,
            &mut cleanup,
            &ctx.clickpipe_e2e_service_name(),
            ctx.clickpipe_e2e_run_tags(),
        )
        .await?;

        // Each stage gets its own cleanup registries to keep ownership disjoint
        // for the concurrent join. Outcomes are folded into the driver's
        // registries before teardown.
        let make_ctx = || StageCtx {
            client: &client,
            ctx: &ctx,
            ch: &ch,
            aws_config: &aws_config,
            s3: &s3,
            iam: &iam,
            ec2: &ec2,
            aws_region: &aws_region,
            cleanup: CleanupRegistry::default(),
            aws_cleanup: AwsCleanupRegistry::default(),
        };

        eprintln!("\n== Running stages in parallel ==");
        let (s3_out, kafka_scram_out, kafka_mtls_out) = tokio::join!(
            run_s3_stage(make_ctx()),
            run_kafka_scram_tls_stage(make_ctx()),
            run_kafka_mtls_stage(make_ctx()),
        );

        // Order matters here: merge cleanup state BEFORE inspecting results,
        // so failures still get teardown.
        let mut stage_failures: Vec<(String, String)> = Vec::new();
        for (name, outcome) in [
            ("s3", s3_out),
            ("kafka-scram-tls", kafka_scram_out),
            ("kafka-mtls", kafka_mtls_out),
        ] {
            cleanup.merge_from(outcome.cleanup);
            aws_cleanup.merge_from(outcome.aws_cleanup);
            match outcome.result {
                Ok(()) => eprintln!("  PASS [{name}]"),
                Err(err) => {
                    eprintln!("  FAIL [{name}]: {}", render_error_chain(err.as_ref()));
                    stage_failures.push((name.to_string(), err.to_string()));
                }
            }
        }

        if !stage_failures.is_empty() {
            let summary = stage_failures
                .iter()
                .map(|(name, err)| format!("  - {name}: {}", first_line(err)))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!("{} stage failure(s):\n{summary}", stage_failures.len()).into());
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    log_phase("Teardown");
    let cleanup_result = cleanup
        .cleanup(&client, &ctx.org_id, ctx.delete_timeout, ctx.poll_interval)
        .await;
    let aws_cleanup_result = aws_cleanup.cleanup(&aws_config, &iam, &ec2).await;

    match (run_result, cleanup_result, aws_cleanup_result) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (Err(error), _, _) => Err(error),
        (Ok(()), Err(cleanup_error), Ok(())) => Err(cleanup_error.into()),
        (Ok(()), Ok(()), Err(aws_error)) => Err(aws_error.into()),
        (Ok(()), Err(cleanup_error), Err(aws_error)) => {
            Err(format!("{cleanup_error}\naws cleanup failed:\n{aws_error}").into())
        }
    }
}

/// Walk the error's `source()` chain so we see the underlying SDK message
/// rather than a top-level Display like `"service error"`.
fn render_error_chain(err: &dyn std::error::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut cur = err.source();
    while let Some(s) = cur {
        parts.push(s.to_string());
        cur = s.source();
    }
    parts.join(" -> ")
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}
