//! Canonical construction of outbound HTTP clients.
//!
//! Two builders, chosen by who receives the request:
//!
//! - [`client_builder`]: the `User-Agent` (built in `crate::user_agent`) only.
//!   For requests that may reach a host ClickHouse does not run — the updater
//!   (GitHub release fallback and downloads), the version manager (GitHub API
//!   and release assets) — and for anonymous telemetry, which must not be
//!   correlated with an agent session.
//! - [`clickhouse_client_builder`]: the `User-Agent` plus the agent
//!   session/trace correlation headers (`agent-session-id`, `traceparent`).
//!   Only for ClickHouse-operated hosts whose backends group a single agent
//!   run's calls: the Cloud API, OAuth, and `feedback`.
//!
//! Clients that talk to something other than these do not use either builder:
//! the Cloud UDF artifact upload (`crate::cloud::udfs`) keeps a bare client so
//! nothing reaches the storage host but the presigned URL, and the loopback
//! probes of a local server (`crate::local::server`, `crate::local::udf`) send
//! nothing worth identifying.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// `agent-session-id`: the calling agent's stable per-session/conversation id.
const AGENT_SESSION_ID: HeaderName = HeaderName::from_static("agent-session-id");
/// `traceparent`: the raw W3C Trace Context value the agent published.
const TRACEPARENT: HeaderName = HeaderName::from_static("traceparent");

/// Default headers that correlate a request to a ClickHouse host with the
/// calling AI agent's session/trace, so backend telemetry can group a single
/// agent run's calls. Empty when not running under a detected agent (or the agent exposes
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

/// `reqwest::ClientBuilder` with only the `User-Agent`: the default for
/// outbound HTTP. Callers chain any extra config (`.timeout(..)`, etc.) and
/// `.build()`.
pub fn client_builder() -> reqwest::ClientBuilder {
    builder_with_headers(HeaderMap::new())
}

/// `reqwest::ClientBuilder` for ClickHouse-operated hosts only: the
/// `User-Agent` plus the agent session/trace correlation headers. Never use it
/// for a client that can reach a third-party host, including via redirects.
pub fn clickhouse_client_builder() -> reqwest::ClientBuilder {
    builder_with_headers(agent_headers())
}

fn builder_with_headers(headers: HeaderMap) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .default_headers(headers)
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

    async fn received_headers(builder: reqwest::ClientBuilder) -> reqwest::header::HeaderMap {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;
        builder
            .build()
            .unwrap()
            .get(server.uri())
            .send()
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        requests[0].headers.clone()
    }

    #[tokio::test]
    async fn plain_builder_sends_user_agent_without_correlation_headers() {
        let headers = received_headers(client_builder()).await;
        assert_eq!(
            headers.get("user-agent").unwrap(),
            crate::user_agent::user_agent().as_str()
        );
        assert!(headers.get("agent-session-id").is_none());
        assert!(headers.get("traceparent").is_none());
    }

    #[tokio::test]
    async fn clickhouse_builder_sends_user_agent_and_correlation_headers() {
        // `clickhouse_client_builder` reads the detected agent from the
        // environment; drive the same construction with fixed ids instead.
        let builder = builder_with_headers(agent_headers_from(Some("sess-123"), Some("00-abc-01")));
        let headers = received_headers(builder).await;
        assert_eq!(
            headers.get("user-agent").unwrap(),
            crate::user_agent::user_agent().as_str()
        );
        assert_eq!(headers.get("agent-session-id").unwrap(), "sess-123");
        assert_eq!(headers.get("traceparent").unwrap(), "00-abc-01");
    }
}
