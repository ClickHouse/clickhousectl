use crate::error::{NetworkCategory, NetworkFailure, NetworkStage};
use reqwest::{Client, ClientBuilder, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::time::Duration;

pub(crate) const METADATA_POLICY: RequestPolicy = RequestPolicy {
    connect_timeout: Duration::from_secs(3),
    read_timeout: Duration::from_secs(5),
    request_timeout: Duration::from_secs(8),
};
pub(crate) const RESOLUTION_OPERATION_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const LIST_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const DOWNLOAD_POLICY: RequestPolicy = RequestPolicy {
    connect_timeout: Duration::from_secs(10),
    read_timeout: Duration::from_secs(30),
    request_timeout: Duration::from_secs(30 * 60),
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RequestPolicy {
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub request_timeout: Duration,
}

#[derive(Debug)]
pub(crate) enum ProbeOutcome {
    Available,
    Missing,
    Failed(NetworkFailure),
}

pub(crate) fn configure_builder(builder: ClientBuilder, policy: RequestPolicy) -> ClientBuilder {
    builder
        .connect_timeout(policy.connect_timeout)
        .read_timeout(policy.read_timeout)
        .timeout(policy.request_timeout)
}

pub(crate) fn client(
    policy: RequestPolicy,
    stage: NetworkStage,
    url: &str,
) -> Result<Client, NetworkFailure> {
    configure_builder(crate::http::client_builder(), policy)
        .build()
        .map_err(|_| NetworkFailure::new(stage, url, NetworkCategory::Transport))
}

/// Environment variables that may hold a GitHub token, in the GitHub CLI's order of precedence.
const GITHUB_TOKEN_VARS: [&str; 2] = ["GH_TOKEN", "GITHUB_TOKEN"];
const GITHUB_API_HOST: &str = "api.github.com";

/// A GET request to the GitHub REST API. Unauthenticated requests share a
/// 60-per-hour limit per IP address, which CI runners and shared networks
/// exhaust quickly, so a token from `GH_TOKEN` or `GITHUB_TOKEN` is sent when set.
pub(crate) fn github_api_get(client: &Client, url: &str) -> RequestBuilder {
    let token = github_token_from(|name| std::env::var(name).ok());
    with_github_token(client.get(url), url, token.as_deref())
}

fn github_token_from(lookup: impl Fn(&str) -> Option<String>) -> Option<String> {
    GITHUB_TOKEN_VARS
        .iter()
        .filter_map(|name| lookup(name))
        .map(|token| token.trim().to_string())
        .find(|token| !token.is_empty())
}

/// Attach the token only to HTTPS requests for the GitHub API host, so it never
/// reaches another host. reqwest drops it on a redirect to another host.
fn with_github_token(request: RequestBuilder, url: &str, token: Option<&str>) -> RequestBuilder {
    let is_github_api = reqwest::Url::parse(url)
        .is_ok_and(|url| url.scheme() == "https" && url.host_str() == Some(GITHUB_API_HOST));
    match token {
        Some(token) if is_github_api => request.bearer_auth(token),
        _ => request,
    }
}

pub(crate) async fn send(
    request: RequestBuilder,
    stage: NetworkStage,
    url: &str,
) -> Result<Response, NetworkFailure> {
    request
        .send()
        .await
        .map_err(|error| failure_from_reqwest(stage, url, &error))
}

pub(crate) fn ensure_success(
    response: Response,
    stage: NetworkStage,
    url: &str,
) -> Result<Response, NetworkFailure> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(failure_from_status(stage, url, response.status()))
    }
}

pub(crate) async fn json<T: DeserializeOwned>(
    response: Response,
    stage: NetworkStage,
    url: &str,
) -> Result<T, NetworkFailure> {
    response
        .json()
        .await
        .map_err(|error| failure_from_reqwest(stage, url, &error))
}

pub(crate) async fn probe(client: &Client, url: &str, stage: NetworkStage) -> ProbeOutcome {
    let response = match send(client.head(url), stage, url).await {
        Ok(response) => response,
        Err(error) => return ProbeOutcome::Failed(error),
    };
    match response.status() {
        status if status.is_success() => ProbeOutcome::Available,
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => ProbeOutcome::Missing,
        status => ProbeOutcome::Failed(failure_from_status(stage, url, status)),
    }
}

pub(crate) async fn with_operation_timeout<T>(
    duration: Duration,
    stage: NetworkStage,
    url: &str,
    future: impl Future<Output = T>,
) -> Result<T, NetworkFailure> {
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| NetworkFailure::new(stage, url, NetworkCategory::Timeout))
}

pub(crate) fn failure_from_reqwest(
    stage: NetworkStage,
    url: &str,
    error: &reqwest::Error,
) -> NetworkFailure {
    let category = if error.is_timeout() {
        NetworkCategory::Timeout
    } else if error.is_connect() {
        NetworkCategory::Connection
    } else if error.is_decode() {
        NetworkCategory::InvalidResponse
    } else {
        NetworkCategory::Transport
    };
    NetworkFailure::new(stage, url, category)
}

pub(crate) fn failure_from_status(
    stage: NetworkStage,
    url: &str,
    status: StatusCode,
) -> NetworkFailure {
    let category = match status {
        StatusCode::FORBIDDEN => NetworkCategory::Forbidden,
        StatusCode::NOT_FOUND => NetworkCategory::NotFound,
        StatusCode::REQUEST_TIMEOUT => NetworkCategory::Timeout,
        StatusCode::TOO_MANY_REQUESTS => NetworkCategory::RateLimited,
        status if status.is_client_error() => NetworkCategory::ClientError,
        status if status.is_server_error() => NetworkCategory::ServerError,
        _ => NetworkCategory::UnexpectedStatus,
    };
    NetworkFailure::new(stage, url, category)
}

