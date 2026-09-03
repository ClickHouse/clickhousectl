use crate::version_manager::{self, VersionSpec};
use clap::{ArgGroup, Args, Subcommand};
use std::str::FromStr;

fn parse_server_name_arg(name: &str) -> Result<String, String> {
    crate::local::server::validate_server_name(name)
        .map(|()| name.to_string())
        .map_err(|error| error.to_string())
}

const INSTALL_AFTER_HELP: &str = "\
CONTEXT FOR AGENTS:
  `clickhousectl local use <version>` auto-installs a missing version and sets it as default.

EXAMPLES:
  clickhousectl local install latest
  clickhousectl local install 26.8
  clickhousectl local install 26.8.1.1760
  clickhousectl local install postgres@18

CLICKHOUSE DOWNLOAD:
  Binaries install at ~/.clickhouse/versions/<version>/clickhouse and are approximately 150 MB.
  Downloads use builds.clickhouse.com, with packages.clickhouse.com fallback on Linux and
  github.com on macOS. To bootstrap without setting a default, run
  `clickhousectl local server start`; it installs `latest` when needed.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallVersionArg {
    ClickHouse(VersionSpec),
    Postgres(String),
}

impl FromStr for InstallVersionArg {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if let Some(tag) = input
            .strip_prefix("postgres@")
            .or_else(|| input.strip_prefix("postgres:"))
        {
            return Ok(Self::Postgres(tag.to_string()));
        }

        version_manager::parse_version_spec(input)
            .map(Self::ClickHouse)
            .map_err(|error| error.to_string())
    }
}

/// Kept distinct from `ServerVersionArg` so each command owns its accepted inputs and errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseVersionArg(VersionSpec);

impl UseVersionArg {
    pub(crate) fn into_spec(self) -> VersionSpec {
        self.0
    }
}

impl FromStr for UseVersionArg {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.starts_with("postgres@") || input.starts_with("postgres:") {
            return Err(
                "Postgres image selectors are only supported by `local install`; `local use` requires a ClickHouse version"
                    .to_string(),
            );
        }

        version_manager::parse_version_spec(input)
            .map(Self)
            .map_err(|error| error.to_string())
    }
}

/// Kept distinct from `UseVersionArg` so each command owns its accepted inputs and errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerVersionArg(VersionSpec);

impl ServerVersionArg {
    pub(crate) fn into_spec(self) -> VersionSpec {
        self.0
    }
}

impl FromStr for ServerVersionArg {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.starts_with("postgres@") || input.starts_with("postgres:") {
            return Err(
                "Postgres image selectors are only supported by `local install`; `local server start --version` requires a ClickHouse version"
                    .to_string(),
            );
        }

        version_manager::parse_version_spec(input)
            .map(Self)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVersionArg(VersionSpec);

impl ClientVersionArg {
    pub(crate) fn into_spec(self) -> VersionSpec {
        self.0
    }
}

impl FromStr for ClientVersionArg {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.starts_with("postgres@") || input.starts_with("postgres:") {
            return Err(
                "Postgres image selectors are only supported by `local install`; `local client --version` requires an installed ClickHouse version"
                    .to_string(),
            );
        }

        let spec = version_manager::parse_version_spec(input).map_err(|error| error.to_string())?;
        if matches!(spec, VersionSpec::Latest | VersionSpec::Channel(_)) {
            return Err(
                "`local client --version` selects an installed numeric version (for example, 25.12 or 25.12.9.61); floating selectors latest, stable, and lts are not supported"
                    .to_string(),
            );
        }
        Ok(Self(spec))
    }
}

#[derive(Args)]
pub struct LocalArgs {
    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: LocalCommands,
}

impl LocalArgs {
    pub(crate) fn postgres_start_validation_error(&self) -> Option<String> {
        let LocalCommands::Postgres {
            command: PostgresCommands::Start { password, env, .. },
        } = &self.command
        else {
            return None;
        };
        crate::local::postgres::validate_pg_start_env_args(password.as_deref(), env).err()
    }
}

