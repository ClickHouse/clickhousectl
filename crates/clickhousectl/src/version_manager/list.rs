use crate::error::{Error, Result};
use crate::paths;
use chrono::Datelike;
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
        if entry.path().is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            // Only include if it has a clickhouse binary
            let binary = entry.path().join("clickhouse");
            if binary.exists() {
                versions.push(name.to_string());
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
        .user_agent(crate::user_agent::user_agent())
        .build()?;

    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("GitHub API request failed: {}", e)))?;
    let releases: Vec<GitHubRelease> = response.json().await?;

    let mut versions = Vec::new();
    for release in releases {
        // Tag format: v25.12.5.44-stable or v24.8.10.6-lts
        let tag = &release.tag_name;
        if let Some(version) = tag.strip_prefix('v')
            && let Some(dash_pos) = version.rfind('-')
        {
            let v = &version[..dash_pos];
            let suffix = &version[dash_pos + 1..];
            if let Some(channel) = Channel::from_tag_suffix(suffix) {
                versions.push(VersionEntry {
                    version: v.to_string(),
                    channel,
                });
            }
        }
    }

    // Sort versions in descending order (newest first)
    versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    Ok(versions)
}

/// Lists available minor versions by probing builds.clickhouse.com with HEAD requests.
/// Scans from current year back to 2020, checking each YY.{1..12} pattern.
/// Returns minor version strings sorted newest-first (e.g., ["26.3", "26.2", ...]).
pub async fn list_available_versions_from_builds() -> Result<Vec<String>> {
    use crate::version_manager::platform::{Platform, builds_probe_url};

    let platform = Platform::detect()?;
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .build()
        .map_err(|e| Error::Download(e.to_string()))?;

    let current_year = chrono::Utc::now().year() as u32;
    // ClickHouse uses YY.MM versioning — scan from current year down to 20 (2020)
    // Use two-digit year format
    let start_yy = current_year % 100;

    let mut available = Vec::new();

    for yy in (20..=start_yy).rev() {
        for mm in (1..=12).rev() {
            let version_path = format!("{}.{}", yy, mm);
            let url = builds_probe_url(&version_path, &platform);
            match client.head(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    available.push(version_path);
                }
                _ => {}
            }
        }
    }

    Ok(available)
}

/// Gets the current default version
pub fn get_default_version() -> Result<String> {
    let default_file = paths::default_file()?;

    if !default_file.exists() {
        return Err(Error::NoDefaultVersion);
    }

    let version = std::fs::read_to_string(&default_file)?.trim().to_string();

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

/// Compare a single version component. Numeric parts are compared numerically;
/// non-numeric parts fall back to lexicographic comparison.
fn compare_part(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
        _ => a.cmp(b),
    }
}

/// Compares two version strings for sorting.
/// Missing parts are treated as 0, so "20.3" < "20.3.1" and "20.3.0" == "20.3".
pub(crate) fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    let max_len = a_parts.len().max(b_parts.len());

    for i in 0..max_len {
        let a_part = a_parts.get(i).copied().unwrap_or("0");
        let b_part = b_parts.get(i).copied().unwrap_or("0");
        match compare_part(a_part, b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_equal_versions() {
        assert_eq!(
            compare_versions("25.12.5.44", "25.12.5.44"),
            Ordering::Equal
        );
    }

    #[test]
    fn test_different_versions() {
        assert_eq!(
            compare_versions("25.12.5.44", "25.12.5.43"),
            Ordering::Greater
        );
        assert_eq!(compare_versions("25.12.5.43", "25.12.5.44"), Ordering::Less);
    }

    #[test]
    fn test_major_minor_difference() {
        assert_eq!(
            compare_versions("25.12.5.44", "24.12.5.44"),
            Ordering::Greater
        );
        assert_eq!(compare_versions("25.11.5.44", "25.12.5.44"), Ordering::Less);
    }

    #[test]
    fn test_missing_parts_treated_as_zero() {
        // 20.3 should be less than 20.3.1 (missing part = 0)
        assert_eq!(compare_versions("20.3", "20.3.1"), Ordering::Less);
        assert_eq!(compare_versions("20.3.1", "20.3"), Ordering::Greater);
    }

    #[test]
    fn test_trailing_zero_equals_shorter() {
        // 20.3.0 should equal 20.3 (missing part = 0)
        assert_eq!(compare_versions("20.3.0", "20.3"), Ordering::Equal);
        assert_eq!(compare_versions("20.3", "20.3.0"), Ordering::Equal);
    }

    #[test]
    fn test_non_numeric_suffix() {
        // 20.3.2-alpha1 should be greater than 20.3.1 (compare_part "2-alpha1" vs "1": lexicographic, "2" > "1")
        assert_eq!(
            compare_versions("20.3.2-alpha1", "20.3.1"),
            Ordering::Greater
        );
    }

    #[test]
    fn test_single_component() {
        assert_eq!(compare_versions("8", "8.0.1"), Ordering::Less);
        assert_eq!(compare_versions("8.0.1", "8"), Ordering::Greater);
    }
}
