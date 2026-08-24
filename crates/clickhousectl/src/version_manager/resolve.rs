use crate::error::{Error, Result};
use crate::version_manager::list::{
    Channel, VersionEntry, list_available_versions_with, list_installed_versions,
};
use crate::version_manager::network::{NetworkFailure, NetworkStage, OperationClient};
use crate::version_manager::platform::{DownloadSource, Platform, builds_probe_url};
use crate::version_manager::spec::VersionSpec;
use serde::Deserialize;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/ClickHouse/ClickHouse/releases?per_page=100";

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

/// Try to satisfy a spec from already-installed versions, without any network call.
/// Returns `None` for floating specs (`Latest`, `Channel(_)`) — those always need the
/// remote — or when no installed version matches.
pub fn try_resolve_local(spec: &VersionSpec) -> Option<String> {
    match spec {
        VersionSpec::Latest | VersionSpec::Channel(_) => None,
        _ => {
            let installed = list_installed_versions().ok()?;
            find_local_match(spec, &installed)
        }
    }
}

/// Pure matcher for `try_resolve_local`. `installed` is expected to be ordered
/// newest-first (as `list_installed_versions` returns), so `.find(...)` returns
/// the highest version that matches.
fn find_local_match(spec: &VersionSpec, installed: &[String]) -> Option<String> {
    match spec {
        VersionSpec::Latest | VersionSpec::Channel(_) => None,
        VersionSpec::Major(major) => {
            let prefix = format!("{}.", major);
            installed.iter().find(|v| v.starts_with(&prefix)).cloned()
        }
        VersionSpec::Minor(major, minor) => {
            let prefix = format!("{}.{}.", major, minor);
            installed.iter().find(|v| v.starts_with(&prefix)).cloned()
        }
        VersionSpec::Exact(version) => installed
            .iter()
            .find(|v| v.as_str() == version.as_str())
            .cloned(),
    }
}

