use crate::error::{Error, Result};
use crate::version_manager::list::{Channel, VersionEntry, list_available_versions};
use crate::version_manager::platform::{DownloadSource, Platform, builds_probe_url};
use crate::version_manager::spec::VersionSpec;
use serde::Deserialize;

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
    Ok(fallback_source(&entry.version, entry.channel, platform))
}

/// `install 25` — probe builds for highest 25.x minor
async fn resolve_major(major: u32, platform: &Platform) -> Result<ResolvedVersion> {
    // Probe builds.clickhouse.com for all possible minors in this major (1..12)
    let mut highest_available: Option<u32> = None;
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .build()
        .map_err(|e| Error::Download(e.to_string()))?;

    for minor in 1..=12 {
        let url = builds_probe_url(&format!("{}.{}", major, minor), platform);
        match crate::agent_signal::add_agent_query_for(client.head(&url), &url)
            .send()
            .await
        {
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

    // Fallback: try GH matching-refs API for each minor, highest first
    for minor in (1..=12).rev() {
        let prefix = format!("{}.{}", major, minor);
        if let Ok(entry) = find_version_by_refs(&prefix).await {
            return Ok(fallback_source(&entry.version, entry.channel, platform));
        }
    }

    Err(Error::NoMatchingVersion(major.to_string()))
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

    // Fallback: targeted GH API call to find exact version for this minor
    let entry = find_version_by_refs(&version_path).await?;

    Ok(fallback_source(&entry.version, entry.channel, platform))
}

/// `install 25.12.9.61` — exact version, needs channel from GH API
async fn resolve_exact(version: &str, platform: &Platform) -> Result<ResolvedVersion> {
    // Use matching-refs to find the exact tag and its channel.
    // For "25.12.9.61", search refs matching "v25.12.9.61" — should return the exact tag.
    // Fail fast if the lookup fails: a wrong channel produces a broken download URL,
    // and silently guessing Stable could fetch the wrong artifact.
    let channel = find_exact_channel(version).await?;
    Ok(fallback_source(version, channel, platform))
}

/// Look up the channel for an exact version via GitHub's matching-refs API
async fn find_exact_channel(version: &str) -> Result<Channel> {
    let url = format!(
        "https://api.github.com/repos/ClickHouse/ClickHouse/git/matching-refs/tags/v{}-",
        version
    );
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("GitHub API request failed: {}", e)))?;

    let refs: Vec<GitRef> = response.json().await?;
    parse_exact_channel(&refs, version)
}

/// Parse the channel from a list of git refs for an exact version.
/// Looks for tags like "refs/tags/v26.4.1.562-stable" and extracts the channel suffix.
fn parse_exact_channel(refs: &[GitRef], version: &str) -> Result<Channel> {
    for git_ref in refs {
        let Some(tag) = git_ref.ref_name.strip_prefix("refs/tags/v") else {
            continue;
        };
        if let Some(dash_pos) = tag.rfind('-') {
            let suffix = &tag[dash_pos + 1..];
            if let Some(channel) = Channel::from_tag_suffix(suffix) {
                return Ok(channel);
            }
        }
    }

    Err(Error::NoMatchingVersion(version.to_string()))
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
        .user_agent(crate::user_agent::user_agent())
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match crate::agent_signal::add_agent_query_for(client.head(&url), &url)
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

#[derive(Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    ref_name: String,
}

/// Find the latest release version matching a prefix using GitHub's matching-refs API.
/// This is a single targeted API call that works regardless of how old the version is.
/// prefix should be like "25.2" or "24.8" — we search for tags matching `v{prefix}.`
async fn find_version_by_refs(prefix: &str) -> Result<VersionEntry> {
    let url = format!(
        "https://api.github.com/repos/ClickHouse/ClickHouse/git/matching-refs/tags/v{}.",
        prefix
    );
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("GitHub API request failed: {}", e)))?;

    let refs: Vec<GitRef> = response.json().await?;
    parse_version_refs(&refs, prefix)
}

