mod cli;
mod cloud;
mod error;
mod init;
mod local_output;
mod paths;
mod server;
mod skills;
mod user_agent;
mod version_manager;

use clap::Parser;
use cli::{
    ActivityCommands, AuthCommands, BackupCommands, BackupConfigCommands, Cli, CloudArgs,
    CloudCommands, Commands, InvitationCommands, KeyCommands, LocalCommands,
    MemberCommands, OrgCommands, PrivateEndpointCommands, QueryEndpointCommands, ServerCommands,
    ServiceCommands, SkillsArgs,
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
        Commands::Local(args) => run_local(args.command, args.json).await,
        Commands::Skills(args) => run_skills(args).await,
        Commands::Cloud(args) => run_cloud(*args).await,
    }
}

async fn run_skills(args: SkillsArgs) -> Result<()> {
    skills::install(args).await
}

async fn run_local(cmd: LocalCommands, json: bool) -> Result<()> {
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
            let output = local_output::InitOutput {
                path: ".clickhouse/".to_string(),
            };
            local_output::print_output(&output, json);
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
        LocalCommands::Server { command } => run_server_commands(command, json),
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

    let output = local_output::InstallOutput {
        version,
        set_as_default,
    };
    local_output::print_output(&output, json);

    Ok(())
}

fn list_installed(json: bool) -> Result<()> {
    let versions = version_manager::list_installed_versions()?;
    let default = version_manager::get_default_version().ok();

    let output = local_output::ListInstalledOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let is_default = Some(&v) == default.as_ref();
                local_output::InstalledVersion {
                    version: v,
                    default: is_default,
                }
            })
            .collect(),
    };
    local_output::print_output(&output, json);

    Ok(())
}

async fn list_available(json: bool) -> Result<()> {
    eprintln!("Checking available versions on builds.clickhouse.com...");
    let versions = version_manager::list_available_versions_from_builds().await?;

    let installed = version_manager::list_installed_versions().unwrap_or_default();

    let output = local_output::ListAvailableOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let prefix = format!("{}.", v);
                let is_installed = installed
                    .iter()
                    .any(|iv| iv.starts_with(&prefix) || iv == &v);
                local_output::AvailableVersion {
                    version: v,
                    installed: is_installed,
                }
            })
            .collect(),
    };
    local_output::print_output(&output, json);

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
    let output = local_output::UseOutput { version };
    local_output::print_output(&output, json);
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
    let output = local_output::RemoveOutput {
        version: version.to_string(),
    };
    local_output::print_output(&output, json);
    Ok(())
}

fn which(json: bool) -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;
    let output = local_output::WhichOutput {
        version,
        binary_path: binary.display().to_string(),
    };
    local_output::print_output(&output, json);
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
    json: bool,
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

        let output = local_output::ServerStartOutput {
            name: server_name,
            pid,
            http_port,
            tcp_port,
            version,
        };
        local_output::print_output(&output, json);
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

