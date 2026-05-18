//! S3 ClickPipe E2E — own CHC service, single stage.
//!
//! For amortised "run everything against one service" use the all-in-one
//! driver: `cargo test --test clickpipe_e2e_test`.

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
async fn cloud_clickpipe_s3() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut harness = E2eHarness::provision("cloud_clickpipe_s3").await?;

    let outcome = run_s3_stage(harness.make_stage_ctx()).await;
    let mut failures = Vec::new();
    harness.collect("s3", outcome, &mut failures);

    harness.teardown(check_failures(failures)).await
}
