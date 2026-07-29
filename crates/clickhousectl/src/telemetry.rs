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
//! Every invocation of the binary counts (#320): bare, `--help`,
//! `--version`, and mistyped commands all show the first-run notice and
//! produce events under the same consent state machine — failed invocations
//! are exactly the signal that shows where the CLI confuses people and
//! agents. A successful parse is captured exactly from `ArgMatches`; a
//! failed parse has none, so [`capture_lossy`] re-walks argv against the
//! clap definitions and records the longest *valid* prefix, the error kind,
//! and clap's own suggestion — the unmatched token itself never leaves the
//! machine.
//!
//! Two commands hand the process over to another program via `exec()`
//! (`local client`, host `psql`), replacing the process image so `main`'s
//! tail never runs on success. Those call sites invoke
//! [`finalize_before_exec`] immediately before `exec()`; it records the
//! stashed invocation with outcome `"exec"` and shares an exactly-once
//! guard with [`finalize`] so a *failed* `exec()` — where the error
//! propagates back to `main` — never produces a second event.
//!
//! Transport is a detached child process (`clickhousectl telemetry send`,
//! hidden): the parent spawns it with all stdio nulled and never waits, so
//! command latency is unaffected even when the endpoint is unreachable. The
//! child fires one POST with a short timeout and dies silently on any failure.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// The executable path, snapshotted at process startup by [`init`].
///
/// Resolved eagerly rather than inside `spawn_send_child` because a successful
/// `clickhousectl update` atomically replaces the binary on disk before the
/// send child is spawned. On Linux, `/proc/self/exe` then resolves to
/// `.../clickhousectl (deleted)`, the spawn fails, and the event for the
/// update itself is silently dropped. At startup the path is always clean;
/// after an update it names the new binary, which is valid to spawn (the
/// hidden `telemetry send` interface is stable across versions — see the pin
/// on `TelemetryCommands::Send` in `cli.rs`).
static EXE_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Snapshot process-wide facts needed at finalize time. Called once at the
/// top of `main`, same eager pattern as `dotenv::init`. If `current_exe()`
/// fails the lock stays empty and the send is skipped — the same failure mode
/// as resolving lazily.
pub fn init() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = EXE_PATH.set(exe);
    }
}

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
    /// Exit code: gh-style (`Error::exit_code`) for dispatched commands —
    /// 0 success, 1 error, 2 cancelled, 4 auth required — and clap's own
    /// code for parse outcomes (0 help/version, 2 usage error). The numeric
    /// clash between "cancelled" and "usage error" is disambiguated by
    /// `outcome` (see #319 for the shell-visible fix).
    exit_code: i32,
    /// How the invocation ended, from the closed vocabulary `"ok"` (parsed
    /// and dispatched), `"exec"` (parsed, dispatched, and the process image
    /// was replaced by `exec()` — the handed-over program's exit status is
    /// unknowable, so `exit_code` is a fixed 0 and not meaningful), or a
    /// direct mapping of clap's `ErrorKind` (`"help"`, `"version"`,
    /// `"invalid_subcommand"`, …). Literal strings only — this field can
    /// never carry user data.
    outcome: &'static str,
    /// Clap's "did you mean" for failed parses. Computed by clap from the
    /// definition set, so it names a defined subcommand or flag — never the
    /// user's input. `null` when clap made no suggestion.
    suggestion: Option<String>,
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
        outcome: invocation.outcome,
        suggestion: invocation.suggestion.clone(),
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
#[derive(Clone)]
pub struct Invocation {
    command: String,
    flags: Vec<String>,
    /// See [`Payload::outcome`]: `"ok"` from [`capture`], an error-kind
    /// mapping from [`capture_lossy`].
    outcome: &'static str,
    /// See [`Payload::suggestion`]: always `None` from [`capture`].
    suggestion: Option<String>,
}

