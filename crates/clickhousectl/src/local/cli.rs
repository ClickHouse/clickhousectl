use clap::{Args, Subcommand};

#[derive(Args)]
pub struct LocalArgs {
    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: LocalCommands,
}

#[derive(Subcommand)]
pub enum LocalCommands {
    /// Install a ClickHouse version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  `clickhousectl local use <version>` will auto-install if the version is missing and set as default.")]
    Install {
        /// Version to install. Accepts: "latest" (recommended), "stable", "lts", partial like "25.12", or exact like "25.12.9.61".
        version: String,

        /// Force re-install even if version is already installed
        #[arg(long)]
        force: bool,
    },

    /// List installed versions
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Without flags: shows locally installed versions (exact version strings).
  With --remote: shows versions available for download from builds.clickhouse.com.
  Use the exact version strings from this output with `clickhousectl local remove` or `clickhousectl local use`.
  Related: `clickhousectl local install <version>` to install, `clickhousectl local which` to see current default.")]
    List {
        /// List versions available for download
        #[arg(long)]
        remote: bool,
    },

    /// Set the default version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Sets the default ClickHouse version used by `clickhousectl local client` and `clickhousectl local server`.
  Accepts version specs: \"latest\" (recommended), \"stable\", \"lts\", partial like \"25.12\", or exact like \"25.12.5.44\".
  Auto-installs the version if not already present.
  Also creates `~/.local/bin/clickhouse` as a symlink to the version's binary so the `clickhouse` command is on PATH. Pass --no-global to skip.
  Related: `clickhousectl local which` to verify, `clickhousectl local server start` to start a server.")]
    Use {
        /// Version to use as default. Accepts: "latest" (recommended), "stable", "lts", partial like "25.12", or exact like "25.12.5.44".
        version: String,

        /// Do not create or update the ~/.local/bin/clickhouse symlink
        #[arg(long)]
        no_global: bool,
    },

