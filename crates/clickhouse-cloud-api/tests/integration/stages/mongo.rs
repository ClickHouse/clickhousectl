//! MongoDB ClickPipe stage: per-test EC2 + Basic auth + CDC (snapshot + change
//! streams). Mirrors `mysql.rs` — an EC2 in the test AWS account hosts a
//! single-node MongoDB replica set, with TLS terminated against a self-signed
//! cert whose SAN matches the pre-allocated EIP that ClickPipes connects to.
//!
//! Mongo CDC requires a replica set (change streams don't exist on
//! standalone mongod), so the user_data runs `rs.initiate()` and waits for
//! PRIMARY before creating users and seeding data.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

use super::{
    StageCtx, StageOutcome, b64, create_pipe_and_wait_running, duration_from_env_or, random_token,
    sanitize_for_topic, verify_seed_rows,
};

const MONGO_TARGET_TABLE_PREFIX: &str = "mongo_users_";
const MONGO_SOURCE_COLLECTION: &str = "users";
const MONGO_SEED_ROW_COUNT: i64 = 3;
const MONGO_USER_DATA: &str = include_str!("mongo_user_data.sh.template");

const MONGO_INSTANCE_TYPE: &str = "t3.medium";
const MONGO_BOOT_TIMEOUT_SECS: u64 = 600;
const DEFAULT_MONGO_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 900;
const DEFAULT_MONGO_INGEST_TIMEOUT_SECS: u64 = 600;

/// Pre-allocate the EIP, generate the server cert with the EIP as SAN, launch
/// the EC2 with `user_data`, associate the EIP, and return `(public_ip, certs)`.
async fn launch_mongo(
    ec2: &aws_sdk_ec2::Client,
    ctx: &TestContext,
    aws_cleanup: &mut AwsCleanupRegistry,
    user_data_with_ip: impl FnOnce(&str, &RedpandaCerts) -> String,
) -> TestResult<(String, RedpandaCerts)> {
    log_phase("Mongo stage: allocate EIP + generate certs");

    let (eip_address, allocation_id) = allocate_elastic_ip(ec2).await?;
    aws_cleanup.register_ec2_elastic_ip(allocation_id.clone());
    eprintln!("  allocated eip {eip_address}");

    // The cert generator is generic — the "client_cn" arg is unused by Mongo
    // (we authenticate with username + password), so pass a placeholder.
    let certs = generate_redpanda_certs(&eip_address, "mongo-unused")?;

    log_phase("Mongo stage: launch MongoDB EC2");
    let vpc_id = default_vpc_id(ec2).await?;
    let subnet_id = first_subnet_in_vpc(ec2, &vpc_id).await?;
    let ami_id = latest_ubuntu_noble_amd64_ami(ec2).await?;

    let sg_name = format!("clickhousectl-e2e-mongo-{}", sanitize_for_topic(&ctx.run_id));
    let sg_id = create_open_security_group(ec2, &vpc_id, &sg_name, &[27017]).await?;
    aws_cleanup.register_ec2_security_group(sg_id.clone());
    eprintln!("  created security group {sg_id}");

    let user_data = user_data_with_ip(&eip_address, &certs);

    let (instance_id, _default_ip) = launch_ec2_instance(
        ec2,
        &ami_id,
        &subnet_id,
        &sg_id,
        MONGO_INSTANCE_TYPE,
        &user_data,
        &format!("clickhousectl-e2e-mongo-{}", sanitize_for_topic(&ctx.run_id)),
    )
    .await?;
    aws_cleanup.register_ec2_instance(instance_id.clone());

    associate_elastic_ip(ec2, &allocation_id, &instance_id).await?;
    eprintln!("  launched instance {instance_id} and associated eip {eip_address}");

    Ok((eip_address, certs))
}

