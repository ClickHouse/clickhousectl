//! Change detection for the floating `latest`/`master` build.
//!
//! `builds.clickhouse.com/master/...` is a single, stable URL whose *content*
//! changes as master moves. The binary carries no externally-readable version
//! (the `--version` string is shared across many master commits, and sibling
//! metadata files 403), so the only cheap change-detection key is the HTTP
//! `etag` (an S3 content hash) exposed on a HEAD request.
//!
//! We record the etag of the installed master build in a small sidecar next to
//! the versions directory. On a later `latest` resolve we do a ~50ms HEAD and
//! compare: unchanged -> reuse the installed binary and skip the ~153MB download
//! *and* the post-download version detection; changed (or no record, or the
//! recorded binary is missing) -> download afresh and re-record.

use crate::error::{NetworkCategory, NetworkFailure, NetworkStage, Result};
use crate::paths;
use crate::version_manager::atomic::{CommitLock, sync_directory};
use crate::version_manager::network;
use crate::version_manager::platform::{DownloadSource, Platform};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One installed master build's change-detection state, per platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterRecord {
    /// HTTP `etag` of the master binary at install time.
    pub etag: String,
    /// HTTP `last-modified` at install time (informational; etag is the key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    /// The detected version string the binary was installed as (the
    /// `versions/<version>/` directory it lives in).
    pub version: String,
}

/// Change-detection headers from a HEAD request to the master URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadInfo {
    pub etag: String,
    pub last_modified: Option<String>,
}

/// The whole sidecar: platform segment (e.g. "macos-aarch64") -> record.
/// Keyed by platform so a shared `~/.clickhouse` survives moving between
/// architectures without a stale-etag false match.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Sidecar {
    #[serde(default)]
    builds: BTreeMap<String, MasterRecord>,
}

/// Path to the sidecar file (`~/.clickhouse/versions/.master-builds.json`).
fn sidecar_path() -> Result<PathBuf> {
    Ok(paths::versions_dir()?.join(".master-builds.json"))
}

fn load_sidecar_at(path: &Path) -> Sidecar {
    let Ok(bytes) = std::fs::read(path) else {
        return Sidecar::default();
    };
    // A corrupt/old-format sidecar is treated as absent -- worst case is one
    // extra download that rewrites it.
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Load the recorded master state for this platform, if any.
fn load_record(platform: &Platform) -> Option<MasterRecord> {
    load_sidecar_at(&sidecar_path().ok()?)
        .builds
        .remove(platform.builds_path())
}

fn clear_version_from(sidecar: &mut Sidecar, version: &str) -> bool {
    let previous_len = sidecar.builds.len();
    sidecar.builds.retain(|_, record| record.version != version);
    sidecar.builds.len() != previous_len
}

fn write_sidecar_atomic(versions_dir: &Path, scratch_dir: &Path, sidecar: &Sidecar) -> Result<()> {
    let temporary_path = scratch_dir.join("master-builds.json.tmp");
    let mut temporary = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)?;
    temporary.write_all(&serde_json::to_vec_pretty(sidecar)?)?;
    temporary.sync_all()?;
    drop(temporary);
    std::fs::rename(&temporary_path, versions_dir.join(".master-builds.json"))?;
    sync_directory(versions_dir)?;
    Ok(())
}

/// Remove every record that points at a version about to be replaced. This is
/// committed before the binary swap so an interruption can only cause an extra
/// download, never reuse a binary that no longer matches its recorded etag.
pub(crate) fn invalidate_version(
    _lock: &CommitLock,
    versions_dir: &Path,
    scratch_dir: &Path,
    version: &str,
) -> Result<()> {
    let path = versions_dir.join(".master-builds.json");
    let mut sidecar = load_sidecar_at(&path);
    if clear_version_from(&mut sidecar, version) {
        write_sidecar_atomic(versions_dir, scratch_dir, &sidecar)?;
    }
    Ok(())
}

