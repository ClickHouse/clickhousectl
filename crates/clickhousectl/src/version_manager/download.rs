use crate::error::{Error, Result};
use crate::version_manager::platform::{DownloadSource, Platform};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Downloads from a DownloadSource to the specified path
pub async fn download_from_source(
    source: &DownloadSource,
    platform: &Platform,
    dest_path: &Path,
) -> Result<()> {
    let url = source.url(platform);
    download_url(&url, dest_path).await
}

/// Downloads a file from a URL to the specified path, with progress bar
pub async fn download_url(url: &str, dest_path: &Path) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .build()?;

    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("Failed to download {}: {}", url, e)))?;

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
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
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    file.flush().await?;
    file.shutdown().await?;
    pb.finish_with_message("Download complete");
    Ok(())
}
