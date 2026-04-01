use crate::error::{Error, Result};
use crate::paths;
use crate::version_manager::download::download_from_source;
use crate::version_manager::list::list_installed_versions;
use crate::version_manager::platform::{DownloadSource, Platform};
use crate::version_manager::resolve::ResolvedVersion;
use std::os::unix::fs::PermissionsExt;

/// Installs a ClickHouse version using the multi-source resolution system.
/// Returns the exact version string of the installed binary.
pub async fn install_resolved(
    resolved: &ResolvedVersion,
    platform: &Platform,
    force: bool,
) -> Result<String> {
    paths::ensure_dirs()?;

    // If we know the exact version upfront, check if already installed
    if let Some(ref version) = resolved.exact_version {
        let version_dir = paths::version_dir(version)?;
        if version_dir.exists() && !force {
            return Err(Error::VersionAlreadyInstalled(version.to_string()));
        }
    }

    // For builds source (minor versions like "25.12"), check if we already have
    // an installed version matching that minor — avoids re-downloading ~150MB
    if !force {
        if let DownloadSource::Builds { ref version_path } = resolved.source {
            if version_path != "master" {
                let prefix = format!("{}.", version_path);
                if let Ok(installed) = list_installed_versions() {
                    if let Some(existing) = installed.iter().find(|v| v.starts_with(&prefix)) {
                        eprintln!(
                            "ClickHouse {} is already installed as {}",
                            version_path, existing
                        );
                        eprintln!("Use --force to re-download the latest build");
                        return Ok(existing.clone());
                    }
                }
            }
        }
    }

    // Download to a temp directory first
    let temp_dir = paths::versions_dir()?.join(".download-temp");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
    }
    std::fs::create_dir_all(&temp_dir)?;

    let binary_path = temp_dir.join("clickhouse");

    eprintln!("Downloading ClickHouse {}...", resolved.display_version);

    if resolved.source.is_tarball(platform) {
        let tarball_path = temp_dir.join("clickhouse.tgz");
        download_from_source(&resolved.source, platform, &tarball_path).await?;
        eprintln!("Extracting...");
        extract_tarball_auto(&tarball_path, &temp_dir)?;
    } else {
        download_from_source(&resolved.source, platform, &binary_path).await?;
    }

    // Make the binary executable
    let mut perms = std::fs::metadata(&binary_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&binary_path, perms)?;

    // Detect the exact version from the binary
    let exact_version = if resolved.exact_version_known {
        resolved.exact_version.clone().unwrap()
    } else {
        eprintln!("Detecting version...");
        detect_binary_version(&binary_path)?
    };

    // Check if this exact version is already installed (post-detection check for builds source)
    let version_dir = paths::version_dir(&exact_version)?;
    if version_dir.exists() && !force {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(Error::VersionAlreadyInstalled(exact_version));
    }

    // Move to final location
    if version_dir.exists() {
        std::fs::remove_dir_all(&version_dir)?;
    }
    std::fs::create_dir_all(&version_dir)?;
    std::fs::rename(&binary_path, version_dir.join("clickhouse"))?;

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    let channel_suffix = match resolved.channel {
        Some(ch) => format!(" ({})", ch),
        None => String::new(),
    };
    eprintln!("Installed ClickHouse {}{}", exact_version, channel_suffix);

    Ok(exact_version)
}

/// Detect the version of a clickhouse binary by running `./clickhouse --version`
fn detect_binary_version(binary_path: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new(binary_path)
        .arg("--version")
        .output()
        .map_err(|e| Error::Exec(format!("Failed to run clickhouse --version: {}", e)))?;

    if !output.status.success() {
        return Err(Error::Exec(
            "clickhouse --version returned non-zero exit code".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version_output(&stdout)
}

/// Parse the version string from clickhouse --version output
/// Example outputs:
///   "ClickHouse client version 25.12.9.61 (official build)."
///   "ClickHouse server version 25.12.9.61 (official build)."
fn parse_version_output(output: &str) -> Result<String> {
    for word in output.split_whitespace() {
        let parts: Vec<&str> = word.trim_end_matches('.').split('.').collect();
        if parts.len() == 4 && parts.iter().all(|p| p.parse::<u64>().is_ok()) {
            return Ok(parts.join("."));
        }
    }

    Err(Error::Exec(format!(
        "Could not parse version from output: {}",
        output.trim()
    )))
}

/// Extract a tarball, finding the clickhouse binary automatically.
/// Handles both packages.clickhouse.com layout (usr/bin/clickhouse inside subdir)
/// and GitHub releases layout (same structure).
fn extract_tarball_auto(
    tarball_path: &std::path::Path,
    dest_dir: &std::path::Path,
) -> Result<()> {
    let status = std::process::Command::new("tar")
        .args(["xzf", &tarball_path.to_string_lossy()])
        .current_dir(dest_dir)
        .status()
        .map_err(|e| Error::Extract(format!("Failed to run tar: {}", e)))?;

    if !status.success() {
        let _ = std::fs::remove_file(tarball_path);
        return Err(Error::Extract("tar extraction failed".to_string()));
    }

    let final_binary = dest_dir.join("clickhouse");

    // Search extracted directories for the binary
    for entry in std::fs::read_dir(dest_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join("usr/bin/clickhouse");
            if candidate.exists() {
                std::fs::rename(&candidate, &final_binary)?;
                let _ = std::fs::remove_file(tarball_path);
                let _ = std::fs::remove_dir_all(&path);
                return Ok(());
            }
        }
    }

    // Binary might already be at top level
    if final_binary.exists() {
        let _ = std::fs::remove_file(tarball_path);
        return Ok(());
    }

    let _ = std::fs::remove_file(tarball_path);
    Err(Error::Extract(
        "Could not find clickhouse binary in extracted tarball".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_output_client() {
        let output = "ClickHouse client version 25.12.9.61 (official build).";
        assert_eq!(parse_version_output(output).unwrap(), "25.12.9.61");
    }

    #[test]
    fn test_parse_version_output_server() {
        let output = "ClickHouse server version 26.3.1.100 (official build).";
        assert_eq!(parse_version_output(output).unwrap(), "26.3.1.100");
    }

    #[test]
    fn test_parse_version_output_multiline() {
        let output = "ClickHouse client version 25.5.2.1 (official build).\nSome other info.";
        assert_eq!(parse_version_output(output).unwrap(), "25.5.2.1");
    }

    #[test]
    fn test_parse_version_output_no_version() {
        let output = "Some random output without a version.";
        assert!(parse_version_output(output).is_err());
    }

    #[test]
    fn test_parse_version_output_empty() {
        assert!(parse_version_output("").is_err());
    }
}