#[derive(Subcommand)]
pub enum LocalCommands {
    /// Install a ClickHouse version
    #[command(after_help = INSTALL_AFTER_HELP)]
    Install {
        /// Version to install. Accepts: "latest" (recommended), "stable", "lts", partial like "25.12", exact like "25.12.9.61", or a Postgres image selector like "postgres@18".
        version: InstallVersionArg,

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
  This makes standard subcommands such as `clickhouse client`, `clickhouse benchmark`, and `clickhouse format` available directly.
  Related: `clickhousectl local which` to verify, `clickhousectl local server start` to start a server.")]
    Use {
        /// Version to use as default. Accepts: "latest" (recommended), "stable", "lts", partial like "25.12", or exact like "25.12.5.44".
        version: UseVersionArg,

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
  Fails if a local server is currently running on this version, in ANY project (versions are
  shared, so a server started from another directory is checked too — see
  `clickhousectl local server list --global`). The error names each blocking server with its
  project root and PID; stop them first, or pass --force to stop them before removing.
  Fails if the version is the current default (removing it clears ~/.clickhouse/default and
  the global ~/.local/bin/clickhouse symlink, and the exact build may not be
  re-downloadable). Switch the default with `clickhousectl local use <other-version>` first,
  or pass --force to remove it and clear both.
  Related: `clickhousectl local list` to see installed versions.")]
    Remove {
        /// Version to remove
        // Keep this opaque: removal matches an installed directory name instead of resolving a version spec.
        version: String,

        /// Stop any running servers using this version, in any project, and remove it even when it is the default (clearing ~/.clickhouse/default and the global symlink)
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
    #[command(
        group(ArgGroup::new("direct").args(["host", "port"]).multiple(true)),
        after_help = "\
CONTEXT FOR AGENTS:
  Two connection modes:
  1. Named server: `clickhousectl local client --name dev` — looks up port and version from a
     locally managed server started via `clickhousectl local server start`. Lookup uses the exact
     current project directory and does not search parents. Defaults to \"default\".
  2. Explicit host/port: `clickhousectl local client --host myhost --port 9000` — connects to any
     ClickHouse server directly, bypassing local server lookup. Host-only uses port 9000; port-only
     connects to localhost. Direct selectors cannot be combined with --name.
  Repeat --query to execute multiple inline queries. --queries-file accepts multiple paths after
  one flag and can also be repeated. ClickHouse rejects combining the two options, so clickhousectl
  reports that combination as a usage error.
  Additional clickhouse-client args can be passed after --.
  Related: `clickhousectl local server start` to start a local server, `clickhousectl local server list` to see servers."
    )]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short, conflicts_with_all = ["host", "port"])]
        name: Option<String>,

        /// Host to connect to (bypasses local server lookup)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to (bypasses local server lookup if set)
        #[arg(
            long,
            short,
            value_parser = clap::value_parser!(u16).range(1..=65535)
        )]
        port: Option<u16>,

        /// Installed local client version for direct host/port mode (e.g. 25, 25.12, or 25.12.9.61). Does not change the default.
        #[arg(long, short = 'v', requires = "direct", conflicts_with = "name")]
        version: Option<ClientVersionArg>,

        /// Execute a SQL query; repeat for multiple queries (requires ClickHouse 23.9.1.1854+)
        #[arg(long, short, conflicts_with = "queries_file")]
        query: Vec<String>,

        /// Execute queries from SQL files; accepts multiple paths or repeated flags
        #[arg(long, num_args = 1.., conflicts_with = "query")]
        queries_file: Vec<String>,

        /// Additional arguments to pass to clickhouse-client
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage local server instances
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage named local server instances. Project-scoped `server list` and `server stop-all`
  include both ClickHouse processes and Docker-backed Postgres containers; other commands
  here manage ClickHouse.
  Project-local commands use `.clickhouse` under the exact current working directory; parent
  `.clickhouse` directories are not searched. Change to the project root before running them.
  Each server has its own data directory.
  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.

  Canonical named lifecycle:
    clickhousectl local server start dev
    clickhousectl local server list
    clickhousectl local server stop dev
    clickhousectl local server remove dev

  If `server start` generates a name, retain the returned name for later `stop` and `remove`
  commands. Use `clickhousectl local server stop-all` for project cleanup.
  `clickhousectl local remove <exact-version>` removes an installed ClickHouse binary;
  `clickhousectl local server remove <server-name>` removes a stopped server and its data.
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
  Without a name, the first server is called \"default\"; if \"default\" is already running,
  a random name is generated (e.g., \"bold-crane\").
  Retain a generated name for later `server stop <name>` and `server remove <name>` commands.
  Pass the name positionally to give a server a stable identity (e.g., `server start dev`).
  The older `--name dev` form remains accepted, but cannot be combined with a positional name.
  Use --version (-v) to run a specific ClickHouse version without changing the default.
  Accepts same specs as install/use: \"latest\" (recommended), stable, lts, 25.12, etc. Installs if needed.
  With no --version and no default set, a bare start bootstraps by installing \"latest\" (without
  setting it as the default, so you keep tracking latest on later starts).
  Ports default to 8123 (HTTP) and 9000 (TCP). If they're in use, free ports are auto-assigned.
  Use --http-port and --tcp-port to set explicit ports.
  Runs in background by default. Use --foreground (-F / --fg) to run in foreground.
  Background starts wait for HTTP health and TCP connections. Use --no-wait to return after spawning.
  If a name is given and that server is already running, the command will error.
  Shows count of already-running servers before starting.
  Use --config <NAME> to apply a custom ClickHouse config file from ~/.clickhouse/configs/
  (see `clickhousectl local server configs`). The file is merged as an overlay on top of
  ClickHouse's built-in defaults (via config.d), so it can contain just the settings you want
  to change (e.g. <query_log>). The data directory and ports stay managed regardless of the
  file's contents (they are forced as command-line overrides).
  Additional clickhouse-server arguments must follow `--`.
  Project-local commands use `.clickhouse` under the exact current working directory; parent
  `.clickhouse` directories are not searched. Change to the project root before running them.
  Related: `clickhousectl local server list` to see servers, `clickhousectl local server stop [name]` to stop one.")]
    Start {
        /// Server name (default: "default", or random if default is already running)
        #[arg(value_name = "NAME", conflicts_with = "name_flag")]
        name: Option<String>,

        /// Compatibility form for the server name; prefer positional NAME
        #[arg(long = "name", value_name = "NAME", conflicts_with = "name")]
        name_flag: Option<String>,

        /// ClickHouse version to use (e.g. "latest" (recommended), stable, lts, 25.12). Installs if needed. Does not change the default version.
        #[arg(long, short = 'v')]
        version: Option<ServerVersionArg>,

        /// HTTP port (default: 8123, auto-assigns a free port if in use)
        #[arg(long)]
        http_port: Option<u16>,

        /// TCP port (default: 9000, auto-assigns a free port if in use)
        #[arg(long)]
        tcp_port: Option<u16>,

        /// Run server in foreground (default: background)
        #[arg(long, alias = "fg", short = 'F')]
        foreground: bool,

        /// Return after spawning without waiting for HTTP and TCP readiness
        #[arg(long, conflicts_with = "foreground")]
        no_wait: bool,

        /// Overlay a named config file from ~/.clickhouse/configs/ on top of the defaults (see `server configs`)
        #[arg(long = "config", alias = "config-file", value_name = "NAME")]
        config_file: Option<String>,

        /// Arguments to pass to clickhouse-server after `--`
        #[arg(last = true, allow_hyphen_values = true, value_name = "CLICKHOUSE_ARG")]
        args: Vec<String>,
    },

    /// List custom config files available to `server start --config`
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Lists ClickHouse config files in ~/.clickhouse/configs/ and prints that directory's path.
  Drop a config file there (e.g. analytics.xml) and start a server with it via
  `clickhousectl local server start --config analytics`. The file is overlaid on top of
  ClickHouse's built-in defaults (config.d merge), so it only needs the settings you want to
  change. Files may be .xml, .yaml, or .yml; reference them by name with or without the
  extension.
  Related: `clickhousectl local server start --config <NAME>`.")]
    Configs,

    /// List all server instances (running and stopped)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows all named ClickHouse server instances and their status.
  Project-local commands use `.clickhouse` under the exact current working directory; parent
  `.clickhouse` directories are not searched. Change to the project root before running them.
  Use --global to list running ClickHouse servers across projects.
  Processes that exited unexpectedly are retained and shown as stopped.
  Running ClickHouse entries also show their PID, version, and ports.
  Related: `clickhousectl local server start` to start a server, `clickhousectl local server stop [name]` to stop one.")]
    List {
        /// System-wide maintenance only: list servers across all projects. You almost certainly want the default project-scoped list instead.
        #[arg(long)]
        global: bool,
    },

    /// Stop a running server by name
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops a ClickHouse server. Without a name, stops \"default\" when it exists, otherwise the sole
  known ClickHouse server. With no ClickHouse servers this is a successful no-op; with multiple
  non-default servers, pass a positional name (e.g., `server stop dev`) or use `server stop-all`.
  Use `clickhousectl local server list` to find names.
  The server's data and metadata are preserved so it remains visible in `server list`.
  Restart with `clickhousectl local server start <name>`.
  Project-local commands use `.clickhouse` under the exact current working directory; parent
  `.clickhouse` directories are not searched. Change to the project root before running them.
  Related: `clickhousectl local server list` to see servers.")]
    Stop {
        /// Name of the server to stop (auto-selects default or a sole ClickHouse server when omitted)
        #[arg(value_name = "NAME", conflicts_with = "name_flag")]
        name: Option<String>,

        /// Compatibility form for the server name; prefer positional NAME
        #[arg(
            long = "name",
            value_name = "NAME",
            conflicts_with = "name",
            hide = true
        )]
        name_flag: Option<String>,

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
  Stops all running ClickHouse and Postgres server instances in this project.
  Project-local commands use `.clickhouse` under the exact current working directory; parent
  `.clickhouse` directories are not searched. Change to the project root before running them.
  ClickHouse processes receive SIGTERM first, then SIGKILL if they don't exit.
  Postgres containers are stopped but retained for a subsequent start.
  With --global, stops ClickHouse servers only; global Postgres discovery is not supported.
  Data and metadata are preserved, and stopped servers remain visible in `server list`.
  Related: `clickhousectl local server list` to see servers.")]
    StopAll {
        /// System-wide maintenance only: stop all ClickHouse servers across all projects. You almost certainly want the default project-scoped stop-all instead.
        #[arg(long)]
        global: bool,
    },

    /// Remove a stopped server and its data
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Permanently deletes a server's data directory. The server must be stopped first.
  This is irreversible: all data for this server instance will be lost.
  Without a name, removes \"default\" only when it exists. It never guesses a custom server;
  use `server list`, then pass a custom name positionally.
  Project-local commands use `.clickhouse` under the exact current working directory; parent
  `.clickhouse` directories are not searched. Change to the project root before running them.
  Unlike `local server remove <server-name>`, `local remove <exact-version>` removes an installed
  ClickHouse binary rather than server data.
  Related: `clickhousectl local server stop [name]` to stop first, `clickhousectl local server list` to see servers.")]
    Remove {
        /// Name of the server to remove (only an existing "default" is selected when omitted)
        #[arg(value_name = "NAME", conflicts_with = "name_flag")]
        name: Option<String>,

        /// Compatibility form for the server name; prefer positional NAME
        #[arg(
            long = "name",
            value_name = "NAME",
            conflicts_with = "name",
            hide = true
        )]
        name_flag: Option<String>,
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
  When --port is omitted, port 5432 is used if free or another free port is auto-selected.
  An explicitly requested port is rejected if it is occupied.
  If a fresh startup fails, its new container and attempt-created data are removed; existing data is preserved.
  A random POSTGRES_PASSWORD is generated unless --password or `-e POSTGRES_PASSWORD=...` is given.
  POSTGRES_USER, POSTGRES_DB, and PGDATA are reserved; use --user/--database for the first two.
  `-e POSTGRES_PASSWORD=...` remains a compatibility alternative to --password, but the two cannot
  be combined. Every --env key must be unique, so generated variables are never duplicated.
  Start waits for PostgreSQL to accept connections, up to --wait-timeout seconds (default: 60).
  Containers are labeled `clickhousectl.engine=postgres`, `clickhousectl.name=<name>`,
  `clickhousectl.major=<major>`, `clickhousectl.project=<cwd>`, and
  `created_by=clickhousectl_<version>` for safe discovery.
  Requires Docker to be installed and running.")]
    Start {
        /// Server name (default: "default", or random if default is already running)
        #[arg(long, value_parser = parse_server_name_arg)]
        name: Option<String>,

        /// Postgres image tag (17 or 18 — e.g. 17, 17-alpine, 18.1, 18-bookworm). Default: 18. Pulls if missing.
        #[arg(long, short = 'v', value_parser = crate::local::postgres::parse_pg_tag_arg)]
        version: Option<String>,

        /// Host TCP port (when omitted: uses 5432 if free, otherwise auto-selects; an occupied explicit port is rejected)
        #[arg(long, value_parser = crate::local::postgres::parse_pg_port_arg)]
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

        /// Extra unique env vars for the container; POSTGRES_PASSWORD is the only supported reserved key
        #[arg(
            short = 'e',
            long = "env",
            value_name = "KEY=VALUE",
            value_parser = crate::local::postgres::parse_pg_env_arg
        )]
        env: Vec<String>,

        /// Seconds to wait for PostgreSQL readiness (maximum: 600)
        #[arg(
            long,
            default_value_t = 60,
            value_parser = clap::value_parser!(u16).range(1..=600)
        )]
        wait_timeout: u16,
    },

    /// Stop a running Postgres container by name
    Stop {
        /// Name of the server to stop (default: "default")
        #[arg(default_value = "default")]
        name: String,
        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,
    },

    /// Stop all running Postgres containers in this project
    StopAll,

    /// Remove a stopped Postgres server and its data directory
    Remove {
        /// Name of the server to remove (default: "default")
        #[arg(default_value = "default")]
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
     Host-only uses port 5432; port-only connects to the local machine. Direct selectors cannot be
     combined with --name or --version.
  If `psql` is on PATH on the host, it is execed directly. Otherwise, falls back to running
  `psql` inside the container via Docker exec (no host psql required).
  --query and --queries-file pass through to psql (-c / -f).
  Additional psql args can be passed after --.")]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short, conflicts_with_all = ["host", "port"])]
        name: Option<String>,

        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v', conflicts_with_all = ["host", "port"])]
        version: Option<String>,

        /// Host to connect to (bypasses local server lookup)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to (bypasses local server lookup if set)
        #[arg(
            long,
            short,
            value_parser = clap::value_parser!(u16).range(1..=65535)
        )]
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
    use crate::version_manager::list::Channel;
    use clap::Parser;

    fn local_args(args: &[&str]) -> LocalArgs {
        let mut argv = vec!["clickhousectl", "local"];
        argv.extend_from_slice(args);
        let cli = Cli::try_parse_from(argv).unwrap();
        let Commands::Local(local) = cli.command else {
            panic!("expected local command");
        };
        local
    }

    fn local_command(args: &[&str]) -> LocalCommands {
        local_args(args).command
    }

    fn assert_version_rejected(args: &[&str], expected: &str) {
        let mut argv = vec!["clickhousectl", "local"];
        argv.extend_from_slice(args);
        let error = Cli::try_parse_from(argv)
            .err()
            .expect("invalid version should fail during clap parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        assert!(error.to_string().contains(expected), "{error}");
    }

    fn postgres_start_parse_error(args: &[&str]) -> clap::Error {
        let mut argv = vec!["clickhousectl", "local", "postgres", "start"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv)
            .err()
            .expect("invalid postgres start arguments should fail during clap parsing")
    }

    fn local_parse_error(args: &[&str]) -> clap::Error {
        let mut argv = vec!["clickhousectl", "local"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv)
            .err()
            .expect("invalid local arguments should fail during clap parsing")
    }

    fn rendered_help(args: &[&str]) -> String {
        let mut argv = vec!["clickhousectl", "local"];
        argv.extend_from_slice(args);
        let error = Cli::try_parse_from(argv)
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        error.to_string()
    }

    #[test]
    fn parses_supported_clickhouse_version_forms_for_each_command() {
        for (input, expected) in [
            ("latest", VersionSpec::Latest),
            ("stable", VersionSpec::Channel(Channel::Stable)),
            ("lts", VersionSpec::Channel(Channel::Lts)),
            ("25", VersionSpec::Major(25)),
            ("25.12", VersionSpec::Minor(25, 12)),
            ("25.12.9.61", VersionSpec::Exact("25.12.9.61".to_string())),
        ] {
            let LocalCommands::Install {
                version: InstallVersionArg::ClickHouse(version),
                ..
            } = local_command(&["install", input])
            else {
                panic!("expected ClickHouse install version for {input}");
            };
            assert_eq!(version, expected);

            let LocalCommands::Use { version, .. } = local_command(&["use", input]) else {
                panic!("expected use version for {input}");
            };
            assert_eq!(version.into_spec(), expected);

            let LocalCommands::Server {
                command: ServerCommands::Start { version, .. },
            } = local_command(&["server", "start", "--version", input])
            else {
                panic!("expected server version for {input}");
            };
            assert_eq!(
                version.expect("version should be present").into_spec(),
                expected
            );
        }
    }

    #[test]
    fn rejects_malformed_clickhouse_versions_for_each_command() {
        let expected = "all parts must be numeric";
        assert_version_rejected(&["install", "not.a.version"], expected);
        assert_version_rejected(&["use", "not.a.version"], expected);
        assert_version_rejected(&["server", "start", "--version", "not.a.version"], expected);
        assert_version_rejected(
            &["client", "--host", "remote", "--version", "not.a.version"],
            expected,
        );
    }

    #[test]
    fn rejects_three_part_clickhouse_versions_for_each_command() {
        let expected = "3-part version '25.12.9' is not supported";
        assert_version_rejected(&["install", "25.12.9"], expected);
        assert_version_rejected(&["use", "25.12.9"], expected);
        assert_version_rejected(&["server", "start", "--version", "25.12.9"], expected);
        assert_version_rejected(
            &["client", "--host", "remote", "--version", "25.12.9"],
            expected,
        );
    }

    #[test]
    fn rejects_unsupported_clickhouse_version_shapes_for_each_command() {
        let expected = "expected 1-2 or 4 parts";
        assert_version_rejected(&["install", "25.12.9.61.2"], expected);
        assert_version_rejected(&["use", "25.12.9.61.2"], expected);
        assert_version_rejected(&["server", "start", "--version", "25.12.9.61.2"], expected);
        assert_version_rejected(
            &["client", "--host", "remote", "--version", "25.12.9.61.2"],
            expected,
        );
    }

    #[test]
    fn postgres_image_selectors_are_install_only() {
        for (input, expected) in [
            ("postgres@18", "18"),
            ("postgres:17-alpine", "17-alpine"),
            ("  postgres@16  ", "16"),
        ] {
            let LocalCommands::Install {
                version: InstallVersionArg::Postgres(tag),
                ..
            } = local_command(&["install", input])
            else {
                panic!("expected Postgres install version for {input}");
            };
            assert_eq!(tag, expected);
        }

        assert_version_rejected(
            &["use", "  postgres@18  "],
            "only supported by `local install`; `local use` requires a ClickHouse version",
        );
        assert_version_rejected(
            &["server", "start", "--version", "  postgres@18  "],
            "only supported by `local install`; `local server start --version` requires a ClickHouse version",
        );
        assert_version_rejected(
            &["client", "--host", "remote", "--version", "postgres@18"],
            "only supported by `local install`; `local client --version` requires an installed ClickHouse version",
        );
    }

    #[test]
    fn clickhouse_client_parses_numeric_installed_version_selectors() {
        for input in ["25", "25.12", "25.12.9.61"] {
            for selectors in [
                vec!["--host", "remote", "--version", input],
                vec!["--version", input, "--port", "9000"],
            ] {
                let mut args = vec!["client"];
                args.extend(selectors);
                let LocalCommands::Client {
                    version: Some(version),
                    ..
                } = local_command(&args)
                else {
                    panic!("expected ClickHouse client version {input}");
                };
                assert_eq!(version.into_spec().to_string(), input);
            }
        }
    }

    #[test]
    fn clickhouse_client_rejects_floating_binary_versions() {
        for input in ["latest", "stable", "lts"] {
            assert_version_rejected(
                &["client", "--host", "remote", "--version", input],
                "selects an installed numeric version",
            );
        }
    }

    #[test]
    fn clickhouse_client_parses_every_valid_selector_combination_and_order() {
        type SelectorCase = (
            &'static [&'static str],
            Option<&'static str>,
            Option<&'static str>,
            Option<u16>,
        );
        let cases: &[SelectorCase] = &[
            (&[], None, None, None),
            (&["--name", "dev"], Some("dev"), None, None),
            (&["--host", "db.example"], None, Some("db.example"), None),
            (&["--port", "1"], None, None, Some(1)),
            (
                &["--host", "db.example", "--port", "65535"],
                None,
                Some("db.example"),
                Some(65535),
            ),
            (
                &["--port", "65535", "--host", "db.example"],
                None,
                Some("db.example"),
                Some(65535),
            ),
        ];

        for (selectors, expected_name, expected_host, expected_port) in cases {
            let args: Vec<&str> = ["client"]
                .into_iter()
                .chain(selectors.iter().copied())
                .collect();
            let LocalCommands::Client {
                name, host, port, ..
            } = local_command(&args)
            else {
                panic!("expected ClickHouse client for {selectors:?}");
            };
            assert_eq!(name.as_deref(), *expected_name, "selectors: {selectors:?}");
            assert_eq!(host.as_deref(), *expected_host, "selectors: {selectors:?}");
            assert_eq!(port, *expected_port, "selectors: {selectors:?}");
        }
    }

    #[test]
    fn clickhouse_client_rejects_named_and_direct_selectors_in_every_order() {
        let conflicting: &[&[&str]] = &[
            &["--name", "dev", "--host", "db.example"],
            &["--host", "db.example", "--name", "dev"],
            &["--name", "dev", "--port", "9000"],
            &["--port", "9000", "--name", "dev"],
            &["--name", "dev", "--host", "db.example", "--port", "9000"],
            &["--name", "dev", "--port", "9000", "--host", "db.example"],
            &["--host", "db.example", "--name", "dev", "--port", "9000"],
            &["--host", "db.example", "--port", "9000", "--name", "dev"],
            &["--port", "9000", "--name", "dev", "--host", "db.example"],
            &["--port", "9000", "--host", "db.example", "--name", "dev"],
        ];

        for selectors in conflicting {
            let args: Vec<&str> = ["client"]
                .into_iter()
                .chain(selectors.iter().copied())
                .collect();
            let error = local_parse_error(&args);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "selectors: {selectors:?}"
            );
            assert!(error.to_string().contains("--name"), "{error}");
        }
    }

    #[test]
    fn clickhouse_client_version_requires_direct_mode_and_conflicts_with_named_mode() {
        let missing_direct = local_parse_error(&["client", "--version", "25.12.9.61"]);
        assert_eq!(
            missing_direct.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(
            missing_direct.to_string().contains("--host"),
            "{missing_direct}"
        );
        assert!(
            missing_direct.to_string().contains("--port"),
            "{missing_direct}"
        );

        for selectors in [
            ["--name", "dev", "--version", "25.12.9.61"],
            ["--version", "25.12.9.61", "--name", "dev"],
        ] {
            let args: Vec<&str> = ["client"].into_iter().chain(selectors).collect();
            let error = local_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
            assert!(error.to_string().contains("--version"), "{error}");
            assert!(error.to_string().contains("--name"), "{error}");
        }
    }

    #[test]
    fn clickhouse_client_rejects_zero_and_nonnumeric_ports() {
        for port in ["0", "not-a-port"] {
            let error = local_parse_error(&["client", "--port", port]);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert!(error.to_string().contains("--port"), "{error}");
        }
    }

    #[test]
    fn clickhouse_client_parses_minimal_and_repeated_query_inputs_in_order() {
        let LocalCommands::Client {
            query,
            queries_file,
            args,
            ..
        } = local_command(&["client"])
        else {
            panic!("expected ClickHouse client");
        };
        assert!(query.is_empty());
        assert!(queries_file.is_empty());
        assert!(args.is_empty());

        let LocalCommands::Client {
            query,
            queries_file,
            args,
            ..
        } = local_command(&[
            "client", "--query", "SELECT 1", "-q", "SELECT 2", "--query", "", "--", "--query",
            "SELECT 3", "--format", "CSV",
        ])
        else {
            panic!("expected ClickHouse client");
        };
        assert_eq!(query, ["SELECT 1", "SELECT 2", ""]);
        assert!(queries_file.is_empty());
        assert_eq!(args, ["--query", "SELECT 3", "--format", "CSV"]);
    }

    #[test]
    fn clickhouse_client_parses_repeated_query_files_and_empty_values_in_order() {
        let LocalCommands::Client {
            query,
            queries_file,
            args,
            ..
        } = local_command(&[
            "client",
            "--queries-file",
            "schema.sql",
            "seed.sql",
            "--queries-file",
            "",
            "verify.sql",
            "--",
            "--queries-file",
            "tail.sql",
        ])
        else {
            panic!("expected ClickHouse client");
        };
        assert!(query.is_empty());
        assert_eq!(queries_file, ["schema.sql", "seed.sql", "", "verify.sql"]);
        assert_eq!(args, ["--queries-file", "tail.sql"]);
    }

    #[test]
    fn clickhouse_client_rejects_combined_query_sources_in_every_order() {
        for inputs in [
            ["--query", "SELECT 1", "--queries-file", "queries.sql"],
            ["--queries-file", "queries.sql", "--query", "SELECT 1"],
        ] {
            let args: Vec<&str> = ["client"].into_iter().chain(inputs).collect();
            let error = local_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
            assert!(error.to_string().contains("--query <QUERY>"), "{error}");
            assert!(
                error.to_string().contains("--queries-file <QUERIES_FILE>"),
                "{error}"
            );
        }
    }

    #[test]
    fn clickhouse_client_preserves_passthrough_selector_like_arguments() {
        let LocalCommands::Client { name, args, .. } = local_command(&[
            "client",
            "--name",
            "dev",
            "--",
            "--host",
            "child-host",
            "--port",
            "0",
            "--version",
            "child-version",
        ]) else {
            panic!("expected ClickHouse client");
        };

        assert_eq!(name.as_deref(), Some("dev"));
        assert_eq!(
            args,
            [
                "--host",
                "child-host",
                "--port",
                "0",
                "--version",
                "child-version"
            ]
        );
    }

    #[test]
    fn clickhouse_client_help_describes_binary_version_selection() {
        let error = Cli::try_parse_from(["clickhousectl", "local", "client", "--help"])
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        for text in [
            "Installed local client version for direct host/port mode",
            "Does not change the default",
            "Lookup uses the exact",
            "current project directory and does not search parents",
        ] {
            assert!(help.contains(text), "missing {text:?} in:\n{help}");
        }
    }

    #[test]
    fn clickhouse_client_help_describes_query_multiplicity_and_exclusion() {
        let help = Cli::try_parse_from(["clickhousectl", "local", "client", "--help"])
            .err()
            .expect("help should exit through clap")
            .to_string();

        assert!(help.contains("repeat for multiple queries"), "{help}");
        assert!(
            help.contains("accepts multiple paths or repeated flags"),
            "{help}"
        );
        assert!(help.contains("ClickHouse rejects combining"), "{help}");
    }

    #[test]
    fn postgres_client_applies_named_and_direct_selector_validation() {
        let valid: &[&[&str]] = &[
            &[],
            &["--name", "dev"],
            &["--version", "18"],
            &["--name", "dev", "--version", "18"],
            &["--version", "18", "--name", "dev"],
            &["--host", "db.example"],
            &["--port", "1"],
            &["--host", "db.example", "--port", "65535"],
            &["--port", "65535", "--host", "db.example"],
        ];
        for selectors in valid {
            let args: Vec<&str> = ["postgres", "client"]
                .into_iter()
                .chain(selectors.iter().copied())
                .collect();
            let LocalCommands::Postgres {
                command: PostgresCommands::Client { .. },
            } = local_command(&args)
            else {
                panic!("expected Postgres client for {selectors:?}");
            };
        }

        let conflicting: &[&[&str]] = &[
            &["--name", "dev", "--host", "db.example"],
            &["--host", "db.example", "--name", "dev"],
            &["--name", "dev", "--port", "5432"],
            &["--port", "5432", "--name", "dev"],
            &["--version", "18", "--host", "db.example"],
            &["--host", "db.example", "--version", "18"],
            &["--version", "18", "--port", "5432"],
            &["--port", "5432", "--version", "18"],
            &[
                "--name",
                "dev",
                "--version",
                "18",
                "--host",
                "db.example",
                "--port",
                "5432",
            ],
            &[
                "--port",
                "5432",
                "--host",
                "db.example",
                "--version",
                "18",
                "--name",
                "dev",
            ],
        ];
        for selectors in conflicting {
            let args: Vec<&str> = ["postgres", "client"]
                .into_iter()
                .chain(selectors.iter().copied())
                .collect();
            assert_eq!(
                local_parse_error(&args).kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "selectors: {selectors:?}"
            );
        }

        for port in ["0", "not-a-port"] {
            let error = local_parse_error(&["postgres", "client", "--port", port]);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert!(error.to_string().contains("--port"), "{error}");
        }
    }

    #[test]
    fn postgres_client_preserves_passthrough_selector_like_arguments() {
        let LocalCommands::Postgres {
            command: PostgresCommands::Client { name, args, .. },
        } = local_command(&[
            "postgres",
            "client",
            "--name",
            "dev",
            "--",
            "--host",
            "child-host",
            "--port",
            "0",
        ])
        else {
            panic!("expected Postgres client");
        };

        assert_eq!(name.as_deref(), Some("dev"));
        assert_eq!(args, ["--host", "child-host", "--port", "0"]);
    }

    #[test]
    fn install_help_covers_install_requirements() {
        let error = Cli::try_parse_from(["clickhousectl", "local", "install", "--help"])
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        for required in [
            "clickhousectl local install latest",
            "~/.clickhouse/versions/<version>/clickhouse",
            "approximately 150 MB",
            "builds.clickhouse.com",
            "packages.clickhouse.com",
            "github.com",
            "clickhousectl local server start",
        ] {
            assert!(
                help.contains(required),
                "missing {required:?} from:\n{help}"
            );
        }
    }

    #[test]
    fn use_help_documents_standard_clickhouse_subcommands() {
        let error = Cli::try_parse_from(["clickhousectl", "local", "use", "--help"])
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        assert!(help.contains("`~/.local/bin/clickhouse`"), "{help}");
        assert!(help.contains("`clickhouse client`"), "{help}");
        assert!(help.contains("`clickhouse benchmark`"), "{help}");
        assert!(help.contains("`clickhouse format`"), "{help}");
    }

    #[test]
    fn remove_help_documents_the_default_version_guard() {
        let error = Cli::try_parse_from(["clickhousectl", "local", "remove", "--help"])
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        for required in [
            "is the current default",
            "~/.clickhouse/default",
            "~/.local/bin/clickhouse",
            "clickhousectl local use <other-version>",
            "--force",
        ] {
            assert!(
                help.contains(required),
                "missing {required:?} from:\n{help}"
            );
        }
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
    fn parses_server_start_config() {
        let LocalCommands::Server {
            command: ServerCommands::Start { config_file, .. },
        } = local_command(&["server", "start", "--config", "analytics"])
        else {
            panic!("expected server start");
        };
        assert_eq!(config_file.as_deref(), Some("analytics"));
    }

    #[test]
    fn parses_server_start_config_file_legacy_alias() {
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
            command:
                ServerCommands::Start {
                    config_file,
                    no_wait,
                    args,
                    ..
                },
        } = local_command(&["server", "start"])
        else {
            panic!("expected server start");
        };
        assert_eq!(config_file, None);
        assert!(!no_wait);
        assert!(args.is_empty());
    }

    #[test]
    fn server_start_help_renders_default_name_without_escaped_quotes() {
        let help = rendered_help(&["server", "start", "--help"]);

        assert!(help.lines().any(|line| {
            line == r#"  [NAME]               Server name (default: "default", or random if default is already running)"#
        }), "{help}");
        assert!(!help.contains(r#"\"default\""#), "{help}");
    }

    #[test]
    fn server_help_pins_scope_workflow_and_removal_distinction() {
        let help = rendered_help(&["server", "--help"]);
        let context = help
            .split_once("CONTEXT FOR AGENTS:\n")
            .expect("agent context")
            .1;

        assert_eq!(
            context,
            concat!(
                "  Manage named local server instances. Project-scoped `server list` and `server stop-all`\n",
                "  include both ClickHouse processes and Docker-backed Postgres containers; other commands\n",
                "  here manage ClickHouse.\n",
                "  Project-local commands use `.clickhouse` under the exact current working directory; parent\n",
                "  `.clickhouse` directories are not searched. Change to the project root before running them.\n",
                "  Each server has its own data directory.\n",
                "  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.\n",
                "\n",
                "  Canonical named lifecycle:\n",
                "    clickhousectl local server start dev\n",
                "    clickhousectl local server list\n",
                "    clickhousectl local server stop dev\n",
                "    clickhousectl local server remove dev\n",
                "\n",
                "  If `server start` generates a name, retain the returned name for later `stop` and `remove`\n",
                "  commands. Use `clickhousectl local server stop-all` for project cleanup.\n",
                "  `clickhousectl local remove <exact-version>` removes an installed ClickHouse binary;\n",
                "  `clickhousectl local server remove <server-name>` removes a stopped server and its data.\n",
                "  Related: `clickhousectl local client` to connect to a running server.\n",
            )
        );
    }

    #[test]
    fn server_stop_help_pins_omitted_name_selection() {
        let help = rendered_help(&["server", "stop", "--help"]);

        assert!(
            help.contains(concat!(
                "  Stops a ClickHouse server. Without a name, stops \"default\" when it exists, otherwise the sole\n",
                "  known ClickHouse server. With no ClickHouse servers this is a successful no-op; with multiple\n",
                "  non-default servers, pass a positional name (e.g., `server stop dev`) or use `server stop-all`."
            )),
            "{help}"
        );
    }

    #[test]
    fn server_remove_help_pins_conservative_default_selection() {
        let help = rendered_help(&["server", "remove", "--help"]);

        assert!(
            help.contains(concat!(
                "  Without a name, removes \"default\" only when it exists. It never guesses a custom server;\n",
                "  use `server list`, then pass a custom name positionally."
            )),
            "{help}"
        );
        assert!(
            help.contains(concat!(
                "  Unlike `local server remove <server-name>`, `local remove <exact-version>` removes an installed\n",
                "  ClickHouse binary rather than server data."
            )),
            "{help}"
        );
    }

    #[test]
    fn readme_contrasts_version_and_server_removal() {
        let readme = include_str!("../../../../README.md");

        assert!(readme.contains(concat!(
            "| `clickhousectl local remove <exact-version>` | An installed ClickHouse binary from the global version store. |\n",
            "| `clickhousectl local server remove <server-name>` | A stopped named server and its data from the exact current project. |"
        )));
    }

    #[test]
    fn parses_server_start_positional_name_before_clickhousectl_options() {
        let LocalCommands::Server {
            command:
                ServerCommands::Start {
                    name,
                    name_flag,
                    version,
                    args,
                    ..
                },
        } = local_command(&["server", "start", "existing", "--version", "25.12.9.61"])
        else {
            panic!("expected server start");
        };
        assert_eq!(name.as_deref(), Some("existing"));
        assert_eq!(name_flag, None);
        assert_eq!(
            version
                .map(ServerVersionArg::into_spec)
                .map(|v| v.to_string()),
            Some("25.12.9.61".to_string())
        );
        assert!(args.is_empty());
    }

    #[test]
    fn parses_server_start_name_flag_for_compatibility() {
        let LocalCommands::Server {
            command: ServerCommands::Start {
                name, name_flag, ..
            },
        } = local_command(&["server", "start", "--name", "existing"])
        else {
            panic!("expected server start");
        };
        assert_eq!(name, None);
        assert_eq!(name_flag.as_deref(), Some("existing"));
    }

    #[test]
    fn server_start_name_forms_conflict() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "local",
            "server",
            "start",
            "existing",
            "--name",
            "other",
        ])
        .err()
        .expect("name forms should conflict");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn server_start_passthrough_requires_boundary() {
        let LocalCommands::Server {
            command:
                ServerCommands::Start {
                    name,
                    version,
                    args,
                    ..
                },
        } = local_command(&[
            "server",
            "start",
            "existing",
            "--version",
            "25.12.9.61",
            "--",
            "--logger.level=trace",
            "--max_server_memory_usage=1000000",
        ])
        else {
            panic!("expected server start");
        };
        assert_eq!(name.as_deref(), Some("existing"));
        assert_eq!(
            version
                .map(ServerVersionArg::into_spec)
                .map(|v| v.to_string()),
            Some("25.12.9.61".to_string())
        );
        assert_eq!(
            args,
            ["--logger.level=trace", "--max_server_memory_usage=1000000"]
        );

        let error = Cli::try_parse_from([
            "clickhousectl",
            "local",
            "server",
            "start",
            "existing",
            "--logger.level=trace",
        ])
        .err()
        .expect("passthrough without -- should fail");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn parses_server_start_no_wait() {
        let LocalCommands::Server {
            command: ServerCommands::Start { no_wait, .. },
        } = local_command(&["server", "start", "--no-wait"])
        else {
            panic!("expected server start");
        };
        assert!(no_wait);
    }

    #[test]
    fn server_start_no_wait_conflicts_with_foreground() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "local",
            "server",
            "start",
            "--no-wait",
            "--foreground",
        ])
        .err()
        .expect("flags should conflict");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
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

    #[test]
    fn server_stop_omitted_name_remains_distinguishable() {
        let LocalCommands::Server {
            command: ServerCommands::Stop {
                name, name_flag, ..
            },
        } = local_command(&["server", "stop"])
        else {
            panic!("expected server stop");
        };
        assert_eq!(name, None);
        assert_eq!(name_flag, None);
    }

    #[test]
    fn server_remove_omitted_name_remains_distinguishable() {
        let LocalCommands::Server {
            command: ServerCommands::Remove { name, name_flag },
        } = local_command(&["server", "remove"])
        else {
            panic!("expected server remove");
        };
        assert_eq!(name, None);
        assert_eq!(name_flag, None);
    }

    #[test]
    fn server_stop_name_forms_allow_trailing_options() {
        for (command, expected_name, expected_name_flag) in [
            (
                &[
                    "server",
                    "stop",
                    "analytics",
                    "--global",
                    "--project",
                    "/tmp/project",
                    "--json",
                ][..],
                Some("analytics"),
                None,
            ),
            (
                &[
                    "server",
                    "stop",
                    "--name",
                    "analytics",
                    "--global",
                    "--project",
                    "/tmp/project",
                    "--json",
                ][..],
                None,
                Some("analytics"),
            ),
        ] {
            let args = local_args(command);
            assert!(args.json);
            let LocalCommands::Server {
                command:
                    ServerCommands::Stop {
                        name,
                        name_flag,
                        global,
                        project,
                    },
            } = args.command
            else {
                panic!("expected server stop");
            };
            assert_eq!(name.as_deref(), expected_name);
            assert_eq!(name_flag.as_deref(), expected_name_flag);
            assert!(global);
            assert_eq!(project.as_deref(), Some("/tmp/project"));
        }
    }

    #[test]
    fn server_remove_name_forms_allow_trailing_options() {
        for (command, expected_name, expected_name_flag) in [
            (
                &["server", "remove", "analytics", "--json"][..],
                Some("analytics"),
                None,
            ),
            (
                &["server", "remove", "--name", "analytics", "--json"][..],
                None,
                Some("analytics"),
            ),
        ] {
            let args = local_args(command);
            assert!(args.json);
            let LocalCommands::Server {
                command: ServerCommands::Remove { name, name_flag },
            } = args.command
            else {
                panic!("expected server remove");
            };
            assert_eq!(name.as_deref(), expected_name);
            assert_eq!(name_flag.as_deref(), expected_name_flag);
        }
    }

    #[test]
    fn server_teardown_name_forms_conflict() {
        for command in ["stop", "remove"] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "local",
                "server",
                command,
                "positional",
                "--name",
                "flagged",
            ])
            .err()
            .expect("name forms should conflict");
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
            assert!(error.to_string().contains("cannot be used with"), "{error}");
        }
    }

    #[test]
    fn server_teardown_help_hides_compatibility_name_flags() {
        for command in ["stop", "remove"] {
            let help = Cli::try_parse_from(["clickhousectl", "local", "server", command, "--help"])
                .err()
                .expect("help should exit through clap")
                .to_string();

            assert!(!help.contains("--name"), "{help}");
            assert!(help.contains("Without a name"), "{help}");
        }
    }

    #[test]
    fn server_stop_all_help_describes_engine_scope() {
        let help = Cli::try_parse_from(["clickhousectl", "local", "server", "stop-all", "--help"])
            .err()
            .expect("help should exit through clap")
            .to_string();

        assert!(help.contains("Stops all running ClickHouse and Postgres server instances"));
        assert!(help.contains("global Postgres discovery is not supported"));
    }

    #[test]
    fn postgres_stop_name_defaults_to_default() {
        let LocalCommands::Postgres {
            command: PostgresCommands::Stop { name, .. },
        } = local_command(&["postgres", "stop"])
        else {
            panic!("expected postgres stop");
        };
        assert_eq!(name, "default");
    }

    #[test]
    fn parses_postgres_start_validation_owned_options() {
        let LocalCommands::Postgres {
            command:
                PostgresCommands::Start {
                    name,
                    version,
                    port,
                    password,
                    env,
                    wait_timeout,
                    ..
                },
        } = local_command(&[
            "postgres",
            "start",
            "--name",
            "analytics",
            "--version",
            "18.1-alpine3.20",
            "--port",
            "55432",
            "--password",
            "secret",
            "-e",
            "APP_MODE=test",
            "--env",
            "DATABASE_URL=postgres://localhost/db?option=a=b",
            "--wait-timeout",
            "75",
        ])
        else {
            panic!("expected postgres start");
        };

        assert_eq!(name.as_deref(), Some("analytics"));
        assert_eq!(version.as_deref(), Some("18.1-alpine3.20"));
        assert_eq!(port, Some(55432));
        assert_eq!(password.as_deref(), Some("secret"));
        assert_eq!(wait_timeout, 75);
        assert_eq!(
            env,
            [
                "APP_MODE=test",
                "DATABASE_URL=postgres://localhost/db?option=a=b"
            ]
        );
    }

    #[test]
    fn postgres_start_readiness_timeout_defaults_and_is_bounded() {
        let LocalCommands::Postgres {
            command: PostgresCommands::Start { wait_timeout, .. },
        } = local_command(&["postgres", "start"])
        else {
            panic!("expected postgres start");
        };
        assert_eq!(wait_timeout, 60);

        for timeout in ["0", "601"] {
            let error = postgres_start_parse_error(&["--wait-timeout", timeout]);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn postgres_start_rejects_invalid_name_tag_port_and_env_at_clap_time() {
        for (args, expected) in [
            (vec!["--name", "../unsafe"], "Invalid server name"),
            (
                vec!["--version", "18garbage"],
                "invalid or unsupported postgres version",
            ),
            (
                vec!["--version", "18..1"],
                "invalid or unsupported postgres version",
            ),
            (
                vec!["--port", "0"],
                "--port 0 is not allowed; pick a specific port or omit the flag",
            ),
            (vec!["--env", "NO_EQUALS"], "expected KEY=VALUE"),
            (vec!["--env", "1KEY=value"], "do not start with a digit"),
            (
                vec!["--env", "POSTGRES_USER=admin"],
                "use --user instead of --env",
            ),
            (
                vec!["--env", "POSTGRES_DB=app"],
                "use --database instead of --env",
            ),
            (
                vec!["--env", "PGDATA=/tmp/postgres"],
                "PGDATA is managed by clickhousectl",
            ),
        ] {
            let error = postgres_start_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn postgres_remove_name_defaults_to_default() {
        let LocalCommands::Postgres {
            command: PostgresCommands::Remove { name, .. },
        } = local_command(&["postgres", "remove"])
        else {
            panic!("expected postgres remove");
        };
        assert_eq!(name, "default");
    }

    #[test]
    fn teardown_commands_preserve_explicit_names() {
        let LocalCommands::Server {
            command: ServerCommands::Stop {
                name, name_flag, ..
            },
        } = local_command(&["server", "stop", "analytics"])
        else {
            panic!("expected server stop");
        };
        assert_eq!(name.as_deref(), Some("analytics"));
        assert_eq!(name_flag, None);

        let LocalCommands::Server {
            command: ServerCommands::Remove { name, name_flag },
        } = local_command(&["server", "remove", "analytics"])
        else {
            panic!("expected server remove");
        };
        assert_eq!(name.as_deref(), Some("analytics"));
        assert_eq!(name_flag, None);

        let LocalCommands::Postgres {
            command: PostgresCommands::Stop { name, .. },
        } = local_command(&["postgres", "stop", "warehouse"])
        else {
            panic!("expected postgres stop");
        };
        assert_eq!(name, "warehouse");

        let LocalCommands::Postgres {
            command: PostgresCommands::Remove { name, .. },
        } = local_command(&["postgres", "remove", "warehouse"])
        else {
            panic!("expected postgres remove");
        };
        assert_eq!(name, "warehouse");
    }
}
