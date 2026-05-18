//! Kafka (Redpanda) ClickPipe E2E — own CHC service, both auth variants
//! (SCRAM-SHA-512 + TLS, MUTUAL_TLS) on one service.

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
async fn cloud_clickpipe_kafka() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut harness = E2eHarness::provision("cloud_clickpipe_kafka").await?;

    eprintln!("\n== Running kafka variants in parallel ==");
    let (scram_out, mtls_out) = tokio::join!(
        run_kafka_scram_tls_stage(harness.make_stage_ctx()),
        run_kafka_mtls_stage(harness.make_stage_ctx()),
    );

    let mut failures = Vec::new();
    for (name, outcome) in [
        ("kafka-scram-tls", scram_out),
        ("kafka-mtls", mtls_out),
    ] {
        harness.collect(name, outcome, &mut failures);
    }

    harness.teardown(check_failures(failures)).await
}
