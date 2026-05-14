#![allow(dead_code)]

//! Shared harness for ClickPipes E2E test binaries.
//!
//! Every test binary (`clickpipe_e2e_test`, `clickpipe_s3_test`,
//! `clickpipe_mysql_test`, etc.) follows the same lifecycle: load env, build
//! AWS clients, provision one ClickHouse Cloud service, run one or more
//! source stages against it, then tear everything down. [`E2eHarness`] holds
//! the boilerplate so each per-source test file is ~15 lines.

use std::time::Duration;

use crate::stages::{StageCtx, StageOutcome};
use crate::support::*;

const DEFAULT_AWS_REGION: &str = "eu-west-1";

pub struct E2eHarness {
    pub client: clickhouse_cloud_api::Client,
    pub ctx: TestContext,
    pub aws_config: aws_config::SdkConfig,
    pub s3: aws_sdk_s3::Client,
    pub iam: aws_sdk_iam::Client,
    pub ec2: aws_sdk_ec2::Client,
    pub aws_region: String,
    pub ch: ProvisionedClickHouse,
    pub cleanup: CleanupRegistry,
    pub aws_cleanup: AwsCleanupRegistry,
}

impl E2eHarness {
    /// Build AWS clients, provision a shared CHC service, and return a harness
    /// ready to run stages against it. The service is registered for cleanup
    /// before any stage runs so teardown happens even on panic.
    pub async fn provision(test_name: &str) -> TestResult<Self> {
        let ctx = TestContext::from_env()?;
        let client = create_client()?;

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
        let aws_cleanup = AwsCleanupRegistry::default();

        log_run_header(test_name, &ctx);

        // If CLICKHOUSE_CLOUD_TEST_SERVICE_ID is set, attach to an existing
        // externally-managed service rather than provisioning a new one.
        // Useful for sharing one CHC service across multiple per-source test
        // runs without re-paying the ~3 min provision/teardown each time.
        // Requires CLICKHOUSE_CLOUD_TEST_SERVICE_PASSWORD too. The service is
        // NOT registered with cleanup — caller is responsible for teardown.
        let ch = match std::env::var("CLICKHOUSE_CLOUD_TEST_SERVICE_ID")
            .ok()
            .filter(|s| !s.is_empty())
        {
            Some(service_id) => {
                let password = std::env::var("CLICKHOUSE_CLOUD_TEST_SERVICE_PASSWORD")
                    .map_err(|_| {
                        "CLICKHOUSE_CLOUD_TEST_SERVICE_ID set but \
                         CLICKHOUSE_CLOUD_TEST_SERVICE_PASSWORD missing"
                    })?;
                log_phase("Attach to existing ClickHouse service");
                attach_clickhouse(&client, &ctx.org_id, &service_id, &password).await?
            }
            None => {
                log_phase("Provision ClickHouse");
                provision_clickhouse(
                    &client,
                    &ctx,
                    &mut cleanup,
                    &ctx.clickpipe_e2e_service_name(),
                    ctx.clickpipe_e2e_run_tags(),
                )
                .await?
            }
        };

        Ok(Self {
            client,
            ctx,
            aws_config,
            s3,
            iam,
            ec2,
            aws_region,
            ch,
            cleanup,
            aws_cleanup,
        })
    }

    /// Build a fresh `StageCtx` for one stage. Each call returns owned cleanup
    /// registries so multiple stages can run concurrently under `tokio::join!`
    /// without overlapping `&mut` borrows.
    pub fn make_stage_ctx(&self) -> StageCtx<'_> {
        StageCtx {
            client: &self.client,
            ctx: &self.ctx,
            ch: &self.ch,
            aws_config: &self.aws_config,
            s3: &self.s3,
            iam: &self.iam,
            ec2: &self.ec2,
            aws_region: &self.aws_region,
            cleanup: CleanupRegistry::default(),
            aws_cleanup: AwsCleanupRegistry::default(),
        }
    }

    /// Merge a stage outcome's cleanup state into the harness and record any
    /// failure. Must be called for every stage outcome before teardown.
    pub fn collect(
        &mut self,
        name: &str,
        outcome: StageOutcome,
        failures: &mut Vec<(String, String)>,
    ) {
        self.cleanup.merge_from(outcome.cleanup);
        self.aws_cleanup.merge_from(outcome.aws_cleanup);
        match outcome.result {
            Ok(()) => eprintln!("  PASS [{name}]"),
            Err(err) => {
                eprintln!("  FAIL [{name}]: {}", render_error_chain(err.as_ref()));
                failures.push((name.to_string(), err.to_string()));
            }
        }
    }

    /// Tear down the CHC service + AWS resources and return the final test
    /// result. Always runs both cleanup passes even if `run_result` is `Err`.
    pub async fn teardown(self, run_result: TestResult<()>) -> TestResult<()> {
        log_phase("Teardown");
        let Self {
            client,
            ctx,
            aws_config,
            iam,
            ec2,
            ch,
            mut cleanup,
            mut aws_cleanup,
            ..
        } = self;

        let cleanup_result = cleanup
            .cleanup(
                &client,
                &ctx.org_id,
                ctx.delete_timeout,
                ctx.poll_interval,
                Some(&ch.query),
            )
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
}

/// Build a single combined `TestResult` from per-stage failure entries. Used
/// after every per-source test collects its outcomes.
pub fn check_failures(failures: Vec<(String, String)>) -> TestResult<()> {
    if failures.is_empty() {
        return Ok(());
    }
    let summary = failures
        .iter()
        .map(|(name, err)| format!("  - {name}: {}", first_line(err)))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!("{} stage failure(s):\n{summary}", failures.len()).into())
}

/// Walk an error's `source()` chain so SDK errors aren't reduced to the
/// opaque `"service error"` Display.
pub fn render_error_chain(err: &dyn std::error::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut cur = err.source();
    while let Some(s) = cur {
        parts.push(s.to_string());
        cur = s.source();
    }
    parts.join(" -> ")
}

pub fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

// Silence "unused" warnings when only some helpers are referenced by a given
// test binary.
#[allow(dead_code)]
const _DURATION_USED: Duration = Duration::from_secs(0);
