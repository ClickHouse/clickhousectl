//! Per-source E2E stages, each operating against a single shared
//! [`ProvisionedClickHouse`]. Stages take a `StageCtx` by value (owning their
//! cleanup registries) and return a `StageOutcome` so the driver can run them
//! concurrently via `tokio::join!` and merge cleanup state afterward.
//!
//! Add a new source by writing `stages/<source>.rs` with a
//! `pub async fn run_<source>_stage(StageCtx) -> StageOutcome` entry point.

// Per-source test binaries only pull in some of the stage re-exports below;
// the unused ones are intentional, not dead code.
#![allow(unused_imports)]

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::Client;

use crate::integration::support::*;

pub mod kafka;
pub mod kinesis;
pub mod mongo;
pub mod mysql;
pub mod postgres;
pub mod s3;

pub use kafka::{run_kafka_mtls_stage, run_kafka_scram_tls_stage};
pub use kinesis::run_kinesis_stage;
pub use mongo::run_mongo_stage;
pub use mysql::run_mysql_stage;
pub use postgres::run_postgres_stage;
pub use s3::run_s3_stage;

// ── Shared stage types ───────────────────────────────────────────────

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

// ── Shared helpers used by multiple stages ───────────────────────────

pub fn duration_from_env_or(name: &str, default_secs: u64) -> TestResult<Duration> {
    match std::env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}

/// Canonical 3-column managed-table destination matching the seed fixtures
/// (id Int64, name String, email String). All current stages seed the same
/// row shape so they can share this.
pub fn managed_destination_users(table: &str) -> ClickPipeMutateDestination {
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

/// Submit the pipe-create request, register the pipe for cleanup, and poll
/// until it reaches `Running`/`Completed` (or fail terminally).
pub async fn create_pipe_and_wait_running(
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

/// Wait until `default.{table}` reflects at least `expected_count` rows, then
/// spot-check a known row: `SELECT name FROM default.{table} WHERE id = {spot_id}`
/// must return `spot_name`. Most callers use `(1, "Ada Lovelace")` — the
/// standard first seed row — but the Postgres multi-mapping variant feeds
/// from `pg_users_more` (ids 101+), so the spot is parameterized.
pub async fn verify_seed_rows(
    ch: &ProvisionedClickHouse,
    table: &str,
    expected_count: i64,
    spot_id: i64,
    spot_name: &str,
    ingest_timeout: Duration,
    poll_interval: Duration,
) -> TestResult<()> {
    poll_until("seeded row count in ClickHouse", ingest_timeout, poll_interval, || {
        let query = ch.query.clone();
        let table = table.to_string();
        async move {
            match query.count_rows(&table).await {
                Ok(count) if count >= expected_count => Ok(Some(count)),
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            }
        }
    })
    .await?;
    let actual = ch
        .query
        .scalar_string(&format!(
            "SELECT name FROM default.{table} WHERE id = {spot_id} LIMIT 1"
        ))
        .await?;
    assert_eq!(
        actual.as_deref(),
        Some(spot_name),
        "id={spot_id} spot-check failed for table {table}"
    );
    Ok(())
}

/// Kafka topic names (and a few other identifiers) are constrained to
/// `[a-zA-Z0-9._-]`. `run_id` already fits, but be defensive — sanitize
/// underscores to hyphens for any identifier derived from it.
pub fn sanitize_for_topic(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '_' => '-',
            _ => c,
        })
        .collect()
}

/// Small URL-safe-ish token for SCRAM passwords / random ids. Not
/// cryptographically rigorous — just enough entropy that two parallel test
/// runs don't collide.
pub fn random_token(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
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

/// Base64-encode a string for embedding in user_data templates without
/// escaping issues (multi-line PEMs etc.).
pub fn b64(s: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    STANDARD.encode(s.as_bytes())
}