/// MongoDB Basic auth ClickPipe stage. Snapshot + CDC against the seeded
/// `users` collection in `e2e_{run_id}`.
pub async fn run_mongo_stage(sctx: StageCtx<'_>) -> StageOutcome {
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
        DEFAULT_MONGO_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_MONGO_INGEST_TIMEOUT_SECS,
    )?;

    // Mongo database names accept `_`; sanitize underscores defensively.
    let db_name = format!("e2e_{}", ctx.run_id.replace('-', "_"));
    let admin_user = format!("admin_{}", ctx.run_id.replace('-', "_"));
    let admin_pass = random_token(32);
    // Per-run target table so a shared CHC service doesn't collide on
    // re-runs (ClickPipes rejects pipe-create against an existing non-empty
    // table).
    let target_table = format!(
        "{}{}",
        MONGO_TARGET_TABLE_PREFIX,
        ctx.run_id.replace('-', "_")
    );
    let clickpipe_user = format!("clickpipe_{}", ctx.run_id.replace('-', "_"));
    let clickpipe_pass = random_token(32);

    let render_ud = |ip: &str, certs: &RedpandaCerts| -> String {
        MONGO_USER_DATA
            .replace("__EIP_HOST__", ip)
            .replace("__DB_NAME__", &db_name)
            .replace("__ADMIN_USER__", &admin_user)
            .replace("__ADMIN_PASS__", &admin_pass)
            .replace("__CLICKPIPE_USER__", &clickpipe_user)
            .replace("__CLICKPIPE_PASS__", &clickpipe_pass)
            .replace("__SERVER_CERT_B64__", &b64(&certs.server_cert_pem))
            .replace("__SERVER_KEY_B64__", &b64(&certs.server_key_pem))
            .replace("__CA_PEM_B64__", &b64(&certs.ca_pem))
    };

    let (host_ip, certs) = launch_mongo(ec2, ctx, aws_cleanup, render_ud).await?;

    log_phase("Mongo stage: wait for MongoDB to be reachable");
    // mongod restarts mid-bootstrap (auth enable). A single-success TCP probe
    // would race that restart and let ClickPipes hit it during the brief
    // down window. Require 12 consecutive TCP successes ~5 s apart (~60 s of
    // continuous availability), which spans both restarts and gives the
    // bootstrap script time to seed data + create the clickpipe user.
    wait_for_stable_tcp_port(
        &host_ip,
        27017,
        12,
        Duration::from_secs(MONGO_BOOT_TIMEOUT_SECS),
    )
    .await?;

    log_phase("Mongo stage: create ClickPipe");
    // Mongo URI carries the host:port and (optionally) credentials. We pass
    // credentials separately in the `credentials` field so they don't end up
    // in API logs as part of the URI; the URI itself is `mongodb://host:port`.
    let mongo_uri = format!("mongodb://{host_ip}:27017");
    let pipe_request = ClickPipePostRequest {
        name: format!("mongo-{}", ctx.run_id),
        // Mongo is a "database pipe" — only `database` is valid at the top
        // level. The per-mapping `targetTable` carries the destination table.
        destination: ClickPipeMutateDestination {
            database: "default".to_string(),
            ..Default::default()
        },
        source: ClickPipePostSource {
            mongodb: Some(ClickPipeMutateMongoDBSource {
                uri: mongo_uri,
                credentials: Some(PLAIN {
                    username: clickpipe_user.clone(),
                    password: clickpipe_pass.clone(),
                }),
                ca_certificate: Some(certs.ca_pem.clone()),
                read_preference: ClickPipeMutateMongoDBSourceReadpreference::Primary,
                settings: ClickPipeMongoDBPipeSettings {
                    // `cdc` = snapshot + change-stream tailing, matching
                    // the MySQL/Postgres CDC stages.
                    replication_mode: ClickPipeMongoDBPipeSettingsReplicationmode::Cdc,
                    ..Default::default()
                },
                table_mappings: vec![ClickPipeMongoDBPipeTableMapping {
                    source_database_name: db_name.clone(),
                    source_collection: MONGO_SOURCE_COLLECTION.to_string(),
                    target_table: target_table.clone(),
                    table_engine: Some(
                        ClickPipeMongoDBPipeTableMappingTableengine::ReplacingMergeTree,
                    ),
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

    // Mongo destination tables don't have the standard `id/name/email`
    // columns the other stages do — ClickPipes maps Mongo docs into a
    // schema with `_id` + a JSON blob of the document. So we can't reuse
    // `verify_seed_rows` (which spot-checks `WHERE id = 1`). Verify row
    // count only; field-level integrity could be added later by extracting
    // from the JSON column once we know the column name.
    poll_until(
        "seeded row count in ClickHouse",
        ingest_timeout,
        ctx.poll_interval,
        || {
            let query = ch.query.clone();
            let target_table = target_table.clone();
            async move {
                match query.count_rows(&target_table).await {
                    Ok(count) if count >= MONGO_SEED_ROW_COUNT => Ok(Some(count)),
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
        },
    )
    .await?;
    Ok(())
}
