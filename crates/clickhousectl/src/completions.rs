//! Dynamic shell completion (#1155).
//!
//! `completions <shell>` prints a small hook. On each Tab press the hook runs
//! the binary again with [`COMPLETE_ENV`] set, and [`complete_if_requested`],
//! the first statement of `main`, answers from the clap tree and exits. That
//! path runs before `.env` loading and telemetry, so a Tab press records no
//! telemetry event, checks for no update and touches no network. The value
//! completers below only read local state and never create it.

use std::io::Write;

use clap::CommandFactory;
use clap_complete::CompletionCandidate;
use clap_complete::env::{Bash, EnvCompleter, Fish, Zsh};

use crate::cli::{Cli, CompletionShell, CompletionsArgs};
use crate::error::Result;
use crate::local::server::Engine;

/// Set by the hook on each Tab press to the shell asking for candidates.
const COMPLETE_ENV: &str = "CHCTL_COMPLETE";

/// Every name the CLI is installed under: the binary and its `chctl` alias.
const BIN_NAMES: [&str; 2] = ["clickhousectl", "chctl"];

/// Answer a completion request and exit when [`COMPLETE_ENV`] is set; return
/// otherwise. Call before anything else in `main`, while the process is still
/// single-threaded: clap_complete removes the variable from the environment.
pub fn complete_if_requested() {
    clap_complete::CompleteEnv::with_factory(Cli::command)
        .var(COMPLETE_ENV)
        .complete();
}

pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut hook = Vec::new();
    write_hook(args.shell, &mut hook)?;
    crate::stdout::stdout().write_all(&hook)?;
    Ok(())
}

/// The hook for `shell`, registered once per installed name. Each name calls
/// itself through `PATH`, so the hook keeps working after `update` and does
/// not pin a path that a package manager may move.
fn write_hook(shell: CompletionShell, out: &mut dyn Write) -> std::io::Result<()> {
    let shell: &dyn EnvCompleter = match shell {
        CompletionShell::Bash => &Bash,
        CompletionShell::Zsh => &Zsh,
        CompletionShell::Fish => &Fish,
    };
    for name in BIN_NAMES {
        shell.write_registration(COMPLETE_ENV, name, name, name, out)?;
    }
    Ok(())
}

/// Installed ClickHouse versions, for `local use` and `local remove`.
pub fn installed_versions() -> Vec<CompletionCandidate> {
    crate::version_manager::list_installed_versions()
        .unwrap_or_default()
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

/// This project's local ClickHouse servers.
pub fn clickhouse_servers() -> Vec<CompletionCandidate> {
    server_candidates(Engine::Clickhouse)
}

/// This project's local Postgres instances.
pub fn postgres_servers() -> Vec<CompletionCandidate> {
    server_candidates(Engine::Postgres)
}

fn server_candidates(engine: Engine) -> Vec<CompletionCandidate> {
    crate::local::server::completion_server_names(engine)
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hook_registers_every_installed_name_through_the_namespaced_variable() {
        for shell in [
            CompletionShell::Bash,
            CompletionShell::Zsh,
            CompletionShell::Fish,
        ] {
            let mut hook = Vec::new();
            write_hook(shell, &mut hook).unwrap();
            let hook = String::from_utf8(hook).unwrap();
            assert!(hook.contains(COMPLETE_ENV), "{shell:?}");
            for name in BIN_NAMES {
                assert!(
                    hook.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .any(|word| word == name),
                    "{shell:?} hook does not register {name}"
                );
            }
        }
    }
}
