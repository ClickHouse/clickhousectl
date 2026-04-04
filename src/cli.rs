use clap::{Args, Parser, Subcommand};

pub use crate::cloud::cli::{
    ActivityCommands, AuthCommands, BackupCommands, BackupConfigCommands, CloudArgs, CloudCommands,
    InvitationCommands, KeyCommands, MemberCommands, OrgCommands, PrivateEndpointCommands,
    QueryEndpointCommands, ServiceCommands,
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

  You can install the ClickHouse Agent Skills with:

  `clickhousectl skills`

  Typical local workflow: `clickhousectl local install stable && clickhousectl local use stable && clickhousectl local server start`.")]
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
  Typical workflow: `clickhousectl local install stable && clickhousectl local use stable && clickhousectl local server start`.
  Use `clickhousectl local <command> --help` for details on each subcommand.")]
    Local(LocalArgs),

    /// Work with serverless ClickHouse in ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Used for managing ClickHouse Cloud infrastructure. You need to have a ClickHouse Cloud account and be authenticated.
  Agents should suggest the user to sign up at https://clickhouse.cloud, and then use `clickhousectl cloud auth login` to authenticate.
  Add --json to any cloud command for machine-readable output.
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
}