fn run_server_commands(command: ServerCommands, json: bool) -> Result<()> {
    match command {
        ServerCommands::Start {
            name,
            http_port,
            tcp_port,
            foreground,
            args,
        } => start_server(name, http_port, tcp_port, foreground, args, json),
        ServerCommands::List => {
            let entries = server::list_all_servers();
            let running_count = entries.iter().filter(|e| e.running).count();
            let total = entries.len();

            let output = local_output::ServerListOutput {
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
                        local_output::ServerListEntry {
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
            local_output::print_output(&output, json);
            Ok(())
        }
        ServerCommands::Stop { name } => {
            if !json {
                println!("Stopping server '{}'...", name);
            }
            server::kill_server(&name)?;
            let output = local_output::ServerStopOutput { name };
            local_output::print_output(&output, json);
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
                        stop_entries.push(local_output::ServerStopEntry {
                            name: s.name.clone(),
                            stopped: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        if !json {
                            println!(" error: {}", e);
                        }
                        stop_entries.push(local_output::ServerStopEntry {
                            name: s.name.clone(),
                            stopped: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            if json {
                let output = local_output::ServerStopAllOutput {
                    servers: stop_entries,
                };
                local_output::print_output(&output, json);
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
            let output = local_output::ServerRemoveOutput { name };
            local_output::print_output(&output, json);
            Ok(())
        }
    }
}

async fn run_cloud(args: CloudArgs) -> Result<()> {
    // Auth subcommands don't need a client
    if let CloudCommands::Auth { command } = args.command {
        return match command {
            AuthCommands::Login {
                interactive,
                api_key,
                api_secret,
            } => {
                if interactive {
                    // Interactive prompt for API key/secret
                    cloud::commands::auth_interactive().map_err(|e| Error::Cloud(e.to_string()))
                } else if api_key.is_some() || api_secret.is_some() {
                    // Non-interactive API key login
                    let key = api_key.ok_or_else(|| {
                        Error::Cloud("--api-key is required when --api-secret is provided".into())
                    })?;
                    let secret = api_secret.ok_or_else(|| {
                        Error::Cloud("--api-secret is required when --api-key is provided".into())
                    })?;
                    let creds = cloud::credentials::Credentials {
                        api_key: key,
                        api_secret: secret,
                    };
                    cloud::credentials::save_credentials(&creds)
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    println!(
                        "Credentials saved to {}",
                        cloud::credentials::credentials_path().display()
                    );
                    Ok(())
                } else {
                    // Default: OAuth device flow
                    let url = args
                        .url
                        .as_deref()
                        .unwrap_or("https://api.clickhouse.cloud");
                    let tokens = cloud::auth::device_auth_login(url)
                        .await
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    cloud::auth::save_tokens(&tokens).map_err(|e| Error::Cloud(e.to_string()))?;
                    println!("Logged in successfully.");
                    println!("Tokens saved to {}", cloud::auth::tokens_path().display());
                    Ok(())
                }
            }
            AuthCommands::Logout => {
                cloud::auth::clear_tokens();
                cloud::credentials::clear_credentials();
                println!("Logged out. All saved credentials cleared.");
                Ok(())
            }
            AuthCommands::Status => {
                match cloud::auth::load_tokens() {
                    Some(tokens) if cloud::auth::is_token_valid(&tokens) => {
                        println!("OAuth: logged in (token valid, url: {})", tokens.api_url);
                    }
                    Some(tokens) => {
                        println!(
                            "OAuth: token expired, url: {} (run `clickhousectl cloud auth login` to refresh)",
                            tokens.api_url
                        );
                    }
                    None => {
                        println!("OAuth: not logged in");
                    }
                }
                let creds = cloud::credentials::load_credentials();
                if creds.is_some() {
                    println!(
                        "API keys: configured ({})",
                        cloud::credentials::credentials_path().display()
                    );
                } else {
                    println!("API keys: not configured");
                }
                Ok(())
            }
        };
    }

    // Refresh OAuth tokens if needed before creating the client
    cloud::auth::ensure_fresh_tokens()
        .await
        .map_err(|e| Error::Cloud(e.to_string()))?;

    let client = CloudClient::new(
        args.api_key.as_deref(),
        args.api_secret.as_deref(),
        args.url.as_deref(),
    )
    .map_err(|e| Error::Cloud(e.to_string()))?;

    let json = args.json;

    let result = match args.command {
        CloudCommands::Auth { .. } => unreachable!("handled above"),
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
                cloud::commands::service_delete(
                    &client,
                    &service_id,
                    force,
                    org_id.as_deref(),
                    json,
                )
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
                    cloud::commands::ServiceScaleOptions {
                        min_replica_memory_gb,
                        max_replica_memory_gb,
                        num_replicas,
                        idle_scaling,
                        idle_timeout_minutes,
                        org_id,
                    },
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
