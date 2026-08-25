//! Handlers for `clickhousectl local postgres ...` subcommands.
//!
//! All Docker work goes through `local::docker`. State is reused from
//! `local::server` — Postgres entries land in the same metadata directory and
//! show up alongside ClickHouse in `local server list`.

use crate::error::{Error, Result};
use crate::local::cli::PostgresCommands;
use crate::local::docker::{self, PostgresRunOpts};
use crate::local::output;
use crate::local::server::{self, Engine, ServerInfo};
use rand::distr::{Alphanumeric, SampleString};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const DEFAULT_PG_PORT: u16 = 5432;
const DEFAULT_USER: &str = "postgres";
const DEFAULT_DATABASE: &str = "postgres";
/// Default image tag when `--version` is not given. Within the supported
/// range; users can override with any 17/18 tag (`17`, `17.0`, `18-bookworm`, etc).
pub const DEFAULT_PG_TAG: &str = "18";
const POSTGRES_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const POSTGRES_READINESS_POLL_INTERVAL: Duration = Duration::from_millis(100);
const POSTGRES_READINESS_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const POSTGRES_DIAGNOSTICS_TIMEOUT: Duration = Duration::from_secs(2);

/// Extract the major-version digits from a Postgres image tag. `17-alpine` →
/// `"17"`, `17.0` → `"17"`, `18-bookworm` → `"18"`. Validation is the caller's
/// responsibility (`validate_pg_tag`) — this only parses.
pub(crate) fn pg_major_from_tag(tag: &str) -> String {
    tag.chars().take_while(|c| c.is_ascii_digit()).collect()
}

/// Accept syntactically valid Docker image tags for Postgres 17 and 18. The
/// major must be the complete first component, followed by an optional `.` or
/// `-` suffix. Examples that pass: `17`, `17.0`, `17-alpine`, `18-bookworm`,
/// `18.1-alpine3.20`. Examples that fail: `latest`, `16`, `19`, `18garbage`.
pub(crate) fn validate_pg_tag(tag: &str) -> Result<()> {
    let suffix = tag.strip_prefix("17").or_else(|| tag.strip_prefix("18"));
    let valid_suffix = suffix.is_some_and(|suffix| {
        suffix.is_empty()
            || (matches!(suffix.as_bytes().first(), Some(b'.' | b'-'))
                && suffix.len() > 1
                && suffix[1..]
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'-')))
    });
    if tag.len() > 128 || !valid_suffix {
        return Err(Error::PostgresValidation(format!(
            "Postgres version '{}' is not supported. Use a valid 17 or 18 image tag \
             (for example: 17, 17-alpine, 18.1, 18-bookworm).",
            tag
        )));
    }
    Ok(())
}

pub async fn run(cmd: PostgresCommands, json: bool) -> Result<()> {
    match cmd {
        PostgresCommands::Start {
            name,
            version,
            port,
            user,
            password,
            database,
            env,
        } => start(name, version, port, user, password, database, env, json).await,
        PostgresCommands::Stop { name, version } => stop(&name, version.as_deref(), json).await,
        PostgresCommands::StopAll => stop_all(json).await,
        PostgresCommands::Remove { name, version } => remove(&name, version.as_deref(), json),
        PostgresCommands::Client {
            name,
            version,
            host,
            port,
            query,
            queries_file,
            args,
        } => client(name, version, host, port, query, queries_file, args).await,
        PostgresCommands::Dotenv {
            name,
            version,
            local,
        } => dotenv(name.as_deref(), version.as_deref(), local, json),
    }
}

