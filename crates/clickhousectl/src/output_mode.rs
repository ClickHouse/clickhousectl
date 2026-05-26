//! Resolves whether to emit machine-readable JSON output.
//!
//! Returns true if any of these hold:
//! - the explicit `--json` flag was passed
//! - a known coding-agent env var is set (CLAUDECODE / CURSOR_AGENT / CODEX_SANDBOX)
//! - stdout is not a terminal (piped/redirected)
//!
//! This mirrors the workos CLI's behavior so agents and pipelines get
//! structured output without needing to pass `--json` explicitly.

use std::io::IsTerminal;

const AGENT_ENV_VARS: &[&str] = &["CLAUDECODE", "CURSOR_AGENT", "CODEX_SANDBOX"];

pub fn should_output_json(flag: bool) -> bool {
    resolve(flag, agent_context_detected(), std::io::stdout().is_terminal())
}

pub fn agent_context_detected() -> bool {
    AGENT_ENV_VARS
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|v| !v.is_empty()))
}

fn resolve(flag: bool, agent_detected: bool, stdout_is_tty: bool) -> bool {
    flag || agent_detected || !stdout_is_tty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_flag_forces_json() {
        assert!(resolve(true, false, true));
        assert!(resolve(true, false, false));
    }

    #[test]
    fn agent_context_forces_json_even_on_tty() {
        assert!(resolve(false, true, true));
    }

    #[test]
    fn non_tty_forces_json() {
        assert!(resolve(false, false, false));
    }

    #[test]
    fn tty_without_flag_or_agent_stays_human() {
        assert!(!resolve(false, false, true));
    }
}
