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
//! The payload carries the command path plus flag and positional *names* only
//! — never any values. It is built from the clap definitions ([`capture`] walks
//! `ArgMatches` ids and `Arg` metadata, never touching `get_one`/`get_raw`), so
//! leaking a value is structurally impossible. Positional *presence* is
//! recorded as the definition-owned id (#480), which is what makes
//! `local server stop` and `local server stop <name>` distinguishable; see
//! [`Payload::positionals`] for the exclusions.
//!
//! Every invocation of the binary goes through the same consent state machine
//! (#320). Bare, `--help`, `--version`, and mistyped commands show the first-run
//! notice. A structured local failure defers a still-pending notice so stderr
//! remains one JSON value; a later human-readable invocation shows it. Failed
//! invocations are exactly the signal that shows where the CLI confuses people
//! and agents. A successful parse is captured exactly from `ArgMatches`; a
//! failed parse has none, so [`capture_lossy`] re-walks argv against the
//! clap definitions and records the longest *valid* prefix, the error kind,
//! and clap's suggestion re-anchored to a definition string — the unmatched
//! token itself never leaves the machine.
//!
//! Two commands hand the process over to another program via `exec()`
//! (`local client`, host `psql`), replacing the process image so `main`'s
//! tail never runs on success. Those call sites invoke
//! [`finalize_before_exec`] immediately before `exec()`; it records the
//! stashed invocation with outcome `"exec_attempt"` and shares an
//! exactly-once guard with [`finalize`] so a *failed* `exec()` — where the
//! error propagates back to `main` — never produces a second event.
//!
//! `"exec_attempt"` is a **censored** outcome (#471): it proves the handoff
//! hook was reached, never that the process image was replaced and never
//! anything about the handed-over program's status. The handoff call sites
//! therefore validate what can be validated *before* the hook runs (the
//! selected binary is a regular file with an execute bit; `psql` is on
//! `PATH`), so the deterministic launch failures are ordinary
//! `"error"`/exit-1 events. What is left — a binary unlinked or chmod-ed
//! between validation and `exec()`, a bad executable format — is a race no
//! pre-flight can close, and it lands on `"exec_attempt"` with the correct
//! exit code and message still reaching the shell.
//!
//! A failed *runtime* invocation can also carry a bounded description of how
//! it failed (#450): `failure_stage`, `failure_kind`, an allowlisted
//! `http_status`, and retry/provisioning/duration buckets, all of them closed
//! vocabularies owned by [`crate::failure`] and recorded only at code-owned
//! error boundaries. The values are `&'static str`s from the definitions and
//! an allowlisted number, so — like every other field here — they cannot
//! carry a value the user typed or the API returned. See [`Payload`] for the
//! omission rules.
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
const DOCS_URL: &str = "https://clickhouse.com/docs/concepts/features/interfaces/cli#telemetry";

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

/// Positional ids are a deduplicated subset of the clap definitions, so the
/// real bound is a handful per command; the cap is belt-and-braces so a future
/// definition change can never widen the field.
const MAX_POSITIONALS: usize = 16;

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
    /// Definition-owned ids of the positional slots the user filled on the
    /// command line (e.g. `["name"]` for `local server stop dev`) — presence
    /// only, never the value (#480). The privacy boundary:
    ///
    /// * every entry is cloned from an `Arg` definition compiled into the
    ///   binary, so the field's vocabulary is closed and cannot carry argv;
    /// * only `ValueSource::CommandLine` slots count, so clap defaults,
    ///   environment-fed values, and names a handler generates at runtime are
    ///   absent — which is exactly what makes "the user named it" and "we
    ///   picked one" distinguishable;
    /// * passthrough slots are excluded ([`is_passthrough_positional`]): the
    ///   argv they forward to another program is not part of this CLI's shape.
    positionals: Vec<String>,
    /// Exit code: `Error::exit_code()` for dispatched commands — 0 success,
    /// 1 error, 3 cancelled, 4 auth required, or a child process's passthrough
    /// code — and clap's own code for parse outcomes (0 help/version, 2 usage
    /// error).
    exit_code: i32,
    /// How the invocation ended, from a closed vocabulary. Dispatched
    /// invocations carry `"ok"`, `"error"` (including non-zero child exits),
    /// `"cancelled"`, or `"auth_required"`. Child exits are explicitly marked
    /// as `"error"`; the remaining dispatched outcomes are derived from the
    /// exit code by [`dispatched_outcome`]. `"exec_attempt"` means the
    /// invocation reached the `exec()` handoff — nothing more. It is a
    /// *censored* observation: the process image may or may not have been
    /// replaced, and the handed-over program's exit status is unknowable
    /// either way, so `exit_code` is a fixed 0 and carries no information.
    /// Consumers must never count it as a native-client success or read its
    /// exit code as a status. Failed
    /// parses carry a direct mapping of clap's `ErrorKind` (`"help"`,
    /// `"version"`, `"invalid_subcommand"`, …).
    /// Literal strings only — this field can never carry user data.
    outcome: &'static str,
    /// Clap's "did you mean" for failed parses, anchored locally: recorded
    /// only when it equality-matches a subcommand name or arg long name in
    /// the clap definitions, and the recorded string is cloned from the
    /// definition itself — the "never the user's input" guarantee is
    /// enforced here, not inherited from clap internals. Always the bare
    /// canonical name (no `--`), for flags and subcommands alike. `null`
    /// when clap made no suggestion or the suggestion matched no definition.
    suggestion: Option<String>,
    is_agent: bool,
    /// Canonical id of the detected coding agent (e.g. "claude-code");
    /// `null` for human invocations.
    agent: Option<String>,
    ci: bool,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    // -- failure classification (#450) --------------------------------------
    //
    // Six fields describing *how* a failure failed, from the closed
    // vocabularies in [`crate::failure`]. Every one is a `&'static str` from
    // an enum's `as_str` or an allowlisted `u16`, so — exactly like
    // `outcome` — no user data can reach them: SQL, identifiers, response
    // bodies and credentials are structurally unrepresentable rather than
    // filtered out. They are present only on events that *are* failures and
    // only when a code-owned boundary classified one (see
    // [`admits_failure_detail`]); `skip_serializing_if` keeps them off the
    // wire otherwise, so an absent category is an absent key rather than a
    // `null` a consumer has to interpret.
    /// Which stage of the run failed (`"query_request"`, `"key_create"`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_stage: Option<&'static str>,
    /// What kind of failure it was (`"sql_error"`, `"rate_limited"`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_kind: Option<&'static str>,
    /// The exact HTTP status, when it is one of the allowlisted statuses.
    /// Absent for non-HTTP failures *and* for a status outside the allowlist,
    /// whose class is still readable from `failure_kind`.
    #[serde(skip_serializing_if = "Option::is_none")]
    http_status: Option<u16>,
    /// Retry attempts as a bucket (`"0"`, `"1"`, `"3_5"`, …), never a count.
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_bucket: Option<&'static str>,
    /// How far Query API credential provisioning had got (`"stored_key"`,
    /// `"provisioning"`, …) — the axis that separates a failed query from a
    /// failed provisioning burst.
    #[serde(skip_serializing_if = "Option::is_none")]
    provisioning_state: Option<&'static str>,
    /// Time from the start of the classified operation to the failure, as a
    /// bucket (`"lt_250ms"`, `"lt_30s"`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_bucket: Option<&'static str>,
}

