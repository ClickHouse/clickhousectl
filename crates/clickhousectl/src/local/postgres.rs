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
        PostgresCommands::Stop { name } => stop(&name, json).await,
        PostgresCommands::StopAll => stop_all(json).await,
        PostgresCommands::Remove { name } => remove(&name, json),
        PostgresCommands::Client {
            name,
            host,
            port,
            query,
            queries_file,
            args,
        } => client(name, host, port, query, queries_file, args).await,
        PostgresCommands::Dotenv { name, local } => dotenv(name.as_deref(), local, json),
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

    let server_name = server::resolve_name(name.as_deref())?;

    if name.is_some() && server::is_server_running(&server_name) {
        return Err(Error::ServerAlreadyRunning(server_name));
    }

    let docker = docker::connect().await?;

    let tag = version.as_deref().unwrap_or("latest");
    if !docker::image_exists(&docker, tag).await? {
        eprintln!("Pulling postgres:{tag}...");
        docker::pull_image(&docker, tag).await?;
    }

    let host_port = resolve_port(port)?;

    server::ensure_server_data_dir(&server_name)?;
    let data_dir = server::server_data_dir(&server_name);

    let project_cwd = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    // Clear out any stale stopped container with the same name (only if we
    // can prove we own it via labels) so create_container doesn't 409.
    docker::ensure_name_free(&docker, &server_name, &project_cwd).await?;

    let user = user.unwrap_or_else(|| DEFAULT_USER.to_string());
    let database = database.unwrap_or_else(|| DEFAULT_DATABASE.to_string());

    // If POSTGRES_PASSWORD is in extra_env, prefer it; else use --password; else generate.
    let password_from_env = extra_env
        .iter()
        .find_map(|kv| kv.strip_prefix("POSTGRES_PASSWORD="))
        .map(|s| s.to_string());
    let password = password_from_env
        .or(password)
        .unwrap_or_else(generate_password);

    let opts = PostgresRunOpts {
        server_name: &server_name,
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
        name: server_name.clone(),
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

    // Health check: wait up to 3s for the container to be running.
    let healthy = wait_running(&docker, &container_id, 30).await;
    if !healthy {
        let logs = docker::container_logs_tail(&docker, &container_id, 50)
            .await
            .unwrap_or_default();
        // Best-effort cleanup so we don't leave a broken container.
        let _ = docker::stop_container(&docker, &container_id).await;
        let _ = docker::remove_container(&docker, &container_id).await;
        server::remove_server_info(&server_name);
        return Err(Error::DockerError(format!(
            "Postgres container '{}' did not start.\n--- container logs ---\n{}",
            server_name, logs
        )));
    }

    let out = output::PostgresStartOutput {
        name: server_name,
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

async fn stop(name: &str, json: bool) -> Result<()> {
    server::validate_server_name(name)?;
    server::recover_current_project_servers();
    if !json {
        println!("Stopping Postgres '{}'...", name);
    }
    server::kill_server(name)?;
    let out = output::ServerStopOutput {
        name: name.to_string(),
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

fn remove(name: &str, json: bool) -> Result<()> {
    server::validate_server_name(name)?;
    server::recover_current_project_servers();
    if server::is_server_running(name) {
        return Err(Error::ServerAlreadyRunning(name.to_string()));
    }
    let data_dir = server::server_data_dir(name);
    if !data_dir.exists() {
        return Err(Error::ServerNotFound(name.to_string()));
    }
    let server_dir = data_dir.parent().unwrap();
    std::fs::remove_dir_all(server_dir)?;
    server::remove_server_info(name);
    let out = output::ServerRemoveOutput {
        name: name.to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn client(
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    query: Option<String>,
    queries_file: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    let server_name = name.as_deref().unwrap_or("default").to_string();

    if host.is_some() || port.is_some() {
        // Direct connect — no server lookup; require host psql.
        let h = host.unwrap_or_else(|| "127.0.0.1".to_string());
        let p = port.unwrap_or(DEFAULT_PG_PORT);
        return exec_host_psql(&h, p, DEFAULT_USER, None, DEFAULT_DATABASE, query, queries_file, extra_args);
    }

    server::recover_current_project_servers();
    let entries = server::list_all_servers();
    let entry = entries
        .iter()
        .find(|e| e.name == server_name)
        .ok_or_else(|| Error::ServerNotFound(server_name.clone()))?;
    let info = entry
        .info
        .as_ref()
        .ok_or_else(|| Error::ServerNotRunning(server_name.clone()))?;
    if info.engine != Engine::Postgres {
        return Err(Error::Exec(format!(
            "server '{}' is a {} server, not Postgres. Use `local client` for ClickHouse.",
            server_name,
            info.engine.as_str()
        )));
    }

    // Read connection info from container env (POSTGRES_USER / DB).
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

    // Build psql args for docker exec.
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

    docker::exec_psql_in_container(&docker, container_id, &psql_args).await
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

fn dotenv(name: Option<&str>, use_local: bool, json: bool) -> Result<()> {
    let server_name = name.unwrap_or("default");
    let entries = server::list_all_servers();
    let entry = entries
        .iter()
        .find(|e| e.name == server_name)
        .ok_or_else(|| Error::ServerNotFound(server_name.to_string()))?;
    let info = entry
        .info
        .as_ref()
        .ok_or_else(|| Error::ServerNotRunning(server_name.to_string()))?;
    if info.engine != Engine::Postgres {
        return Err(Error::Exec(format!(
            "server '{}' is a {} server, not Postgres",
            server_name,
            info.engine.as_str()
        )));
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
    fn generate_password_is_24_alphanumeric() {
        let p = generate_password();
        assert_eq!(p.len(), 24);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
