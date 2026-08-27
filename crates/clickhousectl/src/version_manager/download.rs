use crate::error::{Error, NetworkFailure, NetworkStage, Result};
use crate::version_manager::network;
use crate::version_manager::platform::{DownloadSource, Platform};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use reqwest::header::RETRY_AFTER;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const DOWNLOAD_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const INSTALL_RETRY_POLICY: RetryPolicy = RetryPolicy {
    max_attempts: 3,
    base_delay: Duration::from_millis(500),
    max_delay: Duration::from_secs(5),
    operation_timeout: DOWNLOAD_OPERATION_TIMEOUT,
};

#[derive(Debug, Clone, Copy)]
struct RetryPolicy {
    max_attempts: usize,
    base_delay: Duration,
    max_delay: Duration,
    operation_timeout: Duration,
}

struct DownloadAttemptFailure {
    failure: NetworkFailure,
    retryable: bool,
    retry_after: Option<Duration>,
}

enum DownloadAttemptError {
    Network(DownloadAttemptFailure),
    Io(std::io::Error),
}

/// Downloads from a DownloadSource to the specified path.
pub async fn download_from_source(
    source: &DownloadSource,
    platform: &Platform,
    dest_path: &Path,
) -> Result<()> {
    let url = source.url(platform);
    download_url(&url, dest_path).await
}

/// Downloads a file with bounded connect, idle-read and total deadlines.
/// Only the idempotent GET is retried, and every attempt truncates the partial
/// destination before writing so bytes from failed streams cannot be mixed.
pub async fn download_url(url: &str, dest_path: &Path) -> Result<()> {
    download_url_with_policy(
        url,
        dest_path,
        network::DOWNLOAD_POLICY,
        INSTALL_RETRY_POLICY,
    )
    .await
}

async fn download_url_with_policy(
    url: &str,
    dest_path: &Path,
    request_policy: network::RequestPolicy,
    retry_policy: RetryPolicy,
) -> Result<()> {
    let download = download_with_retries(url, dest_path, request_policy, retry_policy);
    match network::with_operation_timeout(
        retry_policy.operation_timeout,
        NetworkStage::Download,
        url,
        download,
    )
    .await
    {
        Ok(result) => result,
        Err(failure) => {
            let _ = tokio::fs::remove_file(dest_path).await;
            Err(failure.into())
        }
    }
}

async fn download_with_retries(
    url: &str,
    dest_path: &Path,
    request_policy: network::RequestPolicy,
    retry_policy: RetryPolicy,
) -> Result<()> {
    let client = network::client(request_policy, NetworkStage::DownloadHeaders, url)?;
    for attempt in 1..=retry_policy.max_attempts {
        match download_once(&client, url, dest_path).await {
            Ok(()) => return Ok(()),
            Err(DownloadAttemptError::Io(error)) => return Err(Error::Io(error)),
            Err(DownloadAttemptError::Network(error))
                if error.retryable && attempt < retry_policy.max_attempts =>
            {
                let delay = retry_delay(&retry_policy, attempt, error.retry_after);
                tokio::time::sleep(delay).await;
            }
            Err(DownloadAttemptError::Network(error)) => {
                let _ = tokio::fs::remove_file(dest_path).await;
                let failure = if error.retryable {
                    error.failure.after_attempts(attempt)
                } else {
                    error.failure
                };
                return Err(failure.into());
            }
        }
    }
    unreachable!("retry policy always executes at least one attempt")
}

