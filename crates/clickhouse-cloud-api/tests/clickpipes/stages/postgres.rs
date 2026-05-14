#![allow(dead_code)]

//! Postgres-on-EC2 ClickPipe stage: per-test EC2 + Basic auth + CDC.
//!
//! Parallel to the MySQL and Mongo stages — launches a t3.medium with
//! PostgreSQL 16 configured for logical replication, then creates a pipe via
//! the CLI with `--replication-mode cdc` but **without** `--publication-name`
//! / `--replication-slot-name`. That's Al's `4f6c2ba` Bug 1 scenario: a
//! handler regression that sends `""` for those fields would re-trigger
//! `replicationSlotName: ''` server validation here. Distinct from
//! `clickpipes/postgres_cdc_test.rs`, which exercises Al's flow against
//! CHC-managed Postgres via the library, not the CLI.

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::support::*;

use super::{
    StageCtx, StageOutcome, b64, create_pipe_and_wait_running, duration_from_env_or, random_token,
    sanitize_for_topic, verify_seed_rows,
};

const POSTGRES_SOURCE_TABLE: &str = "pg_users";
const POSTGRES_SEED_ROW_COUNT: i64 = 3;
const POSTGRES_USER_DATA: &str = include_str!("postgres_user_data.sh.template");

/// DNS SAN baked into the Postgres server cert so the `cdc-tls-host` variant
/// can validate against a hostname while connecting to the EIP. Picked to be
/// obviously non-routable (`.test` TLD reserved by RFC 6761).
const POSTGRES_TLS_HOST_NAME: &str = "clickpipe-postgres-e2e.test";

const POSTGRES_INSTANCE_TYPE: &str = "t3.medium";
const POSTGRES_BOOT_TIMEOUT_SECS: u64 = 600;
const DEFAULT_POSTGRES_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 900;
const DEFAULT_POSTGRES_INGEST_TIMEOUT_SECS: u64 = 600;

/// One Postgres ClickPipe variant tested against the shared EC2.
struct PostgresVariant {
    label: &'static str,
    /// `(source_table, target_table, spot_id, spot_name)` quads. Each maps a
    /// Postgres table to its CH destination and pins one row we expect to see
    /// land. Most variants have one entry; the multi-mapping variant has two
    /// (different source tables, different spot rows).
    mappings: &'static [(&'static str, &'static str, i64, &'static str)],
    replication_mode: ClickPipePostgresPipeSettingsReplicationmode,
    /// `Some(name)` sets `publicationName` on the pipe settings. `None`
    /// omits it — for cdc this exercises Al's Bug 1 scenario where the
    /// server auto-creates the publication.
    publication_name: Option<&'static str>,
    /// `Some(name)` sets `tlsHost` on the source. Requires the server cert
    /// to have `name` as a DNS SAN (we add `POSTGRES_TLS_HOST_NAME` to every
    /// cert; only the `cdc-tls-host` variant actually exercises it).
    tls_host: Option<&'static str>,
}

