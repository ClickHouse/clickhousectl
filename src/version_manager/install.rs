use crate::error::{Error, Result};
use crate::paths;
use crate::version_manager::download::download_version;
use crate::version_manager::resolve::is_tarball_download;
use std::os::unix::fs::PermissionsExt;

/// Installs a ClickHouse version
pub async fn install_version(version: &str, channel: &str) -> Result<()> {
    paths::ensure_dirs()?;

    let version_dir = paths::version_dir(version)?;

    // Check if already installed
    if version_dir.exists() {
        return Err(Error::VersionAlreadyInstalled(version.to_string()));
    }

    // Create the version directory
    std::fs::create_dir_all(&version_dir)?;

    let binary_path = version_dir.join("clickhouse");

    println!("Downloading ClickHouse {}...", version);

    if is_tarball_download()? {
        // Linux: download tarball, extract, move binary
        let tarball_path = version_dir.join("clickhouse.tgz");
        download_version(version, channel, &tarball_path).await?;

        println!("Extracting...");
        extract_tarball(&tarball_path, &version_dir, version)?;
    } else {
        // macOS: download binary directly
        download_version(version, channel, &binary_path).await?;
    }

    // Make the binary executable
    let mut perms = std::fs::metadata(&binary_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&binary_path, perms)?;

    println!("ClickHouse {} installed successfully", version);
    Ok(())
}

/// Extracts the ClickHouse binary from a Linux tarball
fn extract_tarball(
    tarball_path: &std::path::Path,
    version_dir: &std::path::Path,
    version: &str,
) -> Result<()> {
    // Extract the tarball
    let status = std::process::Command::new("tar")
        .args(["xzf", &tarball_path.to_string_lossy()])
        .current_dir(version_dir)
        .status()
        .map_err(|e| Error::Extract(format!("Failed to run tar: {}", e)))?;

    if !status.success() {
        // Clean up tarball on failure
        let _ = std::fs::remove_file(tarball_path);
        return Err(Error::Extract("tar extraction failed".to_string()));
    }

    // Find the clickhouse binary inside the extracted directory
    let extracted_dir = version_dir.join(format!("clickhouse-common-static-{}", version));
    let extracted_binary = extracted_dir.join("usr/bin/clickhouse");

    if !extracted_binary.exists() {
        // Clean up on failure
        let _ = std::fs::remove_file(tarball_path);
        let _ = std::fs::remove_dir_all(&extracted_dir);
        return Err(Error::Extract(format!(
            "Binary not found at expected path: {}",
            extracted_binary.display()
        )));
    }

    // Move binary to final location
    let final_binary = version_dir.join("clickhouse");
    std::fs::rename(&extracted_binary, &final_binary)?;

    // Clean up tarball and extracted directory
    let _ = std::fs::remove_file(tarball_path);
    let _ = std::fs::remove_dir_all(&extracted_dir);

    Ok(())
}
