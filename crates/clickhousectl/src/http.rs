//! Canonical construction of outbound HTTP clients.
//!
//! Every `reqwest::Client` the CLI builds — Cloud API, OAuth, the updater, the
//! version manager — goes through [`client_builder`], so they uniformly carry
//! the `User-Agent` (built in `crate::user_agent`) and the agent
//! session/trace correlation headers, and any future builder picks these up
//! for free.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// `agent-session-id`: the calling agent's stable per-session/conversation id.
const AGENT_SESSION_ID: HeaderName = HeaderName::from_static("agent-session-id");
/// `traceparent`: the raw W3C Trace Context value the agent published.
const TRACEPARENT: HeaderName = HeaderName::from_static("traceparent");

/// Default headers that correlate every outbound request with the calling AI
/// agent's session/trace, so backend telemetry can group a single agent run's
/// calls. Empty when not running under a detected agent (or the agent exposes
/// neither id).
pub fn agent_headers() -> HeaderMap {
    match is_ai_agent::detect() {
        Some(agent) => {
            agent_headers_from(agent.session_id.as_deref(), agent.traceparent.as_deref())
        }
        None => HeaderMap::new(),
    }
}

/// Build the header map from the raw id values. Split out from [`agent_headers`]
/// so it can be unit-tested without constructing an `is_ai_agent::Agent` (which
/// is `#[non_exhaustive]` and not constructible outside its crate).
fn agent_headers_from(session_id: Option<&str>, traceparent: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    // Session ids / traceparent are opaque vendor strings; an invalid header
    // value is dropped rather than panicking.
    if let Some(value) = session_id.and_then(|s| HeaderValue::from_str(s).ok()) {
        headers.insert(AGENT_SESSION_ID, value);
    }
    if let Some(value) = traceparent.and_then(|s| HeaderValue::from_str(s).ok()) {
        headers.insert(TRACEPARENT, value);
    }
    headers
}

/// Canonical `reqwest::ClientBuilder` for all outbound HTTP: pre-applies the
/// `User-Agent` and the agent session/trace default headers. Callers chain any
/// extra config (`.timeout(..)`, etc.) and `.build()`.
pub fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .default_headers(agent_headers())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_ids_present_yields_both_headers() {
        let headers = agent_headers_from(Some("sess-123"), Some("00-abc-def-01"));
        assert_eq!(headers.get("agent-session-id").unwrap(), "sess-123");
        assert_eq!(headers.get("traceparent").unwrap(), "00-abc-def-01");
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn no_ids_yields_empty_map() {
        let headers = agent_headers_from(None, None);
        assert!(headers.is_empty());
    }

    #[test]
    fn session_only_yields_single_header() {
        let headers = agent_headers_from(Some("sess-123"), None);
        assert_eq!(headers.get("agent-session-id").unwrap(), "sess-123");
        assert!(headers.get("traceparent").is_none());
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn traceparent_only_yields_single_header() {
        let headers = agent_headers_from(None, Some("00-abc-def-01"));
        assert_eq!(headers.get("traceparent").unwrap(), "00-abc-def-01");
        assert!(headers.get("agent-session-id").is_none());
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn invalid_header_value_is_dropped_without_panic() {
        // A newline is not a legal header value; the entry is skipped, the
        // valid one still lands.
        let headers = agent_headers_from(Some("bad\nvalue"), Some("00-ok-01"));
        assert!(headers.get("agent-session-id").is_none());
        assert_eq!(headers.get("traceparent").unwrap(), "00-ok-01");
    }

    #[test]
    fn client_builder_builds() {
        // Smoke: the canonical builder produces a usable client.
        let _client = client_builder().build().unwrap();
    }
}
