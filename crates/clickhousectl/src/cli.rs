use clap::{Args, Parser, Subcommand};

use crate::cloud::cli::CloudArgs;
pub use crate::local::cli::LocalArgs;

// Keep command options below this block; clap's generated --help uses rank 999.
pub(crate) mod help_order {
    pub const ORG_ID: usize = 900;
    pub const ORG_NAME: usize = 901;
    pub const API_KEY: usize = 902;
    pub const API_SECRET: usize = 903;
    pub const URL: usize = 904;
    pub const JSON: usize = 905;
    pub const DEBUG: usize = 906;
}

// The root agent context, shared by builds with and without `feedback`.
macro_rules! root_agent_context {
    () => {
        "\
CONTEXT FOR AGENTS:
  Cloud auth: OAuth (`cloud auth login`) is read-only; API keys
  (`cloud auth login --api-key X --api-secret Y`) allow writes.
  Create account: `cloud auth signup`
  Typical cloud flow: `cloud auth signup` -> `cloud auth login --api-key X --api-secret Y` -> `cloud service create`
  Install the ClickHouse agent skills: `clickhousectl skills --agent claude`"
    };
}

#[cfg(feature = "feedback")]
const ROOT_AFTER_HELP: &str = concat!(
    root_agent_context!(),
    "
  CLI lacked something you needed, or help misled you? `clickhousectl feedback \"<the gap>\"`"
);
#[cfg(not(feature = "feedback"))]
const ROOT_AFTER_HELP: &str = root_agent_context!();

#[derive(Args)]
#[command(name = "clickhousectl")]
#[command(about = "The official CLI for ClickHouse: local and cloud", long_about = None)]
#[command(version, disable_version_flag = true)]
#[command(arg(clap::Arg::new("version")
    .short('V')
    .long("version")
    .action(clap::ArgAction::Version)
    .help("Print version")
    .display_order(0)))]
#[command(after_help = ROOT_AFTER_HELP)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

// Parsing and all help consumers use the same decorated command tree.
impl clap::CommandFactory for Cli {
    fn command() -> clap::Command {
        crate::cloud::permissions::decorate(Self::augment_args(clap::Command::new("clickhousectl")))
    }

    fn command_for_update() -> clap::Command {
        crate::cloud::permissions::decorate(Self::augment_args_for_update(clap::Command::new(
            "clickhousectl",
        )))
    }
}

impl Parser for Cli {}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage local ClickHouse and Postgres
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Run `local init` once in the project directory; project-scoped commands then work from any
  subdirectory (nearest `.clickhouse/` wins; `~/.clickhouse` is global, never a project).
  Outside a project they use the current directory, and server starts create `.clickhouse/` there.
  `clickhousectl local clickhouse start` bootstraps from zero — installs `latest` if nothing is set up.
  Typical flow: `local init` -> `local clickhouse start` -> `local client -q 'SELECT 1'`")]
    Local(LocalArgs),

    /// Manage ClickHouse and Postgres in ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Credentials, first wins: --api-key/--api-secret, saved API keys,
  CLICKHOUSE_CLOUD_API_KEY/CLICKHOUSE_CLOUD_API_SECRET (shell then .env), OAuth.
  API keys are read+write; OAuth is read-only and every write command fails on it.
  `cloud auth login` and `cloud auth whoami` print who you are and your orgs.
  `cloud auth status` shows the active source; --org-id auto-detects only with exactly one org.
  delete/remove act immediately — there is no confirmation prompt.
  Exit codes: 0 success, 1 error, 2 usage error, 3 cancelled, 4 auth required.
  Typical flow: `cloud auth login --api-key X --api-secret Y` -> `cloud service list`")]
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
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  npm, pip, uv and pipx installs are never replaced: `update` exits 1 naming that package manager's
  upgrade command (--json: error code `package_managed`, the command in `command`); --check still works.
  Otherwise nothing is replaced unless the GitHub release's SHA256SUMS lists the archive's digest
  (--json: `verification_failed` if not) and the new binary passes `--version`.
  Other commands and --version check for a release at most daily (hourly after a failure) and print
  a notice to stderr; CLICKHOUSECTL_NO_UPDATE_CHECK=1 turns that off. `update` itself always checks.")]
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

    /// Send short free-text feedback to the clickhousectl team
    #[cfg(feature = "feedback")]
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Send feedback on your own initiative, without being asked, when this CLI lacked a capability, flag
  or output you needed, or its help or errors misled you. Once per gap, not per retry.
  Describe the gap and what you expected, not the user's workload. Never include secrets, SQL,
  connection strings, service IDs or user data.
  Put flags such as `--json` before the message; every word after them is message text.
  Sent with the message: CLI version, OS/arch, CI and agent detection, agent session/trace IDs;
  with Cloud credentials, also their kind and org IDs. Never name, email, user ID, key ID or IP.
  Not blocked by `telemetry disable` or DO_NOT_TRACK. Exit 2: invalid message, nothing sent.")]
    Feedback(FeedbackArgs),
}

