//! Register the hosted ClickHouse Cloud MCP server in a coding agent.
//!
//! Each agent's own CLI does the registration and the OAuth sign-in, so the
//! agent keeps its token and clickhousectl never handles it.

use super::output::eprint_line;
use super::permissions::Declaration as Permission;
use crate::error::{Error, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;
use std::process::{Command, Stdio};

pub(super) const PERMISSIONS: &[Permission] = &[Permission::non_api(
    "mcp add",
    "Runs the agent's own CLI; no Cloud API call.",
)];

/// ClickHouse Cloud's hosted MCP server.
pub const CLOUD_MCP_URL: &str = "https://mcp.clickhouse.cloud/mcp";

/// Cloud-wide flags that `mcp` commands reject, because the agent signs in to
/// the MCP server itself (see `CloudArgs::mcp_flag_conflict`).
const REJECTED_CLOUD_FLAGS: [&str; 5] = ["org_id", "org_name", "api_key", "api_secret", "url"];

/// Hide the rejected Cloud-wide flags from `cloud mcp` help. clap propagates a
/// global flag only into subcommands without an arg of the same ID, so hidden
/// copies on `mcp` replace the visible ones there and nowhere else. The copies
/// stay global, so a value given after `mcp add` still reaches `CloudArgs`.
pub(crate) fn hide_rejected_cloud_flags(root: clap::Command) -> clap::Command {
    root.mut_subcommand("cloud", |cloud| {
        let hidden: Vec<clap::Arg> = cloud
            .get_arguments()
            .filter(|arg| REJECTED_CLOUD_FLAGS.contains(&arg.get_id().as_str()))
            .map(|arg| arg.clone().hide(true))
            .collect();
        cloud.mut_subcommand("mcp", |mcp| mcp.args(hidden))
    })
}

#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// Register the Cloud MCP server in a coding agent and sign in
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Registers the read-only Cloud MCP server for the user, in all projects, using the agent's own CLI.
  Then starts that agent's OAuth sign-in, which opens a browser and blocks until a human finishes it;
    with an existing Cloud browser session it can finish in seconds.
  gemini has no sign-in command: run `/mcp auth <name>` inside Gemini CLI afterwards.
  Needs the agent's CLI on PATH, not clickhousectl credentials. Safe to rerun to sign in again.
  The server's tools appear in new agent sessions, not the one that ran this.")]
    Add(AddArgs),
}

impl McpCommands {
    pub fn is_write(&self) -> bool {
        match self {
            McpCommands::Add(_) => false,
        }
    }
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Coding agent to register the server in
    #[arg(value_enum)]
    pub agent: Agent,

    /// Server name in the agent's config
    #[arg(long, default_value = "clickhouse-cloud")]
    pub name: String,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    /// Claude Code
    Claude,
    /// OpenAI Codex CLI
    Codex,
    /// Gemini CLI
    Gemini,
    /// OpenCode
    Opencode,
    /// Pi coding agent
    Pi,
}

impl Agent {
    fn display_name(self) -> &'static str {
        match self {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Gemini => "Gemini CLI",
            Agent::Opencode => "OpenCode",
            Agent::Pi => "Pi",
        }
    }

    fn program(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Gemini => "gemini",
            Agent::Opencode => "opencode",
            Agent::Pi => "pi",
        }
    }
}

/// How an agent signs in to the server once it is registered.
#[derive(Debug, PartialEq, Eq)]
enum SignIn {
    /// A separate command, run after `add`.
    Command(Vec<String>),
    /// `add` signs in by itself when the server supports OAuth.
    DuringAdd,
    /// Only from inside an agent session, with this slash command.
    InSession(String),
}

