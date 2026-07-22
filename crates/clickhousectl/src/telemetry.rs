//! Anonymous usage telemetry (issue #283).
//!
//! Consent model follows Homebrew: nothing is ever sent before the first-run
//! notice has been shown — unless the user explicitly runs
//! `clickhousectl telemetry enable`, which is itself consent (stronger than
//! passively seeing a notice) and skips the notice. State lives in
//! `~/.clickhouse/telemetry.json`
//! (`{"disabled": false}`), which doubles as the first-run marker — the notice
//! is only printed after the file has been written successfully, so an
//! unwritable config dir fails open to disabled (no send, no error, no
//! repeated notice). `DO_NOT_TRACK` (donottrack.sh convention) overrides
//! everything: no notice, no file write, no send.
//!
//! The payload carries the command path and flag *names* only — never flag
//! values, never positional arguments. It is built from the clap definitions
//! ([`capture`] walks `ArgMatches` ids and `Arg` metadata, never touching
//! `get_one`/`get_raw`), so leaking a value is structurally impossible.
//!
//! Transport is a detached child process (`clickhousectl telemetry send`,
//! hidden): the parent spawns it with all stdio nulled and never waits, so
//! command latency is unaffected even when the endpoint is unreachable. The
//! child fires one POST with a short timeout and dies silently on any failure.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};
use crate::paths;

/// Public documentation for what is collected and how to opt out.
const DOCS_URL: &str = "https://clickhouse.com/docs/interfaces/cli#telemetry";

/// Production ingest endpoint (Cloudflare worker in front of ClickHouse Cloud).
const DEFAULT_ENDPOINT: &str = "https://chctl.clickhouse.com/v1/telemetry";

/// Overrides the ingest endpoint (integration tests, local worker dev).
const URL_ENV: &str = "CHCTL_TELEMETRY_URL";
/// Carries the serialized payload from the parent to the hidden send child.
const PAYLOAD_ENV: &str = "CHCTL_TELEMETRY_PAYLOAD";
/// When truthy: print the exact payload to stderr and send nothing.
const DEBUG_ENV: &str = "CHCTL_TELEMETRY_DEBUG";
/// donottrack.sh convention: when truthy, telemetry is fully silent.
const DNT_ENV: &str = "DO_NOT_TRACK";
/// Standard CI marker, sent as a boolean so pipelines can be filtered out.
const CI_ENV: &str = "CI";

const SEND_TIMEOUT: Duration = Duration::from_secs(2);

/// The ingest worker caps `flags` at 64 entries; truncate client-side too.
const MAX_FLAGS: usize = 64;

// ---------------------------------------------------------------------------
// Consent state
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
struct StateFile {
    disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// No `telemetry.json` yet: the first-run notice has not been shown.
    Missing,
    Enabled,
    Disabled,
}

/// `~/.clickhouse/telemetry.json`. `None` when the home directory cannot be
/// determined, in which case telemetry is silently off.
fn state_path() -> Option<PathBuf> {
    paths::base_dir().ok().map(|dir| dir.join("telemetry.json"))
}

/// A corrupt or unreadable state file counts as `Disabled`, not `Missing`:
/// the notice was shown once already, and when in doubt we don't send.
fn load_state_from(path: &Path) -> State {
    match std::fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<StateFile>(&contents) {
            Ok(state) if !state.disabled => State::Enabled,
            _ => State::Disabled,
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => State::Missing,
        Err(_) => State::Disabled,
    }
}

fn save_state_to(path: &Path, disabled: bool) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string(&StateFile { disabled })
        .expect("StateFile serialization cannot fail");
    std::fs::write(path, json)
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

/// Lookup function for reading process environment variables. Production
/// callers pass a wrapper around `std::env::var`; tests pass a closure over a
/// synthetic map (edition 2024 makes `set_var` unsafe, so tests never mutate
/// the real environment).
type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