/// Map clap's parse-error kind to the closed outcome vocabulary. Every value
/// is a literal owned by this match; the wildcard covers the remaining (and
/// future — `ErrorKind` is non-exhaustive) kinds.
fn outcome_for_error(kind: clap::error::ErrorKind) -> &'static str {
    use clap::error::ErrorKind as K;
    match kind {
        K::DisplayHelp => "help",
        K::DisplayVersion => "version",
        K::InvalidSubcommand => "invalid_subcommand",
        K::UnknownArgument => "unknown_argument",
        // Derive commands with a required subcommand report a bare
        // invocation as the "display help" flavor; for telemetry both kinds
        // are the same fact — no subcommand was given.
        K::MissingSubcommand | K::DisplayHelpOnMissingArgumentOrSubcommand => "missing_subcommand",
        K::MissingRequiredArgument => "missing_required",
        K::InvalidValue | K::ValueValidation => "invalid_value",
        _ => "other_parse_error",
    }
}

/// Clap's "did you mean" for a failed parse, when present. The suggestion is
/// computed by clap from the definition set, so it is definition-derived and
/// safe to record.
fn suggestion_for_error(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    [ContextKind::SuggestedSubcommand, ContextKind::SuggestedArg]
        .iter()
        .find_map(|kind| match error.get(*kind) {
            Some(ContextValue::String(s)) => Some(s.clone()),
            Some(ContextValue::Strings(s)) => s.first().cloned(),
            _ => None,
        })
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
        outcome: "ok",
        suggestion: None,
    }
}

