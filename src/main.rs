mod cli;
mod cloud;
mod error;
mod init;
mod paths;
mod version_manager;

use clap::Parser;
use cli::{
    BackupCommands, CloudArgs, CloudCommands, Cli, Commands, OrgCommands, RunArgs, RunCommands,
    ServiceCommands,
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
        Commands::Install { version } => install(&version).await,
        Commands::List { remote } => {
            if remote {
                list_available().await
            } else {
                list_installed()
            }
        }
        Commands::Use { version } => use_version(&version).await,
        Commands::Remove { version } => remove(&version),
        Commands::Which => which(),
        Commands::Init => {
            init::init()?;
            Ok(())
        }
        Commands::Run(args) => run_clickhouse(args),
        Commands::Cloud(args) => run_cloud(args).await,
    }
}

async fn install(version_spec: &str) -> Result<()> {
    println!("Resolving version {}...", version_spec);
    let entry = version_manager::resolve_version(version_spec).await?;
    println!("Resolved to version {} ({})", entry.version, entry.channel);

    version_manager::install_version(&entry.version, &entry.channel).await?;
    Ok(())
}

fn list_installed() -> Result<()> {
    let versions = version_manager::list_installed_versions()?;
    let default = version_manager::get_default_version().ok();

    if versions.is_empty() {
        println!("No versions installed");
        println!("Run: clickhousectl install stable");
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
        version_manager::install_version(version, &entry.channel).await?;
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

fn run_clickhouse(args: RunArgs) -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    // If --sql is provided, run clickhouse local with the query
    if let Some(sql) = args.sql {
        let mut cmd = Command::new(&binary);
        cmd.arg("local").arg("--query").arg(&sql);
        let err = cmd.exec();
        return Err(Error::Exec(err.to_string()));
    }

    // Otherwise, handle subcommands
    match args.command {
        Some(RunCommands::Server { args }) => {
            let has_config = args
                .iter()
                .any(|a| a.starts_with("--config-file") || a.starts_with("-C"));
            let mut cmd = Command::new(&binary);
            cmd.arg("server");
            cmd.args(&args);
            if !has_config {
                init::ensure_initialized(&version)?;
                cmd.current_dir(init::version_data_dir(&version));
                cmd.args(init::server_flags());
            }
            let err = cmd.exec();
            Err(Error::Exec(err.to_string()))
        }
        Some(RunCommands::Client { args }) => {
            let mut cmd = Command::new(&binary);
            cmd.arg("client").args(&args);
            let err = cmd.exec();
            Err(Error::Exec(err.to_string()))
        }
        Some(RunCommands::Local { args }) => {
            let mut cmd = Command::new(&binary);
            cmd.arg("local").args(&args);
            let err = cmd.exec();
            Err(Error::Exec(err.to_string()))
        }
        None => {
            eprintln!("Usage: clickhousectl run --sql <QUERY>");
            eprintln!("       clickhousectl run server [ARGS...]");
            eprintln!("       clickhousectl run client [ARGS...]");
            eprintln!("       clickhousectl run local [ARGS...]");
            std::process::exit(1);
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
        },
        CloudCommands::Service { command } => match command {
            ServiceCommands::List { org_id } => {
                cloud::commands::service_list(&client, org_id.as_deref(), json).await
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
                byoc_id,
                compliance_type,
                profile,
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
                    byoc_id,
                    compliance_type,
                    profile,
                    org_id,
                };
                cloud::commands::service_create(&client, opts, json).await
            }
            ServiceCommands::Delete { service_id, org_id } => {
                cloud::commands::service_delete(&client, &service_id, org_id.as_deref()).await
            }
            ServiceCommands::Start { service_id, org_id } => {
                cloud::commands::service_start(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Stop { service_id, org_id } => {
                cloud::commands::service_stop(&client, &service_id, org_id.as_deref(), json).await
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