/// Derive the dispatched outcome from the process exit code. [`capture`]
/// marks a successful parse `"ok"` before the command has run; by finalize
/// time the exit code says how dispatch actually ended, so only that
/// placeholder is rewritten — the parse kinds from [`capture_lossy`] and
/// `"exec_attempt"` from [`finalize_before_exec`] pass through untouched. The mapping
/// mirrors `Error::exit_code()`, with arbitrary child exit codes classified as
/// errors.
fn dispatched_outcome(outcome: &'static str, exit_code: i32) -> &'static str {
    if outcome != "ok" {
        return outcome;
    }
    match exit_code {
        0 => "ok",
        3 => "cancelled",
        4 => "auth_required",
        _ => "error",
    }
}

/// Whether failure detail belongs on an event with this outcome. A recorded
/// classification is dropped for a successful run, for the censored
/// `"exec_attempt"` handoff (which says nothing about the handed-over
/// program), and for help/version — which are not failures at all — so the
/// dashboard denominators for parse, auth, completed and censored outcomes
/// keep counting exactly what they counted before.
fn admits_failure_detail(outcome: &str) -> bool {
    !matches!(outcome, "ok" | "exec_attempt" | "help" | "version")
}

fn build_payload(
    invocation: &Invocation,
    exit_code: i32,
    env: EnvLookup<'_>,
    failure: Option<crate::failure::Snapshot>,
) -> Payload {
    let mut flags = invocation.flags.clone();
    flags.truncate(MAX_FLAGS);
    let mut positionals = invocation.positionals.clone();
    positionals.truncate(MAX_POSITIONALS);
    let detected = is_ai_agent::detect();
    let outcome = dispatched_outcome(invocation.outcome, exit_code);
    let failure = failure.filter(|_| admits_failure_detail(outcome));
    Payload {
        command: invocation.command.clone(),
        flags,
        positionals,
        exit_code,
        outcome,
        suggestion: invocation.suggestion.clone(),
        is_agent: detected.is_some(),
        agent: detected.map(|a| a.id.as_str().to_string()),
        ci: env_truthy(env(CI_ENV)),
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        failure_stage: failure.map(|f| f.failure_stage),
        failure_kind: failure.map(|f| f.failure_kind),
        http_status: failure.and_then(|f| f.http_status),
        retry_bucket: failure.map(|f| f.retry_bucket),
        provisioning_state: failure.and_then(|f| f.provisioning_state),
        duration_bucket: failure.and_then(|f| f.duration_bucket),
    }
}

// ---------------------------------------------------------------------------
// Invocation capture
// ---------------------------------------------------------------------------

/// What the user invoked: the subcommand path (e.g. `"local start"`), the long
/// names of the flags they passed, and the ids of the positional slots they
/// filled. Names only — never a value.
#[derive(Clone)]
pub struct Invocation {
    command: String,
    flags: Vec<String>,
    /// See [`Payload::positionals`]: definition-owned ids, presence only.
    positionals: Vec<String>,
    /// See [`Payload::outcome`]: `"ok"` from [`capture`] means *parsed* —
    /// the dispatched outcome is not knowable until the exit code exists, so
    /// [`dispatched_outcome`] derives it at finalize time. From
    /// [`capture_lossy`] this is the error-kind mapping, never rewritten.
    outcome: &'static str,
    /// See [`Payload::suggestion`]: always `None` from [`capture`]; from
    /// [`capture_lossy`] it is a clone of a definition-owned string, never
    /// of clap's error context.
    suggestion: Option<String>,
}

impl Invocation {
    /// Keep a child's raw status while preventing reserved CLI exit codes from
    /// changing its telemetry classification.
    pub fn mark_child_exit(&mut self) {
        self.outcome = "error";
    }
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

/// Clap's "did you mean" for a failed parse, when present. Candidates come
/// out of clap's error context, but the recorded value does not: each one is
/// re-anchored to the built definitions by [`resolve_suggestion`], so the
/// privacy invariant holds locally rather than resting on clap internals.
/// Clap sorts multi-candidate lists by *ascending* similarity (clap_builder
/// `did_you_mean` contract), so the best candidate is the last one that
/// resolves.
fn suggestion_for_error(root: &clap::Command, error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    [ContextKind::SuggestedSubcommand, ContextKind::SuggestedArg]
        .iter()
        .find_map(|kind| match error.get(*kind) {
            Some(ContextValue::String(s)) => resolve_suggestion(root, s),
            Some(ContextValue::Strings(s)) => {
                s.iter().rev().find_map(|s| resolve_suggestion(root, s))
            }
            _ => None,
        })
}

/// Anchor a clap-suggested name to the definition that owns it: strip the
/// `--` prefix clap puts on flag suggestions, then require an equality match
/// against some subcommand name or arg long name in the tree. The returned
/// string is cloned from the *definition*, never from clap's error context,
/// so a recorded suggestion is structurally definition-owned; anything
/// unmatched is dropped. Flags and subcommands are both recorded as the bare
/// canonical name.
fn resolve_suggestion(root: &clap::Command, suggested: &str) -> Option<String> {
    let name = suggested.trim_start_matches('-');
    find_defined_name(root, name).map(str::to_string)
}

/// Depth-first search of the whole command tree for a definition string
/// equal to `name`: arg long names first, then subcommand names, recursing
/// into every subcommand.
fn find_defined_name<'a>(cmd: &'a clap::Command, name: &str) -> Option<&'a str> {
    cmd.get_arguments()
        .filter_map(|a| a.get_long())
        .find(|&long| long == name)
        .or_else(|| {
            cmd.get_subcommands().find_map(|sub| {
                Some(sub.get_name())
                    .filter(|&sub_name| sub_name == name)
                    .or_else(|| find_defined_name(sub, name))
            })
        })
}

/// Whether a positional slot exists to forward raw argv to another program
/// rather than to describe this CLI's own shape (`local client [ARGS]…`,
/// `local postgres client [ARGS]…`, `local server start -- [CLICKHOUSE_ARG]…`).
///
/// The test is structural, not a hand-maintained list of ids: these slots are
/// exactly the ones marked `last`, `trailing_var_arg`, or
/// `allow_hyphen_values`, because forwarding argv verbatim is what those
/// markers are for. Recording their presence would classify another program's
/// argument list — including everything after a `--` — as a clickhousectl
/// positional, so they are skipped ([`Payload::positionals`]). New passthrough
/// slots inherit the exclusion automatically; a new *ordinary* positional is
/// recorded without a config change.
fn is_passthrough_positional(arg: &clap::Arg) -> bool {
    arg.is_last_set() || arg.is_trailing_var_arg_set() || arg.is_allow_hyphen_values_set()
}

#[derive(Default)]
struct PositionalCursor {
    index: usize,
    values: usize,
    active: bool,
}

impl PositionalCursor {
    fn new() -> Self {
        Self {
            index: 1,
            ..Self::default()
        }
    }

    fn current<'a>(&self, cmd: &'a clap::Command) -> Option<&'a clap::Arg> {
        cmd.get_positionals()
            .find(|arg| arg.get_index() == Some(self.index) && !arg.is_last_set())
    }

    fn active_accepts(&self, cmd: &clap::Command, hyphenated: bool) -> bool {
        self.active
            && self.current(cmd).is_some_and(|arg| {
                !hyphenated || arg.is_allow_hyphen_values_set() || arg.is_trailing_var_arg_set()
            })
    }

    fn inactive_accepts_hyphen(&self, cmd: &clap::Command) -> bool {
        !self.active
            && self
                .current(cmd)
                .is_some_and(clap::Arg::is_allow_hyphen_values_set)
    }

    /// Consume a definition-backed positional slot without retaining its
    /// value, recording the slot's *id* in `seen` when it is one of this CLI's
    /// own positionals. The id is cloned from the `Arg` definition, so the
    /// token itself still never enters the result.
    fn consume(
        &mut self,
        cmd: &clap::Command,
        token: &str,
        seen: &mut std::collections::BTreeSet<String>,
    ) -> bool {
        let Some(arg) = self.current(cmd) else {
            return false;
        };
        if arg
            .get_value_terminator()
            .is_some_and(|terminator| terminator.as_str() == token)
        {
            // A terminator token fills no slot: nothing to record.
            self.index += 1;
            self.values = 0;
            self.active = false;
            return true;
        }

        if !is_passthrough_positional(arg) {
            seen.insert(arg.get_id().as_str().to_string());
        }
        self.values += 1;
        self.active = true;
        let max_values = arg.get_num_args().map_or(1, |range| range.max_values());
        if self.values >= max_values {
            self.index += 1;
            self.values = 0;
            self.active = false;
        }
        true
    }

    fn interrupt(&mut self) {
        self.active = false;
    }
}

