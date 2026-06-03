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

/// Accept Postgres image tags whose major version is 17 or 18 — anything
/// else is unsupported for now. Examples that pass: `17`, `17.0`, `17-alpine`,
/// `18-bookworm`, `18.1-alpine3.20`. Examples that fail: `latest`, `16`, `19`.
pub(crate) fn validate_pg_tag(tag: &str) -> Result<()> {
    let major: String = tag.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !matches!(major.as_str(), "17" | "18") {
        return Err(Error::Exec(format!(
            "postgres version '{}' is not supported. Use a 17 or 18 image tag \
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
        PostgresCommands::Dotenv { name, version, local } => {
            dotenv(name.as_deref(), version.as_deref(), local, json)
        }
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
    server::recover_current_project_servers();

    // User-facing name (no version suffix). Defaults to "default" when no
    // postgres "default" is currently running.
    let user_name = match name.as_deref() {
        Some(n) => {
            server::validate_server_name(n)?;
            n.to_string()
        }
        None => default_pg_name(),
    };

    // If `--version` is omitted but there's already exactly one instance for
    // this name, resume it — the user almost certainly wants their existing
    // data, not a freshly-initialized DEFAULT_PG_TAG. With multiple
    // instances, we ask them to disambiguate. Only when zero exist do we
    // default to DEFAULT_PG_TAG.
    let (tag, major) = match version.as_deref() {
        Some(v) => {
            validate_pg_tag(v)?;
            (v.to_string(), pg_major_from_tag(v))
        }
        None => {
            let existing = server::find_pg_instances(&user_name);
            match existing.len() {
                0 => (DEFAULT_PG_TAG.to_string(), pg_major_from_tag(DEFAULT_PG_TAG)),
                1 => {
                    let info = &existing[0];
                    let stored_tag = info.version.strip_prefix("postgres:").unwrap_or(&info.version);
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
                || !extra_env.is_empty()
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
    if !docker::image_exists(&docker, tag).await? {
        eprintln!("Pulling postgres:{tag}...");
        docker::pull_image(&docker, tag).await?;
    }

    let host_port = resolve_port(port)?;

    server::ensure_pg_data_dir(&user_name, &major)?;
    let data_dir = server::pg_data_dir(&user_name, &major);

    // Defensive cleanup of any unmanaged container colliding on our chosen
    // container name (only if labels confirm we own it).
    docker::ensure_name_free(&docker, &user_name, &major, &project_cwd).await?;

    let user = user.unwrap_or_else(|| DEFAULT_USER.to_string());
    let database = database.unwrap_or_else(|| DEFAULT_DATABASE.to_string());

    let password_from_env = extra_env
        .iter()
        .find_map(|kv| kv.strip_prefix("POSTGRES_PASSWORD="))
        .map(|s| s.to_string());
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
    let any_default_running = server::find_pg_instances("default")
        .iter()
        .any(|i| i
            .container_id
            .as_deref()
            .map(docker::is_container_running_blocking)
            .unwrap_or(false));
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
fn resolve_pg_target(
    user_name: &str,
    version: Option<&str>,
) -> Result<server::ServerInfo> {
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
async fn resume_existing(
    docker: &bollard::Docker,
    prior: ServerInfo,
    json: bool,
) -> Result<()> {
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
    if let Some(p) = explicit {
        if p == 0 {
            return Err(Error::Exec(
                "--port 0 is not allowed; pick a specific port or omit the flag".into(),
            ));
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
    Err(Error::Exec("could not find a free TCP port for Postgres".into()))
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
        let display = format!(
            "{} ({})",
            user_name_from_key(&target.name),
            target.version
        );
        println!("Stopping Postgres {}...", display);
    }
    server::kill_server(&target.name)?;
    let out = output::ServerStopOutput {
        name: user_name_from_key(&target.name).to_string(),
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
    let mut stop_entries = Vec::new();
    for s in &servers {
        if !json {
            print!("Stopping '{}'...", s.name);
        }
        match server::kill_server(&s.name) {
            Ok(()) => {
                if !json {
                    println!(" stopped");
                }
                stop_entries.push(output::ServerStopEntry {
                    name: s.name.clone(),
                    stopped: true,
                    error: None,
                });
            }
            Err(e) => {
                if !json {
                    println!(" error: {}", e);
                }
                stop_entries.push(output::ServerStopEntry {
                    name: s.name.clone(),
                    stopped: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    if json {
        let out = output::ServerStopAllOutput {
            servers: stop_entries,
        };
        output::print_output(&out, json);
    } else if servers.is_empty() {
        println!("No running Postgres servers");
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
        return exec_host_psql(&h, p, DEFAULT_USER, None, DEFAULT_DATABASE, query, queries_file, extra_args);
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
    let mut psql_args: Vec<String> = vec![
        "-U".into(), user,
        "-d".into(), database,
    ];
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
    cmd.arg("-h").arg(host)
        .arg("-p").arg(port.to_string())
        .arg("-U").arg(user)
        .arg("-d").arg(database);
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
    let err = cmd.exec();
    Err(Error::Exec(err.to_string()))
}

fn dotenv(
    name: Option<&str>,
    version: Option<&str>,
    use_local: bool,
    json: bool,
) -> Result<()> {
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
    fn resolve_port_passes_through_explicit_value() {
        // Use a port unlikely to be bound; we just want the passthrough path.
        assert_eq!(resolve_port(Some(54321)).unwrap(), 54321);
    }

    #[test]
    fn validate_pg_tag_accepts_supported_majors() {
        for tag in ["17", "18", "17-alpine", "17.0", "18-bookworm", "18.1-alpine3.20"] {
            assert!(validate_pg_tag(tag).is_ok(), "expected `{}` to be accepted", tag);
        }
    }

    #[test]
    fn validate_pg_tag_rejects_unsupported() {
        for tag in ["latest", "15", "16", "16-alpine", "19", "14-alpine", "alpine", ""] {
            assert!(validate_pg_tag(tag).is_err(), "expected `{}` to be rejected", tag);
        }
    }

    #[test]
    fn generate_password_is_24_alphanumeric() {
        let p = generate_password();
        assert_eq!(p.len(), 24);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
