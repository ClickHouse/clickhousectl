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
use std::collections::HashSet;
use std::process::Command;

const DEFAULT_PG_PORT: u16 = 5432;
const DEFAULT_USER: &str = "postgres";
const DEFAULT_DATABASE: &str = "postgres";
/// Default image tag when `--version` is not given. Within the supported
/// range; users can override with any 17/18 tag (`17`, `17.0`, `18-bookworm`, etc).
pub const DEFAULT_PG_TAG: &str = "18";

/// Extract the major-version digits from a Postgres image tag. `17-alpine` →
/// `"17"`, `17.0` → `"17"`, `18-bookworm` → `"18"`. Validation is the caller's
/// responsibility (`validate_pg_tag`) — this only parses.
pub(crate) fn pg_major_from_tag(tag: &str) -> String {
    tag.chars().take_while(|c| c.is_ascii_digit()).collect()
}

/// Accept Postgres image tags in the form `17|18[.<minor>][-<variant>]`.
/// The variant follows Docker's tag character grammar. Examples that pass:
/// `17`, `17.0`, `17-alpine`, `18-bookworm`, `18.1-alpine3.20`.
pub(crate) fn validate_pg_tag(tag: &str) -> Result<()> {
    let valid = tag.len() <= 128 && tag.is_ascii() && {
        let (version, variant) = match tag.split_once('-') {
            Some((version, variant)) => (version, Some(variant)),
            None => (tag, None),
        };
        let variant_valid = variant.is_none_or(|variant| {
            let mut chars = variant.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        });
        let (major, minor) = match version.split_once('.') {
            Some((major, minor)) => (major, Some(minor)),
            None => (version, None),
        };
        let minor_valid = minor
            .is_none_or(|minor| !minor.is_empty() && minor.chars().all(|c| c.is_ascii_digit()));
        matches!(major, "17" | "18") && minor_valid && variant_valid
    };

    if !valid {
        return Err(Error::Exec(format!(
            "invalid or unsupported postgres version '{}'. Use 17 or 18, optionally followed \
             by .<minor> and -<variant> (for example: 17, 17-alpine, 18.1, 18-bookworm).",
            tag
        )));
    }
    Ok(())
}

pub(crate) fn parse_pg_tag_arg(tag: &str) -> std::result::Result<String, String> {
    validate_pg_tag(tag)
        .map(|()| tag.to_string())
        .map_err(|error| error.to_string())
}

pub(crate) fn parse_pg_port_arg(value: &str) -> std::result::Result<u16, String> {
    let port = value
        .parse::<u16>()
        .map_err(|_| format!("invalid port '{value}': expected an integer from 1 to 65535"))?;
    if port == 0 {
        return Err("--port 0 is not allowed; pick a specific port or omit the flag".into());
    }
    Ok(port)
}

fn validate_pg_env_assignment(assignment: &str) -> std::result::Result<(&str, &str), String> {
    let Some((key, value)) = assignment.split_once('=') else {
        return Err(format!(
            "invalid environment variable '{assignment}': expected KEY=VALUE"
        ));
    };
    let mut chars = key.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "invalid environment variable key '{key}': use letters, digits, and underscores, and do not start with a digit"
        ));
    }
    match key {
        "POSTGRES_USER" => {
            Err("POSTGRES_USER is managed by clickhousectl; use --user instead of --env".into())
        }
        "POSTGRES_DB" => {
            Err("POSTGRES_DB is managed by clickhousectl; use --database instead of --env".into())
        }
        "PGDATA" => Err("PGDATA is managed by clickhousectl and cannot be set with --env".into()),
        _ => Ok((key, value)),
    }
}

pub(crate) fn parse_pg_env_arg(assignment: &str) -> std::result::Result<String, String> {
    validate_pg_env_assignment(assignment).map(|_| assignment.to_string())
}

pub(crate) fn validate_pg_start_env_args(
    password: Option<&str>,
    extra_env: &[String],
) -> std::result::Result<(), String> {
    let mut seen = HashSet::new();
    let mut has_password_env = false;
    for assignment in extra_env {
        let (key, _) = validate_pg_env_assignment(assignment)?;
        if !seen.insert(key) {
            return Err(format!(
                "environment variable '{key}' was provided more than once; pass each --env key only once"
            ));
        }
        has_password_env |= key == "POSTGRES_PASSWORD";
    }
    if password.is_some() && has_password_env {
        return Err(
            "POSTGRES_PASSWORD cannot be set with both --password and --env; choose one".into(),
        );
    }
    Ok(())
}

