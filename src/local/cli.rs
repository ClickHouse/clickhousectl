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
        /// Version to install (e.g., latest, stable, lts, 25, 25.12, 25.12.9.61)
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
  Accepts version specs: \"stable\", \"lts\", partial like \"25.12\", or exact like \"25.12.5.44\".
  Auto-installs the version if not already present.
  Related: `clickhousectl local which` to verify, `clickhousectl local server start` to start a server.")]
    Use {
        /// Version to use as default
        version: String,
    },

    /// Remove an installed version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Removes an installed ClickHouse version from ~/.clickhouse/versions/.
  Takes an exact version string as shown by `clickhousectl local list` (e.g., \"25.12.5.44\").
  Does NOT accept keywords like \"stable\" — use the exact version number.
  Related: `clickhousectl local list` to see installed versions.")]
    Remove {
        /// Version to remove
        version: String,
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
  Creates a .clickhouse/ directory (runtime data, git-ignored) and a clickhouse/ project
  scaffold with subdirs: tables/, materialized_views/, queries/, seed/ (each with .gitkeep).
  The clickhouse/ directory is meant to be committed — organize your SQL files there.
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
  Subcommands: start, list, stop, stop-all, remove.
  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.
  Typical: `clickhousectl local server start` (starts \"default\"), `clickhousectl local server start --name test`.
  Related: `clickhousectl local client` to connect to a running server.")]
    Server {
        #[command(subcommand)]
        command: ServerCommands,
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
  Accepts same specs as install/use: stable, lts, latest, 25.12, etc. Installs if needed.
  Ports default to 8123 (HTTP) and 9000 (TCP). If they're in use, free ports are auto-assigned.
  Use --http-port and --tcp-port to set explicit ports.
  Runs in background by default. Use --foreground (-F / --fg) to run in foreground.
  If --name is given and that server is already running, the command will error.
  Shows count of already-running servers before starting.
  Related: `clickhousectl local server list` to see servers, `clickhousectl local server stop <name>` to stop one.")]
    Start {
        /// Server name (default: \"default\", or random if default is already running)
        #[arg(long)]
        name: Option<String>,

        /// ClickHouse version to use (e.g. stable, lts, latest, 25.12). Installs if needed. Does not change the default version.
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

        /// Arguments to pass to clickhouse-server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all server instances (running and stopped)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows all named ClickHouse server instances and their status.
  Automatically cleans up stale entries for processes that are no longer running.
  Shows name, status (running/stopped), PID, version, and ports.
  Related: `clickhousectl local server start` to start a server, `clickhousectl local server stop <name>` to stop one.")]
    List,

    /// Stop a running server by name
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops a named ClickHouse server. Use the name from `clickhousectl local server list`.
  Sends SIGTERM first, then SIGKILL if the process doesn't exit gracefully.
  The server's data directory is preserved — restart with `clickhousectl local server start --name <name>`.
  Related: `clickhousectl local server list` to see servers.")]
    Stop {
        /// Name of the server to stop
        name: String,
    },

    /// Stop all running server instances
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops all running ClickHouse server instances.
  Sends SIGTERM first, then SIGKILL if processes don't exit.
  Data directories are preserved.
  Related: `clickhousectl local server list` to see servers.")]
    StopAll,

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