pub(crate) fn preferred_failure(
    current: Option<NetworkFailure>,
    candidate: NetworkFailure,
) -> Option<NetworkFailure> {
    match current {
        Some(current) if current.category.priority() >= candidate.category.priority() => {
            Some(current)
        }
        _ => Some(candidate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;
    use tokio::sync::Notify;

    fn authorization(url: &str, token: Option<&str>) -> Option<String> {
        let request = with_github_token(Client::new().get(url), url, token)
            .build()
            .unwrap();
        request
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .map(|value| value.to_str().unwrap().to_string())
    }

    #[test]
    fn github_api_requests_carry_the_token_as_a_bearer() {
        let url = "https://api.github.com/repos/ClickHouse/ClickHouse/releases?per_page=100";
        assert_eq!(
            authorization(url, Some("ghs_x")).as_deref(),
            Some("Bearer ghs_x")
        );
        assert_eq!(authorization(url, None), None);
    }

    #[test]
    fn the_token_never_reaches_another_host_or_plain_http() {
        for url in [
            "https://github.com/ClickHouse/ClickHouse/releases/download/v1/clickhouse",
            "https://builds.clickhouse.com/master/amd64/clickhouse",
            "https://api.github.com.example.test/repos",
            "http://api.github.com/repos",
            "http://127.0.0.1:8080/refs",
        ] {
            assert_eq!(authorization(url, Some("ghs_x")), None, "{url}");
        }
    }

    #[test]
    fn gh_token_takes_precedence_and_blank_values_are_skipped() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.to_string())
            }
        };
        let both = env(&[("GH_TOKEN", "gh"), ("GITHUB_TOKEN", "github")]);
        assert_eq!(github_token_from(both).as_deref(), Some("gh"));
        let blank_gh = env(&[("GH_TOKEN", " "), ("GITHUB_TOKEN", "github")]);
        assert_eq!(github_token_from(blank_gh).as_deref(), Some("github"));
        assert_eq!(github_token_from(env(&[("GITHUB_TOKEN", "")])), None);
        assert_eq!(github_token_from(env(&[])), None);
    }

    fn short_policy() -> RequestPolicy {
        RequestPolicy {
            connect_timeout: Duration::from_millis(50),
            read_timeout: Duration::from_millis(60),
            request_timeout: Duration::from_millis(100),
        }
    }

    #[test]
    fn status_categories_are_stable_and_urls_are_redacted() {
        let cases = [
            (StatusCode::FORBIDDEN, NetworkCategory::Forbidden),
            (StatusCode::NOT_FOUND, NetworkCategory::NotFound),
            (StatusCode::REQUEST_TIMEOUT, NetworkCategory::Timeout),
            (StatusCode::TOO_MANY_REQUESTS, NetworkCategory::RateLimited),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                NetworkCategory::ServerError,
            ),
        ];
        for (status, expected) in cases {
            let failure = failure_from_status(
                NetworkStage::BuildProbe,
                "https://user:password@builds.example.test/private?token=secret",
                status,
            );
            let message = failure.to_string();
            assert_eq!(failure.host, "builds.example.test");
            assert_eq!(failure.category, expected);
            assert!(!message.contains("user"));
            assert!(!message.contains("password"));
            assert!(!message.contains("private"));
            assert!(!message.contains("secret"));
        }
    }

    #[test]
    fn rate_limit_is_preferred_over_less_actionable_probe_failures() {
        let server = NetworkFailure::new(
            NetworkStage::BuildProbe,
            "https://builds.example.test",
            NetworkCategory::ServerError,
        );
        let rate_limit = NetworkFailure::new(
            NetworkStage::BuildProbe,
            "https://builds.example.test",
            NetworkCategory::RateLimited,
        );
        let selected = preferred_failure(Some(server), rate_limit.clone()).unwrap();
        assert_eq!(selected, rate_limit);
    }

    #[tokio::test]
    async fn stalled_response_headers_hit_the_read_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let hold = Arc::new(Notify::new());
        let server_hold = hold.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await;
            server_hold.notified().await;
        });
        let url = format!("http://{address}/headers?token=secret");
        let client = client(short_policy(), NetworkStage::VersionList, &url).unwrap();

        let failure = send(client.get(&url), NetworkStage::VersionList, &url)
            .await
            .unwrap_err();

        assert_eq!(failure.stage, NetworkStage::VersionList);
        assert_eq!(failure.host, "127.0.0.1");
        assert_eq!(failure.category, NetworkCategory::Timeout);
        assert!(!failure.to_string().contains("secret"));
        hold.notify_one();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn stalled_connect_proxy_is_bounded_and_redacted() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_address = listener.local_addr().unwrap();
        let hold = Arc::new(Notify::new());
        let server_hold = hold.clone();
        let proxy = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await;
            server_hold.notified().await;
        });
        let target = "https://user:password@example.test/private?token=secret";
        let builder = crate::http::client_builder()
            .proxy(reqwest::Proxy::all(format!("http://{proxy_address}")).unwrap());
        let client = configure_builder(builder, short_policy()).build().unwrap();

        let failure = send(client.head(target), NetworkStage::BuildProbe, target)
            .await
            .unwrap_err();

        assert_eq!(failure.host, "example.test");
        assert_eq!(failure.category, NetworkCategory::Timeout);
        let message = failure.to_string();
        assert!(!message.contains("user"));
        assert!(!message.contains("password"));
        assert!(!message.contains("secret"));
        hold.notify_one();
        proxy.await.unwrap();
    }
}
