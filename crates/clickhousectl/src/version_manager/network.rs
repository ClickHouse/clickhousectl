use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::RETRY_AFTER;
use std::fmt;
use std::time::Duration;
use thiserror::Error;
use tokio::time::Instant;

const METADATA_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const METADATA_READ_TIMEOUT: Duration = Duration::from_secs(10);
const METADATA_TOTAL_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_READ_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TOTAL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkStage {
    BuildProbe,
    BuildsList,
    GithubLookup,
    VersionList,
    MasterCheck,
    Download,
}

impl fmt::Display for NetworkStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::BuildProbe => "build probe",
            Self::BuildsList => "builds list",
            Self::GithubLookup => "GitHub version lookup",
            Self::VersionList => "GitHub version list",
            Self::MasterCheck => "master check",
            Self::Download => "download",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkCategory {
    Timeout,
    Connect,
    Transport,
    InvalidResponse,
    Forbidden,
    NotFound,
    RateLimited,
    Server,
    Client,
    UnexpectedStatus,
}

impl fmt::Display for NetworkCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::Timeout => "timeout",
            Self::Connect => "connection error",
            Self::Transport => "transport error",
            Self::InvalidResponse => "invalid response",
            Self::Forbidden => "forbidden (HTTP 403)",
            Self::NotFound => "not found (HTTP 404)",
            Self::RateLimited => "rate limited (HTTP 429)",
            Self::Server => "server error (HTTP 5xx)",
            Self::Client => "client error (HTTP 4xx)",
            Self::UnexpectedStatus => "unexpected HTTP status",
        };
        f.write_str(category)
    }
}

#[derive(Debug, Clone, Error)]
#[error("{stage} request to {host} failed: {category}")]
pub(crate) struct NetworkFailure {
    pub(crate) stage: NetworkStage,
    pub(crate) host: String,
    pub(crate) category: NetworkCategory,
    pub(crate) retry_after: Option<Duration>,
}

impl NetworkFailure {
    pub(crate) fn from_request(stage: NetworkStage, url: &str, error: &reqwest::Error) -> Self {
        let category = if error.is_timeout() {
            NetworkCategory::Timeout
        } else if error.is_connect() {
            NetworkCategory::Connect
        } else if error.is_decode() {
            NetworkCategory::InvalidResponse
        } else {
            NetworkCategory::Transport
        };
        Self::new(stage, url, category, None)
    }

    pub(crate) fn from_response(
        stage: NetworkStage,
        url: &str,
        response: &reqwest::Response,
    ) -> Self {
        let status = response.status();
        let category = match status.as_u16() {
            403 => NetworkCategory::Forbidden,
            404 => NetworkCategory::NotFound,
            408 => NetworkCategory::Timeout,
            429 => NetworkCategory::RateLimited,
            _ if status.is_server_error() => NetworkCategory::Server,
            _ if status.is_client_error() => NetworkCategory::Client,
            _ => NetworkCategory::UnexpectedStatus,
        };
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_retry_after);
        Self::new(stage, url, category, retry_after)
    }

    pub(crate) fn timeout(stage: NetworkStage, url: &str) -> Self {
        Self::new(stage, url, NetworkCategory::Timeout, None)
    }

    pub(crate) fn is_retryable(&self) -> bool {
        matches!(
            self.category,
            NetworkCategory::Timeout
                | NetworkCategory::Connect
                | NetworkCategory::Transport
                | NetworkCategory::RateLimited
                | NetworkCategory::Server
        )
    }

    fn new(
        stage: NetworkStage,
        url: &str,
        category: NetworkCategory,
        retry_after: Option<Duration>,
    ) -> Self {
        Self {
            stage,
            host: reqwest::Url::parse(url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or_else(|| "unknown host".to_string()),
            category,
            retry_after,
        }
    }
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let retry_at = DateTime::parse_from_rfc2822(value)
        .map(|date| date.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%A, %d-%b-%y %H:%M:%S GMT")
                .map(|date| date.and_utc())
        })
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%a %b %e %H:%M:%S %Y").map(|date| date.and_utc())
        })
        .ok()?;
    retry_at.signed_duration_since(Utc::now()).to_std().ok()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Timeouts {
    connect: Duration,
    read: Duration,
    total: Duration,
}

