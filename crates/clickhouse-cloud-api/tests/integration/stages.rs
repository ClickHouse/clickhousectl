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
/// Bundle of references and owned cleanup registries handed to each stage.
/// Stages own their registries so the driver can run multiple stages
/// concurrently via `tokio::join!` (which forbids overlapping `&mut`
/// borrows). The driver merges each stage's registries back into the parent
/// after the futures resolve.
pub struct StageCtx<'a> {
    pub client: &'a Client,
    pub ctx: &'a TestContext,
    pub ch: &'a ProvisionedClickHouse,
    pub aws_config: &'a aws_config::SdkConfig,
    pub s3: &'a aws_sdk_s3::Client,
    pub iam: &'a aws_sdk_iam::Client,
    pub ec2: &'a aws_sdk_ec2::Client,
    pub aws_region: &'a str,
    pub cleanup: CleanupRegistry,
    pub aws_cleanup: AwsCleanupRegistry,
}

/// Returned from each stage so the driver can collect results + cleanup
/// state without needing shared mutable refs.
pub struct StageOutcome {
    pub result: TestResult<()>,
    pub cleanup: CleanupRegistry,
    pub aws_cleanup: AwsCleanupRegistry,
}

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

    let result = run_s3_inner(client, ctx, ch, s3, iam, aws_region, &mut cleanup, &mut aws_cleanup).await;

    StageOutcome { result, cleanup, aws_cleanup }
}

