mod cli;
mod cloud;
mod error;
mod init;
mod paths;
mod server;
mod skills;
mod version_manager;

use clap::Parser;
use cli::{
    ActivityCommands, BackupCommands, BackupConfigCommands, Cli, CloudArgs, CloudCommands,
    Commands, InvitationCommands, KeyCommands, LocalCommands, MemberCommands, OrgCommands,
    PrivateEndpointCommands, QueryEndpointCommands, ServerCommands, ServiceCommands, SkillsArgs,
};
use cloud::CloudClient;
use error::{Error, Result};
use std::os::unix::process::CommandExt;
use std::process::Command;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = run(cli.command).await;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Local { command } => run_local(command).await,
        Commands::Skills(args) => run_skills(args).await,
        Commands::Cloud(args) => run_cloud(args).await,
    }
}

async fn run_skills(args: SkillsArgs) -> Result<()> {
    skills::install(args).await
}

async fn run_local(cmd: LocalCommands) -> Result<()> {
    match cmd {
        LocalCommands::Install { version } => install(&version).await,
        LocalCommands::List { remote } => {
            if remote {
                list_available().await
            } else {
                list_installed()
            }
        }
        LocalCommands::Use { version } => use_version(&version).await,
        LocalCommands::Remove { version } => remove(&version),
        LocalCommands::Which => which(),
        LocalCommands::Init => {
            init::init()?;
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
        LocalCommands::Server { command } => run_server_commands(command),
    }
}

async fn install(version_spec: &str) -> Result<()> {
    println!("Resolving version {}...", version_spec);
    let entry = version_manager::resolve_version(version_spec).await?;
    println!("Resolved to version {} ({})", entry.version, entry.channel);

    version_manager::install_version(&entry.version, entry.channel).await?;
    Ok(())
}

fn list_installed() -> Result<()> {
    let versions = version_manager::list_installed_versions()?;
    let default = version_manager::get_default_version().ok();

    if versions.is_empty() {
        println!("No versions installed");
        println!("Run: clickhousectl local install stable");
        return Ok(());
    }

    println!("Installed versions:");
    for v in versions {
        if Some(&v) == default.as_ref() {
            println!("  {} (default)", v);
        } else {
            println!("  {}", v);
        }
    }

    Ok(())
}

async fn list_available() -> Result<()> {
    println!("Fetching available versions...");
    let versions = version_manager::list_available_versions().await?;

    if versions.is_empty() {
        println!("No versions available");
        return Ok(());
    }

    let installed = version_manager::list_installed_versions().unwrap_or_default();

    println!("Available versions:");
    for entry in versions.iter().take(20) {
        if installed.contains(&entry.version) {
            println!("  {} [{}] (installed)", entry.version, entry.channel);
        } else {
            println!("  {} [{}]", entry.version, entry.channel);
        }
    }

    if versions.len() > 20 {
        println!("  ... and {} more", versions.len() - 20);
    }

    Ok(())
}

async fn use_version(version_spec: &str) -> Result<()> {
    println!("Resolving version {}...", version_spec);
    let entry = version_manager::resolve_version(version_spec).await?;
    let version = &entry.version;

    // Install if not already installed
    let installed = version_manager::list_installed_versions()?;
    if !installed.contains(version) {
        println!("Version {} not installed, installing...", version);
        version_manager::install_version(version, entry.channel).await?;
    }

    version_manager::set_default_version(version)?;
    println!("Default version set to {}", version);
    Ok(())
}

fn remove(version: &str) -> Result<()> {
    let version_dir = paths::version_dir(version)?;

    if !version_dir.exists() {
        return Err(Error::VersionNotFound(version.to_string()));
    }

    // Check if this is the default version
    if let Ok(default) = version_manager::get_default_version() {
        if default == version {
            let default_file = paths::default_file()?;
            let _ = std::fs::remove_file(default_file);
        }
    }

    std::fs::remove_dir_all(&version_dir)?;
    println!("Removed version {}", version);
    Ok(())
}

fn which() -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;
    println!("{} ({})", version, binary.display());
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

fn start_server(
    name: Option<String>,
    http_port: Option<u16>,
    tcp_port: Option<u16>,
    foreground: bool,
    args: Vec<String>,
) -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    // Resolve server name
    let server_name = server::resolve_name(name.as_deref());

    // If an explicit name was given and it's already running, error
    if name.is_some() && server::is_server_running(&server_name) {
        return Err(Error::ServerAlreadyRunning(server_name));
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

        println!(
            "Server '{}' started in background (PID: {})",
            server_name, pid
        );
        println!("  HTTP port: {}", http_port);
        println!("  TCP port:  {}", tcp_port);
        println!("  Version:   {}", version);
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

fn run_server_commands(command: ServerCommands) -> Result<()> {
    match command {
        ServerCommands::Start {
            name,
            http_port,
            tcp_port,
            foreground,
            args,
        } => start_server(name, http_port, tcp_port, foreground, args),
        ServerCommands::List => {
            let entries = server::list_all_servers();
            if entries.is_empty() {
                println!("No servers");
                return Ok(());
            }
            println!("Servers:");
            let mut running_count = 0;
            for e in &entries {
                if e.running {
                    running_count += 1;
                    let info = e.info.as_ref().unwrap();
                    println!(
                        "  {} [running] PID {} v{} HTTP:{} TCP:{}",
                        e.name, info.pid, info.version, info.http_port, info.tcp_port
                    );
                } else {
                    println!("  {} [stopped]", e.name);
                }
            }
            println!(
                "\n{} server{}, {} running",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" },
                running_count
            );
            Ok(())
        }
        ServerCommands::Stop { name } => {
            println!("Stopping server '{}'...", name);
            server::kill_server(&name)?;
            println!("Server '{}' stopped", name);
            Ok(())
        }
        ServerCommands::StopAll => {
            let servers = server::list_running_servers();
            if servers.is_empty() {
                println!("No running servers");
                return Ok(());
            }
            for s in &servers {
                print!("Stopping '{}'...", s.name);
                match server::kill_server(&s.name) {
                    Ok(()) => println!(" stopped"),
                    Err(e) => println!(" error: {}", e),
                }
            }
            println!("Done");
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
            println!("Server '{}' removed", name);
            Ok(())
        }
    }
}

async fn run_cloud(args: CloudArgs) -> Result<()> {
    if let CloudCommands::Auth = &args.command {
        return cloud::commands::auth_interactive().map_err(|e| Error::Cloud(e.to_string()));
    }

    let client = CloudClient::new(args.api_key.as_deref(), args.api_secret.as_deref())
        .map_err(|e| Error::Cloud(e.to_string()))?;

    let json = args.json;

    let result = match args.command {
        CloudCommands::Org { command } => match command {
            OrgCommands::List => cloud::commands::org_list(&client, json).await,
            OrgCommands::Get { org_id } => cloud::commands::org_get(&client, &org_id, json).await,
            OrgCommands::Update {
                org_id,
                name,
                remove_private_endpoint,
                enable_core_dumps,
            } => {
                let opts = cloud::commands::OrgUpdateOptions {
                    name,
                    remove_private_endpoints: remove_private_endpoint,
                    enable_core_dumps,
                };
                cloud::commands::org_update(&client, &org_id, opts, json).await
            }
            OrgCommands::Prometheus {
                org_id,
                filtered_metrics,
            } => cloud::commands::org_prometheus(&client, &org_id, filtered_metrics, json).await,
            OrgCommands::Usage {
                org_id,
                from_date,
                to_date,
                filter,
            } => {
                cloud::commands::org_usage(&client, &org_id, &from_date, &to_date, &filter, json)
                    .await
            }
        },
        CloudCommands::Service { command } => match command {
            ServiceCommands::List { org_id, filter } => {
                cloud::commands::service_list(&client, org_id.as_deref(), &filter, json).await
            }
            ServiceCommands::Get { service_id, org_id } => {
                cloud::commands::service_get(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Create {
                name,
                provider,
                region,
                min_replica_memory_gb,
                max_replica_memory_gb,
                num_replicas,
                idle_scaling,
                idle_timeout_minutes,
                ip_allow,
                backup_id,
                release_channel,
                data_warehouse_id,
                readonly,
                encryption_key,
                encryption_role,
                enable_tde,
                compliance_type,
                profile,
                tag,
                enable_endpoint,
                disable_endpoint,
                private_preview_terms_checked,
                enable_core_dumps,
                org_id,
            } => {
                let opts = cloud::commands::CreateServiceOptions {
                    name,
                    provider,
                    region,
                    min_replica_memory_gb,
                    max_replica_memory_gb,
                    num_replicas,
                    idle_scaling,
                    idle_timeout_minutes,
                    ip_allow,
                    backup_id,
                    release_channel,
                    data_warehouse_id,
                    is_readonly: readonly,
                    encryption_key,
                    encryption_role,
                    enable_tde,
                    compliance_type,
                    profile,
                    tags: tag,
                    enable_endpoints: enable_endpoint,
                    disable_endpoints: disable_endpoint,
                    private_preview_terms_checked,
                    enable_core_dumps,
                    org_id,
                };
                cloud::commands::service_create(&client, opts, json).await
            }
            ServiceCommands::Delete {
                service_id,
                force,
                org_id,
            } => {
                cloud::commands::service_delete(&client, &service_id, force, org_id.as_deref(), json)
                    .await
            }
            ServiceCommands::Start { service_id, org_id } => {
                cloud::commands::service_start(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Stop { service_id, org_id } => {
                cloud::commands::service_stop(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Update {
                service_id,
                name,
                add_ip_allow,
                remove_ip_allow,
                add_private_endpoint_id,
                remove_private_endpoint_id,
                release_channel,
                enable_endpoint,
                disable_endpoint,
                transparent_data_encryption_key_id,
                add_tag,
                remove_tag,
                enable_core_dumps,
                org_id,
            } => {
                let opts = cloud::commands::ServiceUpdateOptions {
                    name,
                    add_ip_allow,
                    remove_ip_allow,
                    add_private_endpoint_ids: add_private_endpoint_id,
                    remove_private_endpoint_ids: remove_private_endpoint_id,
                    release_channel,
                    enable_endpoints: enable_endpoint,
                    disable_endpoints: disable_endpoint,
                    transparent_data_encryption_key_id,
                    add_tags: add_tag,
                    remove_tags: remove_tag,
                    enable_core_dumps,
                    org_id,
                };
                cloud::commands::service_update(&client, &service_id, opts, json).await
            }
            ServiceCommands::Scale {
                service_id,
                min_replica_memory_gb,
                max_replica_memory_gb,
                num_replicas,
                idle_scaling,
                idle_timeout_minutes,
                org_id,
            } => {
                cloud::commands::service_scale(
                    &client,
                    &service_id,
                    min_replica_memory_gb,
                    max_replica_memory_gb,
                    num_replicas,
                    idle_scaling,
                    idle_timeout_minutes,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ServiceCommands::ResetPassword {
                service_id,
                new_password_hash,
                new_double_sha1_hash,
                org_id,
            } => {
                let opts = cloud::commands::ServiceResetPasswordOptions {
                    new_password_hash,
                    new_double_sha1_hash,
                    org_id,
                };
                cloud::commands::service_reset_password(&client, &service_id, opts, json).await
            }
            ServiceCommands::QueryEndpoint { command } => match command {
                QueryEndpointCommands::Get { service_id, org_id } => {
                    cloud::commands::query_endpoint_get(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                QueryEndpointCommands::Create {
                    service_id,
                    role,
                    open_api_key,
                    allowed_origins,
                    org_id,
                } => {
                    let opts = cloud::commands::QueryEndpointCreateOptions {
                        roles: role,
                        open_api_keys: open_api_key,
                        allowed_origins,
                        org_id,
                    };
                    cloud::commands::query_endpoint_create(&client, &service_id, opts, json).await
                }
                QueryEndpointCommands::Delete { service_id, org_id } => {
                    cloud::commands::query_endpoint_delete(&client, &service_id, org_id.as_deref())
                        .await
                }
            },
            ServiceCommands::PrivateEndpoint { command } => match command {
                PrivateEndpointCommands::Create {
                    service_id,
                    endpoint_id,
                    description,
                    org_id,
                } => {
                    cloud::commands::private_endpoint_create(
                        &client,
                        &service_id,
                        &endpoint_id,
                        description.as_deref(),
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                PrivateEndpointCommands::GetConfig { service_id, org_id } => {
                    cloud::commands::private_endpoint_get_config(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ServiceCommands::BackupConfig { command } => match command {
                BackupConfigCommands::Get { service_id, org_id } => {
                    cloud::commands::backup_config_get(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                BackupConfigCommands::Update {
                    service_id,
                    backup_period_hours,
                    backup_retention_period_hours,
                    backup_start_time,
                    org_id,
                } => {
                    let opts = cloud::commands::BackupConfigUpdateOptions {
                        backup_period_hours,
                        backup_retention_period_hours,
                        backup_start_time,
                        org_id,
                    };
                    cloud::commands::backup_config_update(&client, &service_id, opts, json).await
                }
            },
            ServiceCommands::Client {
                name,
                id,
                query,
                queries_file,
                user,
                password,
                allow_mismatched_client_version,
                generate_password,
                org_id,
                args,
            } => {
                let opts = cloud::commands::ServiceClientOptions {
                    name,
                    id,
                    query,
                    queries_file,
                    user,
                    password,
                    allow_mismatched_client_version,
                    generate_password,
                    org_id,
                    args,
                };
                cloud::commands::service_client(&client, opts).await
            }
            ServiceCommands::Prometheus {
                service_id,
                org_id,
                filtered_metrics,
            } => {
                cloud::commands::service_prometheus(
                    &client,
                    &service_id,
                    org_id.as_deref(),
                    filtered_metrics,
                )
                .await
            }
        },
        CloudCommands::Member { command } => match command {
            MemberCommands::List { org_id } => {
                cloud::commands::member_list(&client, org_id.as_deref(), json).await
            }
            MemberCommands::Get { user_id, org_id } => {
                cloud::commands::member_get(&client, &user_id, org_id.as_deref(), json).await
            }
            MemberCommands::Update {
                user_id,
                role_id,
                org_id,
            } => {
                cloud::commands::member_update(&client, &user_id, &role_id, org_id.as_deref(), json)
                    .await
            }
            MemberCommands::Remove { user_id, org_id } => {
                cloud::commands::member_remove(&client, &user_id, org_id.as_deref()).await
            }
        },
        CloudCommands::Invitation { command } => match command {
            InvitationCommands::List { org_id } => {
                cloud::commands::invitation_list(&client, org_id.as_deref(), json).await
            }
            InvitationCommands::Create {
                email,
                role_id,
                org_id,
            } => {
                cloud::commands::invitation_create(
                    &client,
                    &email,
                    &role_id,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            InvitationCommands::Get {
                invitation_id,
                org_id,
            } => {
                cloud::commands::invitation_get(&client, &invitation_id, org_id.as_deref(), json)
                    .await
            }
            InvitationCommands::Delete {
                invitation_id,
                org_id,
            } => {
                cloud::commands::invitation_delete(&client, &invitation_id, org_id.as_deref()).await
            }
        },
        CloudCommands::Key { command } => match command {
            KeyCommands::List { org_id } => {
                cloud::commands::key_list(&client, org_id.as_deref(), json).await
            }
            KeyCommands::Create {
                name,
                role_id,
                expires_at,
                state,
                ip_allow,
                hash_key_id,
                hash_key_id_suffix,
                hash_key_secret,
                org_id,
            } => {
                let opts = cloud::commands::KeyCreateOptions {
                    name,
                    role_ids: role_id,
                    expires_at,
                    state,
                    ip_allow,
                    hash_key_id,
                    hash_key_id_suffix,
                    hash_key_secret,
                    org_id,
                };
                cloud::commands::key_create(&client, opts, json).await
            }
            KeyCommands::Get { key_id, org_id } => {
                cloud::commands::key_get(&client, &key_id, org_id.as_deref(), json).await
            }
            KeyCommands::Update {
                key_id,
                name,
                role_id,
                expires_at,
                state,
                ip_allow,
                org_id,
            } => {
                let opts = cloud::commands::KeyUpdateOptions {
                    name,
                    role_ids: role_id,
                    expires_at,
                    state,
                    ip_allow,
                    org_id,
                };
                cloud::commands::key_update(&client, &key_id, opts, json).await
            }
            KeyCommands::Delete { key_id, org_id } => {
                cloud::commands::key_delete(&client, &key_id, org_id.as_deref()).await
            }
        },
        CloudCommands::Activity { command } => match command {
            ActivityCommands::List {
                org_id,
                from_date,
                to_date,
            } => {
                cloud::commands::activity_list(
                    &client,
                    org_id.as_deref(),
                    from_date.as_deref(),
                    to_date.as_deref(),
                    json,
                )
                .await
            }
            ActivityCommands::Get {
                activity_id,
                org_id,
            } => {
                cloud::commands::activity_get(&client, &activity_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Auth => unreachable!("handled above"),
        CloudCommands::Backup { command } => match command {
            BackupCommands::List { service_id, org_id } => {
                cloud::commands::backup_list(&client, &service_id, org_id.as_deref(), json).await
            }
            BackupCommands::Get {
                service_id,
                backup_id,
                org_id,
            } => {
                cloud::commands::backup_get(
                    &client,
                    &service_id,
                    &backup_id,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
        },
    };

    result.map_err(|e| Error::Cloud(e.to_string()))
}