#[allow(clippy::too_many_arguments)]
async fn start(
    name: Option<String>,
    version: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    extra_env: Vec<String>,
    json: bool,
) -> Result<()> {
    let has_extra_env = !extra_env.is_empty();
    let StartPreflight {
        host_port,
        extra_env,
        password_from_env,
    } = preflight_start_options(name.as_deref(), version.as_deref(), port, extra_env)?;

    server::recover_current_project_servers();

    // User-facing name (no version suffix). Defaults to "default" when no
    // postgres "default" is currently running.
    let user_name = match name.as_deref() {
        Some(n) => n.to_string(),
        None => default_pg_name(),
    };

    // If `--version` is omitted but there's already exactly one instance for
    // this name, resume it — the user almost certainly wants their existing
    // data, not a freshly-initialized DEFAULT_PG_TAG. With multiple
    // instances, we ask them to disambiguate. Only when zero exist do we
    // default to DEFAULT_PG_TAG.
    let (tag, major) = match version.as_deref() {
        Some(v) => (v.to_string(), pg_major_from_tag(v)),
        None => {
            let existing = server::find_pg_instances(&user_name);
            match existing.len() {
                0 => (
                    DEFAULT_PG_TAG.to_string(),
                    pg_major_from_tag(DEFAULT_PG_TAG),
                ),
                1 => {
                    let info = &existing[0];
                    let stored_tag = info
                        .version
                        .strip_prefix("postgres:")
                        .unwrap_or(&info.version);
                    (stored_tag.to_string(), pg_major_from_tag(stored_tag))
                }
                _ => {
                    let versions: Vec<String> =
                        existing.iter().map(|i| i.version.clone()).collect();
                    return Err(Error::PostgresRuntime(format!(
                        "multiple Postgres instances named '{}' ({}); pass --version to select one",
                        user_name,
                        versions.join(", ")
                    )));
                }
            }
        }
    };
    let tag = tag.as_str();

    // Disk identifier — uniquely scopes (name, major) so two majors of the
    // same name never share container/data/metadata.
    let key = server::pg_instance_key(&user_name, &major);

    let docker = docker::connect().await?;

    let project_cwd = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    // Resume path: an instance for this exact (name, major) already exists.
    if let Some(prior) = server::load_info(&key) {
        let cid = prior.container_id.as_deref().unwrap_or("");
        let container_present =
            !cid.is_empty() && docker.inspect_container(cid, None).await.is_ok();
        if container_present {
            if server::is_server_running(&key) {
                return Err(Error::ServerAlreadyRunning(user_name));
            }
            if !json
                && (port.is_some()
                    || user.is_some()
                    || password.is_some()
                    || database.is_some()
                    || has_extra_env)
            {
                eprintln!(
                    "Note: postgres:{major} '{}' already exists; resuming with stored settings. \
                     Run `local postgres remove {}` to start over.",
                    user_name, user_name
                );
            }
            return resume_existing(&docker, prior, json).await;
        }
        // Metadata orphaned — container removed externally. Force explicit
        // cleanup to avoid silently re-initing against potentially-stale data.
        return Err(Error::PostgresRuntime(format!(
            "Postgres '{}' (postgres:{}) has metadata but the container is gone. \
             Run `clickhousectl local postgres remove {}` to clear the data dir \
             and start fresh.",
            user_name, major, user_name
        )));
    }

    // Fresh create.
    let host_port = match host_port {
        Some(port) => port,
        None => resolve_port(None)?,
    };
    if !docker::image_exists(&docker, tag).await? {
        docker::pull_image(&docker, tag, json).await?;
    }

    let instance_dir = server::servers_dir_join(&key);
    let remove_fresh_data_on_failure = fresh_instance_dir_is_disposable(&instance_dir);
    server::ensure_pg_data_dir(&user_name, &major)?;
    let data_dir = server::pg_data_dir(&user_name, &major);

    // Defensive cleanup of any unmanaged container colliding on our chosen
    // container name (only if labels confirm we own it).
    docker::ensure_name_free(&docker, &user_name, &major, &project_cwd).await?;

    let user = user.unwrap_or_else(|| DEFAULT_USER.to_string());
    let database = database.unwrap_or_else(|| DEFAULT_DATABASE.to_string());

    let password = password_from_env
        .or(password)
        .unwrap_or_else(generate_password);

    let opts = PostgresRunOpts {
        user_name: &user_name,
        major: &major,
        tag,
        host_port,
        data_dir: &data_dir,
        project_cwd: &project_cwd,
        user: &user,
        password: &password,
        database: &database,
        extra_env,
    };

    let container_id = docker::create_postgres(&docker, opts).await?;

    let info = ServerInfo {
        name: key.clone(),
        pid: 0,
        version: format!("postgres:{tag}"),
        http_port: 0,
        tcp_port: host_port,
        started_at: server::now_timestamp(),
        cwd: project_cwd.clone(),
        engine: Engine::Postgres,
        container_id: Some(container_id.clone()),
    };

    let rollback = FreshStartRollback {
        container_id: container_id.clone(),
        metadata_path: server::server_meta_path_for_recovery(&key),
        instance_dir,
        remove_fresh_data_on_failure,
    };
    finish_fresh_start(&docker, rollback, async {
        docker::start_container(&docker, &container_id).await?;
        server::save_server_info(&info)?;
        wait_for_postgres_ready(
            &docker,
            &container_id,
            &user_name,
            &user,
            &database,
            POSTGRES_STARTUP_TIMEOUT,
        )
        .await
    })
    .await?;

    let out = output::PostgresStartOutput {
        name: user_name,
        container_id,
        image: format!("postgres:{tag}"),
        port: host_port,
        user,
        password,
        database,
    };
    output::print_output(&out, json);
    Ok(())
}

struct FreshStartRollback {
    container_id: String,
    metadata_path: PathBuf,
    instance_dir: PathBuf,
    remove_fresh_data_on_failure: bool,
}

/// A fresh attempt owns an absent or empty instance directory, including one
/// containing only an empty `data/` created by an earlier pre-container step.
/// Any pre-existing data has uncertain ownership and is retained on failure.
fn fresh_instance_dir_is_disposable(path: &Path) -> bool {
    let mut entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
        Err(_) => return false,
    };
    let entry = match entries.next() {
        None => return true,
        Some(Ok(entry)) => entry,
        Some(Err(_)) => return false,
    };
    if entries.next().is_some()
        || entry.file_name() != "data"
        || !entry.file_type().is_ok_and(|file_type| file_type.is_dir())
    {
        return false;
    }
    match std::fs::read_dir(entry.path()) {
        Ok(mut data_entries) => data_entries.next().is_none(),
        Err(_) => false,
    }
}

async fn finish_fresh_start<F>(
    docker: &bollard::Docker,
    rollback: FreshStartRollback,
    startup: F,
) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    let primary = match startup.await {
        Ok(()) => return Ok(()),
        Err(error) => error,
    };

    let cleanup = rollback_fresh_start(docker, &rollback).await;
    if cleanup.is_empty() {
        Err(primary)
    } else {
        Err(Error::PostgresStartupRollback {
            primary: Box::new(primary),
            cleanup: cleanup.join("; "),
        })
    }
}

