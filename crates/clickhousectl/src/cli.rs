use clap::{Args, Parser, Subcommand};

use crate::cloud::cli::CloudArgs;
pub use crate::local::cli::LocalArgs;

#[derive(Parser)]
#[command(name = "clickhousectl")]
#[command(about = "The official CLI for ClickHouse: local and cloud", long_about = None)]
#[command(version)]
#[command(after_help = "\
CONTEXT FOR AGENTS:
  Cloud auth: OAuth (`cloud auth login`) is read-only; API keys
  (`cloud auth login --api-key X --api-secret Y`) allow writes.
  Create account: `cloud auth signup`
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
  --all, --detected-only or --agent skip the agent prompt; --global only sets the scope, so agents
  are still prompted.
  Agent selection without one of those three flags needs a TTY and errors out without one.
  Scope: prompted on a TTY, else the current project directory; --global forces your home directory.
  The universal `.agents/skills` target is always installed, alongside any selected agent.")]
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
    #[arg(
        long = "agent",
        value_name = "AGENT",
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::skills::supported_agent_keys())
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
    use clap::CommandFactory;
    use std::collections::BTreeMap;

    fn visit_commands(
        command: &clap::Command,
        path: &str,
        visit: &mut impl FnMut(&clap::Command, &str),
    ) {
        visit(command, path);
        for child in command.get_subcommands() {
            visit_commands(child, &format!("{path} {}", child.get_name()), visit);
        }
    }

    #[test]
    fn whole_command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn whole_command_tree_has_descriptions() {
        let mut failures = Vec::new();
        visit_commands(&Cli::command(), "clickhousectl", &mut |command, path| {
            if command
                .get_about()
                .is_none_or(|about| about.to_string().trim().is_empty())
            {
                failures.push(format!("{path}: missing command description"));
            }
            for arg in command.get_arguments() {
                if arg
                    .get_help()
                    .is_none_or(|help| help.to_string().trim().is_empty())
                {
                    failures.push(format!("{path}: missing description for {}", arg.get_id()));
                }
            }
        });
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn whole_command_tree_follows_help_structure() {
        let mut failures = Vec::new();
        visit_commands(&Cli::command(), "clickhousectl", &mut |command, path| {
            if command
                .get_about()
                .is_some_and(|about| about.to_string().lines().count() != 1)
            {
                failures.push(format!("{path}: about must be one line"));
            }
            if command.get_long_about().is_some() {
                failures.push(format!("{path}: long_about is not allowed"));
            }
            if command.get_before_help().is_some() || command.get_before_long_help().is_some() {
                failures.push(format!("{path}: before_help is not allowed"));
            }
            if command.get_after_long_help().is_some() {
                failures.push(format!("{path}: use after_help for agent context"));
            }
            if command
                .get_subcommand_help_heading()
                .is_some_and(|heading| heading != "Commands")
            {
                failures.push(format!("{path}: use the standard Commands heading"));
            }
            for arg in command.get_arguments() {
                if arg
                    .get_help_heading()
                    .is_some_and(|heading| !["Arguments", "Options"].contains(&heading))
                {
                    failures.push(format!(
                        "{path}: {} has a custom help heading",
                        arg.get_id()
                    ));
                }
            }
            if let Some(after_help) = command.get_after_help() {
                let text = after_help.to_string();
                let mut lines = text.lines().filter(|line| !line.trim().is_empty());
                if lines.next() != Some("CONTEXT FOR AGENTS:") {
                    failures.push(format!(
                        "{path}: after_help must start with CONTEXT FOR AGENTS:"
                    ));
                }
                let content: Vec<_> = lines.collect();
                if content.is_empty() || content.len() > 8 {
                    failures.push(format!(
                        "{path}: agent context has {} content lines (expected 1–8)",
                        content.len()
                    ));
                }
                for line in content {
                    if !line.starts_with("  ") {
                        failures.push(format!(
                            "{path}: context content must be indented by at least two spaces"
                        ));
                    }
                    if line.chars().count() > 120 {
                        failures.push(format!(
                            "{path}: context line exceeds 120 characters: {line}"
                        ));
                    }
                }
            }
        });
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn shared_flags_have_identical_help_at_every_declaration() {
        let shared = ["api-key", "api-secret", "url", "org-id", "json", "debug"];
        let mut declarations = BTreeMap::new();
        let mut failures = Vec::new();
        // Inspect declarations before build() propagates global flags to descendants.
        visit_commands(&Cli::command(), "clickhousectl", &mut |command, path| {
            for arg in command.get_arguments() {
                let Some(flag) = arg.get_long().filter(|flag| shared.contains(flag)) else {
                    continue;
                };
                let help = (
                    arg.get_help().map(ToString::to_string),
                    arg.get_long_help().map(ToString::to_string),
                );
                if let Some((previous_path, previous_help)) = declarations.get(flag) {
                    if previous_help != &help {
                        failures.push(format!("--{flag}: {path} differs from {previous_path}: {help:?} != {previous_help:?}"));
                    }
                } else {
                    declarations.insert(flag.to_owned(), (path.to_owned(), help));
                }
            }
        });
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn unknown_command_exits_with_a_usage_error() {
        assert_eq!(
            Cli::try_parse_from(["clickhousectl", "unknown-command"])
                .err()
                .expect("unknown command must be rejected")
                .exit_code(),
            2
        );
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

    #[test]
    fn skills_agent_accepts_every_supported_agent_and_rejects_unknown_values() {
        let supported = crate::skills::supported_agent_keys().collect::<Vec<_>>();
        let joined = supported.join(",");
        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--agent", &joined]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert_eq!(args.agents, supported);

        let error = Cli::try_parse_from(["clickhousectl", "skills", "--agent", "unknown"])
            .err()
            .expect("unknown agent must be rejected by clap");
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        assert_eq!(error.exit_code(), 2);
        let message = error.to_string();
        for agent in crate::skills::supported_agent_keys() {
            assert!(message.contains(agent), "missing `{agent}`: {message}");
        }
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