/// `cdc_only` is intentionally not tested: it skips the snapshot phase and
/// only captures NEW WAL after the slot is created, so our user_data — which
/// seeds rows BEFORE pipe creation — would deliver 0 rows. Would need a
/// fundamentally different seed pattern (insert rows post-pipe-create) to
/// exercise meaningfully.
const POSTGRES_VARIANTS: &[PostgresVariant] = &[
    PostgresVariant {
        label: "cdc-auto",
        mappings: &[("pg_users", "postgres_cdc_auto_users", 1, "Ada Lovelace")],
        replication_mode: ClickPipePostgresPipeSettingsReplicationmode::Cdc,
        // Both flags omitted — Al's Bug 1 scenario.
        publication_name: None,
        tls_host: None,
    },
    PostgresVariant {
        label: "cdc-explicit",
        mappings: &[("pg_users", "postgres_cdc_explicit_users", 1, "Ada Lovelace")],
        replication_mode: ClickPipePostgresPipeSettingsReplicationmode::Cdc,
        // Pre-created in user_data; pipe references it by name.
        publication_name: Some("pg_pub_explicit"),
        tls_host: None,
    },
    PostgresVariant {
        label: "snapshot",
        mappings: &[("pg_users", "postgres_snapshot_users", 1, "Ada Lovelace")],
        replication_mode: ClickPipePostgresPipeSettingsReplicationmode::Snapshot,
        publication_name: None,
        tls_host: None,
    },
    PostgresVariant {
        label: "cdc-multi-mapping",
        // Two source tables → two destination tables. Spot rows differ per
        // source so the spot-check actually proves the right table got the
        // right rows (rather than passing on aliased data).
        mappings: &[
            ("pg_users", "postgres_multi_a_users", 1, "Ada Lovelace"),
            ("pg_users_more", "postgres_multi_b_users", 101, "Joan Clarke"),
        ],
        replication_mode: ClickPipePostgresPipeSettingsReplicationmode::Cdc,
        publication_name: None,
        tls_host: None,
    },
    PostgresVariant {
        label: "cdc-tls-host",
        // Connect to the EIP but ask TLS to verify against the DNS SAN we
        // baked into the cert. Catches handler regressions that drop or
        // mis-route the `--tls-host` flag in the postgres source body.
        mappings: &[("pg_users", "postgres_tls_host_users", 1, "Ada Lovelace")],
        replication_mode: ClickPipePostgresPipeSettingsReplicationmode::Cdc,
        publication_name: None,
        tls_host: Some(POSTGRES_TLS_HOST_NAME),
    },
];

/// Pre-allocate the EIP, generate the server cert with the EIP as SAN,
/// launch the EC2 with `user_data`, associate the EIP, return
/// `(public_ip, certs)`. Same shape as `launch_mysql` / `launch_mongo`.
async fn launch_postgres(
    ec2: &aws_sdk_ec2::Client,
    ctx: &TestContext,
    aws_cleanup: &mut AwsCleanupRegistry,
    user_data_with_ip: impl FnOnce(&str, &RedpandaCerts) -> String,
) -> TestResult<(String, RedpandaCerts)> {
    log_phase("Postgres stage: allocate EIP + generate certs");

    let (eip_address, allocation_id) = allocate_elastic_ip(ec2).await?;
    aws_cleanup.register_ec2_elastic_ip(allocation_id.clone());
    eprintln!("  allocated eip {eip_address}");

    // The cert generator is generic — `client_cn` is unused for Basic auth,
    // pass a placeholder.
    let certs = generate_redpanda_certs_with_dns_sans(
        &eip_address,
        "postgres-unused",
        &[POSTGRES_TLS_HOST_NAME],
    )?;

    log_phase("Postgres stage: launch PostgreSQL EC2");
    let vpc_id = default_vpc_id(ec2).await?;
    let subnet_id = first_subnet_in_vpc(ec2, &vpc_id).await?;
    let ami_id = latest_ubuntu_noble_amd64_ami(ec2).await?;

    let sg_name = format!(
        "clickhousectl-e2e-postgres-{}",
        sanitize_for_topic(&ctx.run_id)
    );
    let sg_id = create_open_security_group(ec2, &vpc_id, &sg_name, &[5432]).await?;
    aws_cleanup.register_ec2_security_group(sg_id.clone());
    eprintln!("  created security group {sg_id}");

    let user_data = user_data_with_ip(&eip_address, &certs);

    let (instance_id, _default_ip) = launch_ec2_instance(
        ec2,
        &ami_id,
        &subnet_id,
        &sg_id,
        POSTGRES_INSTANCE_TYPE,
        &user_data,
        &format!(
            "clickhousectl-e2e-postgres-{}",
            sanitize_for_topic(&ctx.run_id)
        ),
    )
    .await?;
    aws_cleanup.register_ec2_instance(instance_id.clone());

    associate_elastic_ip(ec2, &allocation_id, &instance_id).await?;
    eprintln!("  launched instance {instance_id} and associated eip {eip_address}");

    Ok((eip_address, certs))
}