async fn rollback_fresh_start(
    docker: &bollard::Docker,
    rollback: &FreshStartRollback,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let container_removed = match docker::remove_container(docker, &rollback.container_id).await {
        Ok(()) => true,
        Err(error) => {
            diagnostics.push(format!(
                "failed to remove container '{}': {error}",
                rollback.container_id
            ));
            false
        }
    };

    if let Err(error) = std::fs::remove_file(&rollback.metadata_path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        diagnostics.push(format!(
            "failed to remove metadata '{}': {error}",
            rollback.metadata_path.display()
        ));
    }

    if rollback.remove_fresh_data_on_failure && container_removed {
        if let Err(error) = docker::remove_host_dir_blocking(&rollback.instance_dir) {
            diagnostics.push(format!(
                "failed to remove fresh Postgres data '{}': {error}",
                rollback.instance_dir.display()
            ));
        }
    } else {
        let reason = if container_removed {
            "it contained data before this start attempt"
        } else {
            "the container could not be removed"
        };
        diagnostics.push(format!(
            "retained Postgres data '{}' because {reason}",
            rollback.instance_dir.display()
        ));
    }

    diagnostics
}

/// Default user-facing name when `--name` is omitted: `"default"` if no
/// postgres "default" is running, otherwise a random adjective-noun.
fn default_pg_name() -> String {
    let any_default_running = server::find_pg_instances("default").iter().any(|i| {
        i.container_id
            .as_deref()
            .map(docker::is_container_running_blocking)
            .unwrap_or(false)
    });
    if any_default_running {
        // Fall back to the existing random-name generator, which checks
        // metadata file uniqueness across engines.
        server::resolve_name(None).unwrap_or_else(|_| "default".into())
    } else {
        "default".into()
    }
}

/// Resolve `--name <X> [--version <V>]` to a single Postgres instance on disk.
/// If `version` is given, target the (X, major(V)) pair directly. Otherwise:
/// 0 instances → ServerNotFound; 1 → use it; >1 → ask for `--version`.
fn resolve_pg_target(user_name: &str, version: Option<&str>) -> Result<server::ServerInfo> {
    if let Some(v) = version {
        validate_pg_tag(v)?;
        let major = pg_major_from_tag(v);
        let key = server::pg_instance_key(user_name, &major);
        return server::load_info(&key)
            .filter(|i| i.engine == Engine::Postgres)
            .ok_or_else(|| Error::ServerNotFound(format!("{user_name} (postgres:{major})")));
    }
    let instances = server::find_pg_instances(user_name);
    match instances.len() {
        0 => Err(Error::ServerNotFound(user_name.to_string())),
        1 => Ok(instances.into_iter().next().unwrap()),
        _ => {
            let versions: Vec<String> = instances.iter().map(|i| i.version.clone()).collect();
            Err(Error::PostgresRuntime(format!(
                "multiple Postgres instances named '{}' ({}); pass --version to select one",
                user_name,
                versions.join(", ")
            )))
        }
    }
}

/// Resume an existing stopped Postgres container. Reads credentials from the
/// container's persisted env (the source of truth — PGDATA was initialized
/// for them) and refreshes the metadata.
async fn resume_existing(docker: &bollard::Docker, prior: ServerInfo, json: bool) -> Result<()> {
    let container_id = prior.container_id.clone().expect("checked by caller");
    let display_name = user_name_from_key(&prior.name).to_string();

    docker::start_existing_blocking(&container_id)?;

    let (user, password, database) = read_pg_env(docker, &container_id).await;
    if let Err(error) = wait_for_postgres_ready(
        docker,
        &container_id,
        &display_name,
        &user,
        &database,
        POSTGRES_STARTUP_TIMEOUT,
    )
    .await
    {
        let _ = docker::stop_container(docker, &container_id).await;
        return Err(error);
    }

    let info = ServerInfo {
        started_at: server::now_timestamp(),
        ..prior
    };
    server::save_server_info(&info)?;

    let out = output::PostgresStartOutput {
        name: display_name,
        container_id,
        image: info.version,
        port: info.tcp_port,
        user,
        password,
        database,
    };
    output::print_output(&out, json);
    Ok(())
}

/// Extract the user-facing name from a disk key. `dev-pg16` → `dev`;
/// anything that doesn't match the suffix shape passes through unchanged.
pub(crate) fn user_name_from_key(key: &str) -> &str {
    if let Some(idx) = key.rfind("-pg") {
        let suffix = &key[idx + 3..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return &key[..idx];
        }
    }
    key
}