async fn run_s3_inner(
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

// ── Kafka / Redpanda stages ──────────────────────────────────────────

const KAFKA_SCRAM_TLS_TARGET_TABLE: &str = "redpanda_scram_tls_users";
const KAFKA_MTLS_TARGET_TABLE: &str = "redpanda_mtls_users";
const KAFKA_SEED_ROW_COUNT: i64 = 3;
const REDPANDA_SCRAM_TLS_USER_DATA: &str =
    include_str!("redpanda_user_data_scram_tls.sh.template");
const REDPANDA_MTLS_USER_DATA: &str =
    include_str!("redpanda_user_data_mtls.sh.template");

const REDPANDA_INSTANCE_TYPE: &str = "t3.medium";
const REDPANDA_BOOT_TIMEOUT_SECS: u64 = 600;
const DEFAULT_KAFKA_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 600;
const DEFAULT_KAFKA_INGEST_TIMEOUT_SECS: u64 = 300;

fn b64(s: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    STANDARD.encode(s.as_bytes())
}

/// Shared launcher used by both Kafka stages: pre-allocate an Elastic IP so we
/// know the broker address before generating certs, launch the EC2 with the
/// given user_data, associate the EIP, return `(public_ip, certs)`.
async fn launch_redpanda(
    ec2: &aws_sdk_ec2::Client,
    ctx: &TestContext,
    aws_cleanup: &mut AwsCleanupRegistry,
    variant_tag: &str,
    user_data_with_ip: impl FnOnce(&str, &RedpandaCerts) -> String,
    client_cn: &str,
) -> TestResult<(String, RedpandaCerts)> {
    log_phase(&format!("Kafka stage ({variant_tag}): allocate EIP + generate certs"));

    let (eip_address, allocation_id) = allocate_elastic_ip(ec2).await?;
    aws_cleanup.register_ec2_elastic_ip(allocation_id.clone());
    eprintln!("  allocated eip {eip_address}");

    let certs = generate_redpanda_certs(&eip_address, client_cn)?;

    log_phase(&format!("Kafka stage ({variant_tag}): launch Redpanda EC2"));
    let vpc_id = default_vpc_id(ec2).await?;
    let subnet_id = first_subnet_in_vpc(ec2, &vpc_id).await?;
    let ami_id = latest_ubuntu_noble_amd64_ami(ec2).await?;

    let sg_name = format!("clickhousectl-e2e-redpanda-{variant_tag}-{}", ctx.run_id);
    let sg_id = create_open_security_group(ec2, &vpc_id, &sg_name, &[9092, 9644]).await?;
    aws_cleanup.register_ec2_security_group(sg_id.clone());
    eprintln!("  created security group {sg_id}");

    let user_data = user_data_with_ip(&eip_address, &certs);

    let (instance_id, _default_ip) = launch_ec2_instance(
        ec2,
        &ami_id,
        &subnet_id,
        &sg_id,
        REDPANDA_INSTANCE_TYPE,
        &user_data,
        &format!("clickhousectl-e2e-redpanda-{variant_tag}-{}", ctx.run_id),
    )
    .await?;
    aws_cleanup.register_ec2_instance(instance_id.clone());

    associate_elastic_ip(ec2, &allocation_id, &instance_id).await?;
    eprintln!("  launched instance {instance_id} and associated eip {eip_address}");

    Ok((eip_address, certs))
}

/// SCRAM-SHA-512 over SASL_SSL.
pub async fn run_kafka_scram_tls_stage(sctx: StageCtx<'_>) -> StageOutcome {
    let StageCtx {
        client,
        ctx,
        ch,
        aws_config: _aws_config,
        s3: _s3,
        iam: _iam,
        ec2,
        aws_region: _aws_region,
        mut cleanup,
        mut aws_cleanup,
    } = sctx;
    let result = run_kafka_scram_tls_inner(client, ctx, ch, ec2, &mut cleanup, &mut aws_cleanup).await;
    StageOutcome { result, cleanup, aws_cleanup }
}

async fn run_kafka_scram_tls_inner(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    ec2: &aws_sdk_ec2::Client,
    cleanup: &mut CleanupRegistry,
    aws_cleanup: &mut AwsCleanupRegistry,
) -> TestResult<()> {

    let clickpipe_ready_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CLICKPIPE_READY_SECS",
        DEFAULT_KAFKA_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_KAFKA_INGEST_TIMEOUT_SECS,
    )?;

    let topic = format!("clickpipe-test-{}", sanitize_for_topic(&ctx.run_id));
    let clickpipe_user = format!("clickpipe-{}", sanitize_for_topic(&ctx.run_id));
    let clickpipe_pass = random_token(32);
    let superuser = "admin";
    let superuser_pass = random_token(32);

    let render_ud = |ip: &str, certs: &RedpandaCerts| -> String {
        REDPANDA_SCRAM_TLS_USER_DATA
            .replace("__BROKER_IP__", ip)
            .replace("__TOPIC__", &topic)
            .replace("__REDPANDA_SUPERUSER__", superuser)
            .replace("__REDPANDA_SUPERUSER_PASS__", &superuser_pass)
            .replace("__CLICKPIPE_USER__", &clickpipe_user)
            .replace("__CLICKPIPE_PASS__", &clickpipe_pass)
            .replace("__SERVER_CERT_B64__", &b64(&certs.server_cert_pem))
            .replace("__SERVER_KEY_B64__", &b64(&certs.server_key_pem))
            .replace("__CA_PEM_B64__", &b64(&certs.ca_pem))
    };

    let (broker_ip, certs) =
        launch_redpanda(ec2, ctx, aws_cleanup, "scram-tls", render_ud, "scram-tls-unused").await?;

    log_phase("Kafka stage (scram-tls): wait for Redpanda to be reachable");
    wait_for_tcp_port(&broker_ip, 9092, Duration::from_secs(REDPANDA_BOOT_TIMEOUT_SECS)).await?;
    // user_data needs extra time after the TCP port opens to create the SCRAM
    // user, set ACLs, and seed messages. Probe the admin API instead of a
    // fixed sleep so we don't race ClickPipes' credential check.
    wait_for_redpanda_scram_user(
        &broker_ip,
        9644,
        &clickpipe_user,
        Duration::from_secs(180),
    )
    .await?;
    // A tiny buffer after the user appears: topic creation + ACL grants
    // happen right after, and ClickPipes can't read without those.
    tokio::time::sleep(Duration::from_secs(15)).await;

    log_phase("Kafka stage (scram-tls): create ClickPipe");
    let pipe_request = ClickPipePostRequest {
        name: format!("kafka-scram-tls-{}", ctx.run_id),
        destination: managed_destination_users(KAFKA_SCRAM_TLS_TARGET_TABLE),
        source: ClickPipePostSource {
            kafka: Some(ClickPipePostKafkaSource {
                r#type: ClickPipePostKafkaSourceType::Redpanda,
                format: ClickPipePostKafkaSourceFormat::JSONEachRow,
                brokers: format!("{broker_ip}:9092"),
                topics: topic.clone(),
                consumer_group: Some(format!("clickpipe-scram-tls-{}", ctx.run_id)),
                authentication: ClickPipePostKafkaSourceAuthentication::SCRAM_SHA_512,
                credentials: serde_json::json!({
                    "username": clickpipe_user,
                    "password": clickpipe_pass,
                }),
                ca_certificate: Some(certs.ca_pem.clone()),
                offset: Some(ClickPipeKafkaOffset {
                    strategy: ClickPipeKafkaOffsetStrategy::From_beginning,
                    timestamp: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let clickpipe_id =
        create_pipe_and_wait_running(client, ctx, ch, cleanup, &pipe_request, clickpipe_ready_timeout)
            .await?;
    let _ = clickpipe_id;

    verify_seed_rows(ch, KAFKA_SCRAM_TLS_TARGET_TABLE, ingest_timeout, ctx.poll_interval).await?;
    Ok(())
}

/// MUTUAL_TLS — Redpanda derives the user identity from the client cert's CN.
pub async fn run_kafka_mtls_stage(sctx: StageCtx<'_>) -> StageOutcome {
    let StageCtx {
        client,
        ctx,
        ch,
        aws_config: _aws_config,
        s3: _s3,
        iam: _iam,
        ec2,
        aws_region: _aws_region,
        mut cleanup,
        mut aws_cleanup,
    } = sctx;
    let result = run_kafka_mtls_inner(client, ctx, ch, ec2, &mut cleanup, &mut aws_cleanup).await;
    StageOutcome { result, cleanup, aws_cleanup }
}

async fn run_kafka_mtls_inner(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    ec2: &aws_sdk_ec2::Client,
    cleanup: &mut CleanupRegistry,
    aws_cleanup: &mut AwsCleanupRegistry,
) -> TestResult<()> {

    let clickpipe_ready_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CLICKPIPE_READY_SECS",
        DEFAULT_KAFKA_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_KAFKA_INGEST_TIMEOUT_SECS,
    )?;

    let topic = format!("clickpipe-mtls-{}", sanitize_for_topic(&ctx.run_id));
    let client_cn = format!("clickpipe-mtls-{}", sanitize_for_topic(&ctx.run_id));

    let render_ud = |ip: &str, certs: &RedpandaCerts| -> String {
        REDPANDA_MTLS_USER_DATA
            .replace("__BROKER_IP__", ip)
            .replace("__TOPIC__", &topic)
            .replace("__CLICKPIPE_CN__", &certs.client_cn)
            .replace("__SERVER_CERT_B64__", &b64(&certs.server_cert_pem))
            .replace("__SERVER_KEY_B64__", &b64(&certs.server_key_pem))
            .replace("__CA_PEM_B64__", &b64(&certs.ca_pem))
            .replace("__CLIENT_CERT_B64__", &b64(&certs.client_cert_pem))
            .replace("__CLIENT_KEY_B64__", &b64(&certs.client_key_pem))
    };

    let (broker_ip, certs) =
        launch_redpanda(ec2, ctx, aws_cleanup, "mtls", render_ud, &client_cn).await?;

    log_phase("Kafka stage (mtls): wait for Redpanda to be reachable");
    wait_for_tcp_port(&broker_ip, 9092, Duration::from_secs(REDPANDA_BOOT_TIMEOUT_SECS)).await?;
    tokio::time::sleep(Duration::from_secs(60)).await;

    log_phase("Kafka stage (mtls): create ClickPipe");
    let pipe_request = ClickPipePostRequest {
        name: format!("kafka-mtls-{}", ctx.run_id),
        destination: managed_destination_users(KAFKA_MTLS_TARGET_TABLE),
        source: ClickPipePostSource {
            kafka: Some(ClickPipePostKafkaSource {
                // ClickPipes rejects MUTUAL_TLS for type=redpanda at the API
                // layer even though the OpenAPI spec lists it as supported.
                // Use generic kafka; the broker is the same Redpanda instance.
                r#type: ClickPipePostKafkaSourceType::Kafka,
                format: ClickPipePostKafkaSourceFormat::JSONEachRow,
                brokers: format!("{broker_ip}:9092"),
                topics: topic.clone(),
                consumer_group: Some(format!("clickpipe-mtls-{}", ctx.run_id)),
                authentication: ClickPipePostKafkaSourceAuthentication::MUTUAL_TLS,
                credentials: serde_json::json!({
                    "certificate": certs.client_cert_pem,
                    "privateKey": certs.client_key_pem,
                }),
                ca_certificate: Some(certs.ca_pem.clone()),
                offset: Some(ClickPipeKafkaOffset {
                    strategy: ClickPipeKafkaOffsetStrategy::From_beginning,
                    timestamp: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
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
    verify_seed_rows(ch, KAFKA_MTLS_TARGET_TABLE, ingest_timeout, ctx.poll_interval).await?;
    Ok(())
}

fn managed_destination_users(table: &str) -> ClickPipeMutateDestination {
    ClickPipeMutateDestination {
        database: "default".to_string(),
        table: Some(table.to_string()),
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
    }
}

async fn create_pipe_and_wait_running(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    cleanup: &mut CleanupRegistry,
    pipe_request: &ClickPipePostRequest,
    ready_timeout: Duration,
) -> TestResult<String> {
    let pipe = client
        .click_pipe_create(&ctx.org_id, &ch.service_id, pipe_request)
        .await?
        .result
        .ok_or("clickpipe create returned no result")?;
    let clickpipe_id = pipe.id.to_string();
    cleanup.register_clickpipe(ch.service_id.clone(), clickpipe_id.clone());
    eprintln!("  provisioned clickpipe id <redacted>");

    poll_until(
        "clickpipe Running state",
        ready_timeout,
        ctx.poll_interval,
        || {
            let client = client.clone();
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

    Ok(clickpipe_id)
}

async fn verify_seed_rows(
    ch: &ProvisionedClickHouse,
    table: &str,
    ingest_timeout: Duration,
    poll_interval: Duration,
) -> TestResult<()> {
    poll_until("seeded row count in ClickHouse", ingest_timeout, poll_interval, || {
        let query = ch.query.clone();
        let table = table.to_string();
        async move {
            match query.count_rows(&table).await {
                Ok(count) if count >= KAFKA_SEED_ROW_COUNT => Ok(Some(count)),
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            }
        }
    })
    .await?;
    let ada = ch
        .query
        .scalar_string(&format!(
            "SELECT name FROM default.{table} WHERE id = 1 LIMIT 1"
        ))
        .await?;
    assert_eq!(ada.as_deref(), Some("Ada Lovelace"), "id=1 spot-check failed");
    Ok(())
}

/// Kafka topic names are constrained to `[a-zA-Z0-9._-]`. `run_id` already
/// fits but be defensive — sanitize underscores to hyphens.
fn sanitize_for_topic(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '_' => '-',
            _ => c,
        })
        .collect()
}

/// Small URL-safe-ish token for SCRAM passwords. Not cryptographically rigorous —
/// just enough entropy that two parallel test runs don't collide.
fn random_token(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Mix nanos + a process-specific value; we don't depend on rand to keep
    // the dev-dep tree small. SCRAM doesn't care about the password's source.
    let mut state: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64);
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        out.push(charset[(state >> 33) as usize % charset.len()] as char);
    }
    out
}
