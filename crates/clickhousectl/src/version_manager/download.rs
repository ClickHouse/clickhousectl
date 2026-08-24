use crate::error::{Error, Result};
use crate::version_manager::network::{NetworkFailure, NetworkStage, OperationClient};
use crate::version_manager::platform::{DownloadSource, Platform};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy)]
struct RetryPolicy {
    max_attempts: usize,
    initial_delay: Duration,
    max_delay: Duration,
}

impl RetryPolicy {
    const INSTALLER: Self = Self {
        max_attempts: 3,
        initial_delay: Duration::from_millis(250),
        max_delay: Duration::from_secs(5),
    };

    fn delay(self, failure: &NetworkFailure, attempt: usize) -> Duration {
        failure
            .retry_after
            .unwrap_or_else(|| {
                let exponent = u32::try_from(attempt.saturating_sub(1)).unwrap_or(u32::MAX);
                self.initial_delay
                    .saturating_mul(2_u32.saturating_pow(exponent))
            })
            .min(self.max_delay)
    }
}

/// Downloads from a DownloadSource to the specified path
pub async fn download_from_source(
    source: &DownloadSource,
    platform: &Platform,
    dest_path: &Path,
    structured_output: bool,
) -> Result<()> {
    let url = source.url(platform);
    download_url(&url, dest_path, structured_output).await
}

/// Downloads a file from a URL to the specified path, with progress bar
pub async fn download_url(url: &str, dest_path: &Path, structured_output: bool) -> Result<()> {
    let client = OperationClient::download(url)?;
    download_url_with(
        &client,
        url,
        dest_path,
        RetryPolicy::INSTALLER,
        structured_output,
    )
    .await
}

async fn download_url_with(
    client: &OperationClient,
    url: &str,
    dest_path: &Path,
    policy: RetryPolicy,
    structured_output: bool,
) -> Result<()> {
    for attempt in 1..=policy.max_attempts {
        match download_once(client, url, dest_path, structured_output).await {
            Ok(()) => return Ok(()),
            Err(Error::VersionNetwork(failure)) if failure.is_retryable() => {
                if attempt == policy.max_attempts {
                    return Err(Error::VersionNetworkRetryExhausted {
                        failure,
                        attempts: attempt,
                    });
                }
                client.sleep(policy.delay(&failure, attempt)).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("download retry policy always has at least one attempt")
}

async fn download_once(
    client: &OperationClient,
    url: &str,
    dest_path: &Path,
    structured_output: bool,
) -> Result<()> {
    let response = client.get(url, NetworkStage::Download).await?;
    if !response.status().is_success() {
        return Err(NetworkFailure::from_response(NetworkStage::Download, url, &response).into());
    }

    let total_size = response.content_length().unwrap_or(0);

    let pb = if structured_output {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(total_size)
    };
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut file = tokio::fs::File::create(dest_path).await?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                pb.abandon();
                return Err(
                    NetworkFailure::from_request(NetworkStage::Download, url, &error).into(),
                );
            }
        };
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    file.flush().await?;
    file.shutdown().await?;
    pb.finish_with_message("Download complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version_manager::network::{NetworkCategory, Timeouts};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    fn test_client(read: Duration, total: Duration) -> OperationClient {
        OperationClient::with_timeouts(Timeouts::new(Duration::from_millis(100), read, total))
            .unwrap()
    }

    fn test_policy(max_attempts: usize, max_delay: Duration) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            initial_delay: Duration::ZERO,
            max_delay,
        }
    }

    #[tokio::test]
    async fn stalled_body_hits_read_deadline_and_exhausts_retries() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let server_requests = Arc::clone(&requests);
        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                server_requests.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let mut request = [0; 1024];
                    let _ = socket.read(&mut request).await;
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nabc",
                        )
                        .await
                        .unwrap();
                    tokio::time::sleep(Duration::from_secs(1)).await;
                });
            }
        });
        let url = format!("http://{address}/binary");
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("clickhouse");

        let error = download_url_with(
            &test_client(Duration::from_millis(40), Duration::from_millis(500)),
            &url,
            &destination,
            test_policy(2, Duration::ZERO),
            false,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            Error::VersionNetworkRetryExhausted {
                failure: NetworkFailure {
                    stage: NetworkStage::Download,
                    category: NetworkCategory::Timeout,
                    ..
                },
                attempts: 2,
            }
        ));
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retryable_server_errors_stop_after_three_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .expect(3)
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();

        let error = download_url_with(
            &test_client(Duration::from_millis(100), Duration::from_secs(2)),
            &server.uri(),
            &temp.path().join("clickhouse"),
            test_policy(3, Duration::ZERO),
            false,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            Error::VersionNetworkRetryExhausted {
                failure: NetworkFailure {
                    category: NetworkCategory::Server,
                    ..
                },
                attempts: 3,
            }
        ));
    }

    #[tokio::test]
    async fn request_timeout_responses_are_classified_and_retried() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(408))
            .expect(3)
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();

        let error = download_url_with(
            &test_client(Duration::from_millis(100), Duration::from_secs(2)),
            &server.uri(),
            &temp.path().join("clickhouse"),
            test_policy(3, Duration::ZERO),
            false,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            Error::VersionNetworkRetryExhausted {
                failure: NetworkFailure {
                    category: NetworkCategory::Timeout,
                    ..
                },
                attempts: 3,
            }
        ));
    }

    #[derive(Clone)]
    struct RateLimitThenSuccess {
        calls: Arc<AtomicUsize>,
    }

    impl Respond for RateLimitThenSuccess {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(429).insert_header("Retry-After", "60")
            } else {
                ResponseTemplate::new(200).set_body_bytes(b"complete".to_vec())
            }
        }
    }

    #[tokio::test]
    async fn retry_after_is_honored_but_capped() {
        let server = MockServer::start().await;
        let calls = Arc::new(AtomicUsize::new(0));
        Mock::given(method("GET"))
            .respond_with(RateLimitThenSuccess {
                calls: Arc::clone(&calls),
            })
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("clickhouse");
        let max_delay = Duration::from_millis(30);
        let started = tokio::time::Instant::now();

        download_url_with(
            &test_client(Duration::from_millis(100), Duration::from_secs(2)),
            &server.uri(),
            &destination,
            test_policy(2, max_delay),
            false,
        )
        .await
        .unwrap();

        assert!(started.elapsed() >= max_delay);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(tokio::fs::read(destination).await.unwrap(), b"complete");
    }
}
