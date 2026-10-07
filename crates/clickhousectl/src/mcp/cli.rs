use clap::{ArgGroup, Args, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct McpArgs {
    /// Output as JSON
    #[arg(long, global = true, display_order = crate::cli::help_order::JSON)]
    pub json: bool,

    #[command(subcommand)]
    pub command: McpCommands,
}

#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// Register the ClickHouse MCP server in a coding agent
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Writes or merges `.mcp.json` in the current directory; other servers are kept.
  --local and --client-config entries launch `clickhousectl mcp run`, so no credentials are written.
  --cloud registers the hosted read-only Cloud MCP server, which signs in with OAuth.")]
    Add(AddArgs),

    /// Launch the ClickHouse MCP server over stdio
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Started by the agent from `.mcp.json`; stdout is the MCP channel and diagnostics go to stderr.
  Requires `uv` on PATH. --client-config needs a ClickHouse binary: if none is installed it runs
  `local use latest` once, and later runs use the `local use` default.
  --local requires the named server to be running (`local server start`).")]
    Run(RunArgs),
}

/// The connection an MCP server entry targets. Each parent declares which of
/// these, or which alternative, is required.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ConnectionArgs {
    /// Use a running local server from this project
    #[arg(
        long,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "default"
    )]
    pub local: Option<String>,

    /// Use a clickhouse-client config; omit PATH to search standard locations
    #[arg(long, value_name = "PATH")]
    pub client_config: Option<Option<PathBuf>>,

    /// Connection name in connections_credentials; only with --client-config
    #[arg(long, value_name = "NAME", conflicts_with = "local")]
    pub connection: Option<String>,

    /// HTTP port for a nonstandard native port; only with --client-config
    #[arg(long, value_name = "PORT", conflicts_with = "local")]
    pub http_port: Option<u16>,
}

#[derive(Args, Debug)]
#[command(group(ArgGroup::new("target").required(true).args(["local", "client_config", "cloud"])))]
pub struct AddArgs {
    /// Coding agent to register the server in
    #[arg(long, value_enum)]
    pub agent: McpAgent,

    /// Server name in the agent config
    #[arg(long, default_value = "clickhouse")]
    pub name: String,

    /// Register the hosted ClickHouse Cloud MCP server
    #[arg(long, conflicts_with_all = ["connection", "http_port"])]
    pub cloud: bool,

    #[command(flatten)]
    pub connection: ConnectionArgs,
}

#[derive(Args, Debug)]
#[command(group(ArgGroup::new("source").required(true).args(["local", "client_config"])))]
pub struct RunArgs {
    #[command(flatten)]
    pub connection: ConnectionArgs,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpAgent {
    /// Claude Code (project `.mcp.json`)
    Claude,
}

/// Where `mcp run` reads its connection from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Local(String),
    ClientConfig {
        /// `None` searches clickhouse-client's standard locations at run time.
        path: Option<PathBuf>,
        connection: Option<String>,
        http_port: Option<u16>,
    },
}

impl ConnectionArgs {
    pub fn into_source(self) -> Source {
        match self.local {
            Some(name) => Source::Local(name),
            None => Source::ClientConfig {
                path: self.client_config.flatten(),
                connection: self.connection,
                http_port: self.http_port,
            },
        }
    }
}

