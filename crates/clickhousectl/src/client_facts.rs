//! Facts about the running client that both outbound payloads carry: the
//! telemetry event (`crate::telemetry`) and the feedback message
//! (`crate::feedback`).
//!
//! Every field comes from the binary or the process environment, from a
//! closed vocabulary: the compiled-in version, `std::env::consts`,
//! `is_ai_agent::detect()`, and the `CI` marker as a boolean. None of them can
//! carry anything the user typed. Field names and order are part of both wire
//! contracts; [`ClientFacts`] is flattened into each payload, so renaming or
//! reordering a field changes both.

/// Standard CI marker, sent as a boolean so pipelines can be filtered out.
const CI_ENV: &str = "CI";

/// The shared client facts, flattened into each payload at the position of
/// the field that holds them.
///
/// Agent and version facts are set here, client-side, rather than derived on
/// the ingest side from the User-Agent header: the CLI already holds both as
/// structured values, so putting them in the payload avoids brittle string
/// extraction. The User-Agent still carries the same facts as transport
/// metadata for prefix-based request filtering.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ClientFacts {
    pub is_agent: bool,
    /// Canonical id of the detected coding agent (e.g. "claude-code");
    /// `null` for human invocations. A `&'static str` from
    /// `AgentId::as_str`, so only the detector's closed vocabulary can reach it.
    pub agent: Option<&'static str>,
    pub ci: bool,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
}

impl ClientFacts {
    /// Detect the facts, reading `CI` through `env` so tests can inject it.
    /// The two agent fields come from one detection and can never disagree.
    pub fn detect(env: &dyn Fn(&str) -> Option<String>) -> Self {
        let detected = is_ai_agent::detect();
        Self {
            is_agent: detected.is_some(),
            agent: detected.map(|a| a.id.as_str()),
            ci: env_truthy(env(CI_ENV)),
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        }
    }
}

/// Read a process environment variable.
pub(crate) fn real_env_lookup(key: &str) -> Option<String> {
    // `var_os` + lossy conversion, not `var`: `std::env::var` returns an error
    // (read here as `None`) for a variable that is set but not valid UTF-8, so
    // a non-UTF-8 `DO_NOT_TRACK` would look absent and telemetry would fail
    // open. A set-but-non-UTF-8 value must still count as an opt-out; the
    // lossy string is non-empty and not "0"/"false", so `env_truthy` sees it
    // as set.
    std::env::var_os(key).map(|v| v.to_string_lossy().into_owned())
}

/// donottrack.sh-style truthiness: set, and not `""`/`"0"`/`"false"`.
pub(crate) fn env_truthy(value: Option<String>) -> bool {
    matches!(value.as_deref(), Some(v) if !v.is_empty() && v != "0" && v != "false")
}