/// Postgres-on-EC2 ClickPipe stage. Provisioning + CLI invocation +
/// row verification.
pub async fn run_postgres_stage(sctx: StageCtx<'_>) -> StageOutcome {
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
    StageOutcome {
        result,
        cleanup,
        aws_cleanup,
    }
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
        DEFAULT_POSTGRES_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_POSTGRES_INGEST_TIMEOUT_SECS,
    )?;

    // Postgres allows `_` in identifiers, but be defensive (matches mysql).
    let db_name = format!("e2e_{}", ctx.run_id.replace('-', "_"));
    let clickpipe_user = format!("clickpipe_{}", ctx.run_id.replace('-', "_"));
    let clickpipe_pass = random_token(32);

    let render_ud = |_ip: &str, certs: &RedpandaCerts| -> String {
        POSTGRES_USER_DATA
            .replace("__DB_NAME__", &db_name)
            .replace("__CLICKPIPE_USER__", &clickpipe_user)
            .replace("__CLICKPIPE_PASS__", &clickpipe_pass)
            .replace("__SERVER_CERT_B64__", &b64(&certs.server_cert_pem))
            .replace("__SERVER_KEY_B64__", &b64(&certs.server_key_pem))
            .replace("__CA_PEM_B64__", &b64(&certs.ca_pem))
    };

    let (host_ip, certs) = launch_postgres(ec2, ctx, aws_cleanup, render_ud).await?;

    log_phase("Postgres stage: wait for PostgreSQL to be reachable");
    wait_for_tcp_port(
        &host_ip,
        5432,
        Duration::from_secs(POSTGRES_BOOT_TIMEOUT_SECS),
    )
    .await?;
    // user_data needs time after the port opens to bootstrap the DB, user,
    // table, and seed rows. Postgres apt install is generally faster than
    // MySQL but the SQL bootstrap takes a few seconds.
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Register ALL variant target tables for teardown up-front so partial
    // failures (one variant fails) still leave every table queued for cleanup.
    for variant in POSTGRES_VARIANTS {
        for (_, target_table, _, _) in variant.mappings {
            cleanup.register_table(*target_table);
        }
    }

    // Run all variants in parallel against the same EC2 / Postgres server.
    // Each variant gets its own scratch CleanupRegistry which we merge back
    // into the caller's after `tokio::join!` resolves — same pattern as the
    // multi-stage driver. No Mutex needed because the registries are
    // disjoint until merge time.
    log_phase("Postgres stage: run all variants in parallel");

    let (out_a, out_b, out_c, out_d, out_e) = tokio::join!(
        run_variant(
            client,
            ctx,
            ch,
            &host_ip,
            &db_name,
            &clickpipe_user,
            &clickpipe_pass,
            &certs.ca_pem,
            &POSTGRES_VARIANTS[0],
            clickpipe_ready_timeout,
            ingest_timeout,
        ),
        run_variant(
            client,
            ctx,
            ch,
            &host_ip,
            &db_name,
            &clickpipe_user,
            &clickpipe_pass,
            &certs.ca_pem,
            &POSTGRES_VARIANTS[1],
            clickpipe_ready_timeout,
            ingest_timeout,
        ),
        run_variant(
            client,
            ctx,
            ch,
            &host_ip,
            &db_name,
            &clickpipe_user,
            &clickpipe_pass,
            &certs.ca_pem,
            &POSTGRES_VARIANTS[2],
            clickpipe_ready_timeout,
            ingest_timeout,
        ),
        run_variant(
            client,
            ctx,
            ch,
            &host_ip,
            &db_name,
            &clickpipe_user,
            &clickpipe_pass,
            &certs.ca_pem,
            &POSTGRES_VARIANTS[3],
            clickpipe_ready_timeout,
            ingest_timeout,
        ),
        run_variant(
            client,
            ctx,
            ch,
            &host_ip,
            &db_name,
            &clickpipe_user,
            &clickpipe_pass,
            &certs.ca_pem,
            &POSTGRES_VARIANTS[4],
            clickpipe_ready_timeout,
            ingest_timeout,
        ),
    );

    // Merge each variant's sub-registry back into the caller's so pipes get
    // torn down even when later variants fail.
    let mut variant_failures: Vec<(String, String)> = Vec::new();
    for (variant, (result, sub_cleanup)) in [
        (&POSTGRES_VARIANTS[0], out_a),
        (&POSTGRES_VARIANTS[1], out_b),
        (&POSTGRES_VARIANTS[2], out_c),
        (&POSTGRES_VARIANTS[3], out_d),
        (&POSTGRES_VARIANTS[4], out_e),
    ] {
        cleanup.merge_from(sub_cleanup);
        match result {
            Ok(()) => eprintln!("  PASS [postgres-{}]", variant.label),
            Err(err) => {
                eprintln!("  FAIL [postgres-{}]: {}", variant.label, err);
                variant_failures.push((variant.label.to_string(), err.to_string()));
            }
        }
    }

    if !variant_failures.is_empty() {
        let summary = variant_failures
            .iter()
            .map(|(name, err)| format!("    - {name}: {err}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "{} variant failure(s):\n{summary}",
            variant_failures.len()
        )
        .into());
    }

    Ok(())
}

