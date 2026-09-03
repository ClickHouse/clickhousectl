use clap::{Args, Parser, Subcommand};

use crate::cloud::cli::CloudArgs;
pub use crate::local::cli::LocalArgs;

#[derive(Parser)]
#[command(name = "clickhousectl")]
#[command(about = "The official CLI for ClickHouse: local and cloud", long_about = None)]
#[command(version)]
#[command(after_help = "\
CONTEXT FOR AGENTS:
  Both `local` and `cloud` manage ClickHouse and Postgres.
  Cloud auth: OAuth (`cloud auth login`) is read-only; API keys
  (`cloud auth login --api-key X --api-secret Y`) allow writes.
  Create account: `cloud auth signup`
  Typical local flow: `local server start` -> `local client -q 'SELECT 1'`
  Typical cloud flow: `cloud auth signup` -> `cloud auth login --api-key X --api-secret Y` -> `cloud service create`
  Install the ClickHouse agent skills: `clickhousectl skills --agent claude`")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage local ClickHouse and Postgres
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Project-scoped commands use `.clickhouse` under the exact current directory; parent directories
  are not searched. Run them from the project root.
  `clickhousectl local server start` bootstraps from zero — installs `latest` if nothing is set up.
  Local Postgres instances are Docker-backed and need Docker running.
  Typical flow: `local server start` -> `local client -q 'SELECT 1'`")]
    Local(LocalArgs),

    /// Manage ClickHouse and Postgres in ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Credential precedence, first wins: --api-key/--api-secret flags, .clickhouse/credentials.json,
  CLICKHOUSE_CLOUD_API_KEY/CLICKHOUSE_CLOUD_API_SECRET (shell then .env), OAuth tokens.
  API keys are read+write; OAuth is read-only and every write command fails on it.
  `cloud auth status` shows the active source; --org-id auto-detects only with exactly one org.
  delete/remove act immediately — there is no confirmation prompt.
  Exit codes: 0 success, 1 error, 2 usage error, 3 cancelled, 4 auth required.
  Typical flow: `cloud auth login --api-key X --api-secret Y` -> `cloud org list` -> `cloud service list`")]
    Cloud(Box<CloudArgs>),

    /// Install ClickHouse agent skills into supported coding agents
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Any flag skips the interactive prompts; with no flags this command prompts on a TTY.
  Scope is the current project directory unless --global.
  The universal `.agents/skills` target is always installed, alongside any selected agent.
  --agent values: claude, cursor, opencode, codex, agent, roo, trae, windsurf, zencoder, neovate,
  pochi, adal, openclaw, cline, command-code, kiro-cli, agents")]
    Skills(SkillsArgs),

    /// Update clickhousectl to the latest version
    Update(UpdateArgs),

    /// Manage anonymous usage telemetry
    #[cfg(feature = "telemetry")]
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Collected: command name, flag and argument names (never their values), success/failure, version,
  OS/arch, CI/agent detection. No user or machine IDs.
  DO_NOT_TRACK=1 also disables telemetry, without writing any config.
  Details: https://clickhouse.com/docs/concepts/features/interfaces/cli#telemetry")]
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
    Status,
    /// (internal) Fire one telemetry POST from CHCTL_TELEMETRY_PAYLOAD and exit
    //
    // Stable cross-version interface — never remove or rename. After a
    // self-update the parent (old version) spawns the freshly installed
    // binary (new version) as `telemetry send` with the payload in
    // CHCTL_TELEMETRY_PAYLOAD, so this subcommand and that env var must keep
    // working across releases.
    #[command(hide = true)]
    Send,
}

#[derive(Args, Debug)]
pub struct SkillsArgs {
    /// Install into specific agents (repeatable, comma-separated)
    #[arg(long = "agent", value_name = "AGENT", value_delimiter = ',')]
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
    use clap::CommandFactory;

    #[test]
    fn cloud_help_documents_credential_precedence() {
        let mut command = Cli::command();
        let help = command
            .find_subcommand_mut("cloud")
            .expect("cloud subcommand")
            .render_long_help()
            .to_string();

        let precedence = help
            .split_once("Credential precedence, first wins:")
            .expect("credential precedence must be documented")
            .1;
        let flags = precedence.find("--api-key/--api-secret flags").unwrap();
        let file = precedence.find(".clickhouse/credentials.json").unwrap();
        let env = precedence
            .find("CLICKHOUSE_CLOUD_API_KEY/CLICKHOUSE_CLOUD_API_SECRET (shell then .env)")
            .unwrap();
        let oauth = precedence.find("OAuth tokens.").unwrap();
        assert!(flags < file && file < env && env < oauth, "{help}");
        assert!(
            help.contains("API keys are read+write; OAuth is read-only"),
            "{help}"
        );
    }

    #[test]
    fn cloud_help_distinguishes_usage_errors_from_cancellation() {
        let mut command = Cli::command();
        let help = command
            .find_subcommand_mut("cloud")
            .expect("cloud subcommand")
            .render_long_help()
            .to_string();

        assert!(help.contains(
            "Exit codes: 0 success, 1 error, 2 usage error, 3 cancelled, 4 auth required."
        ));
        assert_eq!(
            Cli::try_parse_from(["clickhousectl", "unknown-command"])
                .err()
                .expect("unknown command must be rejected")
                .exit_code(),
            2
        );
    }

    #[test]
    fn help_points_agents_to_cloud_signup() {
        let mut command = Cli::command();
        let root_help = command.render_long_help().to_string();
        assert!(root_help.contains("Create account: `cloud auth signup`"));

        let auth = command
            .find_subcommand_mut("cloud")
            .expect("cloud subcommand")
            .find_subcommand_mut("auth")
            .expect("auth subcommand");

        let signup_help = auth
            .find_subcommand_mut("signup")
            .expect("signup subcommand")
            .render_long_help()
            .to_string();
        assert!(signup_help.contains("Create a ClickHouse Cloud account"));
        assert!(!signup_help.to_lowercase().contains("browser"));
    }

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