fn real_env_lookup(key: &str) -> Option<String> {
    // `var_os` + lossy conversion, not `var`: `std::env::var` returns an error
    // (read here as `None`) for a variable that is set but not valid UTF-8, so
    // a non-UTF-8 `DO_NOT_TRACK` would look absent and telemetry would fail
    // open. A set-but-non-UTF-8 value must still count as an opt-out; the
    // lossy string is non-empty and not "0"/"false", so `env_truthy` sees it
    // as set.
    std::env::var_os(key).map(|v| v.to_string_lossy().into_owned())
}

/// donottrack.sh-style truthiness: set, and not `""`/`"0"`/`"false"`.
fn env_truthy(value: Option<String>) -> bool {
    matches!(value.as_deref(), Some(v) if !v.is_empty() && v != "0" && v != "false")
}

// ---------------------------------------------------------------------------
// Payload
// ---------------------------------------------------------------------------

/// The wire payload. Field names match the ingest worker's contract exactly
/// (the worker renames `ci`→`is_ci` server-side).
///
/// Agent and version facts are set here, client-side, rather than derived in
/// the worker from the User-Agent header: the CLI already holds both as
/// structured values (`is_ai_agent::detect()`, `CARGO_PKG_VERSION`), so
/// putting them in the payload avoids brittle string extraction on the
/// ingest side. The User-Agent still carries the same facts as transport
/// metadata for prefix-based request filtering.
#[derive(Debug, serde::Serialize)]
struct Payload {
    command: String,
    flags: Vec<String>,
    /// gh-style exit code (`Error::exit_code`): 0 success, 1 error,
    /// 2 cancelled, 4 auth required.
    exit_code: i32,
    is_agent: bool,
    /// Canonical id of the detected coding agent (e.g. "claude-code");
    /// `null` for human invocations.
    agent: Option<String>,
    ci: bool,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
}