impl Timeouts {
    const METADATA: Self = Self {
        connect: METADATA_CONNECT_TIMEOUT,
        read: METADATA_READ_TIMEOUT,
        total: METADATA_TOTAL_TIMEOUT,
    };
    const DOWNLOAD: Self = Self {
        connect: DOWNLOAD_CONNECT_TIMEOUT,
        read: DOWNLOAD_READ_TIMEOUT,
        total: DOWNLOAD_TOTAL_TIMEOUT,
    };

    #[cfg(test)]
    pub(crate) const fn new(connect: Duration, read: Duration, total: Duration) -> Self {
        Self {
            connect,
            read,
            total,
        }
    }
}

#[derive(Clone)]
pub(crate) struct OperationClient {
    inner: reqwest::Client,
    deadline: Instant,
}

impl OperationClient {
    pub(crate) fn metadata(stage: NetworkStage, url: &str) -> Result<Self, NetworkFailure> {
        Self::build(Timeouts::METADATA)
            .map_err(|error| NetworkFailure::from_request(stage, url, &error))
    }

    pub(crate) fn download(url: &str) -> Result<Self, NetworkFailure> {
        Self::build(Timeouts::DOWNLOAD)
            .map_err(|error| NetworkFailure::from_request(NetworkStage::Download, url, &error))
    }

    fn build(timeouts: Timeouts) -> reqwest::Result<Self> {
        let inner = crate::http::client_builder()
            .connect_timeout(timeouts.connect)
            .read_timeout(timeouts.read)
            .timeout(timeouts.total)
            .build()?;
        Ok(Self {
            inner,
            deadline: Instant::now() + timeouts.total,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_timeouts(timeouts: Timeouts) -> reqwest::Result<Self> {
        Self::build(timeouts)
    }

    pub(crate) async fn get(
        &self,
        url: &str,
        stage: NetworkStage,
    ) -> Result<reqwest::Response, NetworkFailure> {
        self.send(self.inner.get(url), url, stage).await
    }

    pub(crate) async fn head(
        &self,
        url: &str,
        stage: NetworkStage,
    ) -> Result<reqwest::Response, NetworkFailure> {
        self.send(self.inner.head(url), url, stage).await
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        url: &str,
        stage: NetworkStage,
    ) -> Result<reqwest::Response, NetworkFailure> {
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| NetworkFailure::timeout(stage, url))?;
        request
            .timeout(remaining)
            .send()
            .await
            .map_err(|error| NetworkFailure::from_request(stage, url, &error))
    }

    pub(crate) async fn sleep(&self, duration: Duration) {
        let wake_at = (Instant::now() + duration).min(self.deadline);
        tokio::time::sleep_until(wake_at).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn retry_after_seconds_are_parsed() {
        assert_eq!(parse_retry_after("7"), Some(Duration::from_secs(7)));
    }

    #[test]
    fn retry_after_http_date_forms_are_parsed() {
        for value in [
            "Sat, 06 Nov 2060 08:49:37 GMT",
            "Saturday, 06-Nov-60 08:49:37 GMT",
            "Sat Nov  6 08:49:37 2060",
        ] {
            assert!(parse_retry_after(value).is_some(), "{value}");
        }
    }

    #[test]
    fn expired_retry_after_dates_are_ignored() {
        let past = Utc::now() - chrono::Duration::seconds(30);
        for value in [
            past.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
            past.format("%A, %d-%b-%y %H:%M:%S GMT").to_string(),
            past.format("%a %b %e %H:%M:%S %Y").to_string(),
        ] {
            assert_eq!(parse_retry_after(&value), None, "{value}");
        }
    }

    #[test]
    fn errors_expose_only_stage_host_and_category() {
        let failure = NetworkFailure::new(
            NetworkStage::Download,
            "https://user:secret@example.com/private?token=secret",
            NetworkCategory::RateLimited,
            None,
        );

        assert_eq!(failure.host, "example.com");
        assert_eq!(
            failure.to_string(),
            "download request to example.com failed: rate limited (HTTP 429)"
        );
        assert!(!failure.to_string().contains("secret"));
    }

    #[tokio::test]
    async fn total_deadline_bounds_stalled_headers() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        let url = format!("http://{address}/stalled");
        let client = OperationClient::with_timeouts(Timeouts::new(
            Duration::from_millis(100),
            Duration::from_secs(1),
            Duration::from_millis(50),
        ))
        .unwrap();

        let error = client
            .get(&url, NetworkStage::GithubLookup)
            .await
            .unwrap_err();

        assert_eq!(error.stage, NetworkStage::GithubLookup);
        assert_eq!(error.host, "127.0.0.1");
        assert_eq!(error.category, NetworkCategory::Timeout);
    }
}
