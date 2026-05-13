//! Multi-source ClickPipes end-to-end driver.
//!
//! Provisions a single ClickHouse Cloud service, then runs each per-source
//! stage against it in sequence. Stages fail independently — one bad source
//! is recorded but doesn't abort the rest of the run, since CHC service
//! provisioning is the expensive part and we want full coverage per run.
//!
//! Add new sources by writing a `run_<source>_stage` function in
//! [`integration::stages`] and a corresponding call below.

mod integration;

use integration::stages::*;
use integration::support::*;

const AWS_REGION: &str = "us-east-2";

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud + AWS credentials and provisions real resources"]
async fn cloud_clickpipe_e2e_all_sources() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ctx = TestContext::from_env()?;
    let client = create_client()?;

    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(AWS_REGION))
        .load()
        .await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);
    let iam = aws_sdk_iam::Client::new(&aws_config);

    let mut cleanup = CleanupRegistry::default();
    let mut aws_cleanup = AwsCleanupRegistry::default();

    let run_result = async {
        log_run_header("cloud_clickpipe_e2e_all_sources", &ctx);

        // ── Provision ClickHouse (shared across all stages) ─────────
        log_phase("Provision ClickHouse");
        let ch = provision_clickhouse(
            &client,
            &ctx,
            &mut cleanup,
            &ctx.clickpipe_e2e_service_name(),
            ctx.clickpipe_e2e_run_tags(),
        )
        .await?;

        // ── Stages ──────────────────────────────────────────────────
        //
        // Stages are independent — a failure in one is logged but the
        // run continues so later stages still execute on the same service.
        let mut stage_failures: Vec<(String, String)> = Vec::new();

        run_stage(
            "s3",
            &mut stage_failures,
            run_s3_stage(&mut StageCtx {
                client: &client,
                ctx: &ctx,
                ch: &ch,
                aws_config: &aws_config,
                s3: &s3,
                iam: &iam,
                aws_region: AWS_REGION,
                cleanup: &mut cleanup,
                aws_cleanup: &mut aws_cleanup,
            }),
        )
        .await;

        // Future stages plug in here:
        //   run_stage("kinesis", &mut stage_failures,
        //             run_kinesis_stage(&mut StageCtx { ... })).await;
        //   run_stage("kafka",   &mut stage_failures, run_kafka_stage(...)).await;

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
    let aws_cleanup_result = aws_cleanup.cleanup(&aws_config, &iam).await;

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

async fn run_stage<F>(name: &str, failures: &mut Vec<(String, String)>, fut: F)
where
    F: std::future::Future<Output = TestResult<()>>,
{
    eprintln!("\n== Stage: {name} ==");
    match fut.await {
        Ok(()) => eprintln!("  PASS [{name}]"),
        Err(err) => {
            eprintln!("  FAIL [{name}]: {}", first_line(&err.to_string()));
            failures.push((name.to_string(), err.to_string()));
        }
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}