    /// Remove an installed version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Removes an installed ClickHouse version from ~/.clickhouse/versions/.
  Takes an exact version string as shown by `clickhousectl local list` (e.g., \"25.12.5.44\").
  Does NOT accept keywords like \"stable\" — use the exact version number.
  Fails if a local server is currently running on this version; stop it first, or pass
  --force to stop the running server(s) before removing.
  Related: `clickhousectl local list` to see installed versions.")]
    Remove {
        /// Version to remove
        version: String,

        /// Stop any running servers using this version, then remove it
        #[arg(long)]
        force: bool,
    },

    /// Show the current default version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows the current default version and binary path. No arguments needed.
  Use this to verify which version is active before running commands.
  Related: `clickhousectl local use <version>` to change the default.")]
    Which,

    /// Initialize a project-local ClickHouse configuration
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Creates a .clickhouse/ directory (runtime data, git-ignored) plus clickhouse/ and postgres/
  project scaffolds (each subdir has a .gitkeep). clickhouse/: tables/, materialized_views/,
  queries/, seed/. postgres/: tables/, views/, functions/, queries/, seed/. The clickhouse/ and
  postgres/ directories are meant to be committed — organize your SQL files there.
  Related: `clickhousectl local server start` to start a server with project-local data.")]
    Init,

    /// Connect to a running ClickHouse server with clickhouse-client
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Two connection modes:
  1. Named server: `clickhousectl local client --name dev` — looks up port and version from a
     locally managed server started via `clickhousectl local server start`. Defaults to \"default\".
  2. Explicit host/port: `clickhousectl local client --host myhost --port 9000` — connects to any
     ClickHouse server directly, bypassing local server lookup.
  --query and --queries-file execute SQL inline or from a file.
  Additional clickhouse-client args can be passed after --.
  Related: `clickhousectl local server start` to start a local server, `clickhousectl local server list` to see servers.")]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short)]
        name: Option<String>,

        /// Host to connect to (bypasses local server lookup)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to (bypasses local server lookup if set)
        #[arg(long, short)]
        port: Option<u16>,

        /// Execute a SQL query
        #[arg(long, short)]
        query: Option<String>,

        /// Execute queries from a SQL file
        #[arg(long)]
        queries_file: Option<String>,

        /// Additional arguments to pass to clickhouse-client
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage local ClickHouse server instances
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage named ClickHouse server instances. Each server has its own data directory.
  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.
  Typical: `clickhousectl local server start` (starts \"default\"), `clickhousectl local server start --name test`.
  Related: `clickhousectl local client` to connect to a running server.")]
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },

    /// Manage local Postgres instances (Docker-backed)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage named Postgres server instances backed by Docker. Each instance is keyed on
  (name, major version) and runs as a `postgres:<tag>` container with data bind-mounted
  at .clickhouse/servers/<name>-pg<major>/data/.
  Typical: `clickhousectl local postgres start` (starts \"default\" on port 5432).
  `local server list` shows ClickHouse + Postgres entries together.
  Requires Docker to be installed and running.")]
    Postgres {
        #[command(subcommand)]
        command: PostgresCommands,
    },
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Start a ClickHouse server instance
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Starts a named clickhouse-server instance with its own data directory.
  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.
  Without --name, the first server is called \"default\"; if \"default\" is already running,
  a random name is generated (e.g., \"bold-crane\").
  Use --name to give a server a stable identity (e.g., --name dev, --name test).
  Use --version (-v) to run a specific ClickHouse version without changing the default.
  Accepts same specs as install/use: \"latest\" (recommended), stable, lts, 25.12, etc. Installs if needed.
  With no --version and no default set, a bare start bootstraps by installing \"latest\" (without
  setting it as the default, so you keep tracking latest on later starts).
  Ports default to 8123 (HTTP) and 9000 (TCP). If they're in use, free ports are auto-assigned.
  Use --http-port and --tcp-port to set explicit ports.
  Runs in background by default. Use --foreground (-F / --fg) to run in foreground.
  If --name is given and that server is already running, the command will error.
  Shows count of already-running servers before starting.
  Use --config-file <NAME> to apply a custom ClickHouse config file from ~/.clickhouse/configs/
  (see `clickhousectl local server configs`). The file is merged as an overlay on top of
  ClickHouse's built-in defaults (via config.d), so it can contain just the settings you want
  to change (e.g. <query_log>). The data directory and ports stay managed regardless of the
  file's contents (they are forced as command-line overrides).
  Related: `clickhousectl local server list` to see servers, `clickhousectl local server stop <name>` to stop one.")]
    Start {
        /// Server name (default: \"default\", or random if default is already running)
        #[arg(long)]
        name: Option<String>,

        /// ClickHouse version to use (e.g. "latest" (recommended), stable, lts, 25.12). Installs if needed. Does not change the default version.
        #[arg(long, short = 'v')]
        version: Option<String>,

        /// HTTP port (default: 8123, auto-assigns a free port if in use)
        #[arg(long)]
        http_port: Option<u16>,

        /// TCP port (default: 9000, auto-assigns a free port if in use)
        #[arg(long)]
        tcp_port: Option<u16>,

        /// Run server in foreground (default: background)
        #[arg(long, alias = "fg", short = 'F')]
        foreground: bool,

        /// Overlay a named config file from ~/.clickhouse/configs/ on top of the defaults (see `server configs`)
        #[arg(long, value_name = "NAME")]
        config_file: Option<String>,

        /// Arguments to pass to clickhouse-server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List custom config files available to `server start --config-file`
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Lists ClickHouse config files in ~/.clickhouse/configs/ and prints that directory's path.
  Drop a config file there (e.g. analytics.xml) and start a server with it via
  `clickhousectl local server start --config-file analytics`. The file is overlaid on top of
  ClickHouse's built-in defaults (config.d merge), so it only needs the settings you want to
  change. Files may be .xml, .yaml, or .yml; reference them by name with or without the
  extension.
  Related: `clickhousectl local server start --config-file <NAME>`.")]
    Configs,

    /// List all server instances (running and stopped)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows all named ClickHouse server instances and their status.
  Automatically cleans up stale entries for processes that are no longer running.
  Shows name, status (running/stopped), PID, version, and ports.
  Related: `clickhousectl local server start` to start a server, `clickhousectl local server stop <name>` to stop one.")]
    List {
        /// System-wide maintenance only: list servers across all projects. You almost certainly want the default project-scoped list instead.
        #[arg(long)]
        global: bool,
    },

    /// Stop a running server by name
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops a named ClickHouse server. Use the name from `clickhousectl local server list`.
  Sends SIGTERM first, then SIGKILL if the process doesn't exit gracefully.
  The server's data directory is preserved — restart with `clickhousectl local server start --name <name>`.
  Idempotent: a server that exists but is already stopped exits 0 (no error).
  An unknown server name still errors so typos are caught.
  Related: `clickhousectl local server list` to see servers.")]
    Stop {
        /// Name of the server to stop
        name: String,

        /// System-wide maintenance only: stop a server from any project. You almost certainly want the default project-scoped stop instead.
        #[arg(long)]
        global: bool,

        /// Project directory to disambiguate when using --global
        #[arg(long, requires = "global")]
        project: Option<String>,
    },

    /// Stop all running server instances
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops all running ClickHouse server instances.
  Sends SIGTERM first, then SIGKILL if processes don't exit.
  Data directories are preserved.
  Related: `clickhousectl local server list` to see servers.")]
    StopAll {
        /// System-wide maintenance only: stop all servers across all projects. You almost certainly want the default project-scoped stop-all instead.
        #[arg(long)]
        global: bool,
    },

    /// Remove a stopped server and its data
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Permanently deletes a server's data directory. The server must be stopped first.
  This is irreversible — all data for this server instance will be lost.
  Related: `clickhousectl local server stop <name>` to stop first, `clickhousectl local server list` to see servers.")]
    Remove {
        /// Name of the server to remove
        name: String,
    },

    /// Write ClickHouse connection env vars to a .env file
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Writes CLICKHOUSE_HOST, CLICKHOUSE_PORT, and CLICKHOUSE_HTTP_PORT into a .env file
  (or .env.local with --local) based on a running server's actual connection details.
  Optionally includes CLICKHOUSE_USER, CLICKHOUSE_PASSWORD, and CLICKHOUSE_DATABASE when
  the corresponding flags are provided.
  If the file already exists, existing CLICKHOUSE_* vars are replaced in-place. Otherwise the file is created.
  Useful for configuring apps that read from dotenv files.
  Related: `clickhousectl local server start` to start a server, `clickhousectl local server list` to see servers.")]
    Dotenv {
        /// Server name (default: "default")
        #[arg(long)]
        name: Option<String>,

        /// Write to .env.local instead of .env
        #[arg(long)]
        local: bool,

        /// Include CLICKHOUSE_USER with this value
        #[arg(long)]
        user: Option<String>,

        /// Include CLICKHOUSE_PASSWORD with this value
        #[arg(long)]
        password: Option<String>,

        /// Include CLICKHOUSE_DATABASE with this value
        #[arg(long)]
        database: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PostgresCommands {
    /// Start a Postgres container
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Starts a named Postgres server backed by a Docker container.
  Without --name, the first server is called \"default\"; if \"default\" is running,
  a random name is generated (e.g. \"bold-crane\").
  --version (-v) selects a postgres image tag (17 or 18 — e.g. 17, 17-alpine, 18.1, 18-bookworm).
  Defaults to 18. Image is pulled if not already present locally.
  --port defaults to 5432; if taken, a free port is auto-assigned.
  Data persists at .clickhouse/servers/<name>/data/ and is bind-mounted into the container.
  A random POSTGRES_PASSWORD is generated unless --password or `-e POSTGRES_PASSWORD=...` is given.
  Containers are labeled `clickhousectl.engine=postgres`, `clickhousectl.name=<name>`,
  `clickhousectl.project=<cwd>`, `created_by=clickhousectl_<version>` for safe discovery.
  Requires Docker to be installed and running.")]
    Start {
        /// Server name (default: "default", or random if default is already running)
        #[arg(long)]
        name: Option<String>,

        /// Postgres image tag (17 or 18 — e.g. 17, 17-alpine, 18.1, 18-bookworm). Default: 18. Pulls if missing.
        #[arg(long, short = 'v')]
        version: Option<String>,

        /// Host TCP port (default: 5432, auto-assigns a free port if in use)
        #[arg(long)]
        port: Option<u16>,

        /// POSTGRES_USER (default: postgres)
        #[arg(long)]
        user: Option<String>,

        /// POSTGRES_PASSWORD (default: random 24-char alphanumeric)
        #[arg(long)]
        password: Option<String>,

        /// POSTGRES_DB (default: postgres)
        #[arg(long)]
        database: Option<String>,

        /// Extra env vars for the container, repeatable: -e KEY=VALUE
        #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
        env: Vec<String>,
    },

    /// Stop a running Postgres container by name
    Stop {
        /// Name of the server to stop
        name: String,
        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,
    },

    /// Stop all running Postgres containers in this project
    StopAll,

    /// Remove a stopped Postgres server and its data directory
    Remove {
        /// Name of the server to remove
        name: String,
        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,
    },

    /// Connect to a running Postgres instance with psql
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Two connection modes:
  1. Named server: `clickhousectl local postgres client --name dev` — looks up the host port
     and credentials from a locally managed Postgres started via `local postgres start`.
     Defaults to \"default\".
  2. Explicit host/port: `clickhousectl local postgres client --host myhost --port 5432`.
  If `psql` is on PATH on the host, it is execed directly. Otherwise, falls back to running
  `psql` inside the container via Docker exec (no host psql required).
  --query and --queries-file pass through to psql (-c / -f).
  Additional psql args can be passed after --.")]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short)]
        name: Option<String>,

        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,

        /// Host to connect to (bypasses local server lookup)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to (bypasses local server lookup if set)
        #[arg(long, short)]
        port: Option<u16>,

        /// Execute a single SQL query
        #[arg(long, short)]
        query: Option<String>,

        /// Execute queries from a SQL file
        #[arg(long)]
        queries_file: Option<String>,

        /// Additional arguments to pass to psql
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Write Postgres connection env vars to a .env file
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Writes POSTGRES_HOST, POSTGRES_PORT, POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DATABASE
  into .env (or .env.local with --local) based on a running Postgres server.
  If the file already exists, existing POSTGRES_* vars are replaced in-place.")]
    Dotenv {
        /// Server name (default: "default")
        #[arg(long)]
        name: Option<String>,

        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,

        /// Write to .env.local instead of .env
        #[arg(long)]
        local: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn local_command(args: &[&str]) -> LocalCommands {
        let mut argv = vec!["clickhousectl", "local"];
        argv.extend_from_slice(args);
        let cli = Cli::try_parse_from(argv).unwrap();
        let Commands::Local(local) = cli.command else {
            panic!("expected local command");
        };
        local.command
    }

    #[test]
    fn parses_remove_without_force() {
        let LocalCommands::Remove { version, force } = local_command(&["remove", "25.12.5.44"])
        else {
            panic!("expected remove");
        };
        assert_eq!(version, "25.12.5.44");
        assert!(!force);
    }

    #[test]
    fn parses_remove_with_force() {
        let LocalCommands::Remove { version, force } =
            local_command(&["remove", "25.12.5.44", "--force"])
        else {
            panic!("expected remove");
        };
        assert_eq!(version, "25.12.5.44");
        assert!(force);
    }

    #[test]
    fn parses_server_start_config_file() {
        let LocalCommands::Server {
            command: ServerCommands::Start { config_file, .. },
        } = local_command(&["server", "start", "--config-file", "analytics"])
        else {
            panic!("expected server start");
        };
        assert_eq!(config_file.as_deref(), Some("analytics"));
    }

    #[test]
    fn server_start_config_file_defaults_to_none() {
        let LocalCommands::Server {
            command: ServerCommands::Start { config_file, args, .. },
        } = local_command(&["server", "start"])
        else {
            panic!("expected server start");
        };
        assert_eq!(config_file, None);
        assert!(args.is_empty());
    }

    #[test]
    fn parses_server_configs() {
        let LocalCommands::Server {
            command: ServerCommands::Configs,
        } = local_command(&["server", "configs"])
        else {
            panic!("expected server configs");
        };
    }
}
