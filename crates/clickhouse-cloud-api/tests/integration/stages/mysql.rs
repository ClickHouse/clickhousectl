//! MySQL ClickPipe stage: per-test EC2 + Basic auth + CDC (snapshot + binlog
//! replication). Closest in shape to the Kafka stages — an EC2 in the test
//! AWS account hosts MySQL 8.0, with TLS terminated against a self-signed cert
//! whose SAN matches the pre-allocated EIP that ClickPipes connects to.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

use super::{
    StageCtx, StageOutcome, b64, create_pipe_and_wait_running, duration_from_env_or, random_token,
    sanitize_for_topic, verify_seed_rows,
};

const MYSQL_TARGET_TABLE: &str = "mysql_users";
const MYSQL_SOURCE_TABLE: &str = "users";
const MYSQL_SEED_ROW_COUNT: i64 = 3;
const MYSQL_USER_DATA: &str = include_str!("mysql_user_data.sh.template");

const MYSQL_INSTANCE_TYPE: &str = "t3.medium";
const MYSQL_BOOT_TIMEOUT_SECS: u64 = 600;
const DEFAULT_MYSQL_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 900;
const DEFAULT_MYSQL_INGEST_TIMEOUT_SECS: u64 = 600;

/// Pre-allocate the EIP, generate the server cert with the EIP as SAN, launch
/// the EC2 with `user_data`, associate the EIP, and return `(public_ip, certs)`.
async fn launch_mysql(
    ec2: &aws_sdk_ec2::Client,
    ctx: &TestContext,
    aws_cleanup: &mut AwsCleanupRegistry,
    user_data_with_ip: impl FnOnce(&str, &RedpandaCerts) -> String,
) -> TestResult<(String, RedpandaCerts)> {
    log_phase("MySQL stage: allocate EIP + generate certs");

    let (eip_address, allocation_id) = allocate_elastic_ip(ec2).await?;
    aws_cleanup.register_ec2_elastic_ip(allocation_id.clone());
    eprintln!("  allocated eip {eip_address}");

    // The cert generator is generic — the "client_cn" arg is unused by MySQL
    // (we authenticate with username + password), so pass a placeholder.
    let certs = generate_redpanda_certs(&eip_address, "mysql-unused")?;

    log_phase("MySQL stage: launch MySQL EC2");
    let vpc_id = default_vpc_id(ec2).await?;
    let subnet_id = first_subnet_in_vpc(ec2, &vpc_id).await?;
    let ami_id = latest_ubuntu_noble_amd64_ami(ec2).await?;

    let sg_name = format!("clickhousectl-e2e-mysql-{}", sanitize_for_topic(&ctx.run_id));
    let sg_id = create_open_security_group(ec2, &vpc_id, &sg_name, &[3306]).await?;
    aws_cleanup.register_ec2_security_group(sg_id.clone());
    eprintln!("  created security group {sg_id}");

    let user_data = user_data_with_ip(&eip_address, &certs);

    let (instance_id, _default_ip) = launch_ec2_instance(
        ec2,
        &ami_id,
        &subnet_id,
        &sg_id,
        MYSQL_INSTANCE_TYPE,
        &user_data,
        &format!("clickhousectl-e2e-mysql-{}", sanitize_for_topic(&ctx.run_id)),
    )
    .await?;
    aws_cleanup.register_ec2_instance(instance_id.clone());

    associate_elastic_ip(ec2, &allocation_id, &instance_id).await?;
    eprintln!("  launched instance {instance_id} and associated eip {eip_address}");

    Ok((eip_address, certs))
}

/// MySQL Basic auth ClickPipe stage. Snapshot + CDC against the seeded
/// `users` table.
pub async fn run_mysql_stage(sctx: StageCtx<'_>) -> StageOutcome {
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
    let result = run_inner(client, ctx, ch, ec2, &mut cleanup, &mut aws_cleanup).await;
    StageOutcome { result, cleanup, aws_cleanup }
}