async fn download_once(
    client: &reqwest::Client,
    url: &str,
    dest_path: &Path,
) -> std::result::Result<(), DownloadAttemptError> {
    let response = network::send(client.get(url), NetworkStage::DownloadHeaders, url)
        .await
        .map_err(|failure| {
            DownloadAttemptError::Network(DownloadAttemptFailure {
                failure,
                retryable: true,
                retry_after: None,
            })
        })?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| parse_retry_after(value, Utc::now()));
        return Err(DownloadAttemptError::Network(DownloadAttemptFailure {
            failure: network::failure_from_status(NetworkStage::DownloadHeaders, url, status),
            retryable: status == StatusCode::REQUEST_TIMEOUT
                || status == StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error(),
            retry_after,
        }));
    }

    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut file = tokio::fs::File::create(dest_path)
        .await
        .map_err(DownloadAttemptError::Io)?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                pb.abandon();
                return Err(DownloadAttemptError::Network(DownloadAttemptFailure {
                    failure: network::failure_from_reqwest(NetworkStage::DownloadBody, url, &error),
                    retryable: true,
                    retry_after: None,
                }));
            }
        };
        file.write_all(&chunk)
            .await
            .map_err(DownloadAttemptError::Io)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    file.flush().await.map_err(DownloadAttemptError::Io)?;
    file.shutdown().await.map_err(DownloadAttemptError::Io)?;
    pb.finish_with_message("Download complete");
    Ok(())
}

fn retry_delay(policy: &RetryPolicy, attempt: usize, retry_after: Option<Duration>) -> Duration {
    let exponent = u32::try_from(attempt.saturating_sub(1)).unwrap_or(u32::MAX);
    let multiplier = 2_u32.checked_pow(exponent).unwrap_or(u32::MAX);
    let backoff = policy.base_delay.saturating_mul(multiplier);
    backoff
        .max(retry_after.unwrap_or_default())
        .min(policy.max_delay)
}