struct StartPreflight {
    host_port: Option<u16>,
    extra_env: Vec<String>,
    password_from_env: Option<String>,
}

fn validate_start_options(
    name: Option<&str>,
    version: Option<&str>,
    port: Option<u16>,
    password: Option<&str>,
    extra_env: Vec<String>,
) -> Result<StartPreflight> {
    if let Some(name) = name {
        server::validate_server_name(name)?;
    }
    if let Some(version) = version {
        validate_pg_tag(version)?;
    }

    validate_pg_start_env_args(password, &extra_env).map_err(Error::Exec)?;
    let password_from_env = extra_env.iter().find_map(|assignment| {
        assignment
            .strip_prefix("POSTGRES_PASSWORD=")
            .map(str::to_string)
    });
    let validated_env = extra_env
        .into_iter()
        .filter(|assignment| !assignment.starts_with("POSTGRES_PASSWORD="))
        .collect();

    let host_port = port.map(|port| resolve_port(Some(port))).transpose()?;
    Ok(StartPreflight {
        host_port,
        extra_env: validated_env,
        password_from_env,
    })
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
    let preflight = validate_start_options(
        name.as_deref(),
        version.as_deref(),
        port,
        password.as_deref(),
        extra_env,
    )?;
    let host_port = preflight.host_port;
    let extra_env = preflight.extra_env;
    let password_from_env = preflight.password_from_env;

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
                    return Err(Error::Exec(format!(
                        "multiple postgres instances named '{}' ({}); pass --version to select one",
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
            if port.is_some()
                || user.is_some()
                || password.is_some()
                || database.is_some()
                || has_extra_env
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
        return Err(Error::Exec(format!(
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

    let container_id = docker::run_postgres(&docker, opts).await?;

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
    server::save_server_info(&info)?;

    let healthy = wait_running(&docker, &container_id, 30).await;
    if !healthy {
        let logs = docker::container_logs_tail(&docker, &container_id, 50)
            .await
            .unwrap_or_default();
        let _ = docker::stop_container(&docker, &container_id).await;
        let _ = docker::remove_container(&docker, &container_id).await;
        server::remove_server_info(&key);
        return Err(Error::DockerError(format!(
            "Postgres container '{}' did not start.\n--- container logs ---\n{}",
            user_name, logs
        )));
    }

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
            Err(Error::Exec(format!(
                "multiple postgres instances named '{}' ({}); pass --version to select one",
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

    let healthy = wait_running(docker, &container_id, 30).await;
    if !healthy {
        let logs = docker::container_logs_tail(docker, &container_id, 50)
            .await
            .unwrap_or_default();
        return Err(Error::DockerError(format!(
            "Postgres container '{}' did not resume.\n--- container logs ---\n{}",
            display_name, logs
        )));
    }

    let (user, password, database) = read_pg_env(docker, &container_id).await;

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

async fn wait_running(docker: &bollard::Docker, id: &str, attempts: usize) -> bool {
    for _ in 0..attempts {
        if docker::is_container_running(docker, id).await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    false
}

fn resolve_port(explicit: Option<u16>) -> Result<u16> {
    match explicit {
        Some(0) => {
            return Err(Error::Exec(
                "--port 0 is not allowed; pick a specific port or omit the flag".into(),
            ));
        }
        Some(port) if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() => {
            return Ok(port);
        }
        Some(port) => {
            return Err(Error::Exec(format!(
                "Postgres port {port} is already in use; choose another --port or omit --port to auto-select a free port"
            )));
        }
        None => {}
    }
    if std::net::TcpListener::bind(("127.0.0.1", DEFAULT_PG_PORT)).is_ok() {
        return Ok(DEFAULT_PG_PORT);
    }
    for p in (DEFAULT_PG_PORT + 1)..=(DEFAULT_PG_PORT + 100) {
        if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return Ok(p);
        }
    }
    Err(Error::Exec(
        "could not find a free TCP port for Postgres".into(),
    ))
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
    Err(Error::Exec(err.to_string()))
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

    #[test]
    fn resolve_port_rejects_zero() {
        let err = resolve_port(Some(0)).unwrap_err();
        assert!(matches!(err, Error::Exec(msg) if msg.contains("--port 0")));
    }

    #[test]
    fn parse_pg_port_rejects_zero_with_actionable_error() {
        let err = parse_pg_port_arg("0").unwrap_err();
        assert_eq!(
            err,
            "--port 0 is not allowed; pick a specific port or omit the flag"
        );
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
        assert!(
            matches!(err, Error::Exec(msg) if msg.contains(&format!("port {port} is already in use")) && msg.contains("omit --port"))
        );
    }

    #[test]
    fn validate_pg_tag_accepts_supported_majors() {
        for tag in [
            "17",
            "18",
            "17-alpine",
            "17.0",
            "18-bookworm",
            "18-alpine3.20",
            "18.1-alpine3.20",
            "18.01-Custom_variant-1.0",
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
            "",
            "18garbage",
            "18.1garbage",
            "18.",
            "18..1",
            "18.1.2",
            "18-",
            "18-.alpine",
            "18_alpine",
            "18 alpine",
            "18/alpine",
            "18:alpine",
        ] {
            assert!(
                validate_pg_tag(tag).is_err(),
                "expected `{}` to be rejected",
                tag
            );
        }
    }

    #[test]
    fn validate_pg_tag_enforces_docker_tag_length() {
        let max_length = format!("18-{}", "a".repeat(125));
        let too_long = format!("18-{}", "a".repeat(126));

        assert_eq!(max_length.len(), 128);
        assert!(validate_pg_tag(&max_length).is_ok());
        assert_eq!(too_long.len(), 129);
        assert!(validate_pg_tag(&too_long).is_err());
    }

    #[test]
    fn generate_password_is_24_alphanumeric() {
        let p = generate_password();
        assert_eq!(p.len(), 24);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn start_env_accepts_unique_assignments_and_equals_in_values() {
        let preflight = validate_start_options(
            Some("dev"),
            Some("18.1-alpine3.20"),
            None,
            None,
            vec![
                "APP_MODE=test".into(),
                "DATABASE_URL=postgres://user:pass@host/db?sslmode=require".into(),
                "POSTGRES_PASSWORD=a=b".into(),
            ],
        )
        .unwrap();

        assert_eq!(
            preflight.extra_env,
            [
                "APP_MODE=test",
                "DATABASE_URL=postgres://user:pass@host/db?sslmode=require"
            ]
        );
        assert_eq!(preflight.password_from_env.as_deref(), Some("a=b"));
        assert_eq!(preflight.host_port, None);
    }

    #[test]
    fn start_env_rejects_malformed_assignments() {
        for assignment in ["NO_EQUALS", "=value", "1KEY=value", "BAD-KEY=value"] {
            let error = validate_start_options(
                Some("dev"),
                Some("18"),
                None,
                None,
                vec![assignment.into()],
            )
            .err()
            .expect("malformed environment variable should fail");
            assert!(matches!(error, Error::Exec(_)), "{assignment}: {error}");
        }
    }

    #[test]
    fn start_env_rejects_duplicate_keys() {
        let error = validate_start_options(
            Some("dev"),
            Some("18"),
            None,
            None,
            vec!["APP_MODE=dev".into(), "APP_MODE=test".into()],
        )
        .err()
        .expect("duplicate environment variable should fail");

        assert!(
            matches!(error, Error::Exec(msg) if msg.contains("APP_MODE") && msg.contains("more than once"))
        );
    }

    #[test]
    fn start_env_rejects_generated_keys_except_password() {
        for assignment in [
            "POSTGRES_USER=admin",
            "POSTGRES_DB=app",
            "PGDATA=/tmp/postgres",
        ] {
            let error = validate_start_options(
                Some("dev"),
                Some("18"),
                None,
                None,
                vec![assignment.into()],
            )
            .err()
            .expect("reserved environment variable should fail");
            assert!(matches!(error, Error::Exec(msg) if msg.contains("managed by clickhousectl")));
        }
    }

    #[test]
    fn start_env_password_sources_are_unambiguous() {
        let error = validate_start_options(
            Some("dev"),
            Some("18"),
            None,
            Some("from-flag"),
            vec!["POSTGRES_PASSWORD=from-env".into()],
        )
        .err()
        .expect("password sources should conflict");
        assert!(matches!(error, Error::Exec(msg) if msg.contains("both --password and --env")));

        let error = validate_start_options(
            Some("dev"),
            Some("18"),
            None,
            None,
            vec![
                "POSTGRES_PASSWORD=first".into(),
                "POSTGRES_PASSWORD=second".into(),
            ],
        )
        .err()
        .expect("duplicate password should fail");
        assert!(
            matches!(error, Error::Exec(msg) if msg.contains("POSTGRES_PASSWORD") && msg.contains("more than once"))
        );
    }
}
