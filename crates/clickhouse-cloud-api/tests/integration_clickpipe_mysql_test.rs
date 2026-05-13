//! MySQL ClickPipe E2E — own CHC service, single stage.

mod integration;

use integration::driver::*;
use integration::stages::*;
use integration::support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires live ClickHouse Cloud + AWS credentials and provisions real resources"]
async fn cloud_clickpipe_mysql() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut harness = E2eHarness::provision("cloud_clickpipe_mysql").await?;

    let outcome = run_mysql_stage(harness.make_stage_ctx()).await;
    let mut failures = Vec::new();
    harness.collect("mysql", outcome, &mut failures);

    harness.teardown(check_failures(failures)).await
}