/// Parse a list of git refs into the best matching VersionEntry.
/// Prefers stable/lts tags, but falls back to any tagged version (e.g. "-new")
/// so that pre-release or newly-tagged versions can still be resolved.
fn parse_version_refs(refs: &[GitRef], prefix: &str) -> Result<VersionEntry> {
    use super::list::compare_versions;
    use std::cmp::Ordering;

    let mut best: Option<VersionEntry> = None;
    let mut any: Option<VersionEntry> = None;
    for git_ref in refs {
        let Some(tag) = git_ref.ref_name.strip_prefix("refs/tags/v") else {
            continue;
        };
        if let Some(dash_pos) = tag.rfind('-') {
            let version = &tag[..dash_pos];
            let suffix = &tag[dash_pos + 1..];
            let is_higher = |current: &Option<VersionEntry>| match current {
                Some(existing) => {
                    compare_versions(version, &existing.version) == Ordering::Greater
                }
                None => true,
            };
            if let Some(channel) = Channel::from_tag_suffix(suffix) {
                if is_higher(&best) {
                    best = Some(VersionEntry {
                        version: version.to_string(),
                        channel,
                    });
                }
            } else if is_higher(&any) {
                any = Some(VersionEntry {
                    version: version.to_string(),
                    channel: Channel::Stable,
                });
            }
        }
    }

    best.or(any)
        .ok_or_else(|| Error::NoMatchingVersion(prefix.to_string()))
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

    fn make_ref(name: &str) -> GitRef {
        GitRef {
            ref_name: name.to_string(),
        }
    }

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

    // -- parse_version_refs tests --

    #[test]
    fn test_parse_version_refs_stable_tag() {
        let refs = vec![make_ref("refs/tags/v25.12.9.61-stable")];
        let entry = parse_version_refs(&refs, "25.12").unwrap();
        assert_eq!(entry.version, "25.12.9.61");
        assert_eq!(entry.channel, Channel::Stable);
    }

    #[test]
    fn test_parse_version_refs_lts_tag() {
        let refs = vec![make_ref("refs/tags/v24.8.10.6-lts")];
        let entry = parse_version_refs(&refs, "24.8").unwrap();
        assert_eq!(entry.version, "24.8.10.6");
        assert_eq!(entry.channel, Channel::Lts);
    }

    #[test]
    fn test_parse_version_refs_prefers_stable_over_unknown() {
        let refs = vec![
            make_ref("refs/tags/v26.4.1.1-new"),
            make_ref("refs/tags/v26.4.2.5-stable"),
        ];
        let entry = parse_version_refs(&refs, "26.4").unwrap();
        assert_eq!(entry.version, "26.4.2.5");
        assert_eq!(entry.channel, Channel::Stable);
    }

    #[test]
    fn test_parse_version_refs_falls_back_to_unknown_suffix() {
        let refs = vec![make_ref("refs/tags/v26.4.1.1-new")];
        let entry = parse_version_refs(&refs, "26.4").unwrap();
        assert_eq!(entry.version, "26.4.1.1");
        assert_eq!(entry.channel, Channel::Stable);
    }

    #[test]
    fn test_parse_version_refs_empty_refs() {
        let refs: Vec<GitRef> = vec![];
        assert!(parse_version_refs(&refs, "99.99").is_err());
    }

    #[test]
    fn test_parse_version_refs_no_matching_tags() {
        let refs = vec![
            make_ref("refs/heads/main"),
            make_ref("something/else"),
        ];
        assert!(parse_version_refs(&refs, "25.12").is_err());
    }

    #[test]
    fn test_parse_version_refs_no_dash_in_tag() {
        // A tag without a channel suffix at all should be skipped
        let refs = vec![make_ref("refs/tags/v25.12.9.61")];
        assert!(parse_version_refs(&refs, "25.12").is_err());
    }

    #[test]
    fn test_parse_version_refs_picks_highest_stable() {
        // Multiple stable tags — picks the semantically highest version
        let refs = vec![
            make_ref("refs/tags/v25.12.1.10-stable"),
            make_ref("refs/tags/v25.12.9.61-stable"),
        ];
        let entry = parse_version_refs(&refs, "25.12").unwrap();
        assert_eq!(entry.version, "25.12.9.61");
    }

    #[test]
    fn test_parse_version_refs_unordered_picks_highest() {
        // Higher patch version appears before lower — must still pick the higher one
        let refs = vec![
            make_ref("refs/tags/v25.12.10.5-stable"),
            make_ref("refs/tags/v25.12.9.61-stable"),
        ];
        let entry = parse_version_refs(&refs, "25.12").unwrap();
        assert_eq!(entry.version, "25.12.10.5");
    }

    #[test]
    fn test_parse_version_refs_stable_beats_later_unknown() {
        // Even if an unknown-suffix tag appears after a stable one, stable wins
        let refs = vec![
            make_ref("refs/tags/v26.4.2.5-stable"),
            make_ref("refs/tags/v26.4.3.1-beta"),
        ];
        let entry = parse_version_refs(&refs, "26.4").unwrap();
        // stable is overwritten by the second stable-eligible tag, but "beta" is not stable
        // so the last stable still wins
        assert_eq!(entry.version, "26.4.2.5");
        assert_eq!(entry.channel, Channel::Stable);
    }

    // -- parse_exact_channel tests --

    #[test]
    fn test_parse_exact_channel_stable() {
        let refs = vec![make_ref("refs/tags/v25.12.9.61-stable")];
        assert_eq!(
            parse_exact_channel(&refs, "25.12.9.61").unwrap(),
            Channel::Stable
        );
    }

    #[test]
    fn test_parse_exact_channel_lts() {
        let refs = vec![make_ref("refs/tags/v24.8.10.6-lts")];
        assert_eq!(
            parse_exact_channel(&refs, "24.8.10.6").unwrap(),
            Channel::Lts
        );
    }

    #[test]
    fn test_parse_exact_channel_unknown_suffix_errors() {
        // parse_exact_channel does NOT fall back to unknown suffixes
        let refs = vec![make_ref("refs/tags/v26.4.1.1-new")];
        assert!(parse_exact_channel(&refs, "26.4.1.1").is_err());
    }

    #[test]
    fn test_parse_exact_channel_empty_refs() {
        let refs: Vec<GitRef> = vec![];
        assert!(parse_exact_channel(&refs, "25.12.9.61").is_err());
    }
}
