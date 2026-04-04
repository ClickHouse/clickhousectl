pub mod cli;
pub mod output;
pub mod server;

use cli::{LocalCommands, ServerCommands};

use crate::error::{Error, Result};
use crate::{init, paths, version_manager};
use std::os::unix::process::CommandExt;
use std::process::Command;

pub async fn run(cmd: LocalCommands, json: bool) -> Result<()> {
    match cmd {
        LocalCommands::Install { version, force } => install(&version, force, json).await,
        LocalCommands::List { remote } => {
            if remote {
                list_available(json).await
            } else {
                list_installed(json)
            }
        }
        LocalCommands::Use { version } => use_version(&version, json).await,
        LocalCommands::Remove { version } => remove(&version, json),
        LocalCommands::Which => which(json),
        LocalCommands::Init => {
            init::init()?;
            let out = output::InitOutput {
                path: ".clickhouse/".to_string(),
            };
            output::print_output(&out, json);
            Ok(())
        }
        LocalCommands::Client {
            name,
            host,
            port,
            query,
            queries_file,
            args,
        } => run_client(name, host, port, query, queries_file, args),
        LocalCommands::Server { command } => run_server_commands(command, json).await,
    }
}

async fn install(version_spec: &str, force: bool, json: bool) -> Result<()> {
    let spec = version_manager::parse_version_spec(version_spec)?;
    let platform = version_manager::platform::Platform::detect()?;

    eprintln!("Resolving {}...", spec);
    let resolved = version_manager::resolve::resolve(&spec, &platform).await?;

    let version = version_manager::install::install_resolved(&resolved, &platform, force).await?;

    // If this is the first installed version, set it as default
    let set_as_default = version_manager::get_default_version().is_err();
    if set_as_default {
        version_manager::set_default_version(&version)?;
        if !json {
            eprintln!("Set as default version");
        }
    }

    let out = output::InstallOutput {
        version,
        set_as_default,
    };
    output::print_output(&out, json);

    Ok(())
}

fn list_installed(json: bool) -> Result<()> {
    let versions = version_manager::list_installed_versions()?;
    let default = version_manager::get_default_version().ok();

    let out = output::ListInstalledOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let is_default = Some(&v) == default.as_ref();
                output::InstalledVersion {
                    version: v,
                    default: is_default,
                }
            })
            .collect(),
    };
    output::print_output(&out, json);

    Ok(())
}

async fn list_available(json: bool) -> Result<()> {
    eprintln!("Checking available versions on builds.clickhouse.com...");
    let versions = version_manager::list_available_versions_from_builds().await?;

    let installed = version_manager::list_installed_versions().unwrap_or_default();

    let out = output::ListAvailableOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let prefix = format!("{}.", v);
                let is_installed = installed
                    .iter()
                    .any(|iv| iv.starts_with(&prefix) || iv == &v);
                output::AvailableVersion {
                    version: v,
                    installed: is_installed,
                }
            })
            .collect(),
    };
    output::print_output(&out, json);

    Ok(())
}

async fn use_version(version_spec: &str, json: bool) -> Result<()> {
    let spec = version_manager::parse_version_spec(version_spec)?;
    let platform = version_manager::platform::Platform::detect()?;

    eprintln!("Resolving {}...", spec);
    let resolved = version_manager::resolve::resolve(&spec, &platform).await?;

    // If exact version is known, check if already installed
    let version = if let Some(ref v) = resolved.exact_version {
        let installed = version_manager::list_installed_versions()?;
        if installed.contains(v) {
            v.clone()
        } else {
            eprintln!("Version {} not installed, installing...", v);
            version_manager::install::install_resolved(&resolved, &platform, false).await?
        }
    } else {
        // Version not known upfront (builds source) — install will detect it
        version_manager::install::install_resolved(&resolved, &platform, false).await?
    };

    version_manager::set_default_version(&version)?;
    let out = output::UseOutput { version };
    output::print_output(&out, json);
    Ok(())
}

