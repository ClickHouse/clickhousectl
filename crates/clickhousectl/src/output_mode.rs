//! Resolves whether to emit machine-readable JSON output.
//!
//! Returns true when `--json` was passed, stdout is not a terminal, or we're
//! running under a known coding agent — so agents and pipelines get structured
//! output without callers having to opt in.

use std::io::IsTerminal;

pub fn should_output_json(flag: bool) -> bool {
    resolve(flag, agent_context_detected(), std::io::stdout().is_terminal())
}

/// Shares the same detection used for the outbound User-Agent (`user_agent.rs`),
/// so the two stay consistent and cover every agent the `is-ai-agent` crate
/// knows about (the standard `AGENT` var, Claude Code, Cursor, Codex, Gemini
/// CLI, Goose, Devin, …) rather than a hand-maintained subset.
fn agent_context_detected() -> bool {
    is_ai_agent::detect().is_some()
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
