//! Per-source E2E stages, each operating against a single shared
//! [`ProvisionedClickHouse`]. Stages return `TestResult<()>` so they
//! plug straight into `FailureRecorder::run` with `StepKind::NonBlocking`
//! — one bad source doesn't abort the rest of the run.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

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

/// Inputs every source stage needs. Borrowed once per stage so the stages
/// don't have to thread the same six arguments through every call.
pub struct StageCtx<'a> {
    pub client: &'a Client,
    pub ctx: &'a TestContext,
    pub ch: &'a ProvisionedClickHouse,
    pub aws_config: &'a aws_config::SdkConfig,
    pub s3: &'a aws_sdk_s3::Client,
    pub iam: &'a aws_sdk_iam::Client,
    pub aws_region: &'a str,
    pub cleanup: &'a mut CleanupRegistry,
    pub aws_cleanup: &'a mut AwsCleanupRegistry,
}

/// S3 ClickPipe stage: provision bucket+fixture+role, create pipe with
/// IAM_ROLE auth, wait for Running, assert seeded rows arrive.
pub async fn run_s3_stage(stage: &mut StageCtx<'_>) -> TestResult<()> {
    let StageCtx {
        client,
        ctx,
        ch,
        aws_config: _aws_config,
        s3,
        iam,
        aws_region,
        cleanup,
        aws_cleanup,
    } = stage;
    let aws_region = *aws_region;

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

    let object_url = format!("https://{bucket}.s3.{aws_region}.amazonaws.com/{S3_FIXTURE_KEY}");

    let pipe_request = ClickPipePostRequest {
        name: format!("s3-{}", ctx.run_id),
        destination: ClickPipeMutateDestination {
            database: "default".to_string(),
            table: Some(S3_TARGET_TABLE.to_string()),
            managed_table: Some(true),
            columns: vec![
                ClickPipeDestinationColumn {
                    name: "id".to_string(),
                    r#type: "Int64".to_string(),
                },
                ClickPipeDestinationColumn {
                    name: "name".to_string(),
                    r#type: "String".to_string(),
                },
                ClickPipeDestinationColumn {
                    name: "email".to_string(),
                    r#type: "String".to_string(),
                },
            ],
            table_definition: Some(ClickPipeDestinationTableDefinition {
                engine: ClickPipeDestinationTableEngine {
                    r#type: ClickPipeDestinationTableEngineType::MergeTree,
                    ..Default::default()
                },
                primary_key: "id".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
        source: ClickPipePostSource {
            object_storage: Some(ClickPipePostObjectStorageSource {
                r#type: ClickPipePostObjectStorageSourceType::default(),
                format: ClickPipePostObjectStorageSourceFormat::JSONEachRow,
                url: object_url,
                authentication: Some(
                    ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE,
                ),
                iam_role: Some(role_arn.clone()),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let pipe = client
        .click_pipe_create(&ctx.org_id, &ch.service_id, &pipe_request)
        .await?
        .result
        .ok_or("clickpipe create returned no result")?;
    let clickpipe_id = pipe.id.to_string();
    cleanup.register_clickpipe(ch.service_id.clone(), clickpipe_id.clone());
    eprintln!("  provisioned clickpipe id <redacted>");

    log_phase("S3 stage: wait for ClickPipe Running");

    let _ = poll_until(
        "clickpipe Running state",
        clickpipe_ready_timeout,
        ctx.poll_interval,
        || {
            let client = (*client).clone();
            let org_id = ctx.org_id.clone();
            let service_id = ch.service_id.clone();
            let clickpipe_id = clickpipe_id.clone();
            async move {
                let resp = client
                    .click_pipe_get(&org_id, &service_id, &clickpipe_id)
                    .await?;
                let pipe = resp.result.ok_or("clickpipe get returned no result")?;
                match pipe.state {
                    ClickPipeState::Running | ClickPipeState::Completed => Ok(Some(pipe)),
                    ClickPipeState::Failed | ClickPipeState::InternalError => Err(format!(
                        "clickpipe entered terminal failure state {}",
                        pipe.state
                    )
                    .into()),
                    _ => Ok(None),
                }
            }
        },
    )
    .await?;

    log_phase("S3 stage: verify rows in ClickHouse");

    poll_until(
        "fixture row count in ClickHouse",
        ingest_timeout,
        ctx.poll_interval,
        || {
            let query = ch.query.clone();
            async move {
                match query.count_rows(S3_TARGET_TABLE).await {
                    Ok(count) if count >= S3_SEED_ROW_COUNT => Ok(Some(count)),
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
        },
    )
    .await?;

    let ada = ch
        .query
        .scalar_string(&format!(
            "SELECT name FROM default.{S3_TARGET_TABLE} WHERE id = 1 LIMIT 1"
        ))
        .await?;
    assert_eq!(ada.as_deref(), Some("Ada Lovelace"), "id=1 spot-check failed");

    // Pipe gets dropped via cleanup; nothing else to do at the stage level.
    Ok(())
}

fn duration_from_env_or(name: &str, default_secs: u64) -> TestResult<Duration> {
    match std::env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}
