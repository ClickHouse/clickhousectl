//! Postgres-on-EC2 ClickPipe E2E — own CHC service, single stage. Parallels
//! the mysql/mongo per-source binaries. Distinct from the existing
//! `integration_clickpipe_postgres_cdc_test` (Al's library-driven test
//! against CHC-managed Postgres).

mod integration;

use integration::driver::*;
use integration::stages::*;
use integration::support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires live ClickHouse Cloud + AWS credentials and provisions real resources"]
async fn cloud_clickpipe_postgres_ec2() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut harness = E2eHarness::provision("cloud_clickpipe_postgres_ec2").await?;

    let outcome = run_postgres_stage(harness.make_stage_ctx()).await;
    let mut failures = Vec::new();
    harness.collect("postgres-ec2", outcome, &mut failures);

    harness.teardown(check_failures(failures)).await
}
