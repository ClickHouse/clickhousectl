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
