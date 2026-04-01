use crate::error::{Error, Result};
use crate::version_manager::list::{Channel, list_available_versions};
use crate::version_manager::platform::{DownloadSource, Platform, builds_probe_url};
use crate::version_manager::spec::VersionSpec;

/// Result of resolving a version spec — contains everything needed to download
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    /// The download source to use
    pub source: DownloadSource,
    /// Human-readable description of what was resolved (for display)
    pub display_version: String,
    /// Whether the exact version is known before download
    /// (false for builds.clickhouse.com where we detect version post-download)
    pub exact_version_known: bool,
    /// The exact version string, if known
    pub exact_version: Option<String>,
    /// Channel, if known
    pub channel: Option<Channel>,
}

/// Resolve a VersionSpec into a concrete download source
pub async fn resolve(spec: &VersionSpec, platform: &Platform) -> Result<ResolvedVersion> {
    match spec {
        VersionSpec::Latest => resolve_latest(platform).await,
        VersionSpec::Channel(channel) => resolve_channel(*channel, platform).await,
        VersionSpec::Major(major) => resolve_major(*major, platform).await,
        VersionSpec::Minor(major, minor) => resolve_minor(*major, *minor, platform).await,
        VersionSpec::Exact(version) => resolve_exact(version, platform).await,
    }
}

/// `install latest` — always from builds.clickhouse.com/master
async fn resolve_latest(_platform: &Platform) -> Result<ResolvedVersion> {
    Ok(ResolvedVersion {
        source: DownloadSource::Builds {
            version_path: "master".to_string(),
        },
        display_version: "latest".to_string(),
        exact_version_known: false,
        exact_version: None,
        channel: None,
    })
}

/// `install stable` / `install lts` — GH API to find minor, then builds
async fn resolve_channel(channel: Channel, platform: &Platform) -> Result<ResolvedVersion> {
    let available = list_available_versions().await?;
    let entry = available
        .iter()
        .find(|e| e.channel == channel)
        .ok_or_else(|| Error::NoMatchingVersion(channel.to_string()))?;

    // Extract minor version (e.g., "25.12" from "25.12.9.61")
    let minor = extract_minor(&entry.version)?;

    // Try builds first
    if probe_builds(&minor, platform).await {
        return Ok(ResolvedVersion {
            source: DownloadSource::Builds {
                version_path: minor.clone(),
            },
            display_version: format!("{} ({})", minor, channel),
            exact_version_known: false,
            exact_version: None,
            channel: Some(channel),
        });
    }

    // Fallback: use packages (Linux) or GitHub (macOS)
    Ok(fallback_source(
        &entry.version,
        entry.channel,
        platform,
    ))
}

/// `install 25` — probe builds for highest 25.x minor
async fn resolve_major(major: u32, platform: &Platform) -> Result<ResolvedVersion> {
    // Probe builds.clickhouse.com for all possible minors in this major (1..12)
    let mut highest_available: Option<u32> = None;
    let client = reqwest::Client::builder()
        .user_agent("clickhousectl")
        .build()
        .map_err(|e| Error::Download(e.to_string()))?;

    for minor in 1..=12 {
        let url = builds_probe_url(&format!("{}.{}", major, minor), platform);
        match client.head(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                highest_available = Some(minor);
            }
            _ => {}
        }
    }

    if let Some(minor) = highest_available {
        let version_path = format!("{}.{}", major, minor);
        return Ok(ResolvedVersion {
            source: DownloadSource::Builds {
                version_path: version_path.clone(),
            },
            display_version: version_path,
            exact_version_known: false,
            exact_version: None,
            channel: None,
        });
    }

    // Fallback: try GH API
    let available = list_available_versions().await?;
    let prefix = format!("{}.", major);
    let entry = available
        .iter()
        .find(|e| e.version.starts_with(&prefix))
        .ok_or_else(|| Error::NoMatchingVersion(major.to_string()))?;

    Ok(fallback_source(
        &entry.version,
        entry.channel,
        platform,
    ))
}