fn build_payload(invocation: &Invocation, exit_code: i32, env: EnvLookup<'_>) -> Payload {
    let mut flags = invocation.flags.clone();
    flags.truncate(MAX_FLAGS);
    let detected = is_ai_agent::detect();
    Payload {
        command: invocation.command.clone(),
        flags,
        exit_code,
        is_agent: detected.is_some(),
        agent: detected.map(|a| a.id.as_str().to_string()),
        ci: env_truthy(env(CI_ENV)),
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

// ---------------------------------------------------------------------------
// Invocation capture
// ---------------------------------------------------------------------------

/// What the user invoked: the subcommand path (e.g. `"local start"`) and the
/// long names of the flags they passed. No values, no positionals.
pub struct Invocation {
    command: String,
    flags: Vec<String>,
}

/// Derive the command path and passed-flag names from the parsed matches.
///
/// Only ids and `Arg` metadata are consulted — never `get_one`/`get_raw`/
/// `get_many` — so argument *values* are structurally unreachable here.
/// Positionals are skipped entirely (their names could still describe user
/// data), default-valued and env-fed args are excluded by the
/// `ValueSource::CommandLine` filter, and clap's propagation of global flags
/// into subcommand matches is deduplicated by the set.
pub fn capture(root: &clap::Command, matches: &clap::ArgMatches) -> Invocation {
    use clap::parser::ValueSource;

    let mut path: Vec<&str> = Vec::new();
    // Ancestor commands, innermost last: global args propagate into
    // subcommand matches but their `Arg` definition lives on an ancestor.
    let mut stack: Vec<&clap::Command> = vec![root];
    let mut flags = std::collections::BTreeSet::new();
    let mut current = matches;
    loop {
        for id in current.ids() {
            // Global args are propagated upward into ancestor matches whose
            // command doesn't define them; `value_source` would panic on such
            // an id, so skip it here — it is captured again at the level that
            // does define it (globals are propagated downward at build time).
            if !matches!(current.try_contains_id(id.as_str()), Ok(true)) {
                continue;
            }
            if current.value_source(id.as_str()) != Some(ValueSource::CommandLine) {
                continue;
            }
            // Resolve the id to its definition; unresolvable ids (groups) are
            // skipped rather than reported.
            let Some(arg) = stack
                .iter()
                .rev()
                .find_map(|cmd| cmd.get_arguments().find(|a| a.get_id() == id))
            else {
                continue;
            };
            if arg.is_positional() {
                continue;
            }
            flags.insert(arg.get_long().unwrap_or(id.as_str()).to_string());
        }
        let Some((name, sub_matches)) = current.subcommand() else {
            break;
        };
        let Some(sub_cmd) = stack
            .last()
            .expect("stack starts non-empty and only grows")
            .find_subcommand(name)
        else {
            break;
        };
        path.push(sub_cmd.get_name());
        stack.push(sub_cmd);
        current = sub_matches;
    }
    Invocation {
        command: path.join(" "),
        flags: flags.into_iter().collect(),
    }
}

// ---------------------------------------------------------------------------
// Finalize (the per-invocation hook)
// ---------------------------------------------------------------------------

/// What `finalize` should do for this invocation. Split from the side effects
/// so the state machine is unit-testable with injected env and paths.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// DO_NOT_TRACK, disabled, unwritable config dir: do nothing at all.
    Silent,
    /// First run, marker written successfully: show the notice, send nothing.
    Notice,
    /// Enabled: hand the serialized payload to the detached send child.
    Send(String),
    /// Enabled + debug: print the payload to stderr, send nothing.
    Debug(String),
}

fn decide(path: &Path, invocation: &Invocation, exit_code: i32, env: EnvLookup<'_>) -> Action {
    if env_truthy(env(DNT_ENV)) {
        return Action::Silent;
    }
    match load_state_from(path) {
        State::Missing => {
            // Write first, notice only on success: if the dir is unwritable
            // we stay silent forever rather than nagging or erroring, and we
            // never send without having recorded that the notice was shown.
            if save_state_to(path, false).is_ok() {
                Action::Notice
            } else {
                Action::Silent
            }
        }
        State::Disabled => Action::Silent,
        State::Enabled => {
            let json = serde_json::to_string(&build_payload(invocation, exit_code, env))
                .expect("Payload serialization cannot fail");
            if env_truthy(env(DEBUG_ENV)) {
                Action::Debug(json)
            } else {
                Action::Send(json)
            }
        }
    }
}

/// The telemetry hook, called once at the very end of `main` (after the
/// command has run, so `telemetry disable` silences its own event), with the
/// gh-style exit code the process is about to exit with. Never errors, never
/// blocks beyond spawning a detached child.
pub fn finalize(invocation: Invocation, exit_code: i32) {
    let Some(path) = state_path() else { return };
    match decide(&path, &invocation, exit_code, &real_env_lookup) {
        Action::Silent => {}
        Action::Notice => print_first_run_notice(),
        Action::Debug(json) => eprintln!("{json}"),
        Action::Send(json) => spawn_send_child(&json),
    }
}

/// Printed to stderr regardless of TTY so agent/non-interactive usage still
/// sees it exactly once (stdout stays machine-parseable).
fn print_first_run_notice() {
    eprintln!(
        "\nNote: clickhousectl collects anonymous usage data to help improve the CLI:\n\
         command name, flag names (never values or arguments), success/failure, version,\n\
         OS/arch, and CI/agent detection. No user or machine IDs. Nothing was sent this run.\n\
         Opt out: `clickhousectl telemetry disable` or DO_NOT_TRACK=1.\n\
         Details: {DOCS_URL}"
    );
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// Re-invoke this binary as `clickhousectl telemetry send` with the payload
/// in the child's environment, all stdio nulled, and never wait: the parent
/// exits immediately and the child dies silently on any failure.
fn spawn_send_child(payload_json: &str) {
    use std::process::{Command, Stdio};

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .args(["telemetry", "send"])
        .env(PAYLOAD_ENV, payload_json)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// The hidden send child's entire job: one POST, short timeout, ignore every
/// failure. Short-circuited in `main` before the update-cache refresh and the
/// telemetry hook, so a send can never trigger another send.
pub async fn run_child_send() {
    let Ok(payload) = std::env::var(PAYLOAD_ENV) else {
        // Invoked without a payload (directly by a user): do nothing.
        return;
    };
    let url = std::env::var(URL_ENV).unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
    // Deliberately not `crate::http::client_builder()`: the shared builder
    // attaches the agent session/trace correlation headers (`agent-session-id`,
    // `traceparent`), which would let the backend correlate telemetry events
    // with an agent session — telemetry is anonymous, and the agent facts it
    // needs already travel in the payload (`is_agent`/`agent`) by design. Only
    // the canonical User-Agent is kept (the ingest worker filters on its
    // `clickhousectl/<version>` prefix).
    let Ok(client) = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .timeout(SEND_TIMEOUT)
        .build()
    else {
        return;
    };
    let _ = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload)
        .send()
        .await;
}

// ---------------------------------------------------------------------------
// `clickhousectl telemetry` subcommand
// ---------------------------------------------------------------------------

pub fn run_command(cmd: crate::cli::TelemetryCommands) -> Result<()> {
    use crate::cli::TelemetryCommands;

    match cmd {
        TelemetryCommands::Enable => {
            set_disabled(false)?;
            println!("Telemetry enabled.");
            // The preference is recorded either way, but DNT overrides it
            // (see `decide`): without this note the user would see success
            // while telemetry stays fully silent.
            if env_truthy(real_env_lookup(DNT_ENV)) {
                eprintln!(
                    "Note: the DO_NOT_TRACK environment variable is set; telemetry will remain silent while it is set."
                );
            }
            Ok(())
        }
        TelemetryCommands::Disable => {
            set_disabled(true)?;
            println!("Telemetry disabled.");
            Ok(())
        }
        TelemetryCommands::Status => {
            print_status();
            Ok(())
        }
        TelemetryCommands::Send => unreachable!("handled before dispatch in main"),
    }
}

fn set_disabled(disabled: bool) -> Result<()> {
    let path = state_path().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;
    save_state_to(&path, disabled).map_err(Error::Io)
}

fn print_status() {
    if env_truthy(real_env_lookup(DNT_ENV)) {
        println!("Telemetry is disabled (DO_NOT_TRACK environment variable is set).");
        return;
    }
    let Some(path) = state_path() else {
        println!("Telemetry is disabled (could not determine home directory).");
        return;
    };
    match load_state_from(&path) {
        State::Missing => {
            println!("Telemetry is not yet configured; nothing has been sent.");
        }
        State::Disabled => {
            println!("Telemetry is disabled ({}).", path.display());
        }
        State::Enabled => {
            println!(
                "Telemetry is enabled. Disable with `clickhousectl telemetry disable` or DO_NOT_TRACK=1.\nDetails: {DOCS_URL}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Env lookup over a synthetic map; `set_var` is unsafe in edition 2024.
    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: std::collections::HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    fn invocation() -> Invocation {
        Invocation {
            command: "local list".into(),
            flags: vec!["json".into()],
        }
    }

    #[test]
    fn env_truthy_truth_table() {
        assert!(!env_truthy(None));
        assert!(!env_truthy(Some("".into())));
        assert!(!env_truthy(Some("0".into())));
        assert!(!env_truthy(Some("false".into())));
        assert!(env_truthy(Some("1".into())));
        assert!(env_truthy(Some("true".into())));
        assert!(env_truthy(Some("anything".into())));
    }

    #[test]
    fn do_not_track_wins_over_everything() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        // Even with an enabled state file present, DNT is fully silent.
        save_state_to(&path, false).unwrap();
        let env = env_of(&[("DO_NOT_TRACK", "1")]);
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Silent);
    }

    #[test]
    fn do_not_track_prevents_first_run_marker() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        let env = env_of(&[("DO_NOT_TRACK", "1")]);
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Silent);
        assert!(!path.exists(), "DNT must not write the marker file");
    }

    #[test]
    fn first_run_writes_marker_and_notices_without_sending() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Notice);
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, r#"{"disabled":false}"#);
    }

    #[test]
    fn unwritable_dir_fails_open_to_silent() {
        // Parent path is a file, so create_dir_all fails.
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let path = blocker.join("telemetry.json");
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Silent);
        // And again: still silent, never a notice, never a send.
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Silent);
    }

    #[test]
    fn disabled_state_is_silent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, true).unwrap();
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Silent);
    }

    #[test]
    fn corrupt_state_file_is_treated_as_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        std::fs::write(&path, "not json{{").unwrap();
        assert_eq!(load_state_from(&path), State::Disabled);
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env), Action::Silent);
    }

    #[test]
    fn enabled_state_sends_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let env = env_of(&[("CI", "1")]);
        let Action::Send(json) = decide(&path, &invocation(), 4, &env) else {
            panic!("expected Send");
        };
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], "local list");
        assert_eq!(value["flags"], serde_json::json!(["json"]));
        assert_eq!(value["exit_code"], 4);
        assert_eq!(value["ci"], true);
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["os"], std::env::consts::OS);
        assert_eq!(value["arch"], std::env::consts::ARCH);
    }

    #[test]
    fn debug_env_prints_instead_of_sending() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let env = env_of(&[("CHCTL_TELEMETRY_DEBUG", "1")]);
        assert!(matches!(
            decide(&path, &invocation(), 0, &env),
            Action::Debug(_)
        ));
    }

    #[test]
    fn payload_serializes_exactly_the_wire_fields() {
        let payload = build_payload(&invocation(), 0, &env_of(&[]));
        let value = serde_json::to_value(&payload).unwrap();
        let keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "command",
                "flags",
                "exit_code",
                "is_agent",
                "agent",
                "ci",
                "version",
                "os",
                "arch"
            ]
        );
        // The two agent fields are set from the same single detection and can
        // never disagree.
        assert_eq!(
            value["is_agent"].as_bool().unwrap(),
            !value["agent"].is_null()
        );
    }

    #[test]
    fn flags_truncated_to_worker_cap() {
        let inv = Invocation {
            command: "x".into(),
            flags: (0..100).map(|i| format!("flag-{i}")).collect(),
        };
        let payload = build_payload(&inv, 0, &env_of(&[]));
        assert_eq!(payload.flags.len(), MAX_FLAGS);
    }

    // -- capture: values are structurally unreachable ------------------------

    fn capture_from(args: &[&str]) -> Invocation {
        let mut cmd = crate::cli::Cli::command();
        let matches = cmd.try_get_matches_from_mut(args).unwrap();
        capture(&cmd, &matches)
    }

    #[test]
    fn capture_reports_names_only_never_values_or_positionals() {
        let inv = capture_from(&[
            "clickhousectl",
            "cloud",
            "--json",
            "service",
            "get",
            "SECRET-SERVICE-ID",
            "--org-id",
            "SECRET-ORG",
        ]);
        assert_eq!(inv.command, "cloud service get");
        assert_eq!(inv.flags, ["json", "org-id"]);
        let json = serde_json::to_string(&build_payload(&inv, 0, &env_of(&[]))).unwrap();
        assert!(!json.contains("SECRET"), "payload leaked a value: {json}");
    }

    #[test]
    fn capture_dedupes_propagated_global_flags() {
        let inv = capture_from(&["clickhousectl", "cloud", "--json", "service", "list"]);
        assert_eq!(inv.command, "cloud service list");
        assert_eq!(inv.flags, ["json"]);
    }

    #[test]
    fn capture_excludes_default_valued_args() {
        use clap::{Arg, ArgAction, Command};
        let mut cmd = Command::new("root").subcommand(
            Command::new("sub")
                .arg(Arg::new("level").long("level").default_value("info"))
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("target")),
        );
        let matches = cmd
            .try_get_matches_from_mut(["root", "sub", "--verbose", "user-data"])
            .unwrap();
        let inv = capture(&cmd, &matches);
        assert_eq!(inv.command, "sub");
        // `level` has a default (ValueSource::DefaultValue) and `target` is
        // positional — only the explicitly passed named flag is reported.
        assert_eq!(inv.flags, ["verbose"]);
    }

    #[test]
    fn capture_with_no_flags_is_empty() {
        let inv = capture_from(&["clickhousectl", "local", "list"]);
        assert_eq!(inv.command, "local list");
        assert!(inv.flags.is_empty());
    }

    #[test]
    fn state_path_is_telemetry_json_under_base_dir() {
        let path = state_path().unwrap();
        assert!(path.ends_with(".clickhouse/telemetry.json"));
    }
}
