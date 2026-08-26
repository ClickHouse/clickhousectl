//! Live managed-Postgres CDC coverage through the real `clickhousectl` binary.
//!
//! The managed source uses a private CA, so this test cannot cover successful
//! creation without `--ca-certificate`. Success against a publicly trusted
//! PostgreSQL source remains a residual gap until CI provides such a fixture.

#[path = "../common/mod.rs"]
mod common;
mod support;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str::FromStr;
use std::time::Duration;

use clickhouse_cloud_api::models::*;
use rustls::RootCertStore;
use support::*;
use tokio_postgres_rustls::MakeRustlsConnect;

const SOURCE_SCHEMA: &str = "public";
const SOURCE_TABLE: &str = "cdc_users";
const TARGET_TABLE: &str = "cdc_users";
const PUBLICATION: &str = "clickpipe_pub";
const SEED_ROW_COUNT: i64 = 3;
const POST_SEED_ROW_COUNT: i64 = 8;

const DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 600;
const DEFAULT_CDC_LAG_TIMEOUT_SECS: u64 = 300;
const CLICKHOUSECTL_BIN_ENV: &str = "CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN";
const PRIVATE_CA_API_ERROR: &str = "x509: certificate signed by unknown authority";

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and provisions real resources"]
async fn cloud_clickpipe_postgres_cli_cdc() -> TestResult<()> {
    // rustls 0.23 requires a default CryptoProvider be installed before any
    // ClientConfig is constructed. install_default returns an Err if one was
    // already set by another test in the same process, which is harmless here.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ctx = TestContext::from_env()?;
    let clickhousectl = clickhousectl_binary()?;
    let cli_workspace = tempfile::tempdir()?;
    let cli_home = cli_workspace.path().join("home");
    std::fs::create_dir(&cli_home)?;
    let cli_api_url = clickhouse_cloud_api_url();
    let clickpipe_ready_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CLICKPIPE_READY_SECS",
        DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let cdc_lag_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CDC_LAG_SECS",
        DEFAULT_CDC_LAG_TIMEOUT_SECS,
    )?;

    let client = create_client()?;
    let mut cleanup = CleanupRegistry::default();

    let test_result = async {
        log_run_header("cloud_clickpipe_postgres_cli_cdc", &ctx);
        let mut failures = FailureRecorder::default();

        // ── Preflight ───────────────────────────────────────────────

        log_phase("Preflight");

        let leftover_services = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "check for leftover tagged ClickHouse services",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let filters = ctx.clickpipe_run_tag_filters();
                    async move {
                        let filter_refs: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
                        let resp = client.instance_get_list(&org_id, &filter_refs).await?;
                        resp.result
                            .ok_or_else(|| "service list returned no result".into())
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            leftover_services.is_empty(),
            "found an existing tagged ClickHouse service for this run id before create"
        );

        let leftover_postgres = failures
            .run(
                &ctx,
                StepKind::Blocking,
                "check for leftover tagged Postgres services",
                || {
                    let client = client.clone();
                    let org_id = ctx.org_id.clone();
                    let filters = ctx.clickpipe_run_tag_filters();
                    async move {
                        let resp = client.postgres_service_get_list(&org_id).await?;
                        let services = resp.result.ok_or("postgres list returned no result")?;
                        let leftover: Vec<_> = services
                            .into_iter()
                            .filter(|s| {
                                filters_match_tags(&filters, s.tags.as_deref().unwrap_or_default())
                            })
                            .collect();
                        Ok(leftover)
                    }
                },
            )
            .await?
            .expect("blocking steps always return a value");
        assert!(
            leftover_postgres.is_empty(),
            "found an existing tagged Postgres service for this run id before create"
        );

        // ── Provision both services in parallel ─────────────────────

        log_phase("Provision Postgres + ClickHouse");

        let pg_create_body = PostgresServicePostRequest {
            name: ctx.clickpipe_postgres_service_name(),
            provider: PgProvider::Unknown(ctx.provider.clone()),
            region: ctx.region.clone(),
            size: PgSize::R8gd_medium,
            tags: Some(ctx.clickpipe_run_tags()),
            ..Default::default()
        };

        let ch_create_body = ServicePostRequest {
            name: ctx.clickpipe_service_name(),
            provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
            region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
            min_replica_memory_gb: Some(8.0),
            max_replica_memory_gb: Some(8.0),
            num_replicas: Some(1),
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(5.0),
            // Test runner needs to query the service to verify CDC results;
            // the provisioned IPs are unknown ahead of time and the resource
            // is short-lived + tagged for this run only.
            ip_access_list: vec![IpAccessListEntry {
                source: "0.0.0.0/0".to_string(),
                description: Some("clickpipe-cdc integration test".to_string()),
            }],
            tags: Some(ctx.clickpipe_run_tags()),
            ..Default::default()
        };

        log_step("provision Postgres + ClickHouse in parallel");
        let pg_create_fut = {
            let client = client.clone();
            let org_id = ctx.org_id.clone();
            let body = pg_create_body.clone();
            async move { client.postgres_service_create(&org_id, &body).await }
        };
        let ch_create_fut = {
            let client = client.clone();
            let org_id = ctx.org_id.clone();
            let body = ch_create_body.clone();
            async move { client.instance_create(&org_id, &body).await }
        };
        let (pg_create_resp, ch_create_resp) = tokio::join!(pg_create_fut, ch_create_fut);

        let pg_created = pg_create_resp?
            .result
            .ok_or("postgres create returned no result")?;
        let postgres_id = pg_created
            .id
            .ok_or("postgres create returned no id")?
            .to_string();
        // The create response is the only API surface guaranteed to return
        // credentials: from July 31, 2026 the get endpoint stops echoing
        // `password` and `connectionString`, so capture everything
        // credential-derived here rather than from the polled get below.
        let pg_username = pg_created.username.clone().unwrap_or_default();
        let pg_password = pg_created.password.clone().unwrap_or_default();
        let pg_connection_string = pg_created.connection_string.clone().unwrap_or_default();
        assert!(
            !pg_connection_string.is_empty(),
            "postgres create returned no connection string"
        );
        let pg_port = parse_pg_port(&pg_connection_string).unwrap_or(5432);
        let pg_database =
            parse_pg_database(&pg_connection_string).unwrap_or_else(|| "postgres".to_string());
        assert!(
            !pg_username.is_empty(),
            "postgres create returned no username"
        );
        assert!(
            !pg_password.is_empty(),
            "postgres create returned no password"
        );
        cleanup.register_postgres(postgres_id.clone());
        eprintln!("  provisioned postgres id <redacted>");

        let ch_created = ch_create_resp?
            .result
            .ok_or("service create returned no result")?;
        let clickhouse_service = require_field(ch_created.service, "service")?;
        let clickhouse_id = require_field(clickhouse_service.id, "service.id")?.to_string();
        let clickhouse_password = require_field(ch_created.password.clone(), "password")?;
        cleanup.register_service(clickhouse_id.clone());
        eprintln!("  provisioned clickhouse id <redacted>");

        log_phase("Wait for steady state");

        let pg_ready_fut = poll_until(
            "postgres running state",
            ctx.steady_state_timeout,
            ctx.poll_interval,
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let postgres_id = postgres_id.clone();
                async move {
                    let resp = client.postgres_service_get(&org_id, &postgres_id).await?;
                    let svc = resp.result.ok_or("postgres get returned no result")?;
                    if svc
                        .state
                        .as_ref()
                        .is_some_and(|state| state.to_string() == "running")
                    {
                        Ok(Some(svc))
                    } else {
                        Ok(None)
                    }
                }
            },
        );
        let ch_ready_fut = poll_until(
            "clickhouse steady state",
            ctx.steady_state_timeout,
            ctx.poll_interval,
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let clickhouse_id = clickhouse_id.clone();
                async move {
                    let resp = client.instance_get(&org_id, &clickhouse_id).await?;
                    let svc = resp.result.ok_or("service get returned no result")?;
                    let state = service_state(&svc);
                    if matches!(state.as_str(), "running" | "idle") {
                        Ok(Some(svc))
                    } else {
                        Ok(None)
                    }
                }
            },
        );
        let (pg_ready, ch_ready) = tokio::try_join!(pg_ready_fut, ch_ready_fut)?;

        let pg_hostname = pg_ready.hostname.clone().unwrap_or_default();
        assert!(!pg_hostname.is_empty(), "postgres get returned no hostname");
        let ch_endpoint = https_endpoint(&ch_ready)?;
        let ch_username = ch_endpoint
            .username
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // ── Configure Postgres for CDC ──────────────────────────────

        log_phase("Configure Postgres CDC");

        let pg_ca_pem = client
            .postgres_service_certs_get(&ctx.org_id, &postgres_id)
            .await?;
        assert!(
            !pg_ca_pem.trim().is_empty(),
            "postgres certificates endpoint returned an empty CA bundle"
        );
        let pg_client = connect_postgres(
            &pg_hostname,
            pg_port,
            &pg_database,
            &pg_username,
            &pg_password,
            Some(&pg_ca_pem),
        )
        .await?;
        configure_pg_for_cdc(&pg_client).await?;

        // ── Create ClickPipe ────────────────────────────────────────

        log_phase("Create ClickPipe");

        let ca_path = cli_workspace.path().join("postgres-ca.pem");
        std::fs::write(&ca_path, &pg_ca_pem)?;
        let cli_create = PostgresCliCreate {
            api_url: &cli_api_url,
            org_id: &ctx.org_id,
            service_id: &clickhouse_id,
            host: &pg_hostname,
            port: pg_port,
            database: &pg_database,
            username: &pg_username,
            password: &pg_password,
        };

        let no_ca_name = format!("cdc-no-ca-{}", ctx.run_id);
        let no_ca_output = invoke_postgres_cli(
            &clickhousectl,
            cli_workspace.path(),
            &cli_home,
            &cli_create,
            &no_ca_name,
            None,
        )?;
        if no_ca_output.status.success() {
            if let Ok(unexpected_pipe) = serde_json::from_slice::<ClickPipe>(&no_ca_output.stdout)
                && let Ok(unexpected_id) = clickpipe_id(&unexpected_pipe)
            {
                cleanup.register_clickpipe(clickhouse_id.clone(), unexpected_id);
            }
            return Err("clickpipe creation without the private CA unexpectedly succeeded".into());
        }
        if no_ca_output.status.code() != Some(1) {
            return Err(cli_failure("private-CA rejection", &no_ca_output).into());
        }
        let no_ca_stderr = String::from_utf8_lossy(&no_ca_output.stderr);
        let api_error_position = no_ca_stderr.find(PRIVATE_CA_API_ERROR).ok_or_else(|| {
            cli_failure(
                "private-CA rejection did not preserve the API x509 error",
                &no_ca_output,
            )
        })?;
        let hint_position = no_ca_stderr
            .find("Hint: The source certificate chain is not publicly trusted")
            .ok_or_else(|| {
                cli_failure(
                    "private-CA rejection did not include the CLI guidance",
                    &no_ca_output,
                )
            })?;
        assert!(
            api_error_position < hint_position,
            "the original API error must be preserved before the CLI hint"
        );
        assert!(
            no_ca_stderr.contains("--ca-certificate <PATH>"),
            "private-CA guidance omitted the required flag: {no_ca_stderr}"
        );

        let pipe_name = format!("cdc-{}", ctx.run_id);
        let create_output = invoke_postgres_cli(
            &clickhousectl,
            cli_workspace.path(),
            &cli_home,
            &cli_create,
            &pipe_name,
            Some(&ca_path),
        )?;
        if !create_output.status.success() {
            return Err(cli_failure("create with the fetched CA", &create_output).into());
        }
        let pipe: ClickPipe = serde_json::from_slice(&create_output.stdout).map_err(|error| {
            format!("clickhousectl create output was not a ClickPipe JSON object: {error}")
        })?;
        assert_eq!(pipe.name.as_deref(), Some(pipe_name.as_str()));
        let clickpipe_id = clickpipe_id(&pipe)?;
        cleanup.register_clickpipe(clickhouse_id.clone(), clickpipe_id.clone());
        eprintln!("  provisioned clickpipe id <redacted>");

        // ── Wait for ClickPipe to reach Running state ───────────────

        log_phase("Wait for ClickPipe Running");

        let _running_pipe = poll_until(
            "clickpipe Running state",
            clickpipe_ready_timeout,
            ctx.poll_interval,
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let clickhouse_id = clickhouse_id.clone();
                let clickpipe_id = clickpipe_id.clone();
                async move {
                    let resp = client
                        .click_pipe_get(&org_id, &clickhouse_id, &clickpipe_id)
                        .await?;
                    let pipe = resp.result.ok_or("clickpipe get returned no result")?;
                    match pipe.state {
                        Some(ClickPipeState::Running) => Ok(Some(pipe)),
                        Some(ClickPipeState::Failed) | Some(ClickPipeState::InternalError) => {
                            Err(format!(
                                "clickpipe entered terminal failure state {}",
                                clickpipe_state(&pipe)
                            )
                            .into())
                        }
                        _ => Ok(None),
                    }
                }
            },
        )
        .await?;

        // ── Verify seed rows replicated ─────────────────────────────

        log_phase("Verify seed rows in ClickHouse");

        let ch_query = ClickHouseQuery::new(
            &require_field(ch_endpoint.host.clone(), "endpoints[].host")?,
            require_field(ch_endpoint.port, "endpoints[].port")? as u16,
            &ch_username,
            &clickhouse_password,
        );

        poll_until(
            "seed row count in ClickHouse",
            cdc_lag_timeout,
            ctx.poll_interval,
            || {
                let ch_query = ch_query.clone();
                async move {
                    match ch_query.count_rows(TARGET_TABLE).await {
                        Ok(count) if count >= SEED_ROW_COUNT => Ok(Some(count)),
                        Ok(_) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
            },
        )
        .await?;

        let alice_name = ch_query
            .scalar_string(&format!(
                "SELECT name FROM default.{TARGET_TABLE} WHERE id = 1 LIMIT 1"
            ))
            .await?;
        assert_eq!(
            alice_name.as_deref(),
            Some("alice"),
            "row id=1 spot-check failed"
        );

        // ── Insert more rows and verify ongoing CDC ─────────────────

        log_phase("Verify ongoing CDC");

        pg_client
            .execute(
                "INSERT INTO cdc_users (id, name) VALUES \
                 (4, 'dave'), (5, 'eve'), (6, 'frank'), (7, 'grace'), (8, 'henry')",
                &[],
            )
            .await?;

        poll_until(
            "post-seed row count in ClickHouse",
            cdc_lag_timeout,
            ctx.poll_interval,
            || {
                let ch_query = ch_query.clone();
                async move {
                    match ch_query.count_rows(TARGET_TABLE).await {
                        Ok(count) if count >= POST_SEED_ROW_COUNT => Ok(Some(count)),
                        Ok(_) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
            },
        )
        .await?;

        // ── Explicit teardown ───────────────────────────────────────

        log_phase("Teardown");

        client
            .click_pipe_delete(&ctx.org_id, &clickhouse_id, &clickpipe_id)
            .await?;
        cleanup.unregister_clickpipe(&clickhouse_id, &clickpipe_id);

        failures.finish()
    }
    .await;

    let cleanup_result = cleanup
        .cleanup(
            &client,
            &ctx.org_id,
            ctx.delete_timeout,
            ctx.poll_interval,
            None,
        )
        .await;

    match (test_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error.into()),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}\ncleanup failed:\n{cleanup_error}").into())
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn log_step(message: &str) {
    eprintln!("  step: {message}");
}

fn duration_from_env_or(name: &str, default_secs: u64) -> TestResult<Duration> {
    match std::env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}

fn clickhousectl_binary() -> TestResult<PathBuf> {
    let path = std::env::var_os(CLICKHOUSECTL_BIN_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{CLICKHOUSECTL_BIN_ENV} must point to the built clickhousectl"))?;
    if !path.is_file() {
        return Err(format!(
            "{CLICKHOUSECTL_BIN_ENV} does not point to a file: {}",
            path.display()
        )
        .into());
    }
    Ok(path)
}

fn clickhouse_cloud_api_url() -> String {
    std::env::var("CLICKHOUSE_CLOUD_API_BASE_URL")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.clickhouse.cloud".to_string())
}

struct PostgresCliCreate<'a> {
    api_url: &'a str,
    org_id: &'a str,
    service_id: &'a str,
    host: &'a str,
    port: u16,
    database: &'a str,
    username: &'a str,
    password: &'a str,
}

impl PostgresCliCreate<'_> {
    fn args(&self, name: &str, ca_certificate: Option<&Path>) -> Vec<OsString> {
        let mut args = vec![
            "cloud".into(),
            "--url".into(),
            self.api_url.into(),
            "--json".into(),
            "clickpipe".into(),
            "create".into(),
            "postgres".into(),
            self.service_id.into(),
            "--name".into(),
            name.into(),
            "--host".into(),
            self.host.into(),
            "--port".into(),
            self.port.to_string().into(),
            "--pg-database".into(),
            self.database.into(),
            "--username".into(),
            self.username.into(),
            "--password".into(),
            self.password.into(),
            "--auth".into(),
            "basic".into(),
            "--replication-mode".into(),
            "cdc".into(),
            "--publication-name".into(),
            PUBLICATION.into(),
            "--table-mapping".into(),
            format!("{SOURCE_SCHEMA}.{SOURCE_TABLE}:{TARGET_TABLE}").into(),
            "--org-id".into(),
            self.org_id.into(),
        ];
        if let Some(path) = ca_certificate {
            args.push("--ca-certificate".into());
            args.push(path.as_os_str().to_owned());
        }
        args
    }
}

fn invoke_postgres_cli(
    binary: &Path,
    workdir: &Path,
    home: &Path,
    create: &PostgresCliCreate<'_>,
    name: &str,
    ca_certificate: Option<&Path>,
) -> TestResult<Output> {
    Ok(Command::new(binary)
        .current_dir(workdir)
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .args(create.args(name, ca_certificate))
        .output()?)
}

fn cli_failure(action: &str, output: &Output) -> String {
    format!(
        "clickhousectl {action} exited {}\nstderr:\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn postgres_cli_args_only_pass_the_ca_path_when_requested() {
    let create = PostgresCliCreate {
        api_url: "https://api.example.test",
        org_id: "org-id",
        service_id: "service-id",
        host: "postgres.example.test",
        port: 5432,
        database: "postgres",
        username: "clickpipe",
        password: "secret",
    };

    let without_ca = create.args("without-ca", None);
    assert!(!without_ca.iter().any(|arg| arg == "--ca-certificate"));
    assert_eq!(
        without_ca,
        vec![
            "cloud",
            "--url",
            "https://api.example.test",
            "--json",
            "clickpipe",
            "create",
            "postgres",
            "service-id",
            "--name",
            "without-ca",
            "--host",
            "postgres.example.test",
            "--port",
            "5432",
            "--pg-database",
            "postgres",
            "--username",
            "clickpipe",
            "--password",
            "secret",
            "--auth",
            "basic",
            "--replication-mode",
            "cdc",
            "--publication-name",
            PUBLICATION,
            "--table-mapping",
            "public.cdc_users:cdc_users",
            "--org-id",
            "org-id",
        ]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>()
    );

    let ca_path = Path::new("/tmp/postgres-ca.pem");
    let with_ca = create.args("with-ca", Some(ca_path));
    assert_eq!(
        &with_ca[with_ca.len() - 2..],
        [OsString::from("--ca-certificate"), ca_path.into()]
    );
}

fn filters_match_tags(filters: &[String], tags: &[ResourceTagsV1Response]) -> bool {
    filters.iter().all(|filter| {
        let Some(expr) = filter.strip_prefix("tag:") else {
            return true;
        };
        let Some((key, value)) = expr.split_once('=') else {
            return tags.iter().any(|t| t.key.as_deref() == Some(expr));
        };
        tags.iter()
            .any(|t| t.key.as_deref() == Some(key) && t.value.as_deref() == Some(value))
    })
}

// Takes discrete connection parts rather than a connection string: the get
// endpoint stops echoing `connectionString` on July 31, 2026, so callers
// assemble host/port/database/credentials from the create response + get.
async fn connect_postgres(
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: &str,
    extra_ca_pem: Option<&str>,
) -> TestResult<tokio_postgres::Client> {
    let mut roots = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    if let Some(pem) = extra_ca_pem {
        let mut reader = std::io::BufReader::new(pem.as_bytes());
        for cert in rustls_pemfile::certs(&mut reader) {
            let cert = cert?;
            roots.add(cert)?;
        }
    }
    let client_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tls = MakeRustlsConnect::new(client_config);

    let mut config = tokio_postgres::Config::new();
    config
        .host(host)
        .port(port)
        .dbname(database)
        .user(username)
        .password(password);
    // Cloud Postgres requires TLS; force require.
    config.ssl_mode(tokio_postgres::config::SslMode::Require);
    // tokio-postgres-rustls doesn't expose the TLS exporter material that
    // SCRAM-SHA-256-PLUS needs, so disable channel binding even if the Cloud
    // connection string requests it. SCRAM-SHA-256 still runs inside the TLS
    // tunnel, so credentials remain protected.
    config.channel_binding(tokio_postgres::config::ChannelBinding::Disable);

    let (pg_client, pg_connection) = config.connect(tls).await?;
    tokio::spawn(async move {
        if let Err(e) = pg_connection.await {
            eprintln!("  postgres connection error: {e}");
        }
    });
    Ok(pg_client)
}

async fn configure_pg_for_cdc(client: &tokio_postgres::Client) -> TestResult<()> {
    // Idempotent setup so reruns against a long-lived test instance work.
    client
        .batch_execute(
            "DROP PUBLICATION IF EXISTS clickpipe_pub;
             DROP TABLE IF EXISTS cdc_users;
             CREATE TABLE cdc_users (
                 id BIGINT PRIMARY KEY,
                 name TEXT NOT NULL,
                 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
             );
             ALTER TABLE cdc_users REPLICA IDENTITY FULL;
             CREATE PUBLICATION clickpipe_pub FOR TABLE cdc_users;
             INSERT INTO cdc_users (id, name) VALUES (1, 'alice'), (2, 'bob'), (3, 'carol');",
        )
        .await?;
    Ok(())
}

fn parse_pg_port(connection_string: &str) -> Option<u16> {
    let config = tokio_postgres::Config::from_str(connection_string).ok()?;
    config.get_ports().first().copied()
}

fn parse_pg_database(connection_string: &str) -> Option<String> {
    let config = tokio_postgres::Config::from_str(connection_string).ok()?;
    config.get_dbname().map(|s| s.to_string())
}

#[derive(Clone)]
struct ClickHouseQuery {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl ClickHouseQuery {
    fn new(host: &str, port: u16, username: &str, password: &str) -> Self {
        Self {
            base_url: format!("https://{host}:{port}"),
            username: username.to_string(),
            password: password.to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    async fn run_query(&self, query: &str) -> TestResult<String> {
        let resp = self
            .http
            .post(&self.base_url)
            .basic_auth(&self.username, Some(&self.password))
            .body(query.to_string())
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(format!("ClickHouse query failed ({status}): {body}").into());
        }
        Ok(body)
    }

    async fn count_rows(&self, table: &str) -> TestResult<i64> {
        let body = self
            .run_query(&format!(
                "SELECT count() FROM default.{table} FINAL FORMAT TabSeparated"
            ))
            .await?;
        Ok(body.trim().parse::<i64>()?)
    }

    async fn scalar_string(&self, query: &str) -> TestResult<Option<String>> {
        let body = self
            .run_query(&format!("{query} FORMAT TabSeparated"))
            .await?;
        let value = body.trim();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.to_string()))
        }
    }
}