/// `install 25.12` — try builds, fallback to packages/GH
async fn resolve_minor(major: u32, minor: u32, platform: &Platform) -> Result<ResolvedVersion> {
    let version_path = format!("{}.{}", major, minor);

    // Try builds first
    if probe_builds(&version_path, platform).await {
        return Ok(ResolvedVersion {
            source: DownloadSource::Builds {
                version_path: version_path.clone(),
            },
            display_version: version_path,
            exact_version_known: false,
            exact_version: None,
            channel: None,
        });
    }

    // Fallback: GH API to find exact version for this minor
    let available = list_available_versions().await?;
    let prefix = format!("{}.", version_path);
    let entry = available
        .iter()
        .find(|e| e.version.starts_with(&prefix) || e.version == version_path)
        .ok_or_else(|| Error::NoMatchingVersion(version_path))?;

    Ok(fallback_source(
        &entry.version,
        entry.channel,
        platform,
    ))
}

/// `install 25.12.9.61` — exact version, needs channel from GH API
async fn resolve_exact(version: &str, platform: &Platform) -> Result<ResolvedVersion> {
    // Look up channel from GH API
    let available = list_available_versions().await?;
    let channel = available
        .iter()
        .find(|e| e.version == version)
        .map(|e| e.channel)
        .unwrap_or(Channel::Stable); // Default to stable if not found

    Ok(fallback_source(version, channel, platform))
}

/// Build a fallback download source: packages for Linux, GitHub for macOS
fn fallback_source(version: &str, channel: Channel, platform: &Platform) -> ResolvedVersion {
    let source = if platform.packages_arch().is_some() {
        // Linux: use packages.clickhouse.com
        DownloadSource::Packages {
            channel,
            version: version.to_string(),
        }
    } else {
        // macOS: use GitHub releases
        DownloadSource::GitHub {
            version: version.to_string(),
            channel,
        }
    };

    ResolvedVersion {
        source,
        display_version: version.to_string(),
        exact_version_known: true,
        exact_version: Some(version.to_string()),
        channel: Some(channel),
    }
}

/// Probe builds.clickhouse.com with a HEAD request to check if a version exists
async fn probe_builds(version_path: &str, platform: &Platform) -> bool {
    let url = builds_probe_url(version_path, platform);
    let client = match reqwest::Client::builder()
        .user_agent("clickhousectl")
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.head(&url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Extract the minor version from a full version string (e.g., "25.12.9.61" -> "25.12")
fn extract_minor(version: &str) -> Result<String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        Ok(format!("{}.{}", parts[0], parts[1]))
    } else {
        Err(Error::NoMatchingVersion(format!(
            "cannot extract minor version from '{}'",
            version
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version_manager::platform::Os;

    #[test]
    fn test_extract_minor() {
        assert_eq!(extract_minor("25.12.9.61").unwrap(), "25.12");
        assert_eq!(extract_minor("24.8.6.70").unwrap(), "24.8");
        assert_eq!(extract_minor("25.12").unwrap(), "25.12");
    }

    #[test]
    fn test_extract_minor_invalid() {
        assert!(extract_minor("25").is_err());
    }

    #[test]
    fn test_fallback_source_linux() {
        let platform = Platform {
            os: Os::Linux,
            arch: crate::version_manager::platform::Arch::X86_64,
        };
        let resolved = fallback_source("25.12.9.61", Channel::Stable, &platform);
        assert!(matches!(resolved.source, DownloadSource::Packages { .. }));
        assert_eq!(resolved.exact_version, Some("25.12.9.61".to_string()));
        assert!(resolved.exact_version_known);
    }

    #[test]
    fn test_fallback_source_macos() {
        let platform = Platform {
            os: Os::MacOS,
            arch: crate::version_manager::platform::Arch::Aarch64,
        };
        let resolved = fallback_source("25.12.9.61", Channel::Stable, &platform);
        assert!(matches!(resolved.source, DownloadSource::GitHub { .. }));
        assert_eq!(resolved.exact_version, Some("25.12.9.61".to_string()));
        assert!(resolved.exact_version_known);
    }
}
