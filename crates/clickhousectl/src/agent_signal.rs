//! Detect when the CLI is being driven by an AI coding agent and surface that
//! signal as an `agent` query parameter on outbound clickhouse.com /
//! clickhouse.cloud requests, so server-side analytics can attribute usage.
//!
//! Detection is delegated to the `is_ai_agent` crate, which inspects standard
//! and tool-specific environment variables (e.g. `AGENT`, `CLAUDECODE`).

use is_ai_agent::AgentId;
use reqwest::RequestBuilder;

/// Canonical kebab-case identifier for the detected agent, suitable for use
/// as a URL query value. Returns `None` when no agent signal is present.
pub fn detected_agent_id() -> Option<&'static str> {
    is_ai_agent::detect().map(|a| agent_id_str(a.id))
}

fn agent_id_str(id: AgentId) -> &'static str {
    match id {
        AgentId::ClaudeCode => "claude-code",
        AgentId::Cursor => "cursor",
        AgentId::GeminiCli => "gemini-cli",
        AgentId::Codex => "codex",
        AgentId::Augment => "augment",
        AgentId::Cline => "cline",
        AgentId::OpenCode => "opencode",
        AgentId::Trae => "trae",
        AgentId::Goose => "goose",
        AgentId::Amp => "amp",
        AgentId::Devin => "devin",
        AgentId::Unknown => "unknown",
    }
}

/// Append the `agent=<id>` query parameter to a request when an AI coding
/// agent is detected. Pass-through when no agent is present.
pub fn add_agent_query(builder: RequestBuilder) -> RequestBuilder {
    match detected_agent_id() {
        Some(id) => builder.query(&[("agent", id)]),
        None => builder,
    }
}

/// Like `add_agent_query`, but only annotates requests targeting
/// ClickHouse-owned hosts (so we don't leak the signal to GitHub or other
/// third-party download mirrors).
pub fn add_agent_query_for(builder: RequestBuilder, url: &str) -> RequestBuilder {
    if is_clickhouse_url(url) {
        add_agent_query(builder)
    } else {
        builder
    }
}

fn is_clickhouse_url(url: &str) -> bool {
    let host = match url.split_once("://") {
        Some((_, rest)) => rest.split('/').next().unwrap_or(""),
        None => url,
    };
    let host = host.split(':').next().unwrap_or(host);
    host == "clickhouse.com"
        || host == "clickhouse.cloud"
        || host.ends_with(".clickhouse.com")
        || host.ends_with(".clickhouse.cloud")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_known_agent_id() {
        // Exhaustive smoke check that no AgentId variant gets the empty string.
        for id in [
            AgentId::ClaudeCode,
            AgentId::Cursor,
            AgentId::GeminiCli,
            AgentId::Codex,
            AgentId::Augment,
            AgentId::Cline,
            AgentId::OpenCode,
            AgentId::Trae,
            AgentId::Goose,
            AgentId::Amp,
            AgentId::Devin,
            AgentId::Unknown,
        ] {
            let s = agent_id_str(id);
            assert!(!s.is_empty());
            // Identifier must be URL-safe — no spaces or uppercase.
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    #[test]
    fn claude_code_id_is_kebab_case() {
        assert_eq!(agent_id_str(AgentId::ClaudeCode), "claude-code");
        assert_eq!(agent_id_str(AgentId::GeminiCli), "gemini-cli");
    }

    #[test]
    fn detects_clickhouse_owned_hosts() {
        assert!(is_clickhouse_url("https://builds.clickhouse.com/master/amd64/clickhouse"));
        assert!(is_clickhouse_url(
            "https://packages.clickhouse.com/tgz/stable/clickhouse-common-static-25.12.9.61-amd64.tgz"
        ));
        assert!(is_clickhouse_url("https://api.clickhouse.cloud/v1/organizations"));
        assert!(is_clickhouse_url("https://clickhouse.com/docs/cloud/"));
    }

    #[test]
    fn rejects_non_clickhouse_hosts() {
        assert!(!is_clickhouse_url(
            "https://github.com/ClickHouse/ClickHouse/releases/download/v25.12.5.44-stable/clickhouse-macos-aarch64"
        ));
        assert!(!is_clickhouse_url("https://api.github.com/repos/ClickHouse/ClickHouse/releases"));
    }

    #[test]
    fn rejects_lookalike_hosts() {
        // Suffix match must be a true subdomain, not a substring of an attacker-controlled host.
        assert!(!is_clickhouse_url("https://evil-clickhouse.com/path"));
        assert!(!is_clickhouse_url("https://clickhouse.com.attacker.com/x"));
    }
}