async fn run_inner(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    ec2: &aws_sdk_ec2::Client,
    cleanup: &mut CleanupRegistry,
    aws_cleanup: &mut AwsCleanupRegistry,
) -> TestResult<()> {
    let clickpipe_ready_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CLICKPIPE_READY_SECS",
        DEFAULT_MYSQL_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_MYSQL_INGEST_TIMEOUT_SECS,
    )?;

    // MySQL database names accept `_` so we use a raw sanitized run_id; the
    // sanitize step also defends if run_id ever picks up non-DB-safe chars.
    let db_name = format!("e2e_{}", ctx.run_id.replace('-', "_"));
    let clickpipe_user = format!("clickpipe_{}", ctx.run_id.replace('-', "_"));
    let clickpipe_pass = random_token(32);

    let render_ud = |ip: &str, certs: &RedpandaCerts| -> String {
        let _ = ip; // baked into the cert SAN; MySQL itself binds 0.0.0.0
        MYSQL_USER_DATA
            .replace("__DB_NAME__", &db_name)
            .replace("__CLICKPIPE_USER__", &clickpipe_user)
            .replace("__CLICKPIPE_PASS__", &clickpipe_pass)
            .replace("__SERVER_CERT_B64__", &b64(&certs.server_cert_pem))
            .replace("__SERVER_KEY_B64__", &b64(&certs.server_key_pem))
            .replace("__CA_PEM_B64__", &b64(&certs.ca_pem))
    };

    let (host_ip, certs) = launch_mysql(ec2, ctx, aws_cleanup, render_ud).await?;

    log_phase("MySQL stage: wait for MySQL to be reachable");
    wait_for_tcp_port(&host_ip, 3306, Duration::from_secs(MYSQL_BOOT_TIMEOUT_SECS)).await?;
    // The TCP port opens before `mysqld` finishes processing the bootstrap
    // SQL (user creation, GRANTs). Give it a buffer.
    tokio::time::sleep(Duration::from_secs(45)).await;

    log_phase("MySQL stage: create ClickPipe");
    let pipe_request = ClickPipePostRequest {
        name: format!("mysql-{}", ctx.run_id),
        // MySQL is a "database pipe" — only `database` is valid at the top
        // level. The per-mapping `targetTable` carries the destination table.
        destination: ClickPipeMutateDestination {
            database: "default".to_string(),
            ..Default::default()
        },
        source: ClickPipePostSource {
            mysql: Some(ClickPipeMutateMySQLSource {
                r#type: Some(ClickPipeMutateMySQLSourceType::Mysql),
                authentication: Some(ClickPipeMutateMySQLSourceAuthentication::Basic),
                credentials: Some(PLAIN {
                    username: clickpipe_user.clone(),
                    password: clickpipe_pass.clone(),
                }),
                host: host_ip.clone(),
                port: 3306,
                ca_certificate: Some(certs.ca_pem.clone()),
                settings: ClickPipeMySQLPipeSettings {
                    // `cdc` = snapshot + ongoing binlog replication, matching
                    // the Postgres CDC test's shape.
                    replication_mode: ClickPipeMySQLPipeSettingsReplicationmode::Cdc,
                    // GTID is enabled on the source (see user_data); the
                    // alternative is FILE_POS, which we don't need.
                    replication_mechanism: Some(
                        ClickPipeMySQLPipeSettingsReplicationmechanism::GTID,
                    ),
                    ..Default::default()
                },
                table_mappings: vec![ClickPipeMySQLPipeTableMapping {
                    // For MySQL, `sourceSchemaName` is the database name.
                    source_schema_name: db_name.clone(),
                    source_table: MYSQL_SOURCE_TABLE.to_string(),
                    target_table: MYSQL_TARGET_TABLE.to_string(),
                    table_engine: Some(
                        ClickPipeMySQLPipeTableMappingTableengine::ReplacingMergeTree,
                    ),
                    ..Default::default()
                }],
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

    verify_seed_rows(
        ch,
        MYSQL_TARGET_TABLE,
        MYSQL_SEED_ROW_COUNT,
        ingest_timeout,
        ctx.poll_interval,
    )
    .await
}
