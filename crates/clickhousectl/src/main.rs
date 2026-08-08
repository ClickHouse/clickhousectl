mod cli;
mod cloud;
mod dotenv;
mod error;
mod http;
mod init;
mod local;
mod paths;
mod skills;
#[cfg(feature = "telemetry")]
mod telemetry;
mod update;
mod user_agent;
mod version_manager;

use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, Commands, SkillsArgs, UpdateArgs};

use error::{Error, Result};

#[tokio::main]
async fn main() {
    // Snapshot any project-local `.env` before anything else so credential
    // resolution can use it. Safe to call here even though tokio has worker
    // threads — we populate an in-process `OnceLock` rather than touching
    // libc's environ.
    dotenv::init();

    // Snapshot the executable path before the command runs: a successful
    // `clickhousectl update` replaces the binary on disk, after which a lazy
    // `current_exe()` lookup fails on Linux and the update's own telemetry
    // event would be dropped.
    #[cfg(feature = "telemetry")]
    telemetry::init();

    // Parse via ArgMatches (rather than `Cli::try_parse()`) so the telemetry
    // capture below can read the command path and passed-flag *names* from the
    // clap definitions — argument values are never consulted. Argv is
    // collected once so a failed parse can be re-walked by `capture_lossy`.
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let mut cmd = Cli::command();

    // Single-exit invariant (#320): every invocation — bare, help, version,
    // typo, dispatched command — falls through to the common tail below, so
    // the first-run telemetry notice and subsequent events cover all of them.
    // The sole intended exemption is the hidden `telemetry send` child inside
    // `run_parsed`; the `exec()` handoffs (`local client`, host psql) record
    // their event via `telemetry::finalize_before_exec` just before the
    // process image is replaced. Child-process exit codes are returned as
    // `Error::ChildExit` so they also flow through this tail. Do not add exit
    // paths.
    let (exit_code, telemetry_invocation) = match cmd.try_get_matches_from_mut(argv.iter()) {
        Ok(matches) => {
            #[cfg(feature = "telemetry")]
            let mut invocation = telemetry::capture(&cmd, &matches);
            // Stashed so the pre-exec hook can reach it from inside a
            // handler when `exec()` makes the tail below unreachable.
            #[cfg(feature = "telemetry")]
            telemetry::stash_invocation(invocation.clone());
            #[cfg(not(feature = "telemetry"))]
            let invocation = ();
            // The matches were produced by this very `cmd`, so a mismatch is
            // a clap derive bug, not a user error.
            let cli = Cli::from_arg_matches(&matches)
                .expect("Cli::from_arg_matches must accept matches from Cli::command()");
            let (exit_code, is_child_exit) = run_parsed(cli).await;
            #[cfg(feature = "telemetry")]
            if is_child_exit {
                invocation.mark_child_exit();
            }
            #[cfg(not(feature = "telemetry"))]
            let _ = is_child_exit;
            (exit_code, invocation)
        }
        Err(e) => {
            // clap keeps its own formatting and colors; help/version print to
            // stdout, usage errors to stderr. Print failures are swallowed
            // like clap's own `Error::exit` swallows them: a broken pipe must
            // not turn exit 2 into a panic (which would also bypass the
            // telemetry tail below).
            let _ = e.print();
            match e.kind() {
                // --version always hits the network to refresh the cache + timer,
                // then prints the notice from the freshly-updated cache.
                ErrorKind::DisplayVersion => {
                    update::force_refresh_update_cache().await;
                    update::print_cached_update_notice();
                }
                // --help shows the notice from cache (no blocking network call).
                ErrorKind::DisplayHelp => update::print_cached_update_notice(),
                // Usage errors do no update-cache work: a mistyped invocation
                // must not cause network activity beyond the consented
                // telemetry send.
                _ => {}
            }
            #[cfg(feature = "telemetry")]
            let invocation = telemetry::capture_lossy(&mut cmd, &argv, &e);
            #[cfg(not(feature = "telemetry"))]
            let invocation = ();
            // clap's own exit codes: 0 for help/version, 2 for usage errors.
            // Dispatched commands reserve 3 for cancellation, so 2 remains
            // unambiguous to shell callers.
            (e.exit_code(), invocation)
        }
    };

    // Consent is evaluated here, after the command ran, so `telemetry disable`
    // silences its own event and `telemetry enable` sends one.
    #[cfg(feature = "telemetry")]
    telemetry::finalize(telemetry_invocation, exit_code);
    #[cfg(not(feature = "telemetry"))]
    let () = telemetry_invocation;

    std::process::exit(exit_code);
}

