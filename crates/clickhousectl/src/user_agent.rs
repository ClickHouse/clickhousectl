/// Returns the canonical user-agent string for all outbound HTTP requests.
///
/// When invoked under a known AI coding agent (Claude Code, Cursor, OpenAI
/// Codex, Gemini CLI, Goose, Devin, etc.), the agent's canonical id is
/// appended in parentheses — e.g. `clickhousectl/0.1.18 (agent=claude-code)`
/// — so server-side analytics can attribute usage. RFC 7231 allows comments
/// in the User-Agent header, and parenthesised key=value pairs are a
/// conventional shape (cf. browsers' `Mozilla/5.0 (Windows NT 10.0; ...)`).
pub fn user_agent() -> String {
    let base = format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"));
    match is_ai_agent::detect() {
        Some(agent) => format!("{} (agent={})", base, agent.id.as_str()),
        None => base,
    }
}

/// Build a User-Agent string from an explicit (test-injected) detected agent.
/// Mirrors `user_agent` but lets tests exercise both branches deterministically.
#[cfg(test)]
fn user_agent_from(detected: Option<&str>) -> String {
    let base = format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"));
    match detected {
        Some(id) => format!("{} (agent={})", base, id),
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_invocation_is_just_name_and_version() {
        assert_eq!(
            user_agent_from(None),
            format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn agent_invocation_appends_paren_comment() {
        assert_eq!(
            user_agent_from(Some("claude-code")),
            format!("clickhousectl/{} (agent=claude-code)", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn live_user_agent_starts_with_canonical_prefix() {
        // Live process check: we can't control the env reliably across CI
        // hosts, but the prefix invariant must always hold.
        let ua = user_agent();
        let prefix = format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"));
        assert!(ua == prefix || ua.starts_with(&format!("{prefix} (")));
    }
}
