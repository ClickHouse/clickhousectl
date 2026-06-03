//! Resolves whether to emit machine-readable JSON output.
//!
//! Returns true when `--json` was passed or we're running under a known coding
//! agent — so agents get structured output without callers having to opt in.
//! For everyone else (including non-TTY pipes/redirects) output stays
//! human-readable unless `--json` is passed, matching `gh`/`kubectl` norms.

pub fn should_output_json(flag: bool) -> bool {
    resolve(flag, agent_context_detected())
}

/// Shares the same detection used for the outbound User-Agent (`user_agent.rs`),
/// so the two stay consistent and cover every agent the `is-ai-agent` crate
/// knows about (the standard `AGENT` var, Claude Code, Cursor, Codex, Gemini
/// CLI, Goose, Devin, …) rather than a hand-maintained subset.
fn agent_context_detected() -> bool {
    is_ai_agent::detect().is_some()
}

fn resolve(flag: bool, agent_detected: bool) -> bool {
    flag || agent_detected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_flag_forces_json() {
        assert!(resolve(true, false));
    }

    #[test]
    fn agent_context_forces_json() {
        assert!(resolve(false, true));
    }

    #[test]
    fn no_flag_no_agent_stays_human() {
        assert!(!resolve(false, false));
    }
}
