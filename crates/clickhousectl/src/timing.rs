//! Hidden overrides for production waits.
//!
//! Subprocess tests set these so they do not sit out real timeouts and poll
//! intervals. They are not a user interface: no help text or user guide
//! mentions them (see `docs/development.md`).

use std::time::Duration;

/// Bounds how long `local udf deploy` keeps asking a server to reload.
pub const UDF_LOAD_TIMEOUT_ENV: &str = "CHCTL_UDF_LOAD_TIMEOUT_MS";
/// Gap between polls while `cloud service delete --force` waits for a stop.
pub const SERVICE_STOP_POLL_INTERVAL_ENV: &str = "CHCTL_SERVICE_STOP_POLL_INTERVAL_MS";
/// How long a SIGTERMed local server is given before SIGKILL.
pub const TERM_TIMEOUT_ENV: &str = "CHCTL_TERM_TIMEOUT_MS";
/// Gap between polls while `cloud udf` waits on a service.
pub const UDF_POLL_INTERVAL_ENV: &str = "CHCTL_UDF_POLL_INTERVAL_MS";
/// Bounds the Query API readiness wait after a key is bound.
pub const QUERY_READINESS_TIMEOUT_ENV: &str = "CHCTL_QUERY_READINESS_TIMEOUT_MS";
/// Bounds how long a key-binding upsert is retried while a new key propagates.
pub const KEY_PROPAGATION_TIMEOUT_ENV: &str = "CHCTL_KEY_PROPAGATION_TIMEOUT_MS";
/// Bounds the background and `--version` update checks; `0` skips them.
pub const UPDATE_CHECK_TIMEOUT_ENV: &str = "CHCTL_UPDATE_CHECK_TIMEOUT_MS";
/// Bounds the `feedback` send.
#[cfg(feature = "feedback")]
pub const FEEDBACK_SEND_TIMEOUT_ENV: &str = "CHCTL_FEEDBACK_SEND_TIMEOUT_MS";
/// Bounds the `feedback` whoami lookup for Cloud organization IDs.
#[cfg(feature = "feedback")]
pub const FEEDBACK_WHOAMI_TIMEOUT_ENV: &str = "CHCTL_FEEDBACK_WHOAMI_TIMEOUT_MS";

/// `default`, unless `env` holds a whole number of milliseconds.
///
/// Anything else (unset, empty, not a number) keeps the default, so a stray
/// value can never turn a wait into an error.
pub fn duration_override(env: &str, default: Duration) -> Duration {
    parse_override(std::env::var(env).ok().as_deref(), default)
}

fn parse_override(value: Option<&str>, default: Duration) -> Duration {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(default, Duration::from_millis)
}

#[cfg(test)]
mod tests {
    use super::parse_override;
    use std::time::Duration;

    // The parser, not the process environment: `set_var` is `unsafe` in
    // edition 2024 because it races other threads' reads.
    #[test]
    fn overrides_are_whole_milliseconds_and_anything_else_keeps_the_default() {
        let default = Duration::from_secs(7);
        for (value, expected) in [
            (None, default),
            (Some("250"), Duration::from_millis(250)),
            (Some(" 0 "), Duration::ZERO),
            (Some(""), default),
            (Some("1.5"), default),
            (Some("-1"), default),
            (Some("soon"), default),
        ] {
            assert_eq!(parse_override(value, default), expected, "{value:?}");
        }
    }
}
