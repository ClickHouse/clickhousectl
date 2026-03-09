use crate::error::{Error, Result};
use crate::version_manager::list::Channel;
use crate::version_manager::resolve::build_download_url;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Downloads a ClickHouse version to the specified path
pub async fn download_version(version: &str, channel: Channel, dest_path: &Path) -> Result<()> {
    let url = build_download_url(version, channel)?;

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
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

    pb.finish_with_message("Download complete");
    Ok(())
}
