use crate::error::{Error, Result};
use crate::paths;
use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Lts,
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Channel::Stable => write!(f, "stable"),
            Channel::Lts => write!(f, "lts"),
        }
    }
}

impl Channel {
    /// Parse a channel from a release tag suffix (e.g. "stable", "lts")
    pub fn from_tag_suffix(s: &str) -> Option<Self> {
        match s {
            "stable" => Some(Channel::Stable),
            "lts" => Some(Channel::Lts),
            _ => None,
        }
    }
}

/// Lists all installed ClickHouse versions
pub fn list_installed_versions() -> Result<Vec<String>> {
    let versions_dir = paths::versions_dir()?;

    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&versions_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                // Only include if it has a clickhouse binary
                let binary = entry.path().join("clickhouse");
                if binary.exists() {
                    versions.push(name.to_string());
                }
            }
        }
    }

    // Sort versions in descending order (newest first)
    versions.sort_by(|a, b| compare_versions(b, a));
    Ok(versions)
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// A version with its release channel
#[derive(Clone)]
pub struct VersionEntry {
    pub version: String,
    pub channel: Channel,
}

/// Fetches available versions from GitHub releases
pub async fn list_available_versions() -> Result<Vec<VersionEntry>> {
    let url = "https://api.github.com/repos/ClickHouse/ClickHouse/releases?per_page=100";
    let client = reqwest::Client::builder()
        .user_agent("ch-cli")
        .build()?;

    let response = client.get(url).send().await?.error_for_status().map_err(|e| {
        Error::Download(format!("GitHub API request failed: {}", e))
    })?;
    let releases: Vec<GitHubRelease> = response.json().await?;

    let mut versions = Vec::new();
    for release in releases {
        // Tag format: v25.12.5.44-stable or v24.8.10.6-lts
        let tag = &release.tag_name;
        if let Some(version) = tag.strip_prefix('v') {
            if let Some(dash_pos) = version.rfind('-') {
                let v = &version[..dash_pos];
                let suffix = &version[dash_pos + 1..];
                if let Some(channel) = Channel::from_tag_suffix(suffix) {
                    versions.push(VersionEntry { version: v.to_string(), channel });
                }
            }
        }
    }

    // Sort versions in descending order (newest first)
    versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    Ok(versions)
}

/// Gets the current default version
pub fn get_default_version() -> Result<String> {
    let default_file = paths::default_file()?;

    if !default_file.exists() {
        return Err(Error::NoDefaultVersion);
    }

    let version = std::fs::read_to_string(&default_file)?
        .trim()
        .to_string();

    if version.is_empty() {
        return Err(Error::NoDefaultVersion);
    }

    // Verify the version is actually installed
    let binary = paths::binary_path(&version)?;
    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    Ok(version)
}

/// Sets the default version
pub fn set_default_version(version: &str) -> Result<()> {
    // Verify the version is installed
    let binary = paths::binary_path(version)?;
    if !binary.exists() {
        return Err(Error::VersionNotFound(version.to_string()));
    }

    let default_file = paths::default_file()?;
    std::fs::write(&default_file, version)?;
    Ok(())
}

/// Compares two version strings for sorting
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}