fn find_short_arg<'a>(stack: &[&'a clap::Command], ch: char) -> Option<&'a clap::Arg> {
    stack.iter().rev().find_map(|cmd| {
        cmd.get_arguments().find(|arg| {
            arg.get_short() == Some(ch)
                || arg
                    .get_all_short_aliases()
                    .is_some_and(|aliases| aliases.contains(&ch))
        })
    })
}

/// Whether clap can resolve a short cluster without treating it as a
/// hyphenated positional value. A value-taking short owns the token's suffix.
fn short_cluster_is_defined(stack: &[&clap::Command], cluster: &str) -> bool {
    for ch in cluster.chars() {
        let Some(arg) = find_short_arg(stack, ch) else {
            return false;
        };
        if arg.get_action().takes_values() {
            return true;
        }
    }
    true
}

/// Derive the command path, passed-flag names, and filled positional ids from
/// the parsed matches.
///
/// Only ids and `Arg` metadata are consulted — never `get_one`/`get_raw`/
/// `get_many` — so argument *values* are structurally unreachable here. A
/// positional contributes its definition id and nothing else (#480; see
/// [`Payload::positionals`]), passthrough slots contribute nothing at all,
/// default-valued and env-fed args are excluded by the
/// `ValueSource::CommandLine` filter, and clap's propagation of global flags
/// into subcommand matches is deduplicated by the sets.
pub fn capture(root: &clap::Command, matches: &clap::ArgMatches) -> Invocation {
    use clap::parser::ValueSource;

    let mut path: Vec<&str> = Vec::new();
    // Ancestor commands, innermost last: global args propagate into
    // subcommand matches but their `Arg` definition lives on an ancestor.
    let mut stack: Vec<&clap::Command> = vec![root];
    let mut flags = std::collections::BTreeSet::new();
    let mut positionals = std::collections::BTreeSet::new();
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
                if !is_passthrough_positional(arg) {
                    // The definition's id, cloned from the definition — a
                    // positional has no long name to fall back on.
                    positionals.insert(arg.get_id().as_str().to_string());
                }
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
        positionals: positionals.into_iter().collect(),
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
/// nothing. Defined positional slots are consumed without retaining their
/// values, recording the slot id like [`capture`] does and allowing later
/// flags to be captured; an unmatched token for which no slot exists still
/// stops the walk. The token itself is never recorded (a typo is
/// indistinguishable from a secret pasted into the wrong window — see #320).
///
/// Slot assignment here is the walk's own index arithmetic rather than clap's
/// parse, so on a failed parse a recorded id says "a token reached this slot",
/// which is the fact the shape analysis needs; it is definition-owned either
/// way. The walk breaks at `--`, so nothing beyond it is ever attributed to a
/// positional.
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
    let mut positionals = std::collections::BTreeSet::new();
    let mut tokens = argv.iter().skip(1);
    let mut positional = PositionalCursor::new();
    'walk: while let Some(token) = tokens.next() {
        // A non-UTF-8 token cannot match any definition.
        let Some(token) = token.to_str() else { break };
        // Everything after `--` is positional by definition.
        if token == "--" {
            break;
        }
        let hyphenated = token.starts_with('-') && token != "-";
        let current = stack.last().expect("stack starts non-empty and only grows");
        // A multi-value positional that is currently being filled owns plain
        // values (including subcommand-like strings). With hyphen values or a
        // trailing var arg it also owns flag-like strings.
        if positional.active_accepts(current, hyphenated) {
            positional.consume(current, token, &mut positionals);
            continue;
        }
        if let Some(rest) = token.strip_prefix("--") {
            let (name, has_inline_value) = match rest.split_once('=') {
                Some((name, _value)) => (name, true),
                None => (rest, false),
            };
            let Some(arg) = stack.iter().rev().find_map(|cmd| {
                cmd.get_arguments().find(|a| {
                    a.get_long() == Some(name)
                        || a.get_all_aliases()
                            .is_some_and(|aliases| aliases.contains(&name))
                })
            }) else {
                if positional.inactive_accepts_hyphen(current) {
                    positional.consume(current, token, &mut positionals);
                    continue;
                }
                break;
            };
            positional.interrupt();
            // Recorded name is the canonical long, even when the token
            // matched a hidden alias (e.g. `--fg` records `foreground`).
            flags.extend(arg.get_long().map(str::to_string));
            // The definition says this flag consumes the next token as its
            // value: skip it, so a value that happens to equal a sibling
            // subcommand name is never misrecorded as command path.
            if !has_inline_value
                && arg.get_action().takes_values()
                && tokens.next().is_some_and(|value| value == "--")
            {
                break;
            }
        } else if let Some(cluster) = token.strip_prefix('-').filter(|rest| !rest.is_empty()) {
            // A short cluster (`-F`, `-Fv`, `-p9000`): resolve each char the
            // way clap does, innermost-outward like the long path. This also
            // covers the implicit `-h`/`-V`, which `root.build()` above
            // materialized as real definitions. A bare `-` never gets here
            // (empty cluster) and falls through to the subcommand branch,
            // where it breaks the walk like any unknown token.
            if positional.inactive_accepts_hyphen(current)
                && !short_cluster_is_defined(&stack, cluster)
            {
                positional.consume(current, token, &mut positionals);
                continue;
            }
            positional.interrupt();
            for (i, ch) in cluster.char_indices() {
                let Some(arg) = find_short_arg(&stack, ch) else {
                    // An unresolvable char stops the whole walk; flags already
                    // recorded stay — they are definition strings.
                    break 'walk;
                };
                // Canonical long name, falling back to the definition's id
                // for short-only args — the same fallback `capture` uses.
                flags.insert(arg.get_long().unwrap_or(arg.get_id().as_str()).to_string());
                if arg.get_action().takes_values() {
                    // The rest of the token (optionally after `=`) is this
                    // flag's attached value: discard it. When the cluster
                    // ends with no attached value, the next argv token is
                    // the value: skip it, as in the long path.
                    if cluster[i + ch.len_utf8()..].is_empty()
                        && tokens.next().is_some_and(|value| value == "--")
                    {
                        break 'walk;
                    }
                    break;
                }
            }
        } else if let Some(sub) = stack
            .last()
            .expect("stack starts non-empty and only grows")
            .find_subcommand(token)
        {
            // Recorded name is the definition's, even when the token
            // matched an alias.
            path.push(sub.get_name());
            stack.push(sub);
            positional = PositionalCursor::new();
        } else if positional.consume(current, token, &mut positionals) {
            continue;
        } else {
            break;
        }
    }
    Invocation {
        command: path.join(" "),
        flags: flags.into_iter().collect(),
        positionals: positionals.into_iter().collect(),
        outcome: outcome_for_error(error.kind()),
        suggestion: suggestion_for_error(root, error),
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

fn decide(
    path: &Path,
    invocation: &Invocation,
    exit_code: i32,
    env: EnvLookup<'_>,
    failure: Option<crate::failure::Snapshot>,
) -> Action {
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
            let json = serde_json::to_string(&build_payload(invocation, exit_code, env, failure))
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
/// whichever runs first claims it, the other is a no-op. When `exec()` fails
/// the pre-exec hook has already recorded the invocation as `"exec_attempt"`,
/// and the error propagating back to `main`'s tail is not recorded a second
/// time — which is why `"exec_attempt"` is defined as censored rather than as
/// a successful handoff (#471). Launch failures that *can* be detected before
/// the hook runs never reach it, so they are recorded as ordinary errors.
static FINALIZED: AtomicBool = AtomicBool::new(false);

/// `true` for exactly one caller per guard: swap semantics, first wins.
fn claim(guard: &AtomicBool) -> bool {
    !guard.swap(true, Ordering::SeqCst)
}

/// The event recorded when the process image is about to be replaced: the
/// stashed parse result with its outcome rewritten to the `"exec_attempt"`
/// literal (see [`Payload::outcome`]).
fn exec_attempt_invocation(stashed: &Invocation) -> Invocation {
    Invocation {
        outcome: "exec_attempt",
        ..stashed.clone()
    }
}

/// The telemetry hook, called once at the very end of `main` (after the
/// command has run, so `telemetry disable` silences its own event), with the
/// exit code the process is about to exit with. Never errors, never
/// blocks beyond spawning a detached child. A deferred first-run notice leaves
/// the state missing so the next human-readable invocation can show it.
pub fn finalize(invocation: Invocation, exit_code: i32, defer_first_run_notice: bool) {
    if !claim(&FINALIZED) {
        return;
    }
    finalize_inner(&invocation, exit_code, defer_first_run_notice);
}

/// The pre-exec hook, called by the `exec()` handoffs (`local client`, host
/// `psql`) immediately before the process image is replaced and `main`'s tail
/// becomes unreachable. Records the censored `"exec_attempt"` outcome, exactly
/// once per invocation. On a first run this prints the notice to stderr just
/// before the handed-over program starts — acceptable and intended. The
/// detached send child survives the `exec()` because it is a separate
/// process.
///
/// Call it as late as possible: everything a handler can check about the
/// launch belongs *before* this hook, so that failure is a real
/// `"error"`/exit-1 event instead of a censored attempt (#471).
pub fn finalize_before_exec() {
    let Some(stashed) = STASHED_INVOCATION.get() else {
        return;
    };
    if !claim(&FINALIZED) {
        return;
    }
    finalize_inner(&exec_attempt_invocation(stashed), 0, false);
}

fn finalize_inner(invocation: &Invocation, exit_code: i32, defer_first_run_notice: bool) {
    let Some(path) = state_path() else { return };
    if defer_first_run_notice && load_state_from(&path) == State::Missing {
        return;
    }
    match decide(
        &path,
        invocation,
        exit_code,
        &real_env_lookup,
        crate::failure::snapshot(),
    ) {
        Action::Silent => {}
        Action::Notice => print_first_run_notice(),
        Action::Debug(_) if defer_first_run_notice => {}
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
         command name, flag and argument names (never their values), success/failure\n\
         with bounded failure categories, version, OS/arch, and CI/agent detection.\n\
         No user or machine IDs.\n\
         Nothing was sent this run.\n\
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
            positionals: vec![],
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
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Silent);
    }

    #[test]
    fn do_not_track_prevents_first_run_marker() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        let env = env_of(&[("DO_NOT_TRACK", "1")]);
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Silent);
        assert!(!path.exists(), "DNT must not write the marker file");
    }

    #[test]
    fn first_run_writes_marker_and_notices_without_sending() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Notice);
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
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Silent);
        // And again: still silent, never a notice, never a send.
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Silent);
    }

    #[test]
    fn disabled_state_is_silent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, true).unwrap();
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Silent);
    }

    #[test]
    fn corrupt_state_file_is_treated_as_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        std::fs::write(&path, "not json{{").unwrap();
        assert_eq!(load_state_from(&path), State::Disabled);
        let env = env_of(&[]);
        assert_eq!(decide(&path, &invocation(), 0, &env, None), Action::Silent);
    }

    #[test]
    fn enabled_state_sends_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let env = env_of(&[("CI", "1")]);
        let Action::Send(json) = decide(&path, &invocation(), 4, &env, None) else {
            panic!("expected Send");
        };
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], "local list");
        assert_eq!(value["flags"], serde_json::json!(["json"]));
        assert_eq!(value["positionals"], serde_json::json!([]));
        assert_eq!(value["exit_code"], 4);
        // The parse-time "ok" placeholder is rewritten from the exit code.
        assert_eq!(value["outcome"], "auth_required");
        assert_eq!(value["suggestion"], serde_json::Value::Null);
        assert_eq!(value["ci"], true);
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["os"], std::env::consts::OS);
        assert_eq!(value["arch"], std::env::consts::ARCH);
    }

    #[test]
    fn dispatched_outcome_derives_from_the_exit_code() {
        assert_eq!(dispatched_outcome("ok", 0), "ok");
        assert_eq!(dispatched_outcome("ok", 1), "error");
        assert_eq!(dispatched_outcome("ok", 2), "error");
        assert_eq!(dispatched_outcome("ok", 3), "cancelled");
        assert_eq!(dispatched_outcome("ok", 4), "auth_required");
        // Any exit code outside the documented vocabulary is still a failure.
        assert_eq!(dispatched_outcome("ok", 5), "error");
        // The child-exit marker replaces the parse-time placeholder before
        // this mapping, so colliding child statuses remain errors.
        let mut child = invocation();
        child.mark_child_exit();
        assert_eq!(dispatched_outcome(child.outcome, 3), "error");
        assert_eq!(dispatched_outcome(child.outcome, 4), "error");
        // Non-"ok" outcomes are never rewritten, whatever the exit code.
        assert_eq!(
            dispatched_outcome("unknown_argument", 2),
            "unknown_argument"
        );
        assert_eq!(dispatched_outcome("exec_attempt", 0), "exec_attempt");
    }

    /// The `outcome` field of the payload `decide` builds for the given
    /// invocation and exit code, with an enabled state file.
    fn decided_outcome(invocation: &Invocation, exit_code: i32) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let Action::Send(json) = decide(&path, invocation, exit_code, &env_of(&[]), None) else {
            panic!("expected Send");
        };
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["outcome"].as_str().unwrap().to_string()
    }

    #[test]
    fn dispatched_payload_outcomes_track_the_exit_code() {
        assert_eq!(decided_outcome(&invocation(), 0), "ok");
        assert_eq!(decided_outcome(&invocation(), 1), "error");
        assert_eq!(decided_outcome(&invocation(), 3), "cancelled");
    }

    #[test]
    fn lossy_payload_outcome_is_not_rewritten_by_the_exit_code() {
        let inv = Invocation {
            command: "local".into(),
            flags: vec![],
            positionals: vec![],
            outcome: "unknown_argument",
            suggestion: None,
        };
        assert_eq!(decided_outcome(&inv, 2), "unknown_argument");
    }

    #[test]
    fn debug_env_prints_instead_of_sending() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let env = env_of(&[("CHCTL_TELEMETRY_DEBUG", "1")]);
        assert!(matches!(
            decide(&path, &invocation(), 0, &env, None),
            Action::Debug(_)
        ));
    }

    #[test]
    fn payload_serializes_exactly_the_wire_fields() {
        let payload = build_payload(&invocation(), 0, &env_of(&[]), None);
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
                "positionals",
                "exit_code",
                "outcome",
                "suggestion",
                "is_agent",
                "agent",
                "ci",
                "version",
                "os",
                "arch"
            ],
            "an unclassified event carries no failure keys at all — not even null"
        );
        // The two agent fields are set from the same single detection and can
        // never disagree.
        assert_eq!(
            value["is_agent"].as_bool().unwrap(),
            !value["agent"].is_null()
        );
    }

    // -- failure classification (#450) ---------------------------------------

    /// A fully-populated classification, as `crate::failure` would report it.
    fn failure_snapshot() -> crate::failure::Snapshot {
        crate::failure::Snapshot {
            failure_stage: "key_create",
            failure_kind: "rate_limited",
            http_status: Some(429),
            retry_bucket: "3_5",
            provisioning_state: Some("provisioning"),
            duration_bucket: Some("lt_30s"),
        }
    }

    #[test]
    fn classified_failure_serializes_exactly_the_documented_wire_fields() {
        let payload = build_payload(&invocation(), 1, &env_of(&[]), Some(failure_snapshot()));
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
                "positionals",
                "exit_code",
                "outcome",
                "suggestion",
                "is_agent",
                "agent",
                "ci",
                "version",
                "os",
                "arch",
                "failure_stage",
                "failure_kind",
                "http_status",
                "retry_bucket",
                "provisioning_state",
                "duration_bucket",
            ]
        );
        assert_eq!(value["outcome"], "error");
        assert_eq!(value["failure_stage"], "key_create");
        assert_eq!(value["failure_kind"], "rate_limited");
        assert_eq!(value["http_status"], 429);
        assert_eq!(value["retry_bucket"], "3_5");
        assert_eq!(value["provisioning_state"], "provisioning");
        assert_eq!(value["duration_bucket"], "lt_30s");
    }

    #[test]
    fn absent_classification_details_are_omitted_not_nulled() {
        // A failure with no HTTP status, no provisioning state and no timing
        // span reports only the two categories plus the retry bucket.
        let payload = build_payload(
            &invocation(),
            1,
            &env_of(&[]),
            Some(crate::failure::Snapshot {
                failure_stage: "sql_input",
                failure_kind: "other",
                http_status: None,
                retry_bucket: "0",
                provisioning_state: None,
                duration_bucket: None,
            }),
        );
        let value = serde_json::to_value(&payload).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object["failure_stage"], "sql_input");
        assert_eq!(object["failure_kind"], "other");
        assert_eq!(object["retry_bucket"], "0");
        for key in ["http_status", "provisioning_state", "duration_bucket"] {
            assert!(
                !object.contains_key(key),
                "{key} must be omitted when absent, not sent as null: {value}"
            );
        }
    }

    #[test]
    fn failure_detail_rides_only_on_failure_outcomes() {
        assert!(admits_failure_detail("error"));
        assert!(admits_failure_detail("auth_required"));
        assert!(admits_failure_detail("cancelled"));
        assert!(admits_failure_detail("unknown_argument"));
        assert!(!admits_failure_detail("ok"));
        assert!(!admits_failure_detail("exec_attempt"));
        assert!(!admits_failure_detail("help"));
        assert!(!admits_failure_detail("version"));
    }

    /// The classification is dropped for outcomes that are not failures, so
    /// the "completed" and "censored/handoff" denominators keep their exact
    /// meaning even if a boundary recorded something along the way.
    #[test]
    fn a_successful_or_censored_event_never_carries_failure_detail() {
        for (inv, exit_code) in [
            (invocation(), 0),
            (exec_attempt_invocation(&invocation()), 0),
            (
                Invocation {
                    outcome: "help",
                    ..invocation()
                },
                0,
            ),
        ] {
            let payload = build_payload(&inv, exit_code, &env_of(&[]), Some(failure_snapshot()));
            let value = serde_json::to_value(&payload).unwrap();
            let object = value.as_object().unwrap();
            for key in [
                "failure_stage",
                "failure_kind",
                "http_status",
                "retry_bucket",
                "provisioning_state",
                "duration_bucket",
            ] {
                assert!(
                    !object.contains_key(key),
                    "outcome {} must carry no {key}: {value}",
                    object["outcome"]
                );
            }
        }
    }

    /// The classification fields are `&'static str`s and an allowlisted
    /// number, so there is no code path by which SQL, an identifier, a
    /// response body or a credential could reach them. Pin that with a
    /// hostile invocation whose every string looks like a secret.
    #[test]
    fn a_classified_failure_payload_holds_no_free_text() {
        let inv = Invocation {
            command: "cloud service query".into(),
            flags: vec!["query".into(), "org-id".into()],
            positionals: vec![],
            outcome: "ok",
            suggestion: None,
        };
        let json = serde_json::to_string(&build_payload(
            &inv,
            1,
            &env_of(&[]),
            Some(failure_snapshot()),
        ))
        .unwrap();
        for secret in [
            "SELECT",
            "password",
            "AKIA",
            "sk-",
            "org-1",
            "svc-",
            "Bearer",
            "Unknown table",
        ] {
            assert!(
                !json.contains(secret),
                "classified payload leaked {secret}: {json}"
            );
        }
    }

    #[test]
    fn flags_truncated_to_worker_cap() {
        let inv = Invocation {
            command: "x".into(),
            flags: (0..100).map(|i| format!("flag-{i}")).collect(),
            positionals: (0..100).map(|i| format!("pos-{i}")).collect(),
            outcome: "ok",
            suggestion: None,
        };
        let payload = build_payload(&inv, 0, &env_of(&[]), None);
        assert_eq!(payload.flags.len(), MAX_FLAGS);
        assert_eq!(payload.positionals.len(), MAX_POSITIONALS);
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
    fn exec_attempt_invocation_rewrites_only_the_outcome() {
        let stashed = Invocation {
            command: "local client".into(),
            flags: vec!["port".into()],
            positionals: vec!["name".into()],
            outcome: "ok",
            suggestion: None,
        };
        let inv = exec_attempt_invocation(&stashed);
        assert_eq!(inv.outcome, "exec_attempt");
        assert_eq!(inv.command, "local client");
        assert_eq!(inv.flags, ["port"]);
        assert_eq!(inv.positionals, ["name"]);
        assert_eq!(inv.suggestion, None);
    }

    #[test]
    fn exec_attempt_outcome_sends_the_expected_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry.json");
        save_state_to(&path, false).unwrap();
        let inv = exec_attempt_invocation(&Invocation {
            command: "local client".into(),
            flags: vec!["query".into()],
            positionals: vec![],
            outcome: "ok",
            suggestion: None,
        });
        // The hook always passes 0: the handoff is censored, so neither the
        // launch nor the handed-over program's exit status is observable and
        // `outcome` marks the code as carrying no information.
        let Action::Send(json) = decide(&path, &inv, 0, &env_of(&[]), None) else {
            panic!("expected Send");
        };
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], "local client");
        assert_eq!(value["flags"], serde_json::json!(["query"]));
        assert_eq!(value["outcome"], "exec_attempt");
        assert_eq!(value["exit_code"], 0);
    }

    #[test]
    fn exec_attempt_is_never_rewritten_to_a_dispatched_outcome() {
        // A censored attempt must not be laundered into "ok" (or "error") by
        // the exit-code mapping, whatever code the tail would have seen.
        let inv = exec_attempt_invocation(&invocation());
        for exit_code in [0, 1, 3, 4, 23] {
            assert_eq!(decided_outcome(&inv, exit_code), "exec_attempt");
        }
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
        // The positional's definition id is recorded; its value is not (#480).
        assert_eq!(inv.positionals, ["service_id"]);
        let json = serde_json::to_string(&build_payload(&inv, 0, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "payload leaked a value: {json}");
    }

    /// The three lifecycle shapes issue #480 could not tell apart.
    #[test]
    fn capture_distinguishes_bare_named_and_flag_named_stop() {
        let bare = capture_from(&["clickhousectl", "local", "server", "stop"]);
        assert_eq!(bare.command, "local server stop");
        assert!(bare.flags.is_empty());
        assert!(
            bare.positionals.is_empty(),
            "an omitted positional must stay absent: {:?}",
            bare.positionals
        );

        let named = capture_from(&["clickhousectl", "local", "server", "stop", "SECRET-NAME"]);
        assert_eq!(named.command, "local server stop");
        assert!(named.flags.is_empty());
        assert_eq!(named.positionals, ["name"]);

        // The compatibility `--name` form is a flag, so the two ways of naming
        // a server stay distinguishable without a separate source field.
        let flagged = capture_from(&[
            "clickhousectl",
            "local",
            "server",
            "stop",
            "--name",
            "SECRET-NAME",
        ]);
        assert_eq!(flagged.command, "local server stop");
        assert_eq!(flagged.flags, ["name"]);
        assert!(flagged.positionals.is_empty());

        for inv in [&bare, &named, &flagged] {
            let json = serde_json::to_string(&build_payload(inv, 0, &env_of(&[]), None)).unwrap();
            assert!(!json.contains("SECRET"), "payload leaked a value: {json}");
        }
    }

    /// The other half of the #480 signal: a supplied version is visible on the
    /// successful parse, so it is distinguishable from the missing-required
    /// parse failure asserted in `lossy_missing_required_positional_is_absent`.
    #[test]
    fn capture_records_supplied_version_positional() {
        for command in [
            ["clickhousectl", "local", "use", "25.12.9.61"],
            ["clickhousectl", "local", "remove", "25.12.9.61"],
        ] {
            let inv = capture_from(&command);
            assert_eq!(inv.positionals, ["version"], "for {command:?}");
            assert_eq!(inv.outcome, "ok");
        }
    }

    /// Passthrough slots forward argv to another program (`clickhouse-client`,
    /// `psql`, `clickhouse-server`). Their presence is not this CLI's shape and
    /// is never recorded — including everything after a `--`.
    #[test]
    fn capture_excludes_passthrough_positionals() {
        let inv = capture_from(&[
            "clickhousectl",
            "local",
            "client",
            "--",
            "--secret-passthrough-flag",
            "SECRET-VALUE",
        ]);
        assert_eq!(inv.command, "local client");
        assert!(
            inv.positionals.is_empty(),
            "passthrough args were recorded: {:?}",
            inv.positionals
        );

        // `server start` puts its passthrough behind `last = true`: the named
        // server is still recorded, the forwarded arguments are not.
        let inv = capture_from(&[
            "clickhousectl",
            "local",
            "server",
            "start",
            "SECRET-NAME",
            "--",
            "--logger.level=SECRET-LEVEL",
        ]);
        assert_eq!(inv.command, "local server start");
        assert_eq!(inv.positionals, ["name"]);

        let inv = capture_from(&[
            "clickhousectl",
            "local",
            "postgres",
            "client",
            "--",
            "-c",
            "SECRET-SQL",
        ]);
        assert_eq!(inv.command, "local postgres client");
        assert!(inv.positionals.is_empty());
    }

    /// Every passthrough slot in the real command tree is recognized
    /// structurally, and every recorded id is a source-level identifier — the
    /// field's vocabulary is closed by the definitions, not by an allowlist
    /// that can go stale.
    #[test]
    fn positional_ids_are_closed_definition_identifiers() {
        fn walk(cmd: &clap::Command, recorded: &mut Vec<String>, passthrough: &mut Vec<String>) {
            for arg in cmd.get_positionals() {
                if is_passthrough_positional(arg) {
                    passthrough.push(arg.get_id().to_string());
                } else {
                    recorded.push(arg.get_id().to_string());
                }
            }
            for sub in cmd.get_subcommands() {
                walk(sub, recorded, passthrough);
            }
        }
        let mut cmd = crate::cli::Cli::command();
        cmd.build();
        let (mut recorded, mut passthrough) = (Vec::new(), Vec::new());
        walk(&cmd, &mut recorded, &mut passthrough);

        assert!(!recorded.is_empty(), "the tree must have positionals");
        for id in &recorded {
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "positional id {id} is not a Rust identifier, so it is not \
                 definition-owned"
            );
        }
        // The passthrough slots are the argv-forwarding ones; they all happen
        // to be spelled `args`, but the exclusion is by marker, not by name.
        assert!(
            passthrough.iter().all(|id| id == "args"),
            "unexpected passthrough slots: {passthrough:?}"
        );
        assert!(
            !recorded.contains(&"args".to_string()),
            "a forwarding slot is being recorded: {recorded:?}"
        );
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
        // `level` has a default (ValueSource::DefaultValue), so it is not
        // reported; `target` was passed on the command line, so its id is.
        assert_eq!(inv.flags, ["verbose"]);
        assert_eq!(inv.positionals, ["target"]);
    }

    /// A positional that clap filled from a default (or, by the same
    /// `ValueSource::CommandLine` comparison, from the environment) is not a
    /// user-supplied positional and must stay absent — that is what makes an
    /// omitted default distinguishable from an explicit value.
    #[test]
    fn capture_excludes_defaulted_positionals() {
        use clap::{Arg, Command};
        let mut cmd = Command::new("root").subcommand(
            Command::new("sub")
                .arg(Arg::new("target").default_value("default"))
                .arg(Arg::new("other")),
        );
        let matches = cmd.try_get_matches_from_mut(["root", "sub"]).unwrap();
        let inv = capture(&cmd, &matches);
        assert!(
            inv.positionals.is_empty(),
            "a defaulted positional was reported as user-supplied: {:?}",
            inv.positionals
        );
    }

    #[test]
    fn capture_with_no_flags_is_empty() {
        let inv = capture_from(&["clickhousectl", "local", "list"]);
        assert_eq!(inv.command, "local list");
        assert!(inv.flags.is_empty());
        assert!(inv.positionals.is_empty());
    }

    // -- capture_lossy: failed parses, longest valid prefix only -------------

    fn capture_lossy_from(args: &[&str]) -> Invocation {
        capture_lossy_with(crate::cli::Cli::command(), args)
    }

    fn capture_lossy_with(mut cmd: clap::Command, args: &[&str]) -> Invocation {
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
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("servce"), "typo leaked into payload: {json}");
    }

    #[test]
    fn lossy_suggestion_picks_the_most_similar_candidate() {
        use clap::Command;
        use clap::error::{ContextKind, ContextValue};
        // Jaro similarity to the typo "starz" is unambiguously ordered:
        // "start" 0.87 > "stash" 0.73, both above clap's 0.7 threshold, so
        // clap suggests both — ascending, most similar last. Taking the
        // first would record the weakest candidate.
        let mut cmd = Command::new("root")
            .subcommand(Command::new("stash"))
            .subcommand(Command::new("start"));
        let argv: Vec<std::ffi::OsString> = ["root", "starz"].iter().map(Into::into).collect();
        let error = cmd.try_get_matches_from_mut(&argv).unwrap_err();
        // Guard against the premise going stale: both candidates must be
        // present, ascending, or the assertion below proves nothing.
        assert_eq!(
            error.get(ContextKind::SuggestedSubcommand),
            Some(&ContextValue::Strings(vec!["stash".into(), "start".into()]))
        );
        let inv = capture_lossy(&mut cmd, &argv, &error);
        assert_eq!(inv.suggestion.as_deref(), Some("start"));
    }

    #[test]
    fn lossy_flag_suggestion_records_the_bare_definition_name() {
        // Clap's SuggestedArg context carries `--json`; the recorded value is
        // the bare canonical name, cloned from the definition.
        let inv = capture_lossy_from(&["clickhousectl", "cloud", "service", "list", "--jsn"]);
        assert_eq!(inv.outcome, "unknown_argument");
        assert_eq!(inv.suggestion.as_deref(), Some("json"));
    }

    #[test]
    fn suggestion_matching_no_definition_is_dropped() {
        use clap::error::{ContextKind, ContextValue, ErrorKind};
        let mut cmd = crate::cli::Cli::command();
        cmd.build();
        // Fabricate a suggestion clap could never produce: anchoring must
        // reject anything that is not a definition string.
        let mut error = clap::Error::new(ErrorKind::InvalidSubcommand);
        error.insert(
            ContextKind::SuggestedSubcommand,
            ContextValue::Strings(vec!["not-a-defined-name".into()]),
        );
        assert_eq!(suggestion_for_error(&cmd, &error), None);

        // A candidate list where only the weaker (earlier) entry resolves:
        // the unresolvable one is skipped, not recorded.
        let mut error = clap::Error::new(ErrorKind::InvalidSubcommand);
        error.insert(
            ContextKind::SuggestedSubcommand,
            ContextValue::Strings(vec!["service".into(), "not-a-defined-name".into()]),
        );
        assert_eq!(
            suggestion_for_error(&cmd, &error).as_deref(),
            Some("service")
        );
    }

    #[test]
    fn lossy_unknown_flag_stops_the_walk() {
        let inv = capture_lossy_from(&["clickhousectl", "local", "--frobnicate", "list"]);
        assert_eq!(inv.command, "local");
        assert!(inv.flags.is_empty());
        assert_eq!(inv.outcome, "unknown_argument");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("frobnicate"), "unknown flag leaked: {json}");
    }

    #[test]
    fn lossy_required_positional_allows_later_flags() {
        use clap::{Arg, Command, value_parser};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("target").required(true))
                .arg(
                    Arg::new("count")
                        .long("count")
                        .value_parser(value_parser!(u16)),
                ),
        );
        let inv = capture_lossy_with(
            cmd,
            &["root", "run", "SECRET-TARGET", "--count", "SECRET-COUNT"],
        );
        assert_eq!(inv.command, "run");
        assert_eq!(inv.flags, ["count"]);
        assert_eq!(inv.outcome, "invalid_value");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(
            !json.contains("SECRET"),
            "positional or flag value leaked: {json}"
        );
    }

    #[test]
    fn lossy_optional_positional_allows_later_flags() {
        use clap::{Arg, Command, value_parser};
        let cmd = Command::new("root").subcommand(
            Command::new("run").arg(Arg::new("target")).arg(
                Arg::new("count")
                    .long("count")
                    .value_parser(value_parser!(u16)),
            ),
        );
        let inv = capture_lossy_with(
            cmd,
            &["root", "run", "SECRET-TARGET", "--count", "SECRET-COUNT"],
        );
        assert_eq!(inv.command, "run");
        assert_eq!(inv.flags, ["count"]);
        assert_eq!(inv.outcome, "invalid_value");
    }

    #[test]
    fn lossy_variadic_positional_allows_later_flags() {
        use clap::{Arg, Command, value_parser};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("targets").num_args(1..))
                .arg(
                    Arg::new("count")
                        .long("count")
                        .value_parser(value_parser!(u16)),
                ),
        );
        let inv = capture_lossy_with(
            cmd,
            &[
                "root",
                "run",
                "SECRET-ONE",
                "SECRET-TWO",
                "--count",
                "SECRET-COUNT",
            ],
        );
        assert_eq!(inv.command, "run");
        assert_eq!(inv.flags, ["count"]);
        assert_eq!(inv.outcome, "invalid_value");
    }

    #[test]
    fn lossy_positional_values_can_resemble_flags_and_subcommands() {
        use clap::{Arg, ArgAction, Command};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("values").num_args(1..=3).allow_hyphen_values(true))
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .action(ArgAction::SetTrue),
                )
                .subcommand(Command::new("child")),
        );
        let inv = capture_lossy_with(
            cmd,
            &[
                "root",
                "run",
                "SECRET-FIRST",
                "--verbose",
                "child",
                "SECRET-UNMATCHED",
            ],
        );
        assert_eq!(inv.command, "run");
        assert!(inv.flags.is_empty());
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "positional value leaked: {json}");
    }

    #[test]
    fn lossy_flag_like_value_is_not_recorded_as_a_flag() {
        use clap::{Arg, ArgAction, Command, value_parser};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("label").long("label").allow_hyphen_values(true))
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("count")
                        .long("count")
                        .value_parser(value_parser!(u16)),
                ),
        );
        let inv = capture_lossy_with(
            cmd,
            &[
                "root",
                "run",
                "--label",
                "--verbose",
                "--count",
                "SECRET-COUNT",
            ],
        );
        assert_eq!(inv.command, "run");
        assert_eq!(inv.flags, ["count", "label"]);
    }

    #[test]
    fn lossy_unknown_token_after_positional_still_stops_capture() {
        use clap::{Arg, ArgAction, Command};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("known").long("known").action(ArgAction::SetTrue)),
        );
        let inv = capture_lossy_with(
            cmd,
            &[
                "root",
                "run",
                "SECRET-TARGET",
                "SECRET-UNMATCHED",
                "--known",
            ],
        );
        assert_eq!(inv.command, "run");
        assert!(inv.flags.is_empty());
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "unmatched token leaked: {json}");
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
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "inline value leaked: {json}");
    }

    #[test]
    fn lossy_long_aliases_record_the_canonical_names() {
        // `--fg` and `--config-file` are hidden aliases (local/cli.rs); the
        // recorded names are the definitions' canonical longs. The parse
        // fails only on the `--http-port` value.
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "local",
            "server",
            "start",
            "--fg",
            "--config-file",
            "SECRET-CONFIG",
            "--http-port",
            "SECRET-PORT",
        ]);
        assert_eq!(inv.command, "local server start");
        assert_eq!(inv.flags, ["config", "foreground", "http-port"]);
        assert_eq!(inv.outcome, "invalid_value");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "flag value leaked: {json}");
    }

    #[test]
    fn lossy_short_flag_value_equal_to_a_subcommand_name_is_skipped() {
        // Short spelling of lossy_flag_value_equal_to_a_subcommand_name_is_
        // skipped: the `-q` *value* is a subcommand name elsewhere in the
        // tree and must not be misrecorded as command path — and must not
        // break the walk before `-p` (whose bad value fails the parse).
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "local",
            "client",
            "-q",
            "list",
            "-p",
            "SECRET-NOT-A-PORT",
        ]);
        assert_eq!(inv.command, "local client");
        assert_eq!(inv.flags, ["port", "query"]);
        assert_eq!(inv.outcome, "invalid_value");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "flag value leaked: {json}");
    }

    #[test]
    fn lossy_short_cluster_resolves_each_char_and_discards_attached_value() {
        use clap::{Arg, ArgAction, Command};
        let mut cmd = Command::new("root").subcommand(
            Command::new("sub")
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(ArgAction::SetTrue),
                )
                // Short-only: recorded under its id, like `capture`.
                .arg(Arg::new("quiet").short('q').action(ArgAction::SetTrue))
                // `-L` is a short alias; the canonical long is recorded.
                .arg(Arg::new("level").long("level").short('l').short_alias('L')),
        );
        // One cluster: two unit flags, then a value-taking flag whose
        // attached value is the remainder of the token.
        let argv: Vec<std::ffi::OsString> = ["root", "sub", "-qvLSECRET-LEVEL", "junk-token"]
            .iter()
            .map(Into::into)
            .collect();
        let error = cmd.try_get_matches_from_mut(&argv).unwrap_err();
        let inv = capture_lossy(&mut cmd, &argv, &error);
        assert_eq!(inv.command, "sub");
        assert_eq!(inv.flags, ["level", "quiet", "verbose"]);
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "attached value leaked: {json}");
    }

    #[test]
    fn lossy_unknown_short_stops_the_walk() {
        // Nothing after the unresolvable char is recorded, not even a flag
        // clap would have accepted.
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "cloud",
            "service",
            "list",
            "-Z",
            "--org-id",
            "SECRET-ORG",
        ]);
        assert_eq!(inv.command, "cloud service list");
        assert!(inv.flags.is_empty());
        assert_eq!(inv.outcome, "unknown_argument");
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "post-break token leaked: {json}");

        // Mid-cluster: chars resolved before the unknown one stay recorded —
        // they are definition strings.
        use clap::{Arg, ArgAction, Command};
        let mut cmd = Command::new("root").subcommand(
            Command::new("sub").arg(
                Arg::new("verbose")
                    .long("verbose")
                    .short('v')
                    .action(ArgAction::SetTrue),
            ),
        );
        let argv: Vec<std::ffi::OsString> = ["root", "sub", "-vZ"].iter().map(Into::into).collect();
        let error = cmd.try_get_matches_from_mut(&argv).unwrap_err();
        let inv = capture_lossy(&mut cmd, &argv, &error);
        assert_eq!(inv.command, "sub");
        assert_eq!(inv.flags, ["verbose"]);
    }

    #[test]
    fn lossy_double_dash_stops_the_walk() {
        let inv = capture_lossy_from(&["clickhousectl", "local", "--", "SECRET-POSITIONAL"]);
        assert_eq!(inv.command, "local");
        assert!(inv.flags.is_empty());
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "post-`--` token leaked: {json}");
    }

    #[test]
    fn lossy_double_dash_after_positional_stops_the_walk() {
        use clap::{Arg, ArgAction, Command};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("known").long("known").action(ArgAction::SetTrue)),
        );
        let inv = capture_lossy_with(cmd, &["root", "run", "SECRET-TARGET", "--", "--known"]);
        assert_eq!(inv.command, "run");
        assert!(inv.flags.is_empty());
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
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "payload leaked a value: {json}");
    }

    // -- capture_lossy: positional presence on failed parses -----------------

    /// A required positional that was never supplied stays absent, so a bare
    /// `local use` is distinguishable from `local use <version>` (#480).
    #[test]
    fn lossy_missing_required_positional_is_absent() {
        for args in [
            &["clickhousectl", "local", "use"],
            &["clickhousectl", "local", "remove"],
        ] {
            let inv = capture_lossy_from(args);
            assert_eq!(inv.outcome, "missing_required", "for {args:?}");
            assert!(
                inv.positionals.is_empty(),
                "a missing positional was recorded for {args:?}: {:?}",
                inv.positionals
            );
        }
    }

    /// A failed parse that *did* carry a positional records the slot id, so a
    /// handler-side failure after a supplied version is distinguishable from
    /// the missing-argument parse failure above.
    #[test]
    fn lossy_supplied_positional_records_the_slot_id() {
        let inv = capture_lossy_from(&[
            "clickhousectl",
            "local",
            "remove",
            "SECRET-VERSION",
            "--frobnicate",
        ]);
        assert_eq!(inv.command, "local remove");
        assert_eq!(inv.outcome, "unknown_argument");
        assert_eq!(inv.positionals, ["version"]);
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "positional value leaked: {json}");
    }

    /// Nothing after `--` is a clickhousectl positional: the walk stops there,
    /// so a forwarded argument list can never be attributed to a slot.
    #[test]
    fn lossy_tokens_after_double_dash_are_not_positionals() {
        let inv = capture_lossy_from(&["clickhousectl", "local", "--", "SECRET-ONE", "SECRET-TWO"]);
        assert_eq!(inv.command, "local");
        assert!(inv.positionals.is_empty());
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "post-`--` token leaked: {json}");
    }

    /// Passthrough slots are excluded on the lossy path too, including a
    /// trailing var arg that swallows hostile flag-like tokens.
    #[test]
    fn lossy_passthrough_positionals_are_excluded() {
        use clap::{Arg, Command, value_parser};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("target"))
                .arg(
                    Arg::new("args")
                        .num_args(0..)
                        .trailing_var_arg(true)
                        .allow_hyphen_values(true),
                )
                .arg(
                    Arg::new("count")
                        .long("count")
                        .value_parser(value_parser!(u16)),
                ),
        );
        // The bad `--count` value is what fails the parse; the trailing var
        // arg would otherwise swallow the hostile tokens successfully.
        let inv = capture_lossy_with(
            cmd,
            &[
                "root",
                "run",
                "--count",
                "SECRET-COUNT",
                "SECRET-TARGET",
                "--secret-forwarded",
                "SECRET-FORWARDED-VALUE",
            ],
        );
        assert_eq!(inv.command, "run");
        assert_eq!(inv.flags, ["count"]);
        assert_eq!(
            inv.positionals,
            ["target"],
            "only the CLI's own slot may be recorded"
        );
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "forwarded argv leaked: {json}");
    }

    /// A value terminator advances the cursor without filling a slot, so the
    /// terminator token alone must not look like a supplied positional.
    #[test]
    fn lossy_value_terminator_records_no_positional() {
        use clap::{Arg, Command};
        let cmd = Command::new("root").subcommand(
            Command::new("run")
                .arg(Arg::new("values").num_args(1..).value_terminator(";"))
                .arg(Arg::new("after")),
        );
        let inv = capture_lossy_with(cmd, &["root", "run", ";", "SECRET-JUNK", "SECRET-EXTRA"]);
        assert_eq!(inv.command, "run");
        assert_eq!(inv.positionals, ["after"]);
        let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
        assert!(!json.contains("SECRET"), "terminated value leaked: {json}");
    }

    /// Hostile secret-shaped positionals across every capture path: the ids
    /// recorded are definition strings, the values never appear.
    #[test]
    fn hostile_positional_fixtures_never_reach_the_payload() {
        const HOSTILE: &[&str] = &[
            "AKIAIOSFODNN7EXAMPLE",
            "sk-live-0123456789abcdef",
            "postgres://user:pa55w0rd@db.internal:5432/prod",
            "s3://bucket/customer-export.csv",
            "/Users/someone/.ssh/id_rsa",
            "ghp_0123456789abcdefghijklmnopqrstuvwxyz",
        ];
        for hostile in HOSTILE {
            // Successful parse.
            let inv = capture_from(&["clickhousectl", "local", "server", "stop", hostile]);
            assert_eq!(inv.positionals, ["name"]);
            let json = serde_json::to_string(&build_payload(&inv, 0, &env_of(&[]), None)).unwrap();
            assert!(!json.contains(hostile), "leaked {hostile}: {json}");

            // Failed parse (unknown flag after the positional).
            let inv = capture_lossy_from(&[
                "clickhousectl",
                "local",
                "server",
                "stop",
                hostile,
                "--frobnicate",
            ]);
            assert_eq!(inv.positionals, ["name"]);
            let json = serde_json::to_string(&build_payload(&inv, 2, &env_of(&[]), None)).unwrap();
            assert!(!json.contains(hostile), "leaked {hostile}: {json}");

            // Passthrough (forwarded verbatim to clickhouse-client).
            let inv = capture_from(&["clickhousectl", "local", "client", "--", hostile]);
            assert!(inv.positionals.is_empty());
            let json = serde_json::to_string(&build_payload(&inv, 0, &env_of(&[]), None)).unwrap();
            assert!(!json.contains(hostile), "leaked {hostile}: {json}");
        }
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