async fn wait_for_postgres_ready(
    docker: &bollard::Docker,
    id: &str,
    display_name: &str,
    user: &str,
    database: &str,
    timeout: Duration,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut last_probe_error = None;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let inspect = match tokio::time::timeout(remaining, docker.inspect_container(id, None)).await
        {
            Ok(result) => result.map_err(|error| {
                Error::DockerError(format!(
                    "could not inspect container '{display_name}' while waiting for PostgreSQL readiness: {error}"
                ))
            })?,
            Err(_) => {
                last_probe_error = Some("container inspection timed out".to_string());
                break;
            }
        };
        let state = inspect.state.unwrap_or_default();
        if state.running != Some(true) {
            let status = state
                .status
                .map(|status| status.to_string())
                .unwrap_or_else(|| "stopped".to_string());
            let exit_code = state
                .exit_code
                .map(|code| format!(", exit code {code}"))
                .unwrap_or_default();
            let engine_error = state
                .error
                .filter(|error| !error.is_empty())
                .map(|error| format!(", Docker reported: {error}"))
                .unwrap_or_default();
            return Err(Error::DockerStartupExit(
                readiness_diagnostics(
                    docker,
                    id,
                    format!(
                        "Postgres container '{display_name}' exited before PostgreSQL became ready \
                     (state: {status}{exit_code}{engine_error})"
                    ),
                )
                .await,
            ));
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(
            remaining.min(POSTGRES_READINESS_PROBE_TIMEOUT),
            docker::postgres_is_ready(docker, id, user, database),
        )
        .await
        {
            Ok(Ok(true)) => return Ok(()),
            Ok(Ok(false)) => last_probe_error = None,
            Ok(Err(error)) => last_probe_error = Some(error.to_string()),
            Err(_) => last_probe_error = Some("pg_isready probe timed out".to_string()),
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        tokio::time::sleep(remaining.min(POSTGRES_READINESS_POLL_INTERVAL)).await;
    }

    let probe_context = last_probe_error
        .map(|error| format!(" Last probe error: {error}."))
        .unwrap_or_default();
    Err(Error::DockerStartupTimeout(
        readiness_diagnostics(
            docker,
            id,
            format!(
                "Postgres container '{display_name}' did not become ready within {} seconds.{probe_context}",
                timeout.as_secs()
            ),
        )
        .await,
    ))
}

async fn readiness_diagnostics(docker: &bollard::Docker, id: &str, message: String) -> String {
    let logs = match tokio::time::timeout(
        POSTGRES_DIAGNOSTICS_TIMEOUT,
        docker::container_logs_tail(docker, id, 50),
    )
    .await
    {
        Ok(Ok(logs)) if logs.trim().is_empty() => format!(
            "No container logs were available. Run `docker logs {id}` for current diagnostics."
        ),
        Ok(Ok(logs)) => logs,
        Ok(Err(error)) => format!(
            "Could not read container logs ({error}). Run `docker logs {id}` for diagnostics."
        ),
        Err(_) => {
            format!("Timed out reading container logs. Run `docker logs {id}` for diagnostics.")
        }
    };
    format!("{message}\n--- container logs (last 50 lines) ---\n{logs}")
}

fn resolve_port(explicit: Option<u16>) -> Result<u16> {
    if let Some(p) = explicit {
        if p == 0 {
            return Err(Error::PostgresValidation(
                "--port 0 is not allowed; pick a specific port or omit the flag".into(),
            ));
        }
        if std::net::TcpListener::bind(("127.0.0.1", p)).is_err() {
            return Err(Error::PortInUse(format!(
                "explicit Postgres port {p} is already in use; choose a free --port or omit the flag to auto-select"
            )));
        }
        return Ok(p);
    }
    if std::net::TcpListener::bind(("127.0.0.1", DEFAULT_PG_PORT)).is_ok() {
        return Ok(DEFAULT_PG_PORT);
    }
    for p in (DEFAULT_PG_PORT + 1)..=(DEFAULT_PG_PORT + 100) {
        if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return Ok(p);
        }
    }
    Err(Error::PostgresRuntime(
        "could not find a free TCP port for Postgres".into(),
    ))
}

struct StartPreflight {
    host_port: Option<u16>,
    extra_env: Vec<String>,
    password_from_env: Option<String>,
}

fn preflight_start_options(
    name: Option<&str>,
    version: Option<&str>,
    port: Option<u16>,
    extra_env: Vec<String>,
) -> Result<StartPreflight> {
    if let Some(name) = name {
        server::validate_server_name(name)?;
    }
    if let Some(version) = version {
        validate_pg_tag(version)?;
    }

    let (extra_env, password_from_env) = validate_extra_env(extra_env)?;
    let host_port = port.map(|port| resolve_port(Some(port))).transpose()?;
    Ok(StartPreflight {
        host_port,
        extra_env,
        password_from_env,
    })
}

/// Normalize variables managed by the Postgres definition so Docker receives
/// each reserved key once. The first `-e POSTGRES_PASSWORD=...` wins, matching
/// the previously advertised and implemented override behavior. Dedicated
/// options/defaults own POSTGRES_USER and POSTGRES_DB; PGDATA is always fixed.
fn validate_extra_env(extra_env: Vec<String>) -> Result<(Vec<String>, Option<String>)> {
    let mut normalized = Vec::with_capacity(extra_env.len());
    let mut password_from_env = None;

    for (index, entry) in extra_env.into_iter().enumerate() {
        let Some((key, value)) = entry.split_once('=') else {
            return Err(Error::PostgresValidation(format!(
                "invalid container environment variable #{}: expected KEY=VALUE",
                index + 1
            )));
        };
        if key.is_empty() {
            return Err(Error::PostgresValidation(format!(
                "invalid container environment variable #{}: KEY must not be empty",
                index + 1
            )));
        }

        match key {
            "POSTGRES_PASSWORD" => {
                if password_from_env.is_none() {
                    password_from_env = Some(value.to_string());
                }
            }
            "POSTGRES_USER" | "POSTGRES_DB" | "PGDATA" => {}
            _ => normalized.push(entry),
        }
    }

    Ok((normalized, password_from_env))
}

fn generate_password() -> String {
    // 24 alphanumeric chars. Persisted in `.clickhouse/servers/<name>.json`
    // so other processes (and `dotenv`) can recover the value.
    Alphanumeric.sample_string(&mut rand::rng(), 24)
}

async fn stop(name: &str, version: Option<&str>, json: bool) -> Result<()> {
    server::validate_server_name(name)?;
    server::recover_current_project_servers();
    let target = resolve_pg_target(name, version)?;
    if !json {
        let display = format!("{} ({})", user_name_from_key(&target.name), target.version);
        println!("Stopping Postgres {}...", display);
    }
    server::kill_server(&target.name)?;
    let out = output::ServerStopOutput {
        name: user_name_from_key(&target.name).to_string(),
        already_stopped: false,
    };
    output::print_output(&out, json);
    Ok(())
}