/// Derive a lossy invocation from raw argv when parsing failed and no
/// `ArgMatches` exists (help, version, and every usage error).
///
/// Walks the argv tokens against the clap `Command` tree and records only
/// strings owned by the definitions, matched by equality — argv slices never
/// enter the result, the same "structurally impossible to leak a value"
/// guarantee as [`capture`]. The walk stops at the first token that matches
/// nothing, so `command` is the longest *valid* prefix; the unmatched token
/// itself is never recorded (a typo is indistinguishable from a secret
/// pasted into the wrong window — see #320).
pub fn capture_lossy(
    root: &mut clap::Command,
    argv: &[std::ffi::OsString],
    error: &clap::Error,
) -> Invocation {
    // Idempotent; materializes the implicit help/version args so `--help`
    // and `-h` resolve to real definitions below.
    root.build();

    // Ancestor commands, innermost last, like `capture`: flags are resolved
    // against the current command first, then outward (global flags are
    // defined on an ancestor but valid at deeper levels).
    let mut stack: Vec<&clap::Command> = vec![root];
    let mut path: Vec<&str> = Vec::new();
    let mut flags = std::collections::BTreeSet::new();
    let mut tokens = argv.iter().skip(1);
    while let Some(token) = tokens.next() {
        // A non-UTF-8 token cannot match any definition.
        let Some(token) = token.to_str() else { break };
        // Everything after `--` is positional by definition.
        if token == "--" {
            break;
        }
        if let Some(rest) = token.strip_prefix("--") {
            let (name, has_inline_value) = match rest.split_once('=') {
                Some((name, _value)) => (name, true),
                None => (rest, false),
            };
            let Some(arg) = stack
                .iter()
                .rev()
                .find_map(|cmd| cmd.get_arguments().find(|a| a.get_long() == Some(name)))
            else {
                break;
            };
            // `get_long` is `Some` — that is what the token just matched.
            flags.extend(arg.get_long().map(str::to_string));
            // The definition says this flag consumes the next token as its
            // value: skip it, so a value that happens to equal a sibling
            // subcommand name is never misrecorded as command path.
            if !has_inline_value && arg.get_action().takes_values() {
                tokens.next();
            }
        } else if token == "-h" {
            // The short forms of the two implicit flags, recorded under
            // their definitions' long names like everything else.
            flags.insert("help".to_string());
        } else if token == "-V" {
            flags.insert("version".to_string());
        } else if let Some(sub) = stack
            .last()
            .expect("stack starts non-empty and only grows")
            .find_subcommand(token)
        {
            // Recorded name is the definition's, even when the token
            // matched an alias.
            path.push(sub.get_name());
            stack.push(sub);
        } else {
            break;
        }
    }
    Invocation {
        command: path.join(" "),
        flags: flags.into_iter().collect(),
        outcome: outcome_for_error(error.kind()),
        suggestion: suggestion_for_error(error),
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

/// The successfully-parsed invocation, stashed by `main` before dispatch so
/// [`finalize_before_exec`] can build an event from deep inside a handler
/// without threading the capture through every signature. Never set on the
/// parse-failure path — `exec()` is only reachable after a successful parse.
static STASHED_INVOCATION: OnceLock<Invocation> = OnceLock::new();

pub fn stash_invocation(invocation: Invocation) {
    let _ = STASHED_INVOCATION.set(invocation);
}

/// Exactly-once guard shared by [`finalize`] and [`finalize_before_exec`]:
/// whichever runs first claims it, the other is a no-op. When `exec()` fails,
/// the pre-exec hook has already recorded the invocation as `"exec"` and the
/// error propagating back to `main`'s tail is not recorded — losing the
/// exec-failure detail is the accepted price for never emitting two events
/// for one invocation.
static FINALIZED: AtomicBool = AtomicBool::new(false);

/// `true` for exactly one caller per guard: swap semantics, first wins.
fn claim(guard: &AtomicBool) -> bool {
    !guard.swap(true, Ordering::SeqCst)
}

/// The event recorded when the process image is about to be replaced: the
/// stashed parse result with its outcome rewritten to the `"exec"` literal
/// (see [`Payload::outcome`]).
fn exec_invocation(stashed: &Invocation) -> Invocation {
    Invocation {
        outcome: "exec",
        ..stashed.clone()
    }
}

/// The telemetry hook, called once at the very end of `main` (after the
/// command has run, so `telemetry disable` silences its own event), with the
/// gh-style exit code the process is about to exit with. Never errors, never
/// blocks beyond spawning a detached child.
pub fn finalize(invocation: Invocation, exit_code: i32) {
    if !claim(&FINALIZED) {
        return;
    }
    finalize_inner(&invocation, exit_code);
}

/// The pre-exec hook, called by the `exec()` handoffs (`local client`, host
/// `psql`) immediately before the process image is replaced and `main`'s tail
/// becomes unreachable. On a first run this prints the notice to stderr just
/// before the handed-over program starts — acceptable and intended. The
/// detached send child survives the `exec()` because it is a separate
/// process.
pub fn finalize_before_exec() {
    let Some(stashed) = STASHED_INVOCATION.get() else {
        return;
    };
    if !claim(&FINALIZED) {
        return;
    }
    finalize_inner(&exec_invocation(stashed), 0);
}

fn finalize_inner(invocation: &Invocation, exit_code: i32) {
    let Some(path) = state_path() else { return };
    match decide(&path, invocation, exit_code, &real_env_lookup) {
        Action::Silent => {}
        Action::Notice => print_first_run_notice(),
        Action::Debug(json) => {
            use std::io::Write;
            // Not `eprintln!`, which panics on a closed stderr — see
            // `print_first_run_notice`.
            let _ = writeln!(std::io::stderr(), "{json}");
        }
        Action::Send(json) => spawn_send_child(&json),
    }
}

/// Printed to stderr regardless of TTY so agent/non-interactive usage still
/// sees it exactly once (stdout stays machine-parseable). Write failures are
/// ignored, not `eprintln!`-panicked: this hook runs on every invocation —
/// including ones whose stderr is a closed pipe — and must never turn an
/// exit code into a panic.
fn print_first_run_notice() {
    use std::io::Write;
    let _ = writeln!(
        std::io::stderr(),
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
/// exits immediately and the child dies silently on any failure. The path
/// comes from the startup snapshot, so after a self-update this spawns the
/// *new* binary — which may be a newer version than the parent that built
/// the payload.
fn spawn_send_child(payload_json: &str) {
    use std::process::{Command, Stdio};

    let Some(exe) = EXE_PATH.get() else {
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
                use std::io::Write;
                // Not `eprintln!`, which panics on a closed stderr — see
                // `print_first_run_notice`.
                let _ = writeln!(
                    std::io::stderr(),
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
            outcome: "ok",
            suggestion: None,
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
        assert_eq!(value["outcome"], "ok");
        assert_eq!(value["suggestion"], serde_json::Value::Null);
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
                "outcome",
                "suggestion",
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
            outcome: "ok",
            suggestion: None,
        };
        let payload = build_payload(&inv, 0, &env_of(&[]));
        assert_eq!(payload.flags.len(), MAX_FLAGS);
    }

    // -- exec handoffs: pre-exec hook building blocks -------------------------

    #[test]
    fn claim_yields_true_exactly_once() {
        // Tested on a local guard, not the shared static, so this cannot
        // race other tests or depend on execution order.
        let guard = AtomicBool::new(false);
        assert!(claim(&guard));
        assert!(!claim(&guard));
        assert!(!claim(&guard));
    }

    #[test]
    fn exec_invocation_rewrites_only_the_outcome() {
        let stashed = Invocation {
            command: "local client".into(),
            flags: vec!["port".into()],
            outcome: "ok",
            suggestion: None,
        };
        let inv = exec_invocation(&stashed);
        assert_eq!(inv.outcome, "exec");
        assert_eq!(inv.command, "local client");
        assert_eq!(inv.flags, ["port"]);
        assert_eq!(inv.suggestion, None);
    }

    #[test]
    fn exec_outcome_sends_the_expected_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let inv = exec_invocation(&Invocation {
            command: "local client".into(),
            flags: vec!["query".into()],
            outcome: "ok",
            suggestion: None,
        });
        // The hook always passes 0: the handed-over program's exit status is
        // unknowable, and `outcome` marks the code as not meaningful.
        let Action::Send(json) = decide(&path, &inv, 0, &env_of(&[])) else {
            panic!("expected Send");
        };
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], "local client");
        assert_eq!(value["flags"], serde_json::json!(["query"]));
        assert_eq!(value["outcome"], "exec");
        assert_eq!(value["exit_code"], 0);
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

    // -- capture_lossy: failed parses, longest valid prefix only -------------

    fn capture_lossy_from(args: &[&str]) -> Invocation {
        let mut cmd = crate::cli::Cli::command();
        let argv: Vec<std::ffi::OsString> = args.iter().map(Into::into).collect();
        let error = cmd
            .try_get_matches_from_mut(&argv)
            .expect_err("argv must fail to parse for capture_lossy tests");
        capture_lossy(&mut cmd, &argv, &error)
    }

    #[test]
    fn lossy_bare_invocation_is_missing_subcommand() {
        let inv = capture_lossy_from(&["clickhousectl"]);
        assert_eq!(inv.command, "");
        assert!(inv.flags.is_empty());
        assert_eq!(inv.outcome, "missing_subcommand");
        assert_eq!(inv.suggestion, None);
    }

    #[test]
    fn lossy_root_help_records_the_help_flag() {
        let inv = capture_lossy_from(&["clickhousectl", "--help"]);
        assert_eq!(inv.command, "");
        assert_eq!(inv.flags, ["help"]);
        assert_eq!(inv.outcome, "help");
    }

    #[test]
    fn lossy_nested_help_keeps_the_command_path() {
        let inv = capture_lossy_from(&["clickhousectl", "cloud", "service", "--help"]);
        assert_eq!(inv.command, "cloud service");
        assert_eq!(inv.flags, ["help"]);
        assert_eq!(inv.outcome, "help");
    }

    #[test]
    fn lossy_short_help_and_version_record_long_names() {
        let inv = capture_lossy_from(&["clickhousectl", "local", "-h"]);
        assert_eq!(inv.command, "local");
        assert_eq!(inv.flags, ["help"]);
        assert_eq!(inv.outcome, "help");

        let inv = capture_lossy_from(&["clickhousectl", "-V"]);
        assert_eq!(inv.command, "");
        assert_eq!(inv.flags, ["version"]);
        assert_eq!(inv.outcome, "version");
    }

    #[test]
    fn lossy_typoed_subcommand_stops_and_carries_the_suggestion() {
        let inv = capture_lossy_from(&["clickhousectl", "cloud", "servce", "list"]);
        // The typo'd token is never recorded; the path is the valid prefix.
        assert_eq!(inv.command, "cloud");
        assert!(inv.flags.is_empty());
        assert_eq!(inv.outcome, "invalid_subcommand");
        // Clap's did-you-mean names a *defined* subcommand.
        assert_eq!(inv.suggestion.as_deref(), Some("service"));
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]))).unwrap();
        assert!(!json.contains("servce"), "typo leaked into payload: {json}");
    }

    #[test]
    fn lossy_unknown_flag_stops_the_walk() {
        let inv = capture_lossy_from(&["clickhousectl", "local", "--frobnicate", "list"]);
        assert_eq!(inv.command, "local");
        assert!(inv.flags.is_empty());
        assert_eq!(inv.outcome, "unknown_argument");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]))).unwrap();
        assert!(!json.contains("frobnicate"), "unknown flag leaked: {json}");
    }

    #[test]
    fn lossy_flag_value_equal_to_a_subcommand_name_is_skipped() {
        // `get` is missing its required positional, so the parse fails; the
        // `--org-id` *value* happens to be the name of a sibling subcommand
        // and must not be misrecorded as command path.
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "cloud",
            "service",
            "get",
            "--org-id",
            "list",
        ]);
        assert_eq!(inv.command, "cloud service get");
        assert_eq!(inv.flags, ["org-id"]);
        assert_eq!(inv.outcome, "missing_required");
    }

    #[test]
    fn lossy_inline_flag_value_is_discarded() {
        // `--org-id=SECRET` fails only because of the trailing junk token;
        // the name part is matched, the inline value never recorded.
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "cloud",
            "service",
            "list",
            "--org-id=SECRET-ORG",
            "junk-token",
        ]);
        assert_eq!(inv.command, "cloud service list");
        assert_eq!(inv.flags, ["org-id"]);
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]))).unwrap();
        assert!(!json.contains("SECRET"), "inline value leaked: {json}");
    }

    #[test]
    fn lossy_double_dash_stops_the_walk() {
        let inv = capture_lossy_from(&["clickhousectl", "local", "--", "SECRET-POSITIONAL"]);
        assert_eq!(inv.command, "local");
        assert!(inv.flags.is_empty());
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]))).unwrap();
        assert!(!json.contains("SECRET"), "post-`--` token leaked: {json}");
    }

    #[test]
    fn lossy_hostile_argv_never_reaches_the_payload() {
        // Mirrors capture_reports_names_only_never_values_or_positionals:
        // positionals, flag values, and unmatched tokens must all be absent.
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "cloud",
            "--json",
            "service",
            "get",
            "SECRET-SERVICE-ID",
            "--org-id",
            "SECRET-ORG",
            "--wat",
            "SECRET-TRAILING",
        ]);
        assert_eq!(inv.command, "cloud service get");
        assert_eq!(inv.outcome, "unknown_argument");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]))).unwrap();
        assert!(!json.contains("SECRET"), "payload leaked a value: {json}");
    }

    #[test]
    fn init_snapshots_the_executable_path_once() {
        // Under `cargo test` the test binary always has a resolvable path.
        init();
        let first = EXE_PATH.get().expect("init must snapshot the exe path");
        // Idempotent: a second call keeps the first snapshot.
        init();
        assert_eq!(EXE_PATH.get(), Some(first));
    }

    #[test]
    fn state_path_is_telemetry_json_under_base_dir() {
        let path = state_path().unwrap();
        assert!(path.ends_with(".clickhouse/telemetry.json"));
    }
}
