use clap::{Args, Parser, Subcommand};

pub use crate::cloud::cli::{
    ActivityCommands, AuthCommands, BackupCommands, BackupConfigCommands, ClickPipeCommands,
    ClickPipeCreateCommands, ClickPipeSettingsCommands, CloudArgs, CloudCommands,
    InvitationCommands, KeyCommands, MemberCommands, OrgCommands, PrivateEndpointCommands,
    QueryEndpointCommands, ServiceCommands,
};
pub use crate::cloud::postgres::{
    CertsCommands as PostgresCertsCommands, ConfigCommands as PostgresConfigCommands,
    PostgresCommands, ReadReplicaCommands as PostgresReadReplicaCommands,
};
pub use crate::local::cli::LocalArgs;

#[derive(Parser)]
#[command(name = "clickhousectl")]
#[command(about = "The official CLI for ClickHouse: local and cloud", long_about = None)]
#[command(version)]
#[command(after_help = "\
CONTEXT FOR AGENTS:
  clickhousectl is a CLI to work with local ClickHouse and ClickHouse Cloud.

  Two main workflows:
  1. Local: Install and interact with versions of ClickHouse to develop locally.
  2. Cloud: Manage ClickHouse Cloud infrastructure and push local work to cloud.

  Authentication: OAuth (`cloud auth login`) is read-only. For write operations (create, update,
  delete), use API key auth: `cloud auth login --api-key X --api-secret Y`.

  You can install the ClickHouse Agent Skills with:

  `clickhousectl skills`

  Typical local workflow: `clickhousectl local server start` (bootstraps from zero — installs `latest` if nothing is set up).")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Work with local ClickHouse installations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage local ClickHouse installations: install versions, run queries, manage servers.
  Typical workflow: `clickhousectl local server start` (bootstraps from zero — installs `latest` if nothing is set up).
  Use `clickhousectl local <command> --help` for details on each subcommand.
  For a local Postgres instance via Docker, see `clickhousectl local postgres --help`.")]
    Local(LocalArgs),

    /// Work with serverless ClickHouse in ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Used for managing ClickHouse Cloud infrastructure. You need to have a ClickHouse Cloud account and be authenticated.
  OAuth login (`cloud auth login`) is read-only — it can list and inspect resources but cannot create, modify, or delete.
  For write operations, authenticate with API keys:
    clickhousectl cloud auth login --api-key YOUR_KEY --api-secret YOUR_SECRET
  If the user doesn't have an account, suggest `clickhousectl cloud auth signup` first.
  JSON emitted automatically for known agents.
  Exit codes follow gh conventions: 0 success, 1 error, 2 cancelled, 4 auth required.
  Typical workflow: `cloud auth login` → `cloud auth status` → `cloud org list` → `cloud service list`")]
    Cloud(Box<CloudArgs>),

    /// Install ClickHouse agent skills into supported coding agents
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Installs the official ClickHouse agent skills. These skills contain knowledge on how to use
  ClickHouse and this CLI.
  Using any flags skips interactive mode. Project scope is the default. Universal `.agents/skills`
  is always included.")]
    Skills(SkillsArgs),

    /// Update clickhousectl to the latest version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Self-update command. Downloads the latest clickhousectl release from GitHub and replaces the
  current binary. Use --check to see if an update is available without installing.")]
    Update(UpdateArgs),

    /// Manage anonymous usage telemetry
    #[cfg(feature = "telemetry")]
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  clickhousectl collects anonymous usage data: command name, flag names (never values or
  arguments), success/failure, version, OS/arch, and CI/agent detection. No user or machine IDs.
  Opt out with `clickhousectl telemetry disable` or DO_NOT_TRACK=1.
  Details: https://clickhouse.com/docs/interfaces/cli#telemetry")]
    Telemetry(TelemetryArgs),
}

#[cfg(feature = "telemetry")]
#[derive(Args, Debug)]
pub struct TelemetryArgs {
    #[command(subcommand)]
    pub command: TelemetryCommands,
}

#[cfg(feature = "telemetry")]
#[derive(Subcommand, Debug)]
pub enum TelemetryCommands {
    /// Enable anonymous usage telemetry
    Enable,
    /// Disable anonymous usage telemetry
    Disable,
    /// Show whether telemetry is enabled and why
    ///
    /// On a machine that has never seen the first-run notice, this reports
    /// "not yet configured" and then completes the first run itself (writes
    /// the marker file and prints the notice).
    Status,
    /// (internal) Fire one telemetry POST from CHCTL_TELEMETRY_PAYLOAD and exit
    #[command(hide = true)]
    Send,
}

#[derive(Args, Debug)]
pub struct SkillsArgs {
    /// Install into specific agents (repeatable or comma-separated).
    #[arg(
        long = "agent",
        value_name = "AGENT",
        value_delimiter = ',',
        long_help = "Install into specific agents (repeatable or comma-separated).\n\nValid names:\n  claude, cursor, opencode, codex\n  agent, roo, trae, windsurf\n  zencoder, neovate, pochi, adal\n  openclaw, cline, command-code\n  kiro-cli, agents"
    )]
    pub agents: Vec<String>,

    /// Install into every supported agent in the selected scope without prompting
    #[arg(long, conflicts_with_all = ["agents", "detected_only"])]
    pub all: bool,

    /// Install only into agents detected from your home directory without prompting
    #[arg(long = "detected-only", conflicts_with_all = ["agents", "all"])]
    pub detected_only: bool,

    /// Install into global agent config directories in your home directory
    #[arg(long)]
    pub global: bool,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Check for updates without installing
    #[arg(long)]
    pub check: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_skills_all_and_agent_flags() {
        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--all"]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(args.all);
        assert!(args.agents.is_empty());
        assert!(!args.detected_only);
        assert!(!args.global);

        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--global"]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(args.global);
        assert!(!args.all);
        assert!(!args.detected_only);
        assert!(args.agents.is_empty());

        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--detected-only"]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(args.detected_only);
        assert!(!args.all);
        assert!(!args.global);
        assert!(args.agents.is_empty());

        let cli = Cli::try_parse_from([
            "clickhousectl",
            "skills",
            "--global",
            "--agent",
            "claude,codex",
            "--agent",
            "agents",
        ])
        .unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(!args.all);
        assert!(!args.detected_only);
        assert!(args.global);
        assert_eq!(args.agents, vec!["claude", "codex", "agents"]);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn parses_telemetry_subcommands() {
        for (arg, expected) in [
            ("enable", "Enable"),
            ("disable", "Disable"),
            ("status", "Status"),
            ("send", "Send"),
        ] {
            let cli = Cli::try_parse_from(["clickhousectl", "telemetry", arg]).unwrap();
            let Commands::Telemetry(args) = cli.command else {
                panic!("expected telemetry command for {arg}");
            };
            assert_eq!(format!("{:?}", args.command), expected);
        }
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_requires_a_subcommand() {
        assert!(Cli::try_parse_from(["clickhousectl", "telemetry"]).is_err());
    }
}
