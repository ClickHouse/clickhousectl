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
  The first ClickHouse version installed becomes the default; later installs do not change it.
  `clickhousectl local use <version>` auto-installs a missing version and sets it as default.
  `postgres@<tag>` pulls a Docker image instead (needs Docker running) and never sets a default.";

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
    /// Install a ClickHouse version or Postgres image
    #[command(after_help = INSTALL_AFTER_HELP)]
    Install {
        /// Version ("latest", "stable", "lts", 25.12, 25.12.9.61) or image selector (postgres@18)
        version: InstallVersionArg,

        /// Re-install even if already installed
        #[arg(long)]
        force: bool,
    },

    /// List installed or available ClickHouse versions
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Default output is exact installed version strings — the form `clickhousectl local remove` needs.
  --remote lists minor versions (e.g. 26.3) probed from builds.clickhouse.com, not exact builds.")]
    List {
        /// List minor versions available for download
        #[arg(long)]
        remote: bool,
    },

    /// Set the default ClickHouse version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Auto-installs the version if it is not already present.
  Symlinks `~/.local/bin/clickhouse` to it, putting `clickhouse client`, `clickhouse benchmark`
  and `clickhouse format` on PATH; --no-global skips that.
  Sets the version used by `clickhousectl local client` and `clickhousectl local server`.")]
    Use {
        /// Version to set as default ("latest", "stable", "lts", 25.12, or 25.12.5.44)
        version: UseVersionArg,

        /// Do not create or update the ~/.local/bin/clickhouse symlink
        #[arg(long)]
        no_global: bool,
    },

    /// Remove an installed ClickHouse version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Fails if any server is running on this version, in any project — versions are shared, so a server
  started from another directory blocks removal too (`local server list --global` lists them).
  Fails if the version is the current default; switch with `clickhousectl local use <other-version>` first.
  --force overrides both guards. The exact build may not be re-downloadable.")]
    Remove {
        /// Exact installed version to remove (not "latest"/"stable"/"lts")
        // Keep this opaque: removal matches an installed directory name instead of resolving a version spec.
        version: String,

        /// Stop any server using this version and remove it even if it is the default
        ///
        /// Servers in other projects are stopped too; clears ~/.clickhouse/default and
        /// the ~/.local/bin/clickhouse symlink when the default is removed.
        #[arg(long)]
        force: bool,
    },

    /// Show the current default ClickHouse version
    Which,

    /// Initialize a project directory for ClickHouse and Postgres
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  `.clickhouse/` holds runtime data and is git-ignored; the `clickhouse/` and `postgres/` SQL
  scaffolds are meant to be committed.
  Idempotent — re-running only creates what is missing.
  Next: `clickhousectl local server start`")]
    Init,

    /// Connect to a running ClickHouse server with clickhouse-client
    #[command(
        group(ArgGroup::new("direct").args(["host", "port"]).multiple(true)),
        after_help = "\
CONTEXT FOR AGENTS:
  Default mode looks up a server started by `clickhousectl local server start`; the name defaults
  to \"default\".
  Extra clickhouse-client arguments go after `--`.
  Next: `clickhousectl local server list` to see running servers."
    )]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short, conflicts_with_all = ["host", "port"])]
        name: Option<String>,

        /// Host to connect to directly, bypassing local server lookup (port 9000)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to directly, bypassing local server lookup (host localhost)
        #[arg(
            long,
            short,
            value_parser = clap::value_parser!(u16).range(1..=65535)
        )]
        port: Option<u16>,

        /// Installed local client version for direct host/port mode
        ///
        /// Requires --host or --port and conflicts with --name. Numeric versions only
        /// (25, 25.12, 25.12.9.61). Does not change the default.
        #[arg(long, short = 'v', requires = "direct", conflicts_with = "name")]
        version: Option<ClientVersionArg>,

        /// Execute a SQL query; repeatable (repeats need ClickHouse 23.9.1.1854+)
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
  `list` and `stop-all` cover ClickHouse and Docker-backed Postgres; other subcommands are
  ClickHouse-only.
  Data persists across stop/start; only `remove` deletes it.
  Retain the name `start` returns (it may be generated) for later `stop`/`remove`.
  `local remove <version>` deletes an installed binary, not server data.
  Typical flow: `server start dev` -> `local client --name dev` -> `server stop dev`")]
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },

    /// Manage local Postgres instances (Docker-backed)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Requires Docker installed and running.
  Each instance is keyed on (name, major version); pass --version when one name has two majors.
  There is no `postgres list` — `local server list` shows ClickHouse and Postgres together.
  Typical flow: `postgres start` -> `postgres client` -> `postgres dotenv --local` -> `postgres stop`")]
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
  Starting a name that is already running is an error; a bare start with \"default\" already
  running picks a new generated name instead.
  With no --version and no default set, start installs \"latest\" first (~150 MB) without making
  it the default.")]
    Start {
        /// Server name (default: "default", or random if default is already running)
        #[arg(value_name = "NAME", conflicts_with = "name_flag")]
        name: Option<String>,

        /// Compatibility form for the server name; prefer positional NAME
        #[arg(long = "name", value_name = "NAME", conflicts_with = "name")]
        name_flag: Option<String>,

        /// Version or channel to run: latest, stable, lts, 25.12 (installs if needed)
        ///
        /// Does not change the default version set by `local use`.
        #[arg(long, short = 'v')]
        version: Option<ServerVersionArg>,

        /// HTTP port; when omitted, 8123 if free else an auto-selected free port
        ///
        /// An explicitly requested port that is already in use is rejected.
        #[arg(long)]
        http_port: Option<u16>,

        /// TCP port; when omitted, 9000 if free else an auto-selected free port
        ///
        /// An explicitly requested port that is already in use is rejected.
        #[arg(long)]
        tcp_port: Option<u16>,

        /// Run in foreground instead of background (alias: --fg)
        #[arg(long, alias = "fg", short = 'F')]
        foreground: bool,

        /// Return after spawning without waiting for HTTP and TCP readiness
        ///
        /// Otherwise a background start waits up to 30s for HTTP and TCP. Not with --foreground.
        #[arg(long, conflicts_with = "foreground")]
        no_wait: bool,

        /// Named config file from ~/.clickhouse/configs/ (see `server configs`)
        #[arg(long = "config", alias = "config-file", value_name = "NAME")]
        config_file: Option<String>,

        /// Arguments passed to clickhouse-server after `--`
        ///
        /// --config, --config-file and -C are rejected here; use `--config <NAME>` instead.
        #[arg(last = true, allow_hyphen_values = true, value_name = "CLICKHOUSE_ARG")]
        args: Vec<String>,
    },

    /// List custom config files available to `server start --config`
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Lists ~/.clickhouse/configs/ (.xml, .yaml, .yml) and prints that path.
  Drop a file there, then `clickhousectl local server start --config <name>`; the extension is
  optional in <name>, but an ambiguous stem is an error.
  Merged as a config.d overlay on ClickHouse's defaults, so it needs only the settings you change.")]
    Configs,

    /// List all server instances (running and stopped)
    List {
        /// List ClickHouse servers in all projects; the default is project-scoped
        #[arg(long)]
        global: bool,
    },

    /// Stop a running server
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Omitting NAME with no ClickHouse servers succeeds as a no-op; with several non-default servers
  it errors — pass a name or use `stop-all`.")]
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

        /// Stop a ClickHouse server in any project; the default is project-scoped
        #[arg(long)]
        global: bool,

        /// Project directory to disambiguate when using --global
        #[arg(long, requires = "global")]
        project: Option<String>,
    },

    /// Stop all ClickHouse and Postgres servers in this project
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  ClickHouse processes get SIGTERM, then SIGKILL if they do not exit in time.")]
    StopAll {
        /// Stop ClickHouse servers in all projects; the default is project-scoped
        #[arg(long)]
        global: bool,
    },

    /// Remove a stopped server and its data
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Irreversible: deletes the server's data directory. Stop it first — removing a running server
  errors.
  Omitting NAME removes only an existing \"default\"; it never guesses a custom name, even when
  exactly one exists.")]
    Remove {
        /// Name of the server to remove (defaults to "default" if it exists)
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
  Requires a running server; reads its actual ports.
  Writes CLICKHOUSE_HOST, CLICKHOUSE_PORT and CLICKHOUSE_HTTP_PORT, plus CLICKHOUSE_USER,
  CLICKHOUSE_PASSWORD and CLICKHOUSE_DATABASE only when their flags are given.
  An existing file is edited in place: only the keys written here are replaced, other
  CLICKHOUSE_* lines are kept.")]
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
    /// Start a Postgres instance
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  An existing stopped instance for the same (name, major) is resumed with its stored settings, so
  --port/--user/--password/--database/-e are ignored on a resume.
  Without --version, an existing instance selects the major; two majors under one name error.
  The generated password is printed once by start — re-read connection details later with
  `postgres dotenv` or `postgres client`.
  A failed fresh start rolls back the container and data it created; pre-existing data is kept.")]
    Start {
        /// Server name (default: "default", or random if default is already running)
        #[arg(long, value_parser = parse_server_name_arg)]
        name: Option<String>,

        /// Postgres image tag, major 17 or 18 (e.g. 17-alpine, 18.1). Default: 18
        ///
        /// Pulls the image if it is not present locally.
        #[arg(long, short = 'v', value_parser = crate::local::postgres::parse_pg_tag_arg)]
        version: Option<String>,

        /// Host TCP port; when omitted, 5432 if free else an auto-selected free port
        ///
        /// An explicitly requested port that is already in use is rejected.
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

        /// Extra container env vars; repeatable, each key at most once
        ///
        /// POSTGRES_USER, POSTGRES_DB and PGDATA are managed and rejected here — use
        /// --user/--database. POSTGRES_PASSWORD is accepted, but not together with --password.
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

    /// Stop a running Postgres instance
    Stop {
        /// Name of the instance to stop
        #[arg(default_value = "default")]
        name: String,
        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,
    },

    /// Stop all Postgres instances in this project
    StopAll,

    /// Remove a stopped Postgres instance and its data
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Irreversible: removes the container and deletes its data directory. Stop the instance first —
  removing a running one errors.")]
    Remove {
        /// Name of the instance to remove
        #[arg(default_value = "default")]
        name: String,
        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v')]
        version: Option<String>,
    },

    /// Connect to a running Postgres instance with psql
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Managed mode (the default; --name selects one) execs host `psql` when it is on PATH, else runs
  psql inside the container via `docker exec`.
  Direct mode (--host/--port) requires `psql` on PATH and connects as user/database \"postgres\"
  with no password; it does not read managed credentials.
  Extra psql arguments go after `--`.")]
    Client {
        /// Managed instance to connect to (default: "default")
        #[arg(long, short, conflicts_with_all = ["host", "port"])]
        name: Option<String>,

        /// Postgres version to disambiguate when multiple share a name
        #[arg(long, short = 'v', conflicts_with_all = ["host", "port"])]
        version: Option<String>,

        /// Host to connect to directly, bypassing managed lookup (port 5432)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to directly, bypassing managed lookup (host 127.0.0.1)
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
  Writes POSTGRES_HOST, POSTGRES_PORT, POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DATABASE.
  The instance must be running.
  Managed POSTGRES_* keys are replaced in place; other lines in the file are preserved.
  Contains the password in plaintext — prefer --local and keep it out of version control.")]
    Dotenv {
        /// Instance name (default: "default")
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
    fn clickhouse_client_query_sources_are_mutually_exclusive() {
        // The mutual exclusion is enforced by clap itself, not documented in help text.
        assert_eq!(
            local_parse_error(&["client", "--query", "SELECT 1", "--queries-file", "q.sql"]).kind(),
            clap::error::ErrorKind::ArgumentConflict,
        );
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

        // clap renders the doc comment's quotes literally, never as escaped `\"` sequences.
        assert!(help.contains(r#"(default: "default""#), "{help}");
        assert!(!help.contains(r#"\"default\""#), "{help}");
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
        }
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