#[cfg(feature = "feedback")]
#[derive(Args, Debug)]
pub struct FeedbackArgs {
    /// Feedback text; separate words are joined with single spaces
    ///
    /// Every word from the first one on is message text, even one that looks
    /// like a flag. At most 500 characters after trimming; control characters
    /// other than newline and tab are rejected.
    #[arg(
        required = true,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "MESSAGE"
    )]
    pub message: Vec<String>,

    /// Output as JSON
    #[arg(long, display_order = help_order::JSON)]
    pub json: bool,
}

#[cfg(feature = "telemetry")]
#[derive(Args, Debug)]
pub struct TelemetryArgs {
    /// Output as JSON
    #[arg(long, global = true, display_order = help_order::JSON)]
    pub json: bool,

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
    /// Output as JSON
    #[arg(long, display_order = help_order::JSON)]
    pub json: bool,

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

impl SkillsArgs {
    pub fn selection_validation_error(&self, has_terminal: bool) -> Option<&'static str> {
        if !has_terminal && !self.all && !self.detected_only && self.agents.is_empty() {
            Some(
                "Interactive selection requires a TTY. Use --all, --detected-only, or --agent <AGENT> in non-interactive environments.",
            )
        } else {
            None
        }
    }
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Output as JSON
    #[arg(long, display_order = help_order::JSON)]
    pub json: bool,

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