async fn stop_all(json: bool) -> Result<()> {
    server::recover_current_project_servers();
    let servers: Vec<_> = server::list_running_servers()
        .into_iter()
        .filter(|s| s.engine == Engine::Postgres)
        .collect();
    if !json && servers.is_empty() {
        println!("No running Postgres servers");
        return Ok(());
    }

    let out = super::stop_servers(&servers, json, server::kill_server);
    if json {
        output::print_output(&out, json);
    } else {
        println!("Done");
    }
    Ok(())
}

fn remove(name: &str, version: Option<&str>, json: bool) -> Result<()> {
    server::validate_server_name(name)?;
    server::recover_current_project_servers();

    let target = resolve_pg_target(name, version)?;
    let key = target.name.clone();
    if server::is_server_running(&key) {
        return Err(Error::ServerAlreadyRunning(name.to_string()));
    }

    if let Some(cid) = target.container_id.as_deref() {
        let _ = docker::stop_and_remove_blocking(cid);
    }

    // Postgres data dir lives at .clickhouse/servers/<key>/data/. Remove the
    // <key>/ wrapper so the (name, version) pair leaves no on-disk state.
    // On Linux the bind-mounted PGDATA contains files owned by uid 999, so
    // a plain rm fails — `remove_host_dir_blocking` falls back to a
    // privileged container in that case.
    let pg_dir = server::servers_dir_join(&key);
    docker::remove_host_dir_blocking(&pg_dir)?;
    server::remove_server_info(&key);
    let out = output::ServerRemoveOutput {
        name: name.to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn client(
    name: Option<String>,
    version: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    query: Option<String>,
    queries_file: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    if host.is_some() || port.is_some() {
        // Direct connect — no server lookup; require host psql.
        let h = host.unwrap_or_else(|| "127.0.0.1".to_string());
        let p = port.unwrap_or(DEFAULT_PG_PORT);
        return exec_host_psql(
            &h,
            p,
            DEFAULT_USER,
            None,
            DEFAULT_DATABASE,
            query,
            queries_file,
            extra_args,
        );
    }

    server::recover_current_project_servers();
    let server_name = name.as_deref().unwrap_or("default");
    let info = resolve_pg_target(server_name, version.as_deref())?;
    if !server::is_server_running(&info.name) {
        return Err(Error::ServerNotRunning(server_name.to_string()));
    }

    let docker = docker::connect().await?;
    let container_id = info
        .container_id
        .as_deref()
        .ok_or_else(|| Error::DockerError("missing container_id".into()))?;
    let (user, password, database) = read_pg_env(&docker, container_id).await;

    // Prefer host psql; fall back to docker exec.
    if host_has_psql() {
        return exec_host_psql(
            "127.0.0.1",
            info.tcp_port,
            &user,
            Some(&password),
            &database,
            query,
            queries_file,
            extra_args,
        );
    }

    let one_shot = query.is_some() || queries_file.is_some();
    let mut psql_args: Vec<String> = vec!["-U".into(), user, "-d".into(), database];
    if let Some(q) = query {
        psql_args.push("-c".into());
        psql_args.push(q);
    }
    if let Some(f) = queries_file {
        psql_args.push("-f".into());
        psql_args.push(f);
    }
    psql_args.extend(extra_args);

    if one_shot {
        // Non-interactive: no TTY, no raw mode, output goes to stdout/stderr
        // so the caller can pipe / capture / redirect.
        docker::exec_psql_one_shot(&docker, container_id, &psql_args).await
    } else {
        docker::exec_psql_in_container(&docker, container_id, &psql_args).await
    }
}

/// Read POSTGRES_USER/PASSWORD/DB from the container's effective env so we
/// don't lose track of user-provided values across recoveries.
async fn read_pg_env(docker: &bollard::Docker, id: &str) -> (String, String, String) {
    let inspect = docker.inspect_container(id, None).await.ok();
    let env: Vec<String> = inspect
        .and_then(|c| c.config)
        .and_then(|c| c.env)
        .unwrap_or_default();
    let get = |k: &str| -> Option<String> {
        env.iter()
            .find_map(|e| e.strip_prefix(&format!("{k}=")).map(|s| s.to_string()))
    };
    (
        get("POSTGRES_USER").unwrap_or_else(|| DEFAULT_USER.into()),
        get("POSTGRES_PASSWORD").unwrap_or_default(),
        get("POSTGRES_DB").unwrap_or_else(|| DEFAULT_DATABASE.into()),
    )
}

fn host_has_psql() -> bool {
    Command::new("psql")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn exec_host_psql(
    host: &str,
    port: u16,
    user: &str,
    password: Option<&str>,
    database: &str,
    query: Option<String>,
    queries_file: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new("psql");
    cmd.arg("-h")
        .arg(host)
        .arg("-p")
        .arg(port.to_string())
        .arg("-U")
        .arg(user)
        .arg("-d")
        .arg(database);
    if let Some(p) = password {
        cmd.env("PGPASSWORD", p);
    }
    if let Some(q) = query {
        cmd.arg("-c").arg(q);
    }
    if let Some(f) = queries_file {
        cmd.arg("-f").arg(f);
    }
    cmd.args(&extra_args);
    // `exec()` replaces the process image on success, so `main`'s telemetry
    // tail never runs for this invocation; record the event now (#320).
    #[cfg(feature = "telemetry")]
    crate::telemetry::finalize_before_exec();
    let err = cmd.exec();
    Err(Error::PostgresRuntime(format!(
        "could not execute psql: {err}"
    )))
}

fn dotenv(name: Option<&str>, version: Option<&str>, use_local: bool, json: bool) -> Result<()> {
    server::recover_current_project_servers();
    let server_name = name.unwrap_or("default");
    let info = resolve_pg_target(server_name, version)?;
    if !server::is_server_running(&info.name) {
        return Err(Error::ServerNotRunning(server_name.to_string()));
    }

    // Read user/password/db from the container env so we always emit accurate creds.
    let (user, password, database) = docker::block_on(read_pg_env_for_dotenv(
        info.container_id.as_deref().unwrap_or_default(),
    ));

    let vars: Vec<(&str, String)> = vec![
        ("POSTGRES_HOST", "127.0.0.1".to_string()),
        ("POSTGRES_PORT", info.tcp_port.to_string()),
        ("POSTGRES_USER", user),
        ("POSTGRES_PASSWORD", password),
        ("POSTGRES_DATABASE", database),
    ];

    let filename = if use_local { ".env.local" } else { ".env" };
    let path = std::path::Path::new(filename);

    let content = if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        crate::local::update_dotenv(&existing, "POSTGRES_", &vars)
    } else {
        vars.iter()
            .map(|(k, v)| crate::local::format_dotenv_line("", k, v))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    std::fs::write(path, &content)?;

    let out = output::PostgresDotenvOutput {
        file: filename.to_string(),
        server: server_name.to_string(),
        vars: vars
            .into_iter()
            .map(|(k, v)| output::DotenvVar {
                key: k.to_string(),
                value: v,
            })
            .collect(),
    };
    output::print_output(&out, json);
    Ok(())
}

async fn read_pg_env_for_dotenv(container_id: &str) -> (String, String, String) {
    if container_id.is_empty() {
        return (DEFAULT_USER.into(), String::new(), DEFAULT_DATABASE.into());
    }
    match docker::connect().await {
        Ok(d) => read_pg_env(&d, container_id).await,
        Err(_) => (DEFAULT_USER.into(), String::new(), DEFAULT_DATABASE.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ErrorKind, Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::{self, JoinHandle};

    struct FakeDockerState {
        container_running: bool,
        ready_on_probe: Option<usize>,
        probes: AtomicUsize,
        removes: AtomicUsize,
        saw_probe_args: AtomicBool,
        stall_logs: AtomicBool,
    }

    struct FakeDocker {
        client: bollard::Docker,
        state: Arc<FakeDockerState>,
        stop: Arc<AtomicBool>,
        daemon: Option<JoinHandle<()>>,
        _tempdir: tempfile::TempDir,
    }

    impl FakeDocker {
        fn spawn(container_running: bool, ready_on_probe: Option<usize>) -> Self {
            let tempdir = tempfile::tempdir().expect("create fake Docker tempdir");
            let socket_path = tempdir.path().join("docker.sock");
            let listener = UnixListener::bind(&socket_path).expect("bind fake Docker socket");
            listener
                .set_nonblocking(true)
                .expect("make fake Docker socket nonblocking");

            let state = Arc::new(FakeDockerState {
                container_running,
                ready_on_probe,
                probes: AtomicUsize::new(0),
                removes: AtomicUsize::new(0),
                saw_probe_args: AtomicBool::new(false),
                stall_logs: AtomicBool::new(false),
            });
            let stop = Arc::new(AtomicBool::new(false));
            let daemon_state = Arc::clone(&state);
            let daemon_stop = Arc::clone(&stop);
            let daemon = thread::spawn(move || {
                while !daemon_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_nonblocking(true)
                                .expect("make fake Docker connection nonblocking");
                            if let Some(request) = read_docker_request(&mut stream) {
                                stream
                                    .set_nonblocking(false)
                                    .expect("make fake Docker response blocking");
                                respond_to_docker_request(&mut stream, &request, &daemon_state);
                            }
                        }
                        Err(error) if error.kind() == ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept fake Docker connection: {error}"),
                    }
                }
            });
            let client = bollard::Docker::connect_with_unix(
                socket_path.to_str().expect("UTF-8 socket path"),
                10,
                bollard::API_DEFAULT_VERSION,
            )
            .expect("connect fake Docker client");

            Self {
                client,
                state,
                stop,
                daemon: Some(daemon),
                _tempdir: tempdir,
            }
        }

        fn probes(&self) -> usize {
            self.state.probes.load(Ordering::SeqCst)
        }

        fn removes(&self) -> usize {
            self.state.removes.load(Ordering::SeqCst)
        }

        fn stall_logs(&self) {
            self.state.stall_logs.store(true, Ordering::SeqCst);
        }

        fn finish(mut self) {
            self.stop_daemon();
        }

        fn stop_daemon(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(daemon) = self.daemon.take() {
                let joined = daemon.join();
                if !thread::panicking() {
                    joined.expect("join fake Docker daemon");
                }
            }
        }
    }

    impl Drop for FakeDocker {
        fn drop(&mut self) {
            self.stop_daemon();
        }
    }

    fn read_docker_request(stream: &mut UnixStream) -> Option<String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let mut request = Vec::new();
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            if !read_docker_bytes(stream, &mut request, deadline) {
                return None;
            }
        }

        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("Docker header terminator")
            + 4;
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        while request.len() - header_end < content_length {
            if !read_docker_bytes(stream, &mut request, deadline) {
                return None;
            }
        }
        Some(String::from_utf8(request).expect("Docker request is UTF-8"))
    }

    fn read_docker_bytes(
        stream: &mut UnixStream,
        request: &mut Vec<u8>,
        deadline: std::time::Instant,
    ) -> bool {
        let mut buffer = [0_u8; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => return false,
                Ok(bytes) => {
                    request.extend_from_slice(&buffer[..bytes]);
                    return true;
                }
                Err(error)
                    if error.kind() == ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => return false,
                Err(error) => panic!("read Docker request: {error}"),
            }
        }
    }

    fn write_docker_response(stream: &mut UnixStream, body: &str) {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        if let Err(error) = stream.write_all(response.as_bytes())
            && error.kind() != ErrorKind::BrokenPipe
        {
            panic!("write fake Docker response: {error}");
        }
    }

    fn respond_to_docker_request(stream: &mut UnixStream, request: &str, state: &FakeDockerState) {
        let request_line = request.lines().next().expect("Docker request line");
        if request_line.starts_with("GET ") && request_line.contains("/containers/test/logs?") {
            if state.stall_logs.load(Ordering::SeqCst) {
                thread::sleep(POSTGRES_DIAGNOSTICS_TIMEOUT + Duration::from_secs(1));
            }
            write_docker_response(stream, "database system is starting up\n");
        } else if request_line.starts_with("GET ")
            && request_line.contains("/containers/test/json ")
        {
            if state.container_running {
                write_docker_response(
                    stream,
                    r#"{"State":{"Status":"running","Running":true,"ExitCode":0}}"#,
                );
            } else {
                write_docker_response(
                    stream,
                    r#"{"State":{"Status":"exited","Running":false,"ExitCode":7}}"#,
                );
            }
        } else if request_line.starts_with("POST ")
            && request_line.contains("/containers/test/exec ")
        {
            let probe = state.probes.fetch_add(1, Ordering::SeqCst) + 1;
            state.saw_probe_args.store(
                request.contains("pg_isready")
                    && request.contains("127.0.0.1")
                    && request.contains("test-user")
                    && request.contains("test-db")
                    && request.contains(r#""-t","1""#),
                Ordering::SeqCst,
            );
            write_docker_response(stream, &format!(r#"{{"Id":"probe-{probe}"}}"#));
        } else if request_line.starts_with("POST ") && request_line.contains("/exec/probe-") {
            assert!(
                request.contains(r#""Detach":true"#),
                "readiness probe was not detached: {request}"
            );
            write_docker_response(stream, "");
        } else if request_line.starts_with("GET ") && request_line.contains("/exec/probe-") {
            let probe = request_line
                .split("/exec/probe-")
                .nth(1)
                .and_then(|tail| tail.split('/').next())
                .and_then(|probe| probe.parse::<usize>().ok())
                .expect("probe number");
            let exit_code = if state.ready_on_probe.is_some_and(|ready| probe >= ready) {
                0
            } else {
                1
            };
            write_docker_response(
                stream,
                &format!(r#"{{"Running":false,"ExitCode":{exit_code}}}"#),
            );
        } else if request_line.starts_with("DELETE ") && request_line.contains("/containers/test?")
        {
            state.removes.fetch_add(1, Ordering::SeqCst);
            write_docker_response(stream, "");
        } else {
            panic!("unexpected fake Docker request: {request_line}");
        }
    }

    #[tokio::test]
    async fn running_container_is_not_postgres_readiness() {
        let docker = FakeDocker::spawn(true, None);

        assert!(
            !docker::postgres_is_ready(&docker.client, "test", "test-user", "test-db")
                .await
                .unwrap()
        );
        assert_eq!(docker.probes(), 1);
        assert!(docker.state.saw_probe_args.load(Ordering::SeqCst));
        docker.finish();
    }

    #[tokio::test]
    async fn postgres_readiness_waits_for_delayed_success() {
        let docker = FakeDocker::spawn(true, Some(3));

        wait_for_postgres_ready(
            &docker.client,
            "test",
            "delayed",
            "test-user",
            "test-db",
            Duration::from_secs(2),
        )
        .await
        .unwrap();

        assert_eq!(docker.probes(), 3);
        docker.finish();
    }

    #[tokio::test]
    async fn postgres_readiness_reports_immediate_container_exit_with_logs() {
        let docker = FakeDocker::spawn(false, None);

        let error = wait_for_postgres_ready(
            &docker.client,
            "test",
            "crashed",
            "test-user",
            "test-db",
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();

        let Error::DockerStartupExit(details) = error else {
            panic!("expected DockerStartupExit")
        };
        assert!(details.contains("exited before PostgreSQL became ready"));
        assert!(details.contains("exit code 7"));
        assert!(details.contains("database system is starting up"));
        assert_eq!(docker.probes(), 0);
        docker.finish();
    }

    #[tokio::test]
    async fn postgres_readiness_timeout_is_bounded_and_includes_logs() {
        let docker = FakeDocker::spawn(true, None);
        let started = std::time::Instant::now();

        let error = wait_for_postgres_ready(
            &docker.client,
            "test",
            "stuck",
            "test-user",
            "test-db",
            Duration::from_secs(1),
        )
        .await
        .unwrap_err();

        assert!(started.elapsed() < Duration::from_secs(2));
        let Error::DockerStartupTimeout(details) = error else {
            panic!("expected DockerStartupTimeout")
        };
        assert!(details.contains("did not become ready within 1 seconds"));
        assert!(details.contains("database system is starting up"));
        assert!(docker.probes() >= 2);
        docker.finish();
    }

    #[tokio::test]
    async fn fresh_initialization_timeout_removes_container_metadata_and_pgdata() {
        let docker = FakeDocker::spawn(true, None);
        let tempdir = tempfile::tempdir().expect("create rollback tempdir");
        let instance_dir = tempdir.path().join("stuck-pg18");
        let data_dir = instance_dir.join("data");
        let metadata_path = tempdir.path().join("stuck-pg18.json");
        std::fs::create_dir_all(&data_dir).expect("create partial PGDATA");
        std::fs::write(data_dir.join("PG_VERSION"), "18").expect("write partial PGDATA");
        std::fs::write(&metadata_path, "partial metadata").expect("write metadata");

        let error = finish_fresh_start(
            &docker.client,
            FreshStartRollback {
                container_id: "test".to_string(),
                metadata_path: metadata_path.clone(),
                instance_dir: instance_dir.clone(),
                remove_fresh_data_on_failure: true,
            },
            wait_for_postgres_ready(
                &docker.client,
                "test",
                "stuck",
                "test-user",
                "test-db",
                Duration::from_millis(150),
            ),
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("did not become ready"));
        assert_eq!(docker.removes(), 1);
        assert!(!metadata_path.exists());
        assert!(!instance_dir.exists());
        docker.finish();
    }

    #[tokio::test]
    async fn postgres_readiness_log_collection_is_bounded() {
        let docker = FakeDocker::spawn(true, None);
        docker.stall_logs();
        let started = std::time::Instant::now();

        let error =
            readiness_diagnostics(&docker.client, "test", "readiness failed".to_string()).await;

        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(error.contains("Timed out reading container logs"));
        assert!(error.contains("docker logs test"));
        docker.finish();
    }

    #[test]
    fn resolve_port_rejects_zero() {
        let err = resolve_port(Some(0)).unwrap_err();
        assert!(matches!(err, Error::PostgresValidation(msg) if msg.contains("--port 0")));
    }

    #[test]
    fn resolve_port_passes_through_explicit_value() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        assert_eq!(resolve_port(Some(port)).unwrap(), port);
    }

    #[test]
    fn resolve_port_rejects_bound_explicit_value() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let err = resolve_port(Some(port)).unwrap_err();
        assert!(matches!(err, Error::PortInUse(msg) if msg == format!(
            "explicit Postgres port {port} is already in use; choose a free --port or omit the flag to auto-select"
        )));
    }

    #[test]
    fn resolve_port_auto_selects_when_default_is_bound() {
        // If another process already owns 5432, leaving this as None still
        // exercises the same occupied-default path.
        let _listener = std::net::TcpListener::bind(("127.0.0.1", DEFAULT_PG_PORT)).ok();

        let port = resolve_port(None).unwrap();
        assert_ne!(port, DEFAULT_PG_PORT);
    }

    #[test]
    fn validate_pg_tag_accepts_supported_majors() {
        for tag in [
            "17",
            "18",
            "17-alpine",
            "17.0",
            "18-bookworm",
            "18.1-alpine3.20",
        ] {
            assert!(
                validate_pg_tag(tag).is_ok(),
                "expected `{}` to be accepted",
                tag
            );
        }
    }

    #[test]
    fn validate_pg_tag_rejects_unsupported() {
        for tag in [
            "latest",
            "15",
            "16",
            "16-alpine",
            "19",
            "14-alpine",
            "alpine",
            "18garbage",
            "17beta",
            "18/invalid",
            "18-invalid/tag",
            "18-",
            "18.",
            "",
        ] {
            assert!(
                validate_pg_tag(tag).is_err(),
                "expected `{}` to be rejected",
                tag
            );
        }
    }

    #[test]
    fn validate_pg_tag_rejects_tags_over_docker_limit() {
        let tag = format!("18-{}", "a".repeat(126));
        assert_eq!(tag.len(), 129);
        assert!(validate_pg_tag(&tag).is_err());
    }

    #[test]
    fn extra_env_requires_key_value_entries() {
        for env in [vec!["NO_EQUALS".to_string()], vec!["=value".to_string()]] {
            let err = validate_extra_env(env).unwrap_err();
            assert!(
                matches!(err, Error::PostgresValidation(msg) if msg.contains("environment variable"))
            );
        }
    }

    #[test]
    fn extra_env_normalizes_reserved_variable_precedence() {
        let (extra_env, password) = validate_extra_env(vec![
            "CUSTOM=first=value".to_string(),
            "POSTGRES_USER=from-env".to_string(),
            "POSTGRES_PASSWORD=first".to_string(),
            "POSTGRES_PASSWORD=second".to_string(),
            "POSTGRES_DB=from-env".to_string(),
            "PGDATA=/tmp/pgdata".to_string(),
            "OTHER=".to_string(),
        ])
        .unwrap();

        assert_eq!(
            extra_env,
            vec!["CUSTOM=first=value".to_string(), "OTHER=".to_string()]
        );
        assert_eq!(password.as_deref(), Some("first"));
    }

    #[test]
    fn generate_password_is_24_alphanumeric() {
        let p = generate_password();
        assert_eq!(p.len(), 24);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn fresh_data_cleanup_ownership_is_conservative() {
        let tempdir = tempfile::tempdir().expect("create policy tempdir");
        let instance_dir = tempdir.path().join("policy-pg18");
        assert!(fresh_instance_dir_is_disposable(&instance_dir));

        std::fs::create_dir(&instance_dir).expect("create empty instance dir");
        assert!(fresh_instance_dir_is_disposable(&instance_dir));

        let data_dir = instance_dir.join("data");
        std::fs::create_dir(&data_dir).expect("create empty data dir");
        assert!(fresh_instance_dir_is_disposable(&instance_dir));

        std::fs::write(data_dir.join("PG_VERSION"), "existing").expect("write existing PGDATA");
        assert!(!fresh_instance_dir_is_disposable(&instance_dir));
    }
}
