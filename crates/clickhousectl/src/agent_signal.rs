//! Detect when the CLI is being driven by an AI coding agent and surface that
//! signal as an `agent` query parameter on outbound clickhouse.com /
//! clickhouse.cloud requests, so server-side analytics can attribute usage.
//!
//! Detection is delegated to the `is_ai_agent` crate, which inspects standard
//! and tool-specific environment variables (e.g. `AGENT`, `CLAUDECODE`).

use reqwest::RequestBuilder;

/// Canonical kebab-case identifier for the detected agent, as defined by the
/// `is-ai-agent` crate (`AgentId::as_str`). Returns `None` when no agent
/// signal is present.
pub fn detected_agent_id() -> Option<&'static str> {
    is_ai_agent::detect().map(|a| a.id.as_str())
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
    fn detected_agent_id_uses_crate_canonical_id() {
        // Smoke test: when the crate's CLAUDECODE detection fires, we surface
        // its `AgentId::as_str` ("claude-code") verbatim. Captures the
        // contract we rely on rather than reasserting the crate's table.
        let agent = is_ai_agent::detect_with(
            |name| (name == "CLAUDECODE").then(|| "1".to_string()),
            |_| false,
        )
        .expect("CLAUDECODE should resolve to an Agent");
        assert_eq!(agent.id.as_str(), "claude-code");
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