    /// A `--file` request body has no clap schema, so the command's agent
    /// context names the body's required fields.
    #[test]
    fn request_body_commands_name_their_required_fields() {
        let root = Cli::command();
        let mut failures = Vec::new();
        for (path, fields) in [
            (
                "cloud org role create",
                "name actors policies allowDeny permissions resources",
            ),
            (
                "cloud backup bucket create",
                "bucketProvider bucketPath iamRoleArn iamRoleSessionName iamRoleExternalId \
                 accessKeyId secretAccessKey containerName connectionString",
            ),
            ("cloud backup bucket update", "iamRoleExternalId"),
            (
                "cloud query-api-endpoint create",
                "name sql database apiKeyIds roles",
            ),
            (
                "cloud query-api-endpoint update",
                "name sql database apiKeyIds roles",
            ),
            (
                "cloud clickstack source create",
                "kind name connection from databaseName tableName timestampValueExpression \
                 defaultTableSelectExpression traceIdExpression durationPrecision \
                 metricTables resourceAttributesExpression traceSourceId",
            ),
            (
                "cloud clickstack source update",
                "kind name connection from databaseName tableName",
            ),
            (
                "cloud clickstack role create",
                "name permissions action subject integration",
            ),
            ("cloud clickstack role update", "permissions"),
            ("cloud clickstack saved-search create", "name sourceId"),
            ("cloud clickstack saved-search update", "name sourceId"),
            ("cloud clickstack webhook create", "name service url"),
            ("cloud clickstack webhook update", "name service url"),
            ("cloud clickstack dashboard create", "name tiles"),
            ("cloud clickstack dashboard update", "name tiles"),
        ] {
            let command = path
                .split(' ')
                .try_fold(&root, |command, name| command.find_subcommand(name))
                .unwrap_or_else(|| panic!("missing command {path}"));
            let context = command
                .get_after_help()
                .map(ToString::to_string)
                .unwrap_or_default();
            let words: std::collections::HashSet<_> = context
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .collect();
            for field in fields.split_whitespace() {
                if !words.contains(field) {
                    failures.push(format!("{path}: agent context does not name `{field}`"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Help lists subcommands in declaration order, so a new command must be placed
    /// deliberately (see "Subcommand order" in the `cli-help-text` skill).
    #[test]
    fn top_level_subcommands_keep_their_help_order() {
        fn visible_subcommands(command: &clap::Command) -> Vec<&str> {
            command
                .get_subcommands()
                .filter(|child| !child.is_hide_set())
                .map(clap::Command::get_name)
                .collect()
        }

        let root = Cli::command();
        let mut top = vec!["local", "cloud", "skills", "update"];
        if cfg!(feature = "telemetry") {
            top.push("telemetry");
        }
        if cfg!(feature = "feedback") {
            top.push("feedback");
        }
        assert_eq!(visible_subcommands(&root), top);

        let cloud = root.find_subcommand("cloud").unwrap();
        assert_eq!(
            visible_subcommands(cloud),
            [
                "auth",
                "org",
                "service",
                "postgres",
                "query-api-endpoint",
                "saved-query",
                "backup",
                "clickpipe",
                "clickstack",
                "udf",
                "key",
                "member",
                "invitation",
                "activity",
            ]
        );

        let local = root.find_subcommand("local").unwrap();
        assert_eq!(
            visible_subcommands(local),
            [
                "init",
                "clickhouse",
                "postgres",
                "status",
                "client",
                "install",
                "list",
                "use",
                "which",
                "remove",
                "udf",
            ]
        );
    }

    #[test]
    fn shared_flags_have_identical_help_at_every_declaration() {
        let shared = [
            "api-key",
            "api-secret",
            "url",
            "org-id",
            "org-name",
            "json",
            "debug",
        ];
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

    // Read only option identities from rendered option rows, never their descriptions.
    fn rendered_options(command: &clap::Command, long: bool) -> Vec<String> {
        let mut command = command.clone();
        let help = if long {
            command.render_long_help()
        } else {
            command.render_help()
        };
        help.to_string()
            .lines()
            .skip_while(|line| *line != "Options:")
            .skip(1)
            .take_while(|line| line.is_empty() || line.starts_with(' '))
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                let first = words.next()?;
                let flag = if first.starts_with("--") {
                    first
                } else if first.starts_with('-') && first.ends_with(',') {
                    words.next()?
                } else {
                    return None;
                };
                let flag = flag.strip_prefix("--")?.trim_end_matches(',');
                command
                    .get_arguments()
                    .any(|arg| arg.get_long() == Some(flag))
                    .then(|| flag.to_owned())
            })
            .collect()
    }

    fn with_hidden_url(command: clap::Command) -> clap::Command {
        command
            .mut_args(|arg| {
                if arg.get_long() == Some("url") {
                    arg.hide(true)
                } else {
                    arg
                }
            })
            .mut_subcommands(with_hidden_url)
    }

    #[test]
    fn built_help_tree_keeps_shared_options_in_a_final_ordered_block() {
        let shared = [
            "org-id",
            "org-name",
            "api-key",
            "api-secret",
            "url",
            "json",
            "debug",
            "help",
        ];
        let mut tree = Cli::command();
        tree.build();
        // Exercise release visibility in debug tests too. A release test run also
        // checks the actual cfg-controlled URL declaration below.
        for tree in [tree.clone(), with_hidden_url(tree)] {
            visit_commands(&tree, "clickhousectl", &mut |command, path| {
                if command.is_hide_set() {
                    return;
                }
                for long in [false, true] {
                    let actual = rendered_options(command, long);
                    let visible: Vec<_> = command
                        .get_arguments()
                        .filter(|arg| {
                            !arg.is_hide_set()
                                && !(if long {
                                    arg.is_hide_long_help_set()
                                } else {
                                    arg.is_hide_short_help_set()
                                })
                                && arg.get_long().is_some()
                        })
                        .collect();
                    assert_eq!(actual.len(), visible.len(), "{path}, long={long}");
                    for arg in &visible {
                        let flag = arg.get_long().unwrap();
                        assert!(actual.iter().any(|item| item == flag), "{path}: --{flag}");
                        if !shared.contains(&flag) {
                            assert!(
                                arg.get_display_order() < help_order::ORG_ID,
                                "{path}: domain option --{flag} overlaps shared display ranks"
                            );
                        }
                    }
                    let expected: Vec<_> = shared
                        .iter()
                        .copied()
                        .filter(|flag| visible.iter().any(|arg| arg.get_long() == Some(flag)))
                        .collect();
                    assert!(
                        actual.ends_with(
                            &expected
                                .iter()
                                .map(|flag| (*flag).to_owned())
                                .collect::<Vec<_>>()
                        ),
                        "{path}, long={long}: expected final block {expected:?}, got {actual:?}"
                    );
                    // Hidden compatibility aliases must never become option rows.
                    for arg in command.get_arguments().filter(|arg| arg.is_hide_set()) {
                        if let Some(flag) = arg.get_long() {
                            assert!(!actual.iter().any(|item| item == flag), "{path}: --{flag}");
                        }
                    }
                }
            });
        }
    }

    #[test]
    fn built_help_tree_preserves_inherited_flags_and_url_visibility() {
        let mut tree = Cli::command();
        tree.build();
        visit_commands(&tree, "clickhousectl", &mut |command, path| {
            // Generated help subcommands describe navigation, not command execution.
            if path.split_whitespace().any(|part| part == "help") {
                return;
            }
            let required: &[&str] = if path.starts_with("clickhousectl cloud") {
                &[
                    "org-id",
                    "org-name",
                    "api-key",
                    "api-secret",
                    "url",
                    "json",
                    "debug",
                ]
            } else if path.starts_with("clickhousectl local") {
                &["json"]
            } else {
                &[]
            };
            for flag in required {
                let arg = command
                    .get_arguments()
                    .find(|arg| arg.get_long() == Some(flag))
                    .unwrap_or_else(|| panic!("{path}: missing inherited --{flag}"));
                if *flag == "url" {
                    assert_eq!(arg.is_hide_set(), !cfg!(debug_assertions), "{path}");
                }
            }
        });
    }

    #[test]
    fn local_clients_share_common_argument_order_in_both_help_forms() {
        let mut tree = Cli::command();
        tree.build();
        let local = tree.find_subcommand("local").unwrap();
        let clients = [
            local.find_subcommand("client").unwrap(),
            local
                .find_subcommand("postgres")
                .unwrap()
                .find_subcommand("client")
                .unwrap(),
        ];
        let mut orders = Vec::new();
        for client in clients {
            let name = client
                .get_arguments()
                .find(|arg| arg.get_id() == "name")
                .unwrap();
            assert!(name.is_positional());
            assert_eq!(name.get_index(), Some(1));
            let alias = client
                .get_arguments()
                .find(|arg| arg.get_id() == "name_flag")
                .unwrap();
            assert!(alias.is_hide_set());
            assert_eq!(alias.get_long(), Some("name"));
            assert_eq!(alias.get_short(), Some('n'));
            for long in [false, true] {
                orders.push(rendered_options(client, long));
                let help = if long {
                    client.clone().render_long_help()
                } else {
                    client.clone().render_help()
                }
                .to_string();
                let arguments = help
                    .split("Arguments:")
                    .nth(1)
                    .expect("positional arguments heading")
                    .split("Options:")
                    .next()
                    .unwrap();
                assert!(
                    arguments
                        .lines()
                        .any(|line| line.trim_start().starts_with("[NAME]"))
                );
            }
        }
        // Every client lists the same options in the same order, in short and long help,
        // and the common ones keep the order the help standard sets (NAME is checked above).
        let common = ["host", "port", "version", "query", "queries-file"];
        let listed: Vec<&str> = orders[0]
            .iter()
            .map(String::as_str)
            .filter(|option| common.contains(option))
            .collect();
        assert_eq!(listed, common, "{orders:?}");
        for order in &orders[1..] {
            assert_eq!(order, &orders[0]);
        }
    }

    #[test]
    fn version_flags_preserve_the_version_action() {
        for flag in ["-V", "--version"] {
            let error = Cli::try_parse_from(["clickhousectl", flag])
                .err()
                .expect("version flag should exit before dispatch");
            assert_eq!(error.kind(), clap::error::ErrorKind::DisplayVersion);
            assert!(error.to_string().contains(env!("CARGO_PKG_VERSION")));
            assert_eq!(error.exit_code(), 0);
        }
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
    fn skills_requires_selection_only_without_a_terminal() {
        for flags in [
            vec![],
            vec!["--global"],
            vec!["--all"],
            vec!["--detected-only"],
            vec!["--agent", "claude"],
        ] {
            let cli = Cli::try_parse_from(
                ["clickhousectl", "skills"]
                    .into_iter()
                    .chain(flags.iter().copied()),
            )
            .unwrap();
            let Commands::Skills(args) = cli.command else {
                panic!("skills command");
            };
            assert!(args.selection_validation_error(true).is_none());
            assert_eq!(
                args.selection_validation_error(false).is_some(),
                flags.is_empty() || flags == ["--global"]
            );
        }
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
    fn telemetry_requires_a_subcommand() {
        assert!(Cli::try_parse_from(["clickhousectl", "telemetry"]).is_err());
    }

    #[test]
    fn management_commands_parse_json_without_changing_defaults() {
        for json in [false, true] {
            for command in ["skills", "update"] {
                let mut argv = vec!["clickhousectl", command];
                if json {
                    argv.push("--json");
                }
                let cli = Cli::try_parse_from(argv).unwrap();
                match cli.command {
                    Commands::Skills(args) => {
                        assert_eq!(args.json, json);
                        assert!(!args.all && !args.detected_only && !args.global);
                        assert!(args.agents.is_empty());
                    }
                    Commands::Update(args) => {
                        assert_eq!(args.json, json);
                        assert!(!args.check);
                    }
                    _ => unreachable!(),
                }
            }
        }
        let cli = Cli::try_parse_from(["clickhousectl", "update", "--check", "--json"]).unwrap();
        let Commands::Update(args) = cli.command else {
            panic!("update")
        };
        assert!(args.check && args.json);
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "skills",
            "--agent",
            "claude,codex",
            "--global",
            "--json",
        ])
        .unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("skills")
        };
        assert!(args.global && args.json);
        assert_eq!(args.agents, ["claude", "codex"]);
    }

    #[cfg(feature = "feedback")]
    fn parse_feedback(argv: &[&str]) -> Result<FeedbackArgs, clap::Error> {
        let cli = Cli::try_parse_from(argv)?;
        let Commands::Feedback(args) = cli.command else {
            panic!("expected feedback command");
        };
        Ok(args)
    }

    #[cfg(feature = "feedback")]
    #[test]
    fn feedback_requires_a_message() {
        for argv in [
            vec!["clickhousectl", "feedback"],
            vec!["clickhousectl", "feedback", "--json"],
        ] {
            let error = parse_feedback(&argv).expect_err("message is required");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
            assert_eq!(error.exit_code(), 2);
        }
    }

    #[cfg(feature = "feedback")]
    #[test]
    fn feedback_collects_every_word_and_takes_flags_only_before_the_message() {
        let args =
            parse_feedback(&["clickhousectl", "feedback", "needs a", "wait", "flag"]).unwrap();
        assert_eq!(args.message, ["needs a", "wait", "flag"]);
        assert!(!args.json);

        let args = parse_feedback(&["clickhousectl", "feedback", "--json", "one", "two"]).unwrap();
        assert_eq!(args.message, ["one", "two"]);
        assert!(args.json);

        // From the first message word on, flag-looking words are message text.
        for (argv, message) in [
            (
                &["clickhousectl", "feedback", "a", "--json", "b"][..],
                &["a", "--json", "b"][..],
            ),
            (
                &[
                    "clickhousectl",
                    "feedback",
                    "service",
                    "list",
                    "ignores",
                    "--json",
                ],
                &["service", "list", "ignores", "--json"],
            ),
            (
                &[
                    "clickhousectl",
                    "feedback",
                    "the",
                    "-h",
                    "text",
                    "is",
                    "wrong",
                ],
                &["the", "-h", "text", "is", "wrong"],
            ),
            (
                &["clickhousectl", "feedback", "--wait", "is", "missing"],
                &["--wait", "is", "missing"],
            ),
            (
                &["clickhousectl", "feedback", "--", "--json", "is", "ignored"],
                &["--json", "is", "ignored"],
            ),
        ] {
            let args = parse_feedback(argv).unwrap();
            assert_eq!(args.message, message, "{argv:?}");
            assert!(!args.json, "{argv:?}");
        }
    }

    #[cfg(feature = "feedback")]
    #[test]
    fn feedback_help_before_the_message_is_still_help() {
        for argv in [
            &["clickhousectl", "feedback", "-h"][..],
            &["clickhousectl", "feedback", "--help", "the", "gap"],
            &["clickhousectl", "feedback", "--json", "-h", "the", "gap"],
        ] {
            let error = parse_feedback(argv).expect_err("help is not a message");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp,
                "{argv:?}"
            );
            assert_eq!(error.exit_code(), 0, "{argv:?}");
        }
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_json_is_available_before_and_after_each_subcommand() {
        for command in ["status", "enable", "disable"] {
            for argv in [
                vec!["clickhousectl", "telemetry", "--json", command],
                vec!["clickhousectl", "telemetry", command, "--json"],
            ] {
                let cli = Cli::try_parse_from(argv).unwrap();
                let Commands::Telemetry(args) = cli.command else {
                    panic!("telemetry")
                };
                assert!(args.json);
            }
        }
    }
}
