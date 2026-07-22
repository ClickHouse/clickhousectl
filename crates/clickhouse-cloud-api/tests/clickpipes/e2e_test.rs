//! Multi-source ClickPipes E2E driver.
//!
//! Provisions one ClickHouse Cloud service, then runs every per-source stage
//! against it in parallel via `tokio::join!`. Stages fail independently — one
//! bad source is recorded but doesn't abort the rest of the run.
//!
//! For debugging a single source in isolation, use the per-source test
//! binaries (`clickpipe_s3_test`, `clickpipe_kafka_test`,
//! `clickpipe_kinesis_test`, `clickpipe_mysql_test`). They use the same
//! harness but with their own freshly-provisioned CHC service.

#[path = "../common/mod.rs"]
mod common;
mod driver;
mod stages;
mod support;

use driver::*;
use stages::*;
use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires live ClickHouse Cloud + AWS credentials and provisions real resources"]
async fn cloud_clickpipe_e2e_all_sources() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut harness = E2eHarness::provision("cloud_clickpipe_e2e_all_sources").await?;

    eprintln!("\n== Running stages in parallel ==");
    let (s3_out, kafka_scram_out, kafka_mtls_out, kinesis_out, mysql_out, mongo_out, postgres_out) = tokio::join!(
        run_s3_stage(harness.make_stage_ctx()),
        run_kafka_scram_tls_stage(harness.make_stage_ctx()),
        run_kafka_mtls_stage(harness.make_stage_ctx()),
        run_kinesis_stage(harness.make_stage_ctx()),
        run_mysql_stage(harness.make_stage_ctx()),
        run_mongo_stage(harness.make_stage_ctx()),
        run_postgres_stage(harness.make_stage_ctx()),
    );

    let mut failures = Vec::new();
    for (name, outcome) in [
        ("s3", s3_out),
        ("kafka-scram-tls", kafka_scram_out),
        ("kafka-mtls", kafka_mtls_out),
        ("kinesis", kinesis_out),
        ("mysql", mysql_out),
        ("mongo", mongo_out),
        ("postgres-ec2", postgres_out),
    ] {
        harness.collect(name, outcome, &mut failures);
    }

    harness.teardown(check_failures(failures)).await
}