fn remove(version: &str, json: bool) -> Result<()> {
    let version_dir = paths::version_dir(version)?;

    if !version_dir.exists() {
        return Err(Error::VersionNotFound(version.to_string()));
    }

    // Check if this is the default version
    if let Ok(default) = version_manager::get_default_version()
        && default == version
    {
        let default_file = paths::default_file()?;
        let _ = std::fs::remove_file(default_file);
    }

    std::fs::remove_dir_all(&version_dir)?;
    let out = output::RemoveOutput {
        version: version.to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

fn which(json: bool) -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;
    let out = output::WhichOutput {
        version,
        binary_path: binary.display().to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

fn run_client(
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    query: Option<String>,
    queries_file: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    // If --host or --port is set, connect directly (bypass local server lookup).
    // Otherwise, look up the named server for port and version.
    let (resolved_host, tcp_port, version) = if host.is_some() || port.is_some() {
        let h = host.unwrap_or_else(|| "localhost".to_string());
        let p = port.unwrap_or(9000);
        let v = version_manager::get_default_version()?;
        (h, p, v)
    } else {
        let server_name = name.as_deref().unwrap_or("default");
        let entries = server::list_all_servers();
        let entry = entries
            .iter()
            .find(|e| e.name == server_name)
            .ok_or_else(|| Error::ServerNotFound(server_name.to_string()))?;
        let info = entry
            .info
            .as_ref()
            .ok_or_else(|| Error::ServerNotRunning(server_name.to_string()))?;
        ("localhost".to_string(), info.tcp_port, info.version.clone())
    };

    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    let mut cmd = Command::new(&binary);
    cmd.arg("client")
        .arg("--host")
        .arg(&resolved_host)
        .arg("--port")
        .arg(tcp_port.to_string());

    if let Some(q) = &query {
        cmd.arg("--query").arg(q);
    }

    if let Some(f) = &queries_file {
        cmd.arg("--queries-file").arg(f);
    }

    cmd.args(&args);
    let err = cmd.exec();
    Err(Error::Exec(err.to_string()))
}

async fn start_server(
    name: Option<String>,
    version_spec: Option<String>,
    http_port: Option<u16>,
    tcp_port: Option<u16>,
    foreground: bool,
    args: Vec<String>,
    json: bool,
) -> Result<()> {
    if json && foreground {
        return Err(Error::JsonForegroundConflict);
    }

    // Resolve server name and check for collisions before any downloads
    let server_name = server::resolve_name(name.as_deref());

    if name.is_some() && server::is_server_running(&server_name) {
        return Err(Error::ServerAlreadyRunning(server_name));
    }

    let version = if let Some(spec_str) = &version_spec {
        let spec = version_manager::parse_version_spec(spec_str)?;
        let platform = version_manager::platform::Platform::detect()?;
        eprintln!("Resolving {}...", spec);
        let resolved = version_manager::resolve::resolve(&spec, &platform).await?;
        version_manager::install::ensure_installed(&resolved, &platform).await?
    } else {
        version_manager::get_default_version()?
    };
    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    // Show running server count
    let running = server::running_server_count();
    if running > 0 {
        eprintln!(
            "Note: {} server{} already running (use `clickhousectl local server list` to see them)",
            running,
            if running == 1 { "" } else { "s" }
        );
    }

    let (http_port, tcp_port, auto_assigned) = server::resolve_ports(http_port, tcp_port)?;
    if auto_assigned {
        eprintln!(
            "Note: default ports in use, auto-assigned HTTP:{} TCP:{}",
            http_port, tcp_port
        );
    }
    // Check if the user passed their own config in the trailing args (after --).
    // If not, we inject our managed data directory and --path flag.
    // If they did, we leave ClickHouse to use their config as-is.
    let has_config = args
        .iter()
        .any(|a| a.starts_with("--config-file") || a.starts_with("-C"));

    let mut cmd = Command::new(&binary);
    cmd.arg("server");

    if !has_config {
        server::ensure_server_data_dir(&server_name)?;
        cmd.current_dir(server::server_data_dir(&server_name));
        cmd.args(init::server_flags());
    }

    cmd.args(server::port_flags(http_port, tcp_port));
    cmd.args(&args);

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    if !foreground {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        let child = cmd.spawn().map_err(|e| Error::Exec(e.to_string()))?;
        let pid = child.id();

        let info = server::ServerInfo {
            name: server_name.clone(),
            pid,
            version: version.clone(),
            http_port,
            tcp_port,
            started_at: server::now_timestamp(),
            cwd,
        };
        server::save_server_info(&info)?;

        // Check that it actually started
        server::check_spawn_health(pid, &server_name)?;

        let out = output::ServerStartOutput {
            name: server_name,
            pid,
            http_port,
            tcp_port,
            version,
        };
        output::print_output(&out, json);
        Ok(())
    } else {
        let mut child = cmd.spawn().map_err(|e| Error::Exec(e.to_string()))?;
        let pid = child.id();

        let info = server::ServerInfo {
            name: server_name.clone(),
            pid,
            version: version.clone(),
            http_port,
            tcp_port,
            started_at: server::now_timestamp(),
            cwd,
        };
        server::save_server_info(&info)?;

        eprintln!(
            "Server '{}' running (PID: {}, HTTP: {}, TCP: {})",
            server_name, pid, http_port, tcp_port
        );

        let status = child.wait().map_err(|e| Error::Exec(e.to_string()))?;
        server::remove_server_info(&server_name);

        if !status.success()
            && let Some(code) = status.code()
        {
            std::process::exit(code);
        }
        Ok(())
    }
}

async fn run_server_commands(command: ServerCommands, json: bool) -> Result<()> {
    match command {
        ServerCommands::Start {
            name,
            version,
            http_port,
            tcp_port,
            foreground,
            args,
        } => start_server(name, version, http_port, tcp_port, foreground, args, json).await,
        ServerCommands::List => {
            let entries = server::list_all_servers();
            let running_count = entries.iter().filter(|e| e.running).count();
            let total = entries.len();

            let out = output::ServerListOutput {
                servers: entries
                    .into_iter()
                    .map(|e| {
                        let (pid, version, http_port, tcp_port) = match e.info {
                            Some(info) => (
                                Some(info.pid),
                                Some(info.version),
                                Some(info.http_port),
                                Some(info.tcp_port),
                            ),
                            None => (None, None, None, None),
                        };
                        output::ServerListEntry {
                            name: e.name,
                            running: e.running,
                            pid,
                            version,
                            http_port,
                            tcp_port,
                        }
                    })
                    .collect(),
                total_servers: total,
                total_running_servers: running_count,
            };
            output::print_output(&out, json);
            Ok(())
        }
        ServerCommands::Stop { name } => {
            if !json {
                println!("Stopping server '{}'...", name);
            }
            server::kill_server(&name)?;
            let out = output::ServerStopOutput { name };
            output::print_output(&out, json);
            Ok(())
        }
        ServerCommands::StopAll => {
            let servers = server::list_running_servers();
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
                println!("No running servers");
            } else {
                println!("Done");
            }
            Ok(())
        }
        ServerCommands::Remove { name } => {
            if server::is_server_running(&name) {
                return Err(Error::ServerAlreadyRunning(name));
            }
            let data_dir = server::server_data_dir(&name);
            if !data_dir.exists() {
                return Err(Error::ServerNotFound(name));
            }
            // Remove the whole server directory (parent of data/)
            let server_dir = data_dir.parent().unwrap();
            std::fs::remove_dir_all(server_dir)?;
            server::remove_server_info(&name);
            let out = output::ServerRemoveOutput { name };
            output::print_output(&out, json);
            Ok(())
        }
    }
}