/// Persist the master state for this platform, merging into any existing
/// sidecar so other platforms' records are preserved.
pub(crate) fn record_install(
    _lock: &CommitLock,
    versions_dir: &Path,
    scratch_dir: &Path,
    platform: &Platform,
    head: &HeadInfo,
    version: &str,
) -> Result<()> {
    let mut sidecar = load_sidecar_at(&versions_dir.join(".master-builds.json"));
    sidecar.builds.insert(
        platform.builds_path().to_string(),
        MasterRecord {
            etag: head.etag.clone(),
            last_modified: head.last_modified.clone(),
            version: version.to_string(),
        },
    );
    write_sidecar_atomic(versions_dir, scratch_dir, &sidecar)
}

/// HEAD the master URL and pull the change-detection headers.
/// Callers treat failure as best-effort and continue with a download, but the
/// classified error is returned so it can be reported without leaking the URL.
pub async fn head_info(platform: &Platform) -> Result<Option<HeadInfo>> {
    let url = DownloadSource::Builds {
        version_path: "master".to_string(),
    }
    .url(platform);
    head_info_url(&url).await
}

async fn head_info_url(url: &str) -> Result<Option<HeadInfo>> {
    head_info_url_with_policy(url, network::METADATA_POLICY).await
}

async fn head_info_url_with_policy(
    url: &str,
    policy: network::RequestPolicy,
) -> Result<Option<HeadInfo>> {
    let client = network::client(policy, NetworkStage::MasterCheck, url)?;
    let resp = network::send(client.head(url), NetworkStage::MasterCheck, url).await?;
    let resp = network::ensure_success(resp, NetworkStage::MasterCheck, url)?;
    let header = |name: reqwest::header::HeaderName| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };
    let Some(etag) = header(reqwest::header::ETAG) else {
        return Err(NetworkFailure::new(
            NetworkStage::MasterCheck,
            url,
            NetworkCategory::InvalidResponse,
        )
        .into());
    };
    let last_modified = header(reqwest::header::LAST_MODIFIED);
    Ok(Some(HeadInfo {
        etag,
        last_modified,
    }))
}

/// Pure reuse decision: reuse the recorded build only when we have a record,
/// a remote etag, they match, and the recorded binary still exists on disk.
fn should_reuse(
    record: Option<&MasterRecord>,
    remote_etag: Option<&str>,
    binary_exists: bool,
) -> bool {
    match (record, remote_etag) {
        (Some(rec), Some(etag)) => rec.etag == etag && binary_exists,
        _ => false,
    }
}

