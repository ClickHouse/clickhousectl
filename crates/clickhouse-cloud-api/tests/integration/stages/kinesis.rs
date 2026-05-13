//! Kinesis ClickPipe stage: per-test stream + IAM_ROLE auth.
//!
//! Mirrors the S3 stage's IAM_ROLE pattern — the trust policy is scoped to
//! the per-test CHC service principal, so the role can only be assumed by
//! this one ClickPipes deployment.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

use super::{StageCtx, StageOutcome, duration_from_env_or};

const KINESIS_TARGET_TABLE: &str = "kinesis_users";
const KINESIS_SEED_ROW_COUNT: i64 = 3;
const KINESIS_SEED_RECORDS: &[(&str, &str)] = &[
    ("1", "{\"id\": 1, \"name\": \"Ada Lovelace\", \"email\": \"ada@example.com\"}"),
    ("2", "{\"id\": 2, \"name\": \"Grace Hopper\", \"email\": \"grace@example.com\"}"),
    ("3", "{\"id\": 3, \"name\": \"Margaret Hamilton\", \"email\": \"margaret@example.com\"}"),
];

const DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 600;
const DEFAULT_INGEST_TIMEOUT_SECS: u64 = 300;

/// Kinesis ClickPipe stage: provision stream+role, seed 3 records, create pipe
/// with IAM_ROLE auth + TRIM_HORIZON iterator, wait for Running, assert
/// seeded rows arrive.
pub async fn run_kinesis_stage(sctx: StageCtx<'_>) -> StageOutcome {
    let StageCtx {
        client,
        ctx,
        ch,
        aws_config,
        s3: _s3,
        iam,
        ec2: _ec2,
        aws_region,
        mut cleanup,
        mut aws_cleanup,
    } = sctx;

    let result = run_inner(
        client,
        ctx,
        ch,
        aws_config,
        iam,
        aws_region,
        &mut cleanup,
        &mut aws_cleanup,
    )
    .await;

    StageOutcome { result, cleanup, aws_cleanup }
}

#[allow(clippy::too_many_arguments)]
async fn run_inner(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    aws_config: &aws_config::SdkConfig,
    iam: &aws_sdk_iam::Client,
    aws_region: &str,
    cleanup: &mut CleanupRegistry,
    aws_cleanup: &mut AwsCleanupRegistry,
) -> TestResult<()> {
    let clickpipe_ready_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CLICKPIPE_READY_SECS",
        DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_INGEST_TIMEOUT_SECS,
    )?;

    log_phase("Kinesis stage: provision AWS stream + role");

    let stream_name = ctx.aws_kinesis_stream_name();
    let role_name = ctx.aws_kinesis_iam_role_name();
    let aws_tags = vec![
        ("managed_by".to_string(), "clickhousectl_e2e".to_string()),
        ("run_id".to_string(), ctx.run_id.clone()),
        ("suite".to_string(), "clickpipe_kinesis".to_string()),
    ];

    // Build a Kinesis client pinned to the test region.
    let kinesis_config = aws_sdk_kinesis::config::Builder::from(aws_config)
        .region(aws_sdk_kinesis::config::Region::new(aws_region.to_string()))
        .build();
    let kinesis = aws_sdk_kinesis::Client::from_conf(kinesis_config);

    // Register cleanup BEFORE awaiting creation — if create_stream succeeds
    // partially (stream exists but never reaches ACTIVE), we still need
    // teardown to drop it.
    aws_cleanup.register_kinesis_stream(aws_region.to_string(), stream_name.clone());
    let stream_arn = create_kinesis_stream(&kinesis, &stream_name, &aws_tags).await?;
    eprintln!("  created kinesis stream {stream_name}");

    // Seed 3 JSON records before pipe creation so TRIM_HORIZON sees them all.
    for (partition_key, body) in KINESIS_SEED_RECORDS {
        put_kinesis_record(&kinesis, &stream_name, partition_key, body.as_bytes()).await?;
    }
    eprintln!("  seeded {} kinesis records", KINESIS_SEED_RECORDS.len());

    // Brief sleep so the records are durably committed before the pipe begins
    // reading. Kinesis acks PutRecord synchronously, but a few seconds of
    // padding avoids any edge cases in the consumer's initial poll.
    tokio::time::sleep(Duration::from_secs(3)).await;

    let read_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Sid": "ReadStream",
                "Effect": "Allow",
                "Action": [
                    "kinesis:GetRecords",
                    "kinesis:GetShardIterator",
                    "kinesis:DescribeStream",
                    "kinesis:DescribeStreamSummary",
                    "kinesis:ListShards",
                    "kinesis:SubscribeToShard",
                ],
                "Resource": stream_arn,
            },
            {
                "Sid": "ListStreams",
                "Effect": "Allow",
                "Action": ["kinesis:ListStreams"],
                "Resource": "*",
            }
        ]
    })
    .to_string();

    let role_arn = create_clickpipes_iam_role(
        iam,
        &role_name,
        &ch.iam_role,
        &read_policy,
        &aws_tags,
    )
    .await?;
    aws_cleanup.register_iam_role(role_name.clone());
    eprintln!("  created iam role {role_name}");

    // IAM has eventual consistency: a freshly-created role can take up to
    // ~10s to be assumable. Sleep so ClickPipes' first connect doesn't trip
    // on a transient AccessDenied.
    tokio::time::sleep(Duration::from_secs(10)).await;

    log_phase("Kinesis stage: create ClickPipe");

    let pipe_request = ClickPipePostRequest {
        name: format!("kinesis-{}", ctx.run_id),
        destination: super::managed_destination_users(KINESIS_TARGET_TABLE),
        source: ClickPipePostSource {
            kinesis: Some(ClickPipePostKinesisSource {
                authentication: ClickPipePostKinesisSourceAuthentication::IAM_ROLE,
                iam_role: Some(role_arn.clone()),
                stream_name: stream_name.clone(),
                region: aws_region.to_string(),
                format: ClickPipePostKinesisSourceFormat::JSONEachRow,
                iterator_type: ClickPipePostKinesisSourceIteratortype::TRIM_HORIZON,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let _ = super::create_pipe_and_wait_running(
        client,
        ctx,
        ch,
        cleanup,
        &pipe_request,
        clickpipe_ready_timeout,
    )
    .await?;

    log_phase("Kinesis stage: verify rows in ClickHouse");
    super::verify_seed_rows(
        ch,
        KINESIS_TARGET_TABLE,
        KINESIS_SEED_ROW_COUNT,
        ingest_timeout,
        ctx.poll_interval,
    )
    .await
}