/// Resolve a VersionSpec into a concrete download source
pub async fn resolve(spec: &VersionSpec, platform: &Platform) -> Result<ResolvedVersion> {
    if matches!(spec, VersionSpec::Latest) {
        return resolve_latest(platform).await;
    }

    let (stage, initial_url) = match spec {
        VersionSpec::Channel(_) | VersionSpec::Exact(_) => {
            (NetworkStage::GithubLookup, GITHUB_RELEASES_URL.to_string())
        }
        VersionSpec::Major(major) => (
            NetworkStage::BuildProbe,
            builds_probe_url(&format!("{}.1", major), platform),
        ),
        VersionSpec::Minor(major, minor) => (
            NetworkStage::BuildProbe,
            builds_probe_url(&format!("{}.{}", major, minor), platform),
        ),
        VersionSpec::Latest => unreachable!(),
    };
    let client = OperationClient::metadata(stage, &initial_url)?;

    match spec {
        VersionSpec::Latest => unreachable!(),
        VersionSpec::Channel(channel) => resolve_channel(*channel, platform, &client).await,
        VersionSpec::Major(major) => resolve_major(*major, platform, &client).await,
        VersionSpec::Minor(major, minor) => resolve_minor(*major, *minor, platform, &client).await,
        VersionSpec::Exact(version) => resolve_exact(version, platform, &client).await,
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
async fn resolve_channel(
    channel: Channel,
    platform: &Platform,
    client: &OperationClient,
) -> Result<ResolvedVersion> {
    let available = list_available_versions_with(client, GITHUB_RELEASES_URL).await?;
    let entry = available
        .iter()
        .find(|e| e.channel == channel)
        .ok_or_else(|| Error::NoMatchingVersion(channel.to_string()))?;

    // Extract minor version (e.g., "25.12" from "25.12.9.61")
    let minor = extract_minor(&entry.version)?;

    // Try builds first
    if matches!(
        probe_builds(client, &builds_probe_url(&minor, platform)).await,
        Ok(ProbeOutcome::Available)
    ) {
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
async fn resolve_major(
    major: u32,
    platform: &Platform,
    client: &OperationClient,
) -> Result<ResolvedVersion> {
    // Probe builds.clickhouse.com for all possible minors in this major (1..12)
    let mut highest_available: Option<u32> = None;
    let mut first_probe_failure = None;

    for minor in 1..=12 {
        let url = builds_probe_url(&format!("{}.{}", major, minor), platform);
        match probe_builds(client, &url).await {
            Ok(ProbeOutcome::Available) => highest_available = Some(minor),
            Ok(ProbeOutcome::Unavailable(_)) => {}
            Err(error) => {
                first_probe_failure = Some(error);
                break;
            }
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
        let url = version_refs_url(&prefix);
        match find_version_by_refs(client, &prefix, &url).await {
            Ok(entry) => return Ok(fallback_source(&entry.version, entry.channel, platform)),
            Err(Error::NoMatchingVersion(_)) => {}
            Err(error) => return Err(preserve_probe_failure(first_probe_failure, error)),
        }
    }

    Err(preserve_probe_failure(
        first_probe_failure,
        Error::NoMatchingVersion(major.to_string()),
    ))
}

/// `install 25.12` — try builds, fallback to packages/GH
async fn resolve_minor(
    major: u32,
    minor: u32,
    platform: &Platform,
    client: &OperationClient,
) -> Result<ResolvedVersion> {
    let version_path = format!("{}.{}", major, minor);
    let build_url = builds_probe_url(&version_path, platform);
    let refs_url = version_refs_url(&version_path);
    resolve_minor_from_urls(client, &version_path, platform, &build_url, &refs_url).await
}

async fn resolve_minor_from_urls(
    client: &OperationClient,
    version_path: &str,
    platform: &Platform,
    build_url: &str,
    refs_url: &str,
) -> Result<ResolvedVersion> {
    // Try builds first
    let probe_failure = match probe_builds(client, build_url).await {
        Ok(ProbeOutcome::Available) => {
            return Ok(ResolvedVersion {
                source: DownloadSource::Builds {
                    version_path: version_path.to_string(),
                },
                display_version: version_path.to_string(),
                exact_version_known: false,
                exact_version: None,
                channel: None,
            });
        }
        Ok(ProbeOutcome::Unavailable(_)) => None,
        Err(error) => Some(error),
    };

    // Fallback: targeted GH API call to find exact version for this minor
    let entry = find_version_by_refs(client, version_path, refs_url)
        .await
        .map_err(|error| preserve_probe_failure(probe_failure, error))?;

    Ok(fallback_source(&entry.version, entry.channel, platform))
}

/// `install 25.12.9.61` — exact version, needs channel from GH API
async fn resolve_exact(
    version: &str,
    platform: &Platform,
    client: &OperationClient,
) -> Result<ResolvedVersion> {
    // Use matching-refs to find the exact tag and its channel.
    // For "25.12.9.61", search refs matching "v25.12.9.61" — should return the exact tag.
    // Fail fast if the lookup fails: a wrong channel produces a broken download URL,
    // and silently guessing Stable could fetch the wrong artifact.
    match find_exact_channel(client, version).await {
        Ok(channel) => Ok(fallback_source(version, channel, platform)),
        Err(Error::NoMatchingVersion(_)) => {
            let series = extract_minor(version)?;
            // The exact miss is definitive; fetching a retry hint is best-effort.
            let url = version_refs_url(&series);
            let available = find_version_by_refs(client, &series, &url).await.ok();

            Err(exact_version_no_match(version, &series, available.as_ref()))
        }
        Err(error) => Err(error),
    }
}

fn exact_version_no_match(version: &str, series: &str, available: Option<&VersionEntry>) -> Error {
    match available {
        Some(entry) => Error::ExactVersionUnavailable {
            version: version.to_string(),
            series: series.to_string(),
            available: entry.version.clone(),
        },
        None => Error::NoMatchingVersion(version.to_string()),
    }
}

/// Look up the channel for an exact version via GitHub's matching-refs API
async fn find_exact_channel(client: &OperationClient, version: &str) -> Result<Channel> {
    let url = format!(
        "https://api.github.com/repos/ClickHouse/ClickHouse/git/matching-refs/tags/v{}-",
        version
    );
    let response = client.get(&url, NetworkStage::GithubLookup).await?;
    if !response.status().is_success() {
        return Err(
            NetworkFailure::from_response(NetworkStage::GithubLookup, &url, &response).into(),
        );
    }

    let refs: Vec<GitRef> = response
        .json()
        .await
        .map_err(|error| NetworkFailure::from_request(NetworkStage::GithubLookup, &url, &error))?;
    parse_exact_channel(&refs, version)
}

/// Parse the channel from a list of git refs for an exact version.
/// Looks for tags like "refs/tags/v26.4.1.562-stable" and extracts the channel suffix.
fn parse_exact_channel(refs: &[GitRef], version: &str) -> Result<Channel> {
    let version_prefix = format!("{}-", version);
    let mut exact_tag_found = false;

    for git_ref in refs {
        let Some(tag) = git_ref.ref_name.strip_prefix("refs/tags/v") else {
            continue;
        };
        if !tag.starts_with(&version_prefix) {
            continue;
        }
        exact_tag_found = true;

        if let Some(dash_pos) = tag.rfind('-') {
            let suffix = &tag[dash_pos + 1..];
            if let Some(channel) = Channel::from_tag_suffix(suffix) {
                return Ok(channel);
            }
        }
    }

    if exact_tag_found {
        Err(Error::UnknownVersionChannel(version.to_string()))
    } else {
        Err(Error::NoMatchingVersion(version.to_string()))
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeMiss {
    Forbidden,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeOutcome {
    Available,
    Unavailable(ProbeMiss),
}

/// Probe builds.clickhouse.com with a HEAD request to check if a version exists.
/// Only 403/404 mean that fallback is expected; outages remain errors.
async fn probe_builds(
    client: &OperationClient,
    url: &str,
) -> std::result::Result<ProbeOutcome, NetworkFailure> {
    let response = client.head(url, NetworkStage::BuildProbe).await?;
    match response.status().as_u16() {
        _ if response.status().is_success() => Ok(ProbeOutcome::Available),
        403 => Ok(ProbeOutcome::Unavailable(ProbeMiss::Forbidden)),
        404 => Ok(ProbeOutcome::Unavailable(ProbeMiss::NotFound)),
        _ => Err(NetworkFailure::from_response(
            NetworkStage::BuildProbe,
            url,
            &response,
        )),
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
fn version_refs_url(prefix: &str) -> String {
    format!(
        "https://api.github.com/repos/ClickHouse/ClickHouse/git/matching-refs/tags/v{}.",
        prefix
    )
}

async fn find_version_by_refs(
    client: &OperationClient,
    prefix: &str,
    url: &str,
) -> Result<VersionEntry> {
    let response = client.get(url, NetworkStage::GithubLookup).await?;
    if !response.status().is_success() {
        return Err(
            NetworkFailure::from_response(NetworkStage::GithubLookup, url, &response).into(),
        );
    }

    let refs: Vec<GitRef> = response
        .json()
        .await
        .map_err(|error| NetworkFailure::from_request(NetworkStage::GithubLookup, url, &error))?;
    parse_version_refs(&refs, prefix)
}

fn preserve_probe_failure(probe: Option<NetworkFailure>, fallback: Error) -> Error {
    match probe {
        Some(probe) => Error::VersionFallback {
            probe,
            fallback: Box::new(fallback),
        },
        None => fallback,
    }
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
                Some(existing) => compare_versions(version, &existing.version) == Ordering::Greater,
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
    use crate::version_manager::network::{NetworkCategory, Timeouts};
    use crate::version_manager::platform::Os;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_ref(name: &str) -> GitRef {
        GitRef {
            ref_name: name.to_string(),
        }
    }

    fn test_client() -> OperationClient {
        OperationClient::with_timeouts(Timeouts::new(
            Duration::from_millis(100),
            Duration::from_millis(100),
            Duration::from_secs(2),
        ))
        .unwrap()
    }

    fn macos_platform() -> Platform {
        Platform {
            os: Os::MacOS,
            arch: crate::version_manager::platform::Arch::Aarch64,
        }
    }

    #[tokio::test]
    async fn build_probe_distinguishes_403_and_404_as_expected_misses() {
        for (status, expected) in [(403, ProbeMiss::Forbidden), (404, ProbeMiss::NotFound)] {
            let server = MockServer::start().await;
            Mock::given(method("HEAD"))
                .respond_with(ResponseTemplate::new(status))
                .mount(&server)
                .await;

            let outcome = probe_builds(&test_client(), &server.uri()).await.unwrap();
            assert_eq!(outcome, ProbeOutcome::Unavailable(expected));
        }
    }

    #[tokio::test]
    async fn build_probe_classifies_429_and_5xx_as_failures() {
        for (status, expected) in [
            (429, NetworkCategory::RateLimited),
            (503, NetworkCategory::Server),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("HEAD"))
                .respond_with(ResponseTemplate::new(status))
                .mount(&server)
                .await;

            let error = probe_builds(&test_client(), &server.uri())
                .await
                .unwrap_err();
            assert_eq!(error.stage, NetworkStage::BuildProbe);
            assert_eq!(error.category, expected);
        }
    }

    #[tokio::test]
    async fn expected_probe_miss_falls_back_to_github() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/build"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/refs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"ref": "refs/tags/v25.12.9.61-stable"}
            ])))
            .mount(&server)
            .await;

        let resolved = resolve_minor_from_urls(
            &test_client(),
            "25.12",
            &macos_platform(),
            &format!("{}/build", server.uri()),
            &format!("{}/refs", server.uri()),
        )
        .await
        .unwrap();

        assert!(matches!(
            resolved.source,
            DownloadSource::GitHub {
                ref version,
                channel: Channel::Stable,
            } if version == "25.12.9.61"
        ));
    }

    #[tokio::test]
    async fn fallback_failure_preserves_original_probe_failure() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/build"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/refs"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let error = resolve_minor_from_urls(
            &test_client(),
            "25.12",
            &macos_platform(),
            &format!("{}/build", server.uri()),
            &format!("{}/refs", server.uri()),
        )
        .await
        .unwrap_err();

        match error {
            Error::VersionFallback { probe, fallback } => {
                assert_eq!(probe.stage, NetworkStage::BuildProbe);
                assert_eq!(probe.category, NetworkCategory::Server);
                assert!(matches!(
                    *fallback,
                    Error::VersionNetwork(NetworkFailure {
                        stage: NetworkStage::GithubLookup,
                        category: NetworkCategory::RateLimited,
                        ..
                    })
                ));
            }
            other => panic!("expected preserved fallback error, got {other:?}"),
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
        let refs = vec![make_ref("refs/heads/main"), make_ref("something/else")];
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
        let error = parse_exact_channel(&refs, "26.4.1.1").unwrap_err();

        assert_eq!(
            error.to_string(),
            "build 26.4.1.1 exists, but its release channel could not be determined"
        );
        assert!(matches!(
            error,
            Error::UnknownVersionChannel(ref version) if version == "26.4.1.1"
        ));
    }

    #[test]
    fn test_parse_exact_channel_empty_refs() {
        let refs: Vec<GitRef> = vec![];
        let error = parse_exact_channel(&refs, "25.12.9.61").unwrap_err();

        assert!(matches!(
            error,
            Error::NoMatchingVersion(ref version) if version == "25.12.9.61"
        ));
    }

    #[test]
    fn test_exact_version_no_match_hints_highest_series_version() {
        let refs = vec![
            make_ref("refs/tags/v26.2.19.43-stable"),
            make_ref("refs/tags/v26.2.9.9-stable"),
            make_ref("refs/tags/v26.2.20.4-stable"),
        ];
        let available = parse_version_refs(&refs, "26.2").unwrap();

        let error = exact_version_no_match("26.2.8.7", "26.2", Some(&available));

        assert!(matches!(
            error,
            Error::ExactVersionUnavailable {
                ref version,
                ref series,
                ref available,
            } if version == "26.2.8.7"
                && series == "26.2"
                && available == "26.2.20.4"
        ));
    }

    #[test]
    fn test_exact_version_no_match_without_series_preserves_generic_error() {
        let error = exact_version_no_match("99.99.1.1", "99.99", None);

        assert!(matches!(
            error,
            Error::NoMatchingVersion(ref version) if version == "99.99.1.1"
        ));
    }

    // -- find_local_match tests --

    fn installed(versions: &[&str]) -> Vec<String> {
        versions.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_find_local_match_latest_returns_none() {
        assert_eq!(
            find_local_match(&VersionSpec::Latest, &installed(&["25.12.9.61"])),
            None
        );
    }

    #[test]
    fn test_find_local_match_channel_returns_none() {
        assert_eq!(
            find_local_match(
                &VersionSpec::Channel(Channel::Stable),
                &installed(&["25.12.9.61"])
            ),
            None
        );
        assert_eq!(
            find_local_match(
                &VersionSpec::Channel(Channel::Lts),
                &installed(&["25.12.9.61"])
            ),
            None
        );
    }

    #[test]
    fn test_find_local_match_major_picks_first() {
        assert_eq!(
            find_local_match(
                &VersionSpec::Major(25),
                &installed(&["25.12.9.61", "24.8.6.70"])
            ),
            Some("25.12.9.61".to_string())
        );
    }

    #[test]
    fn test_find_local_match_major_rejects_numeric_prefix() {
        // "250.1.2.3" must not match Major(25) — the trailing "." is the boundary guard
        assert_eq!(
            find_local_match(&VersionSpec::Major(25), &installed(&["250.1.2.3"])),
            None
        );
    }

    #[test]
    fn test_find_local_match_major_no_match() {
        assert_eq!(
            find_local_match(&VersionSpec::Major(25), &installed(&["24.12.9.61"])),
            None
        );
    }

    #[test]
    fn test_find_local_match_minor_component_boundary() {
        // Minor(25, 12) must match 25.12.9.61 but not 25.120.1.1
        assert_eq!(
            find_local_match(
                &VersionSpec::Minor(25, 12),
                &installed(&["25.120.1.1", "25.12.9.61"])
            ),
            Some("25.12.9.61".to_string())
        );
    }

    #[test]
    fn test_find_local_match_minor_no_match() {
        assert_eq!(
            find_local_match(&VersionSpec::Minor(25, 12), &installed(&["25.11.9.61"])),
            None
        );
    }

    #[test]
    fn test_find_local_match_exact_matches() {
        assert_eq!(
            find_local_match(
                &VersionSpec::Exact("25.12.9.61".to_string()),
                &installed(&["25.12.9.61"])
            ),
            Some("25.12.9.61".to_string())
        );
    }

    #[test]
    fn test_find_local_match_exact_rejects_partial() {
        // Exact match must not accept shorter or longer strings
        assert_eq!(
            find_local_match(
                &VersionSpec::Exact("25.12.9.61".to_string()),
                &installed(&["25.12.9.6", "25.12.9.611"])
            ),
            None
        );
    }

    #[test]
    fn test_find_local_match_newest_wins() {
        // list_installed_versions returns descending order — first match wins
        assert_eq!(
            find_local_match(
                &VersionSpec::Major(25),
                &installed(&["25.12.9.61", "25.5.2.1"])
            ),
            Some("25.12.9.61".to_string())
        );
    }
}
