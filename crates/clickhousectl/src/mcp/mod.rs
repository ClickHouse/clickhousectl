//! `clickhousectl mcp`: register and launch mcp-clickhouse.
//!
//! `mcp add` writes an agent config entry that points at `mcp run`, never at
//! mcp-clickhouse directly. `mcp run` resolves the connection each time the
//! agent starts it and `exec()`s mcp-clickhouse with the matching environment,
//! so no settings or credentials are stored in the agent config.

pub mod agent;
pub mod cli;
pub mod client_config;

use cli::{AddArgs, McpAgent, McpCommands, RunArgs, Source};

use crate::error::{Error, Result};
use crate::local::server;
use crate::{local, paths, version_manager};
use serde::Serialize;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The PyPI package (and console script) `mcp run` launches.
const MCP_PACKAGE: &str = "mcp-clickhouse";

pub async fn run(cmd: McpCommands, json: bool) -> Result<()> {
    match cmd {
        McpCommands::Add(args) => add(args, json).await,
        McpCommands::Run(args) => run_server(args).await,
    }
}

#[derive(Serialize)]
struct AddOutput {
    agent: &'static str,
    file: String,
    name: String,
    status: agent::WriteStatus,
    entry: serde_json::Value,
}

async fn add(args: AddArgs, json: bool) -> Result<()> {
    let McpAgent::Claude = args.agent;
    let mut source = args.source();
    if let Some(Source::ClientConfig {
        path,
        connection,
        http_port,
    }) = &mut source
    {
        // Store an absolute path: the agent may launch from another directory.
        if let Some(given) = path {
            *given = std::fs::canonicalize(&*given).map_err(|e| {
                Error::Mcp(format!(
                    "Client config {} is not readable: {e}",
                    given.display()
                ))
            })?;
        }
        // Fail now, not on the agent's first launch, if the connection cannot
        // be resolved or mapped to an HTTP port.
        resolve_client_config(path.as_deref(), connection.as_deref(), *http_port, json).await?;
    }

    let entry = agent::server_entry(source.as_ref());
    let cwd = std::env::current_dir()?;
    let status = agent::write_claude_project(&cwd, &args.name, entry.clone())?;

    if json {
        let out = AddOutput {
            agent: "claude",
            file: agent::CLAUDE_PROJECT_FILE.to_string(),
            name: args.name,
            status,
            entry,
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        let verb = match status {
            agent::WriteStatus::Created => "Added",
            agent::WriteStatus::Updated => "Updated",
            agent::WriteStatus::Unchanged => "Unchanged:",
        };
        println!(
            "{verb} MCP server '{}' in {} (Claude Code)",
            args.name,
            agent::CLAUDE_PROJECT_FILE
        );
        println!(
            "Claude Code asks you to approve project MCP servers the first time it loads them."
        );
    }
    Ok(())
}

async fn run_server(args: RunArgs) -> Result<()> {
    let env = match args.connection.into_source() {
        Source::Local(name) => local_env(&name)?,
        Source::ClientConfig {
            path,
            connection,
            http_port,
        } => {
            resolve_client_config(path.as_deref(), connection.as_deref(), http_port, false).await?
        }
    };

    let uv = find_on_path("uv").ok_or_else(|| {
        Error::Mcp(
            "`uv` was not found on PATH; it is needed to run mcp-clickhouse. \
             Install it from https://docs.astral.sh/uv/getting-started/installation/"
                .into(),
        )
    })?;

    let mut cmd = Command::new(uv);
    // `uv tool run` runs in an isolated environment, unlike `uv run`, which
    // would sync a uv project in the agent's working directory.
    cmd.args(["tool", "run", MCP_PACKAGE]);
    // An inherited database would otherwise apply to a connection that set none.
    cmd.env_remove("CLICKHOUSE_DATABASE");
    cmd.envs(env);

    // stdout is the MCP channel: mcp-clickhouse inherits it, and nothing
    // above writes to it. `exec()` replaces this process, so record the
    // telemetry handoff first, as `local client` does.
    #[cfg(feature = "telemetry")]
    crate::telemetry::finalize_before_exec();
    let err = cmd.exec();
    Err(Error::Exec(err.to_string()))
}

/// mcp-clickhouse settings for a chctl-managed server in this project.
fn local_env(name: &str) -> Result<Vec<(&'static str, String)>> {
    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let entry = server::server_entry_locked(name, &metadata_lock)?
        .ok_or_else(|| Error::ServerNotFound(name.to_string()))?;
    let info = entry
        .info
        .filter(|_| entry.running)
        .ok_or_else(|| Error::ServerNotRunning(name.to_string()))?;
    if info.engine != server::Engine::Clickhouse {
        return Err(Error::Mcp(format!(
            "Server '{name}' is a Postgres server; mcp-clickhouse needs a ClickHouse server"
        )));
    }
    Ok(vec![
        ("CLICKHOUSE_HOST", "localhost".into()),
        ("CLICKHOUSE_PORT", info.http_port.to_string()),
        ("CLICKHOUSE_SECURE", "false".into()),
        ("CLICKHOUSE_VERIFY", "true".into()),
        ("CLICKHOUSE_USER", "default".into()),
        ("CLICKHOUSE_PASSWORD", String::new()),
    ])
}

async fn resolve_client_config(
    path: Option<&Path>,
    connection: Option<&str>,
    http_port: Option<u16>,
    json: bool,
) -> Result<Vec<(&'static str, String)>> {
    let config = match path {
        Some(path) => path.to_path_buf(),
        None => client_config::find_default_config()?,
    };
    if !config.is_file() {
        return Err(Error::Mcp(format!(
            "Client config {} does not exist",
            config.display()
        )));
    }
    let binary = clickhouse_binary(json).await?;
    let resolved = client_config::resolve(
        &mut |key| client_config::extract_key(&binary, &config, key),
        connection,
    )?;
    let port = client_config::http_port(&resolved, http_port)?;
    Ok(client_config::mcp_env(&resolved, port))
}

/// The ClickHouse binary used to read client configs: the `local use` default.
/// Only when no version is installed at all does this run `local use latest`,
/// which installs it, makes it the default and links it onto PATH. Later runs
/// keep using that default rather than tracking latest.
async fn clickhouse_binary(json: bool) -> Result<PathBuf> {
    let version = match version_manager::get_default_version() {
        Ok(version) => version,
        Err(Error::NoDefaultVersion) if version_manager::list_installed_versions()?.is_empty() => {
            if !json {
                eprintln!(
                    "No ClickHouse version installed; running `clickhousectl local use latest`"
                );
            }
            local::activate_version(&version_manager::VersionSpec::Latest, false, json).await?
        }
        Err(Error::NoDefaultVersion) => {
            return Err(Error::Mcp(
                "ClickHouse versions are installed but none is the default. \
                 Run `clickhousectl local use <version>` (see `clickhousectl local list`)."
                    .into(),
            ));
        }
        Err(Error::VersionNotFound(version)) => {
            return Err(Error::Mcp(format!(
                "Default ClickHouse version '{version}' is not installed. \
                 Repair it with `clickhousectl local use <version>`."
            )));
        }
        Err(error) => return Err(error),
    };
    let binary = paths::binary_path(&version)?;
    local::ensure_launchable(&binary, &version)?;
    Ok(binary)
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}
