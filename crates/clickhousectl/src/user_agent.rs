/// Returns the canonical user-agent string for all outbound HTTP requests.
///
/// When invoked under a known AI coding agent (Claude Code, Cursor, OpenAI
/// Codex, Gemini CLI, Goose, Devin, etc.), the agent's canonical id is
/// appended in parentheses — e.g. `clickhousectl/0.1.18 (agent=claude-code)`
/// — so server-side analytics can attribute usage. RFC 7231 allows comments
/// in the User-Agent header, and parenthesised key=value pairs are a
/// conventional shape (cf. browsers' `Mozilla/5.0 (Windows NT 10.0; ...)`).
pub fn user_agent() -> String {
    user_agent_from(is_ai_agent::detect().map(|agent| agent.id.as_str()))
}

/// Build the User-Agent from the detected agent id, if any.
fn user_agent_from(detected: Option<&str>) -> String {
    let base = format!("clickhousectl/{}", crate::build_info::VERSION);
    match detected {
        Some(id) => format!("{base} (agent={id})"),
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_invocation_is_just_name_and_version() {
        let ua = user_agent_from(None);
        assert_eq!(ua, format!("clickhousectl/{}", crate::build_info::VERSION));
        if cfg!(feature = "experimental") {
            assert!(ua.contains("experimental"), "got: {ua}");
        }
    }

    #[test]
    fn agent_invocation_appends_paren_comment() {
        assert_eq!(
            user_agent_from(Some("claude-code")),
            format!(
                "clickhousectl/{} (agent=claude-code)",
                crate::build_info::VERSION
            )
        );
    }

    #[test]
    fn live_user_agent_starts_with_canonical_prefix() {
        // Live process check: we can't control the env reliably across CI
        // hosts, but the prefix invariant must always hold.
        let ua = user_agent();
        let prefix = format!("clickhousectl/{}", crate::build_info::VERSION);
        assert!(ua == prefix || ua.starts_with(&format!("{prefix} (")));
    }
}
