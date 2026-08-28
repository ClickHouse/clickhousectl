use crate::error::{Error, NetworkStage, Result};
use crate::paths;
use crate::version_manager::network::{self, ProbeOutcome};
use chrono::Datelike;
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use std::fmt;
use std::time::Duration;

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
    let client = network::client(network::METADATA_POLICY, NetworkStage::VersionList, url)?;
    let response = network::send(client.get(url), NetworkStage::VersionList, url).await?;
    let response = network::ensure_success(response, NetworkStage::VersionList, url)?;
    let releases: Vec<GitHubRelease> =
        network::json(response, NetworkStage::VersionList, url).await?;

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
    let current_year = chrono::Utc::now().year() as u32;
    // ClickHouse uses YY.MM versioning — scan from current year down to 20 (2020)
    // Use two-digit year format
    let start_yy = current_year % 100;

    let mut candidates = Vec::new();
    for yy in (20..=start_yy).rev() {
        for mm in (1..=12).rev() {
            let version_path = format!("{}.{}", yy, mm);
            let url = builds_probe_url(&version_path, &platform);
            candidates.push((version_path, url));
        }
    }
    scan_build_candidates(candidates, network::LIST_OPERATION_TIMEOUT).await
}

async fn scan_build_candidates(
    candidates: Vec<(String, String)>,
    operation_timeout: Duration,
) -> Result<Vec<String>> {
    let Some((_, first_url)) = candidates.first() else {
        return Ok(Vec::new());
    };
    let first_url = first_url.clone();
    let client = network::client(
        network::METADATA_POLICY,
        NetworkStage::VersionList,
        &first_url,
    )?;
    let scan = stream::iter(candidates)
        .map(|(version, url)| {
            let client = client.clone();
            async move {
                let outcome = network::probe(&client, &url, NetworkStage::VersionList).await;
                (version, outcome)
            }
        })
        .buffer_unordered(8)
        .collect::<Vec<_>>();
    let outcomes = network::with_operation_timeout(
        operation_timeout,
        NetworkStage::VersionList,
        &first_url,
        scan,
    )
    .await?;

    let mut available = Vec::new();
    let mut failure = None;
    for (version, outcome) in outcomes {
        match outcome {
            ProbeOutcome::Available => available.push(version),
            ProbeOutcome::Missing => {}
            ProbeOutcome::Failed(candidate) => {
                failure = network::preferred_failure(failure, candidate);
            }
        }
    }
    if let Some(failure) = failure {
        return Err(failure.into());
    }
    available.sort_by(|a, b| compare_versions(b, a));
    Ok(available)
}

/// Trims the `~/.clickhouse/default` marker's contents, treating a blank marker
/// as no default at all.
fn parse_default_marker(contents: &str) -> Option<String> {
    let version = contents.trim();
    (!version.is_empty()).then(|| version.to_string())
}

/// Reads the version named by the `~/.clickhouse/default` marker *without*
/// checking that it is still installed. `None` when the marker is missing,
/// unreadable, or blank.
///
/// Destructive guards need the raw marker rather than [`get_default_version`]:
/// a marker naming a version whose binary is already gone is exactly the state
/// [`get_default_version`] rejects, and clearing it is still part of removing
/// that version.
pub fn default_version_marker() -> Option<String> {
    let default_file = paths::default_file().ok()?;
    let contents = std::fs::read_to_string(default_file).ok()?;
    parse_default_marker(&contents)
}

/// Gets the current default version
pub fn get_default_version() -> Result<String> {
    let default_file = paths::default_file()?;

    if !default_file.exists() {
        return Err(Error::NoDefaultVersion);
    }

    let contents = std::fs::read_to_string(&default_file)?;
    let version = parse_default_marker(&contents).ok_or(Error::NoDefaultVersion)?;

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
    use crate::error::{NetworkCategory, NetworkFailure};
    use std::cmp::Ordering;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn default_marker_is_trimmed() {
        assert_eq!(
            parse_default_marker("25.12.9.61\n").as_deref(),
            Some("25.12.9.61")
        );
        assert_eq!(
            parse_default_marker("  25.12.9.61  ").as_deref(),
            Some("25.12.9.61")
        );
    }

    #[test]
    fn blank_default_marker_is_no_default() {
        assert_eq!(parse_default_marker(""), None);
        assert_eq!(parse_default_marker("\n"), None);
        assert_eq!(parse_default_marker("   \t\n"), None);
    }

    async fn mount_head(server: &MockServer, endpoint: &str, status: u16) {
        Mock::given(method("HEAD"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(status))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn build_list_treats_404_as_absent_and_keeps_successes_sorted() {
        let server = MockServer::start().await;
        mount_head(&server, "/new", 200).await;
        mount_head(&server, "/old", 200).await;
        mount_head(&server, "/missing", 404).await;
        let candidates = vec![
            ("25.2".to_string(), format!("{}/old", server.uri())),
            ("25.12".to_string(), format!("{}/new", server.uri())),
            ("25.11".to_string(), format!("{}/missing", server.uri())),
        ];

        let versions = scan_build_candidates(candidates, Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(versions, ["25.12", "25.2"]);
    }

    #[tokio::test]
    async fn build_list_reports_rate_limit_instead_of_partial_results() {
        let server = MockServer::start().await;
        mount_head(&server, "/available", 200).await;
        mount_head(&server, "/server-error", 500).await;
        mount_head(&server, "/rate-limit", 429).await;
        let candidates = vec![
            ("25.12".to_string(), format!("{}/available", server.uri())),
            (
                "25.11".to_string(),
                format!("{}/server-error", server.uri()),
            ),
            ("25.10".to_string(), format!("{}/rate-limit", server.uri())),
        ];

        let error = scan_build_candidates(candidates, Duration::from_secs(1))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            Error::Network(NetworkFailure {
                stage: NetworkStage::VersionList,
                category: NetworkCategory::RateLimited,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn build_list_scan_has_a_total_operation_deadline() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(500)))
            .mount(&server)
            .await;
        let candidates = (1..=20)
            .map(|minor| (format!("25.{minor}"), format!("{}/slow", server.uri())))
            .collect();
        let started = tokio::time::Instant::now();

        let error = scan_build_candidates(candidates, Duration::from_millis(60))
            .await
            .unwrap_err();

        assert!(started.elapsed() < Duration::from_millis(250));
        assert!(matches!(
            error,
            Error::Network(NetworkFailure {
                stage: NetworkStage::VersionList,
                category: NetworkCategory::Timeout,
                ..
            })
        ));
    }

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