/// Run a successfully parsed invocation to completion and report the exit
/// code for `main`'s single exit plus whether it came from a child process.
/// The hidden `telemetry send` child is the one deliberate early exit in the
/// binary: it does exactly one POST — no update-cache refresh, no dispatch,
/// and no telemetry hook of its own, so a send can never trigger another send.
async fn run_parsed(cli: Cli) -> (i32, bool) {
    #[cfg(feature = "telemetry")]
    if matches!(
        cli.command,
        Commands::Telemetry(cli::TelemetryArgs {
            command: cli::TelemetryCommands::Send
        })
    ) {
        telemetry::run_child_send().await;
        std::process::exit(0);
    }

    // Spawn a background task to refresh the update cache for non-update
    // commands. The refresh is gated to one network call per 24h; the notice
    // below is driven off whatever the cache currently holds.
    let is_update_cmd = matches!(cli.command, Commands::Update(_));
    let cache_refresh = if !is_update_cmd {
        Some(tokio::spawn(update::refresh_update_cache()))
    } else {
        None
    };

    // Decide whether to surface the update notice before `run` consumes the
    // command. Shown on every command that does not emit machine-readable JSON.
    let show_notice = should_show_update_notice(&cli.command);

    let result = run(cli.command).await;

    // Give the cache refresh a brief window to finish so short-lived commands
    // don't always drop it before the write completes. The background HTTP
    // request itself has a 400ms timeout, so 500ms here is enough headroom.
    if let Some(handle) = cache_refresh {
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    }

    let (exit_code, is_child_exit) = match result {
        Ok(()) => (0, false),
        Err(e) => {
            let is_child_exit = matches!(&e, Error::ChildExit(_));
            if !is_child_exit {
                use std::io::Write;
                // Not `eprintln!`, which panics on a closed stderr — see
                // `telemetry::print_first_run_notice`.
                let _ = writeln!(std::io::stderr(), "Error: {}", e);
            }
            (e.exit_code(), is_child_exit)
        }
    };

    // Always print the notice at the very end, after the command's own output
    // (stdout) and any error message.
    if show_notice {
        update::print_cached_update_notice();
    }

    (exit_code, is_child_exit)
}

/// The explicit `--json` flag for a command, or `None` for commands that never
/// surface the update notice (the `update` command itself). `Skills` has no
/// `--json` flag, so it reports `false`. Kept separate from agent detection so
/// the mapping is deterministic and unit-testable regardless of environment.
fn command_json_flag(cmd: &Commands) -> Option<bool> {
    match cmd {
        Commands::Update(_) => None,
        Commands::Local(args) => Some(args.json),
        Commands::Cloud(args) => Some(args.json),
        Commands::Skills(_) => Some(false),
        #[cfg(feature = "telemetry")]
        Commands::Telemetry(_) => Some(false),
    }
}

/// Whether to surface the cached update notice for this invocation. Shown for
/// every command that does not emit machine-readable JSON (`--json` or a
/// detected coding agent both suppress it), except the `update` command itself.
fn should_show_update_notice(cmd: &Commands) -> bool {
    match command_json_flag(cmd) {
        None => false,
        Some(flag) => !json_output(flag),
    }
}

/// Resolve whether to emit machine-readable JSON. True when `--json` was passed
/// or we're running under a known coding agent (same detection as the outbound
/// User-Agent in `user_agent.rs`). Pipes/redirects stay human-readable unless
/// `--json` is passed, matching `gh`/`kubectl` norms.
fn json_output(flag: bool) -> bool {
    flag || is_ai_agent::detect().is_some()
}

async fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Local(args) => local::run(args.command, json_output(args.json)).await,
        Commands::Skills(args) => run_skills(args).await,
        Commands::Cloud(args) => {
            let json = json_output(args.json);
            cloud::run(*args, json).await
        }
        Commands::Update(args) => run_update(args).await,
        #[cfg(feature = "telemetry")]
        Commands::Telemetry(args) => telemetry::run_command(args.command),
    }
}

async fn run_update(args: UpdateArgs) -> Result<()> {
    if args.check {
        match update::check_for_update().await? {
            Some((current, latest)) => {
                println!("Update available: v{} → v{}", current, latest);
                println!("Run `clickhousectl update` to upgrade.");
            }
            None => {
                println!("Already up to date (v{}).", env!("CARGO_PKG_VERSION"));
            }
        }
        Ok(())
    } else {
        update::perform_update().await
    }
}

async fn run_skills(args: SkillsArgs) -> Result<()> {
    skills::install(args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn json_output_true_when_flag_set() {
        assert!(json_output(true));
    }

    fn parse(args: &[&str]) -> Commands {
        Cli::try_parse_from(args).unwrap().command
    }

    #[test]
    fn command_json_flag_tracks_each_command() {
        // Human-readable commands report an explicit `false` flag.
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "cloud", "service", "list"])),
            Some(false)
        );
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "local", "list"])),
            Some(false)
        );
        // --json is picked up on both cloud and local (global flag).
        assert_eq!(
            command_json_flag(&parse(&[
                "clickhousectl",
                "cloud",
                "--json",
                "service",
                "list"
            ])),
            Some(true)
        );
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "local", "--json", "list"])),
            Some(true)
        );
        // Skills has no --json flag, so it always reports `false`.
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "skills"])),
            Some(false)
        );
        // The update command never surfaces the notice.
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "update"])),
            None
        );
        // Telemetry management commands are human-readable output.
        #[cfg(feature = "telemetry")]
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "telemetry", "status"])),
            Some(false)
        );
    }

    #[test]
    fn update_notice_suppressed_for_json_and_update() {
        // --json suppresses the notice so machine output stays clean,
        // regardless of agent detection.
        assert!(!should_show_update_notice(&parse(&[
            "clickhousectl",
            "cloud",
            "--json",
            "service",
            "list"
        ])));
        assert!(!should_show_update_notice(&parse(&[
            "clickhousectl",
            "local",
            "--json",
            "list"
        ])));
        // The update command never nags about itself.
        assert!(!should_show_update_notice(&parse(&[
            "clickhousectl",
            "update"
        ])));
    }
}