/// Run one Postgres variant against the shared EC2: build the create
/// request, register the pipe + poll Running, then verify rows landed in
/// the variant's target table. Returns the sub-registry so the caller can
/// merge it for teardown (pipes still get cleaned up on failure).
#[allow(clippy::too_many_arguments)]
async fn run_variant(
    client: &Client,
    ctx: &TestContext,
    ch: &ProvisionedClickHouse,
    host_ip: &str,
    db_name: &str,
    clickpipe_user: &str,
    clickpipe_pass: &str,
    ca_pem: &str,
    variant: &PostgresVariant,
    clickpipe_ready_timeout: Duration,
    ingest_timeout: Duration,
) -> (TestResult<()>, CleanupRegistry) {
    let mut sub_cleanup = CleanupRegistry::default();
    let result = async {
        let pipe_request = ClickPipePostRequest {
            name: format!("postgres-{}-{}", variant.label, ctx.run_id),
            // Postgres is a "database pipe" — only `database` is valid at
            // the top level. The per-mapping `targetTable` carries the
            // destination table.
            destination: ClickPipeMutateDestination {
                database: "default".to_string(),
                ..Default::default()
            },
            source: ClickPipePostSource {
                postgres: Some(ClickPipeMutatePostgresSource {
                    r#type: Some(ClickPipeMutatePostgresSourceType::Postgres),
                    authentication: ClickPipeMutatePostgresSourceAuthentication::Basic,
                    credentials: PLAIN {
                        username: clickpipe_user.to_string(),
                        password: clickpipe_pass.to_string(),
                    },
                    host: host_ip.to_string(),
                    port: 5432,
                    database: db_name.to_string(),
                    ca_certificate: Some(ca_pem.to_string()),
                    tls_host: variant.tls_host.map(str::to_string),
                    settings: ClickPipePostgresPipeSettings {
                        replication_mode: variant.replication_mode.clone(),
                        publication_name: variant.publication_name.map(str::to_string),
                        ..Default::default()
                    },
                    table_mappings: variant
                        .mappings
                        .iter()
                        .map(|(src, dst, _, _)| ClickPipePostgresPipeTableMapping {
                            source_schema_name: "public".to_string(),
                            source_table: (*src).to_string(),
                            target_table: (*dst).to_string(),
                            table_engine:
                                ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
                            ..Default::default()
                        })
                        .collect(),
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
            &mut sub_cleanup,
            &pipe_request,
            clickpipe_ready_timeout,
        )
        .await?;

        // Verify each mapping's destination table got the expected rows.
        // Spot row varies per source table — the multi-mapping variant's
        // second target (pg_users_more) has no id=1, so it pins id=101.
        for (_, target_table, spot_id, spot_name) in variant.mappings {
            verify_seed_rows(
                ch,
                target_table,
                POSTGRES_SEED_ROW_COUNT,
                *spot_id,
                spot_name,
                ingest_timeout,
                ctx.poll_interval,
            )
            .await?;
        }
        Ok(())
    }
    .await;

    (result, sub_cleanup)
}
