//! S3 (object storage) ClickPipe stage: per-test bucket + fixture + IAM role.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

use super::{
    StageCtx, StageOutcome, create_pipe_and_wait_running, duration_from_env_or,
    managed_destination_users,
};

const S3_TARGET_TABLE: &str = "s3_users";
const S3_SEED_ROW_COUNT: i64 = 3;
const S3_FIXTURE_KEY: &str = "fixtures/users.json";
const S3_FIXTURE_BODY: &str = concat!(
    "{\"id\": 1, \"name\": \"Ada Lovelace\",     \"email\": \"ada@example.com\"}\n",
    "{\"id\": 2, \"name\": \"Grace Hopper\",      \"email\": \"grace@example.com\"}\n",
    "{\"id\": 3, \"name\": \"Margaret Hamilton\", \"email\": \"margaret@example.com\"}\n",
);

const DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 600;
const DEFAULT_INGEST_TIMEOUT_SECS: u64 = 300;

/// S3 ClickPipe stage: provision bucket+fixture+role, create pipe with
/// IAM_ROLE auth, wait for Running, assert seeded rows arrive.
pub async fn run_s3_stage(sctx: StageCtx<'_>) -> StageOutcome {
    let StageCtx {
        client,
        ctx,
        ch,
        aws_config: _aws_config,
        s3,
        iam,
        ec2: _ec2,
        aws_region,
        mut cleanup,
        mut aws_cleanup,
    } = sctx;

    let result = run_inner(client, ctx, ch, s3, iam, aws_region, &mut cleanup, &mut aws_cleanup).await;

    StageOutcome { result, cleanup, aws_cleanup }
}

#[allow(clippy::too_many_arguments)]
async fn run_inner(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    s3: &aws_sdk_s3::Client,
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

    log_phase("S3 stage: provision AWS bucket + role");

    let bucket = ctx.aws_s3_bucket_name();
    let role_name = ctx.aws_iam_role_name();
    let aws_tags = vec![
        ("managed_by".to_string(), "clickhousectl_e2e".to_string()),
        ("run_id".to_string(), ctx.run_id.clone()),
        ("suite".to_string(), "clickpipe_s3".to_string()),
    ];

    create_private_bucket(s3, aws_region, &bucket, &aws_tags).await?;
    aws_cleanup.register_s3_bucket(aws_sdk_s3::config::Region::new(aws_region.to_string()), bucket.clone());
    eprintln!("  created bucket {bucket}");

    put_object_bytes(
        s3,
        &bucket,
        S3_FIXTURE_KEY,
        S3_FIXTURE_BODY.as_bytes().to_vec(),
        "application/x-ndjson",
    )
    .await?;
    eprintln!("  uploaded fixture {S3_FIXTURE_KEY}");

    let read_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Sid": "ReadFixtures",
                "Effect": "Allow",
                "Action": ["s3:GetObject"],
                "Resource": format!("arn:aws:s3:::{bucket}/*"),
            },
            {
                "Sid": "ListBucket",
                "Effect": "Allow",
                "Action": ["s3:ListBucket", "s3:GetBucketLocation"],
                "Resource": format!("arn:aws:s3:::{bucket}"),
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

    log_phase("S3 stage: create ClickPipe");

    // Register the destination table for teardown — when running against a
    // shared CHC service, the table outlives the pipe and a re-run would
    // collide on "table exists and is not empty".
    cleanup.register_table(S3_TARGET_TABLE);

    let object_url = format!("https://{bucket}.s3.{aws_region}.amazonaws.com/{S3_FIXTURE_KEY}");
    let pipe_name = format!("s3-{}", ctx.run_id);

    let pipe_request = ClickPipePostRequest {
        name: pipe_name,
        source: ClickPipePostSource {
            object_storage: Some(ClickPipePostObjectStorageSource {
                r#type: ClickPipePostObjectStorageSourceType::default(),
                format: ClickPipePostObjectStorageSourceFormat::JSONEachRow,
                url: object_url,
                authentication: Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE),
                iam_role: Some(role_arn),
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: managed_destination_users(S3_TARGET_TABLE),
        ..Default::default()
    };

    let _ = create_pipe_and_wait_running(
        client,
        ctx,
        ch,
        cleanup,
        &pipe_request,
        clickpipe_ready_timeout,
    )
    .await?;

    log_phase("S3 stage: verify rows in ClickHouse");
    super::verify_seed_rows(
        ch,
        S3_TARGET_TABLE,
        S3_SEED_ROW_COUNT,
        1,
        "Ada Lovelace",
        ingest_timeout,
        ctx.poll_interval,
    )
    .await
}