impl AddArgs {
    /// `None` for `--cloud`, which has no `mcp run` source.
    pub fn source(&self) -> Option<Source> {
        if self.cloud {
            return None;
        }
        Some(self.connection.clone().into_source())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use clap::error::ErrorKind;

    fn parse(args: &[&str]) -> McpCommands {
        let mut argv = vec!["clickhousectl", "mcp"];
        argv.extend(args);
        match Cli::try_parse_from(argv).unwrap().command {
            Commands::Mcp(args) => args.command,
            _ => unreachable!(),
        }
    }

    fn parse_err(args: &[&str]) -> ErrorKind {
        let mut argv = vec!["clickhousectl", "mcp"];
        argv.extend(args);
        match Cli::try_parse_from(argv) {
            Ok(_) => panic!("{args:?} should not parse"),
            Err(error) => error.kind(),
        }
    }

    fn run_source(args: &[&str]) -> Source {
        let mut argv = vec!["run"];
        argv.extend(args);
        let McpCommands::Run(run) = parse(&argv) else {
            panic!("run")
        };
        run.connection.into_source()
    }

    #[test]
    fn run_local_defaults_to_the_default_server() {
        assert_eq!(run_source(&["--local"]), Source::Local("default".into()));
        assert_eq!(run_source(&["--local", "dev"]), Source::Local("dev".into()));
    }

    #[test]
    fn run_client_config_path_is_optional() {
        assert_eq!(
            run_source(&["--client-config"]),
            Source::ClientConfig {
                path: None,
                connection: None,
                http_port: None
            }
        );
        assert_eq!(
            run_source(&[
                "--client-config",
                "/etc/c.xml",
                "--connection",
                "prod",
                "--http-port",
                "8124"
            ]),
            Source::ClientConfig {
                path: Some("/etc/c.xml".into()),
                connection: Some("prod".into()),
                http_port: Some(8124)
            }
        );
    }

    #[test]
    fn run_requires_exactly_one_source() {
        assert_eq!(parse_err(&["run"]), ErrorKind::MissingRequiredArgument);
        assert_eq!(
            parse_err(&["run", "--local", "--client-config", "c.xml"]),
            ErrorKind::ArgumentConflict
        );
    }

    #[test]
    fn connection_and_http_port_conflict_with_local_and_cloud() {
        for flag in [["--connection", "prod"], ["--http-port", "8123"]] {
            let mut args = vec!["run", "--local"];
            args.extend(flag);
            assert_eq!(parse_err(&args), ErrorKind::ArgumentConflict);
            let mut args = vec!["add", "--agent", "claude", "--cloud"];
            args.extend(flag);
            assert_eq!(parse_err(&args), ErrorKind::ArgumentConflict);
        }
        assert_eq!(
            parse_err(&["run", "--client-config", "--http-port", "notaport"]),
            ErrorKind::ValueValidation
        );
    }

    #[test]
    fn add_parses_each_target() {
        let McpCommands::Add(add) = parse(&["add", "--agent", "claude", "--cloud"]) else {
            panic!("add")
        };
        assert_eq!(add.agent, McpAgent::Claude);
        assert_eq!(add.name, "clickhouse");
        assert!(add.cloud);
        assert_eq!(add.source(), None);

        let McpCommands::Add(add) = parse(&[
            "add",
            "--agent",
            "claude",
            "--name",
            "ch-prod",
            "--client-config",
            "c.xml",
            "--connection",
            "prod",
        ]) else {
            panic!("add")
        };
        assert_eq!(add.name, "ch-prod");
        assert_eq!(
            add.source(),
            Some(Source::ClientConfig {
                path: Some("c.xml".into()),
                connection: Some("prod".into()),
                http_port: None
            })
        );

        let McpCommands::Add(add) = parse(&["add", "--agent", "claude", "--local"]) else {
            panic!("add")
        };
        assert_eq!(add.source(), Some(Source::Local("default".into())));
    }

    #[test]
    fn add_requires_agent_and_one_target() {
        assert_eq!(
            parse_err(&["add", "--cloud"]),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            parse_err(&["add", "--agent", "claude"]),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            parse_err(&["add", "--agent", "claude", "--cloud", "--local"]),
            ErrorKind::ArgumentConflict
        );
        assert_eq!(
            parse_err(&["add", "--agent", "cursor", "--cloud"]),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn json_is_accepted_on_both_subcommands() {
        for argv in [
            vec![
                "clickhousectl",
                "mcp",
                "--json",
                "add",
                "--agent",
                "claude",
                "--cloud",
            ],
            vec![
                "clickhousectl",
                "mcp",
                "add",
                "--agent",
                "claude",
                "--cloud",
                "--json",
            ],
        ] {
            let Commands::Mcp(args) = Cli::try_parse_from(argv).unwrap().command else {
                panic!("mcp")
            };
            assert!(args.json);
        }
    }
}
