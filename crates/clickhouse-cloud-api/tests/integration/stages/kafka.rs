//! Kafka (Redpanda) ClickPipe stages: SCRAM-SHA-512 over SASL_SSL, and
//! MUTUAL_TLS. Each variant launches its own per-test EC2 with Redpanda
//! configured via `user_data`, a pre-allocated EIP so server certs match the
//! advertised broker address, and seeded JSON messages.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

use super::{
    StageCtx, StageOutcome, b64, create_pipe_and_wait_running, duration_from_env_or,
    managed_destination_users, random_token, sanitize_for_topic, verify_seed_rows,
};

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
    let result = run_scram_tls_inner(client, ctx, ch, ec2, &mut cleanup, &mut aws_cleanup).await;
    StageOutcome { result, cleanup, aws_cleanup }
}

async fn run_scram_tls_inner(
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
    wait_for_redpanda_scram_user(
        &broker_ip,
        9644,
        &clickpipe_user,
        Duration::from_secs(180),
    )
    .await?;
    // Tiny buffer after user appears: topic creation + ACL grants happen
    // right after, and ClickPipes can't consume without those.
    tokio::time::sleep(Duration::from_secs(15)).await;

    log_phase("Kafka stage (scram-tls): create ClickPipe (via CLI)");

    cleanup.register_table(KAFKA_SCRAM_TLS_TARGET_TABLE);

    let pipe_name = format!("kafka-scram-tls-{}", ctx.run_id);
    let brokers = format!("{broker_ip}:9092");
    let consumer_group = format!("clickpipe-scram-tls-{}", ctx.run_id);
    let ca_path =
        crate::integration::cli::write_temp_file(&ctx.run_id, "kafka-scram-ca.pem", &certs.ca_pem)?;

    let cli = crate::integration::cli::ClickhousectlCli::from_env()?;
    let _ = super::create_pipe_via_cli_and_wait_running(
        &cli,
        client,
        ctx,
        ch,
        cleanup,
        &[
            "clickpipe",
            "create",
            "kafka",
            &ch.service_id,
            "--name",
            &pipe_name,
            "--brokers",
            &brokers,
            "--topics",
            &topic,
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            KAFKA_SCRAM_TLS_TARGET_TABLE,
            "--column",
            "id:Int64",
            "--column",
            "name:String",
            "--column",
            "email:String",
            "--kafka-type",
            "redpanda",
            "--consumer-group",
            &consumer_group,
            "--auth",
            "SCRAM-SHA-512",
            "--username",
            &clickpipe_user,
            "--password",
            &clickpipe_pass,
            "--ca-certificate",
            &ca_path,
            "--offset",
            "from_beginning",
            "--org-id",
            &ctx.org_id,
        ],
        clickpipe_ready_timeout,
    )
    .await?;

    verify_seed_rows(
        ch,
        KAFKA_SCRAM_TLS_TARGET_TABLE,
        KAFKA_SEED_ROW_COUNT,
        1,
        "Ada Lovelace",
        ingest_timeout,
        ctx.poll_interval,
    )
    .await
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
    let result = run_mtls_inner(client, ctx, ch, ec2, &mut cleanup, &mut aws_cleanup).await;
    StageOutcome { result, cleanup, aws_cleanup }
}

async fn run_mtls_inner(
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

    log_phase("Kafka stage (mtls): create ClickPipe (via CLI)");

    cleanup.register_table(KAFKA_MTLS_TARGET_TABLE);

    let pipe_name = format!("kafka-mtls-{}", ctx.run_id);
    let brokers = format!("{broker_ip}:9092");
    let consumer_group = format!("clickpipe-mtls-{}", ctx.run_id);
    let ca_path = crate::integration::cli::write_temp_file(
        &ctx.run_id,
        "kafka-mtls-ca.pem",
        &certs.ca_pem,
    )?;
    let client_cert_path = crate::integration::cli::write_temp_file(
        &ctx.run_id,
        "kafka-mtls-client.crt",
        &certs.client_cert_pem,
    )?;
    let client_key_path = crate::integration::cli::write_temp_file(
        &ctx.run_id,
        "kafka-mtls-client.key",
        &certs.client_key_pem,
    )?;

    let cli = crate::integration::cli::ClickhousectlCli::from_env()?;
    let _ = super::create_pipe_via_cli_and_wait_running(
        &cli,
        client,
        ctx,
        ch,
        cleanup,
        &[
            "clickpipe",
            "create",
            "kafka",
            &ch.service_id,
            "--name",
            &pipe_name,
            "--brokers",
            &brokers,
            "--topics",
            &topic,
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            KAFKA_MTLS_TARGET_TABLE,
            "--column",
            "id:Int64",
            "--column",
            "name:String",
            "--column",
            "email:String",
            // type=redpanda rejects MUTUAL_TLS; use generic kafka (broker
            // is still the Redpanda instance).
            "--kafka-type",
            "kafka",
            "--consumer-group",
            &consumer_group,
            "--auth",
            "MUTUAL_TLS",
            "--client-certificate",
            &client_cert_path,
            "--client-key",
            &client_key_path,
            "--ca-certificate",
            &ca_path,
            "--offset",
            "from_beginning",
            "--org-id",
            &ctx.org_id,
        ],
        clickpipe_ready_timeout,
    )
    .await?;

    verify_seed_rows(
        ch,
        KAFKA_MTLS_TARGET_TABLE,
        KAFKA_SEED_ROW_COUNT,
        1,
        "Ada Lovelace",
        ingest_timeout,
        ctx.poll_interval,
    )
    .await
}