fn parse_retry_after(value: &str, now: DateTime<Utc>) -> Option<Duration> {
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let deadline = DateTime::parse_from_rfc2822(value)
        .map(|date| date.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%A, %d-%b-%y %H:%M:%S GMT")
                .map(|date| date.and_utc())
        })
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%a %b %e %H:%M:%S %Y").map(|date| date.and_utc())
        })
        .ok()?;
    (deadline - now).to_std().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{NetworkCategory, NetworkStage};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Semaphore;

    fn test_request_policy() -> network::RequestPolicy {
        network::RequestPolicy {
            connect_timeout: Duration::from_millis(50),
            read_timeout: Duration::from_millis(40),
            request_timeout: Duration::from_millis(100),
        }
    }

    fn test_retry_policy(max_attempts: usize) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            base_delay: Duration::from_millis(2),
            max_delay: Duration::from_millis(40),
            operation_timeout: Duration::from_millis(500),
        }
    }

    async fn scripted_status_server(
        statuses: Vec<(&'static str, Option<&'static str>)>,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let server_requests = requests.clone();
        let server = tokio::spawn(async move {
            for (status, retry_after) in statuses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 1024];
                let _ = socket.read(&mut request).await;
                server_requests.fetch_add(1, Ordering::SeqCst);
                let retry_header = retry_after
                    .map(|value| format!("Retry-After: {value}\r\n"))
                    .unwrap_or_default();
                let body = if status.starts_with("200") { "ok" } else { "" };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{retry_header}Connection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (format!("http://{address}/artifact"), requests, server)
    }

    #[test]
    fn retry_after_supports_seconds_and_all_http_date_forms() {
        let now = DateTime::parse_from_rfc2822("Wed, 21 Oct 2015 07:28:00 GMT")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(parse_retry_after("7", now), Some(Duration::from_secs(7)));
        for value in [
            "Wed, 21 Oct 2015 07:28:05 GMT",
            "Wednesday, 21-Oct-15 07:28:05 GMT",
            "Wed Oct 21 07:28:05 2015",
        ] {
            assert_eq!(
                parse_retry_after(value, now),
                Some(Duration::from_secs(5)),
                "{value}"
            );
        }
    }

    #[test]
    fn expired_retry_after_dates_are_ignored() {
        let now = DateTime::parse_from_rfc2822("Wed, 21 Oct 2015 07:28:00 GMT")
            .unwrap()
            .with_timezone(&Utc);
        for value in [
            "Wed, 21 Oct 2015 07:27:59 GMT",
            "Wednesday, 21-Oct-15 07:27:59 GMT",
            "Wed Oct 21 07:27:59 2015",
        ] {
            assert_eq!(parse_retry_after(value, now), None, "{value}");
        }
    }

    #[tokio::test]
    async fn rate_limit_retry_respects_capped_retry_after_then_succeeds() {
        let (url, requests, server) =
            scripted_status_server(vec![("429 Too Many Requests", Some("1")), ("200 OK", None)])
                .await;
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("artifact");
        let started = tokio::time::Instant::now();

        download_url_with_policy(
            &url,
            &destination,
            test_request_policy(),
            test_retry_policy(2),
        )
        .await
        .unwrap();

        assert!(started.elapsed() >= Duration::from_millis(40));
        assert_eq!(requests.load(Ordering::SeqCst), 2);
        assert_eq!(tokio::fs::read(destination).await.unwrap(), b"ok");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn server_errors_stop_after_retry_exhaustion() {
        let (url, requests, server) = scripted_status_server(vec![
            ("503 Service Unavailable", None),
            ("503 Service Unavailable", None),
            ("503 Service Unavailable", None),
        ])
        .await;
        let temp = tempfile::tempdir().unwrap();

        let error = download_url_with_policy(
            &url,
            &temp.path().join("artifact"),
            test_request_policy(),
            test_retry_policy(3),
        )
        .await
        .unwrap_err();

        let Error::Network(failure) = error else {
            panic!("expected network failure");
        };
        assert_eq!(failure.stage, NetworkStage::DownloadHeaders);
        assert_eq!(failure.category, NetworkCategory::ServerError);
        assert_eq!(failure.attempts, Some(3));
        assert_eq!(requests.load(Ordering::SeqCst), 3);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn request_timeout_responses_stop_after_three_attempts() {
        let (url, requests, server) = scripted_status_server(vec![
            ("408 Request Timeout", None),
            ("408 Request Timeout", None),
            ("408 Request Timeout", None),
        ])
        .await;
        let temp = tempfile::tempdir().unwrap();

        let error = download_url_with_policy(
            &url,
            &temp.path().join("artifact"),
            test_request_policy(),
            test_retry_policy(3),
        )
        .await
        .unwrap_err();

        let Error::Network(failure) = error else {
            panic!("expected network failure");
        };
        assert_eq!(failure.stage, NetworkStage::DownloadHeaders);
        assert_eq!(failure.category, NetworkCategory::Timeout);
        assert_eq!(failure.attempts, Some(3));
        assert_eq!(requests.load(Ordering::SeqCst), 3);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn stalled_body_retries_then_reports_body_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let server_requests = requests.clone();
        let held_connections = Arc::new(Semaphore::new(0));
        let server_held_connections = held_connections.clone();
        let server = tokio::spawn(async move {
            let mut connections = Vec::new();
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                server_requests.fetch_add(1, Ordering::SeqCst);
                let held_connections = server_held_connections.clone();
                connections.push(tokio::spawn(async move {
                    let mut request = [0_u8; 1024];
                    let _ = socket.read(&mut request).await;
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nx",
                        )
                        .await
                        .unwrap();
                    let _permit = held_connections.acquire().await.unwrap();
                }));
            }
            for connection in connections {
                connection.await.unwrap();
            }
        });
        let url = format!("http://{address}/artifact?token=secret");
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("artifact");
        // Keep the enclosing deadlines well clear of the idle-read timeout under test.
        let request_policy = network::RequestPolicy {
            request_timeout: Duration::from_secs(5),
            ..test_request_policy()
        };
        let retry_policy = RetryPolicy {
            operation_timeout: Duration::from_secs(30),
            ..test_retry_policy(2)
        };

        let error = download_url_with_policy(&url, &destination, request_policy, retry_policy)
            .await
            .unwrap_err();
        held_connections.add_permits(2);

        let Error::Network(failure) = error else {
            panic!("expected network failure");
        };
        assert_eq!(failure.stage, NetworkStage::DownloadBody);
        assert_eq!(failure.category, NetworkCategory::Timeout);
        assert_eq!(failure.attempts, Some(2));
        assert!(!failure.to_string().contains("secret"));
        assert!(!destination.exists());
        assert_eq!(requests.load(Ordering::SeqCst), 2);
        server.await.unwrap();
    }
}