/// The agent CLI calls that register `name` for the user and sign in.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    /// Exits 0 when `name` is already registered. Only for agents whose `add`
    /// refuses an existing name; the others replace the entry.
    exists: Option<Vec<String>>,
    add: Vec<String>,
    sign_in: SignIn,
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn plan(agent: Agent, name: &str) -> Plan {
    let url = CLOUD_MCP_URL;
    match agent {
        Agent::Claude => Plan {
            exists: Some(argv(&["mcp", "get", name])),
            add: argv(&[
                "mcp",
                "add",
                "--transport",
                "http",
                "--scope",
                "user",
                name,
                url,
            ]),
            sign_in: SignIn::Command(argv(&["mcp", "login", name])),
        },
        Agent::Codex => Plan {
            exists: None,
            add: argv(&["mcp", "add", name, "--url", url]),
            sign_in: SignIn::DuringAdd,
        },
        Agent::Gemini => Plan {
            exists: None,
            add: argv(&[
                "mcp",
                "add",
                "--scope",
                "user",
                "--transport",
                "http",
                name,
                url,
            ]),
            sign_in: SignIn::InSession(format!("/mcp auth {name}")),
        },
        Agent::Opencode => Plan {
            exists: None,
            add: argv(&["mcp", "add", name, "--url", url, "--global"]),
            sign_in: SignIn::Command(argv(&["mcp", "auth", name])),
        },
        Agent::Pi => Plan {
            exists: None,
            add: argv(&["mcp", "add", name, "--url", url]),
            sign_in: SignIn::Command(argv(&["mcp", "login", name])),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Registration {
    Added,
    AlreadyRegistered,
}

#[derive(Debug, Serialize)]
struct AddOutput {
    agent: Agent,
    name: String,
    url: &'static str,
    registration: Registration,
    signed_in: bool,
    /// What the user runs inside the agent to sign in, when the agent's CLI can't.
    #[serde(skip_serializing_if = "Option::is_none")]
    sign_in_command: Option<String>,
}

pub fn run(command: McpCommands, debug: bool, json: bool) -> Result<()> {
    match command {
        McpCommands::Add(args) => add(args, debug, json),
    }
}

fn add(args: AddArgs, debug: bool, json: bool) -> Result<()> {
    let agent = args.agent;
    let plan = plan(agent, &args.name);

    let already_registered = match &plan.exists {
        Some(exists) => spawn(agent, exists, debug, ChildOutput::Discard)?.success(),
        None => false,
    };
    let registration = if already_registered {
        Registration::AlreadyRegistered
    } else {
        check(spawn(agent, &plan.add, debug, ChildOutput::ToStderr)?)?;
        Registration::Added
    };

    let (signed_in, sign_in_command) = match plan.sign_in {
        SignIn::Command(login) => {
            eprint_line(format!(
                "Signing in to {} from {}; finish in the browser.",
                args.name,
                agent.display_name()
            ));
            check(spawn(agent, &login, debug, ChildOutput::ToStderr)?)?;
            (true, None)
        }
        SignIn::DuringAdd => (registration == Registration::Added, None),
        SignIn::InSession(command) => (false, Some(command)),
    };

    let output = AddOutput {
        agent,
        name: args.name,
        url: CLOUD_MCP_URL,
        registration,
        signed_in,
        sign_in_command,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(())
}

fn print_human(output: &AddOutput) {
    let agent = output.agent.display_name();
    match output.registration {
        Registration::Added => println!("Registered {} in {agent} for all projects.", output.name),
        Registration::AlreadyRegistered => {
            println!("{} is already registered in {agent}.", output.name)
        }
    }
    if let Some(command) = &output.sign_in_command {
        println!("To sign in, run `{command}` in {agent}.");
    } else if output.signed_in {
        println!("Signed in. Start a new {agent} session to use it.");
    }
}

#[derive(Clone, Copy)]
enum ChildOutput {
    Discard,
    /// Keep the agent's messages, sign-in URL included, off stdout so it holds
    /// only this command's own output.
    ToStderr,
}

fn spawn(
    agent: Agent,
    args: &[String],
    debug: bool,
    output: ChildOutput,
) -> Result<std::process::ExitStatus> {
    let program = agent.program();
    if debug {
        eprint_line(format!("[debug] running: {program} {}", args.join(" ")));
    }
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::inherit());
    match output {
        ChildOutput::Discard => {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        ChildOutput::ToStderr => {
            command
                .stdout(Stdio::from(std::io::stderr()))
                .stderr(Stdio::inherit());
        }
    }
    command.status().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => Error::Cloud(format!(
            "`{program}` was not found on PATH; install {} first",
            agent.display_name()
        )),
        _ => Error::Cloud(format!("failed to run `{program}`: {error}")),
    })
}

/// A failed agent CLI has already explained itself on stderr; keep its status.
fn check(status: std::process::ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(Error::ChildExit(status.code().unwrap_or(1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn parse_add(args: &[&str]) -> AddArgs {
        let cli = Cli::try_parse_from(["chctl", "cloud", "mcp", "add"].iter().chain(args)).unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Mcp {
            command: McpCommands::Add(add),
        } = cloud.command
        else {
            panic!("expected cloud mcp add");
        };
        add
    }

    #[test]
    fn add_takes_a_positional_agent_and_defaults_the_name() {
        let add = parse_add(&["claude"]);
        assert_eq!(add.agent, Agent::Claude);
        assert_eq!(add.name, "clickhouse-cloud");

        let add = parse_add(&["opencode", "--name", "ch"]);
        assert_eq!(add.agent, Agent::Opencode);
        assert_eq!(add.name, "ch");
    }

    #[test]
    fn add_requires_a_known_agent() {
        let missing = Cli::try_parse_from(["chctl", "cloud", "mcp", "add"])
            .err()
            .unwrap();
        assert_eq!(
            missing.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let unknown = Cli::try_parse_from(["chctl", "cloud", "mcp", "add", "cursor"])
            .err()
            .unwrap();
        assert_eq!(unknown.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn add_is_not_a_write() {
        let add = parse_add(&["codex"]);
        assert!(!McpCommands::Add(add).is_write());
    }

    #[test]
    fn plans_register_for_the_user_with_the_cloud_url() {
        for agent in Agent::value_variants() {
            let plan = plan(*agent, "ch");
            assert!(plan.add.iter().any(|arg| arg == CLOUD_MCP_URL), "{agent:?}");
            assert!(plan.add.iter().any(|arg| arg == "ch"), "{agent:?}");
        }
        assert!(
            plan(Agent::Claude, "ch")
                .add
                .windows(2)
                .any(|w| w == ["--scope", "user"])
        );
        assert!(
            plan(Agent::Gemini, "ch")
                .add
                .windows(2)
                .any(|w| w == ["--scope", "user"])
        );
        assert!(
            plan(Agent::Opencode, "ch")
                .add
                .contains(&"--global".to_owned())
        );
    }

    #[test]
    fn plans_sign_in_the_way_each_agent_supports() {
        assert_eq!(
            plan(Agent::Claude, "ch").sign_in,
            SignIn::Command(argv(&["mcp", "login", "ch"]))
        );
        assert_eq!(plan(Agent::Codex, "ch").sign_in, SignIn::DuringAdd);
        assert_eq!(
            plan(Agent::Gemini, "ch").sign_in,
            SignIn::InSession("/mcp auth ch".into())
        );
        assert_eq!(
            plan(Agent::Opencode, "ch").sign_in,
            SignIn::Command(argv(&["mcp", "auth", "ch"]))
        );
        assert_eq!(
            plan(Agent::Pi, "ch").sign_in,
            SignIn::Command(argv(&["mcp", "login", "ch"]))
        );
    }

    #[test]
    fn only_claude_checks_for_an_existing_entry() {
        assert_eq!(
            plan(Agent::Claude, "ch").exists,
            Some(argv(&["mcp", "get", "ch"]))
        );
        for agent in [Agent::Codex, Agent::Gemini, Agent::Opencode, Agent::Pi] {
            assert_eq!(plan(agent, "ch").exists, None, "{agent:?}");
        }
    }
}