/// If the installed master build is unchanged from the remote, return the
/// version to reuse (download can be skipped). Otherwise `None`.
///
/// `head` is the result of [`head_info`]; pass it through so the same HEAD
/// result drives both the reuse check and the post-download [`record`].
pub fn reuse_if_unchanged(platform: &Platform, head: Option<&HeadInfo>) -> Option<String> {
    let record = load_record(platform)?;
    let binary_exists = paths::binary_path(&record.version)
        .map(|p| p.exists())
        .unwrap_or(false);
    if should_reuse(Some(&record), head.map(|h| h.etag.as_str()), binary_exists) {
        Some(record.version)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Error, NetworkCategory};
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn rec(etag: &str, version: &str) -> MasterRecord {
        MasterRecord {
            etag: etag.to_string(),
            last_modified: None,
            version: version.to_string(),
        }
    }

    #[tokio::test]
    async fn master_check_reads_change_headers() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/master"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "\"abc-1\"")
                    .insert_header("last-modified", "Wed, 21 Oct 2015 07:28:00 GMT"),
            )
            .mount(&server)
            .await;

        let head = head_info_url(&format!("{}/master", server.uri()))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(head.etag, "\"abc-1\"");
        assert_eq!(
            head.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
    }

    #[tokio::test]
    async fn stalled_master_headers_are_bounded_and_classified() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/master"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(300)))
            .mount(&server)
            .await;
        let policy = network::RequestPolicy {
            connect_timeout: Duration::from_millis(30),
            read_timeout: Duration::from_millis(40),
            request_timeout: Duration::from_millis(60),
        };

        let error =
            head_info_url_with_policy(&format!("{}/master?token=secret", server.uri()), policy)
                .await
                .unwrap_err();

        let Error::Network(failure) = error else {
            panic!("expected network failure");
        };
        assert_eq!(failure.stage, NetworkStage::MasterCheck);
        assert_eq!(failure.category, NetworkCategory::Timeout);
        assert!(!failure.to_string().contains("secret"));
    }

    #[tokio::test]
    async fn master_server_status_is_not_treated_as_unchanged() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/master"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let error = head_info_url(&format!("{}/master", server.uri()))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            Error::Network(NetworkFailure {
                stage: NetworkStage::MasterCheck,
                category: NetworkCategory::ServerError,
                ..
            })
        ));
    }

    #[test]
    fn reuse_when_etag_matches_and_binary_present() {
        assert!(should_reuse(
            Some(&rec("\"abc-1\"", "26.5.1.1")),
            Some("\"abc-1\""),
            true
        ));
    }

    #[test]
    fn no_reuse_when_etag_differs() {
        assert!(!should_reuse(
            Some(&rec("\"abc-1\"", "26.5.1.1")),
            Some("\"def-2\""),
            true
        ));
    }

    #[test]
    fn no_reuse_when_binary_missing() {
        // etag recorded but the installed binary was removed
        assert!(!should_reuse(
            Some(&rec("\"abc-1\"", "26.5.1.1")),
            Some("\"abc-1\""),
            false
        ));
    }

    #[test]
    fn no_reuse_when_no_record() {
        assert!(!should_reuse(None, Some("\"abc-1\""), true));
    }

    #[test]
    fn no_reuse_when_head_failed() {
        // HEAD returned nothing (network error) -- never reuse blindly
        assert!(!should_reuse(
            Some(&rec("\"abc-1\"", "26.5.1.1")),
            None,
            true
        ));
    }

    #[test]
    fn sidecar_round_trips_and_preserves_other_platforms() {
        let mut sidecar = Sidecar::default();
        sidecar
            .builds
            .insert("amd64".to_string(), rec("\"x-1\"", "26.5.1.1"));
        sidecar
            .builds
            .insert("macos-aarch64".to_string(), rec("\"y-2\"", "26.5.1.1"));
        let json = serde_json::to_vec_pretty(&sidecar).unwrap();
        let back: Sidecar = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.builds.get("amd64").unwrap().etag, "\"x-1\"");
        assert_eq!(back.builds.get("macos-aarch64").unwrap().etag, "\"y-2\"");
    }

    #[test]
    fn corrupt_sidecar_deserializes_to_default() {
        let back: Sidecar = serde_json::from_slice(b"not json").unwrap_or_default();
        assert!(back.builds.is_empty());
    }

    #[test]
    fn clear_version_removes_matching_record() {
        let mut sidecar = Sidecar::default();
        sidecar
            .builds
            .insert("macos-aarch64".to_string(), rec("\"x-1\"", "26.5.1.1"));
        assert!(clear_version_from(&mut sidecar, "26.5.1.1"));
        assert!(!sidecar.builds.contains_key("macos-aarch64"));
    }

    #[test]
    fn clear_version_keeps_record_for_other_version() {
        // The record points at a different version dir than the one being
        // overwritten — it still describes the binary on disk, keep it.
        let mut sidecar = Sidecar::default();
        sidecar
            .builds
            .insert("macos-aarch64".to_string(), rec("\"x-1\"", "26.5.1.1"));
        assert!(!clear_version_from(&mut sidecar, "25.12.9.61"));
        assert!(sidecar.builds.contains_key("macos-aarch64"));
    }

    #[test]
    fn clear_version_removes_every_platform_pointing_at_replaced_directory() {
        let mut sidecar = Sidecar::default();
        sidecar
            .builds
            .insert("amd64".to_string(), rec("\"x-1\"", "26.5.1.1"));
        sidecar
            .builds
            .insert("macos-aarch64".to_string(), rec("\"y-2\"", "26.5.1.1"));
        assert!(clear_version_from(&mut sidecar, "26.5.1.1"));
        assert!(sidecar.builds.is_empty());
    }

    #[test]
    fn clear_version_no_record_is_noop() {
        let mut sidecar = Sidecar::default();
        assert!(!clear_version_from(&mut sidecar, "26.5.1.1"));
    }
}
