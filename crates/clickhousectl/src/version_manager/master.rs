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

use crate::error::Result;
use crate::paths;
use crate::version_manager::lock::FileLock;
use crate::version_manager::network::{NetworkFailure, NetworkStage, OperationClient};
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

const SIDECAR_NAME: &str = ".master-builds.json";

/// Path to the sidecar file (`~/.clickhouse/versions/.master-builds.json`).
fn sidecar_path() -> Result<PathBuf> {
    Ok(sidecar_path_in(&paths::versions_dir()?))
}

fn sidecar_path_in(versions_dir: &Path) -> PathBuf {
    versions_dir.join(SIDECAR_NAME)
}

fn load_sidecar_from(path: &Path) -> Sidecar {
    let Ok(bytes) = std::fs::read(path) else {
        return Sidecar::default();
    };
    // A corrupt/old-format sidecar is treated as absent -- worst case is one
    // extra download that rewrites it.
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn load_sidecar() -> Sidecar {
    let Ok(path) = sidecar_path() else {
        return Sidecar::default();
    };
    load_sidecar_from(&path)
}

/// Load the recorded master state for this platform, if any.
fn load_record(platform: &Platform) -> Option<MasterRecord> {
    load_sidecar().builds.remove(platform.builds_path())
}

/// Pure core of [`clear_record_for_version`]: drop the platform's record iff it
/// records exactly `version`. Returns whether the sidecar changed.
fn clear_version_from(sidecar: &mut Sidecar, platform_key: &str, version: &str) -> bool {
    if sidecar
        .builds
        .get(platform_key)
        .is_some_and(|r| r.version == version)
    {
        sidecar.builds.remove(platform_key);
        true
    } else {
        false
    }
}

/// Invalidate the record when a non-master install overwrites `versions/<version>/`:
/// the recorded etag no longer describes the binary on disk, and a stale match
/// would make a later `latest` resolve silently reuse the wrong build.
pub fn clear_record_for_version(platform: &Platform, version: &str) -> Result<()> {
    clear_record_for_version_in(&paths::versions_dir()?, platform, version)
}

pub(crate) fn clear_record_for_version_in(
    versions_dir: &Path,
    platform: &Platform,
    version: &str,
) -> Result<()> {
    let _lock = FileLock::acquire(&versions_dir.join(".locks/master-sidecar.lock"))?;
    let path = sidecar_path_in(versions_dir);
    let mut sidecar = load_sidecar_from(&path);
    pause_after_sidecar_load_for_test();
    if clear_version_from(&mut sidecar, platform.builds_path(), version) {
        write_sidecar_atomically(&path, &sidecar)?;
    }
    Ok(())
}

/// Commit a staged binary while keeping a matching master record safe across
/// process interruption. The old record is invalidated before the binary swap;
/// after that point an interruption can only cause a later redundant download.
pub(crate) fn commit_install_in<F>(
    versions_dir: &Path,
    platform: &Platform,
    version: &str,
    new_head: Option<&HeadInfo>,
    commit_binary: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let _lock = FileLock::acquire(&versions_dir.join(".locks/master-sidecar.lock"))?;
    let path = sidecar_path_in(versions_dir);
    let mut sidecar = load_sidecar_from(&path);
    pause_after_sidecar_load_for_test();

    if clear_version_from(&mut sidecar, platform.builds_path(), version) {
        write_sidecar_atomically(&path, &sidecar)?;
    }

    pause_before_binary_commit_for_test();
    commit_binary()?;

    if let Some(head) = new_head {
        sidecar.builds.insert(
            platform.builds_path().to_string(),
            MasterRecord {
                etag: head.etag.clone(),
                last_modified: head.last_modified.clone(),
                version: version.to_string(),
            },
        );
        // The binary is already safely committed. Failure to restore the
        // optional cache record only forces a later download.
        let _ = write_sidecar_atomically(&path, &sidecar);
    }

    Ok(())
}

fn write_sidecar_atomically(path: &Path, sidecar: &Sidecar) -> Result<()> {
    let json = serde_json::to_vec_pretty(sidecar)?;
    let parent = path.parent().expect("sidecar path has a parent");
    let temp_path = parent.join(format!(
        ".master-builds-{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        file.write_all(&json)?;
        file.sync_all()?;
        std::fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
fn pause_after_sidecar_load_for_test() {
    let (Ok(marker), Ok(release)) = (
        std::env::var("CHCTL_TEST_SIDECAR_LOCKED"),
        std::env::var("CHCTL_TEST_SIDECAR_RELEASE"),
    ) else {
        return;
    };
    std::fs::write(marker, b"locked").expect("write sidecar lock marker");
    let release = PathBuf::from(release);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !release.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release sidecar lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[cfg(not(test))]
fn pause_after_sidecar_load_for_test() {}

#[cfg(test)]
fn pause_before_binary_commit_for_test() {
    let (Ok(marker), Ok(release)) = (
        std::env::var("CHCTL_TEST_BINARY_COMMIT_PAUSED"),
        std::env::var("CHCTL_TEST_BINARY_COMMIT_RELEASE"),
    ) else {
        return;
    };
    std::fs::write(marker, b"paused").expect("write binary commit marker");
    let release = PathBuf::from(release);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !release.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to commit binary"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[cfg(not(test))]
fn pause_before_binary_commit_for_test() {}

/// HEAD the master URL and pull the change-detection headers.
/// Best-effort: returns `None` on any network/build error, so callers fall
/// back to an unconditional download rather than failing.
pub async fn head_info(platform: &Platform) -> Option<HeadInfo> {
    let url = DownloadSource::Builds {
        version_path: "master".to_string(),
    }
    .url(platform);

    let client = OperationClient::metadata(NetworkStage::MasterCheck, &url).ok()?;
    head_info_from_url(&client, &url).await.ok().flatten()
}

async fn head_info_from_url(
    client: &OperationClient,
    url: &str,
) -> std::result::Result<Option<HeadInfo>, NetworkFailure> {
    let resp = client.head(url, NetworkStage::MasterCheck).await?;
    if !resp.status().is_success() {
        return Err(NetworkFailure::from_response(
            NetworkStage::MasterCheck,
            url,
            &resp,
        ));
    }
    let header = |name: reqwest::header::HeaderName| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };
    let Some(etag) = header(reqwest::header::ETAG) else {
        return Ok(None);
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
/// result drives both the reuse check and the post-download master record.
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

    fn rec(etag: &str, version: &str) -> MasterRecord {
        MasterRecord {
            etag: etag.to_string(),
            last_modified: None,
            version: version.to_string(),
        }
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
        assert!(clear_version_from(
            &mut sidecar,
            "macos-aarch64",
            "26.5.1.1"
        ));
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
        assert!(!clear_version_from(
            &mut sidecar,
            "macos-aarch64",
            "25.12.9.61"
        ));
        assert!(sidecar.builds.contains_key("macos-aarch64"));
    }

    #[test]
    fn clear_version_keeps_other_platforms() {
        let mut sidecar = Sidecar::default();
        sidecar
            .builds
            .insert("amd64".to_string(), rec("\"x-1\"", "26.5.1.1"));
        sidecar
            .builds
            .insert("macos-aarch64".to_string(), rec("\"y-2\"", "26.5.1.1"));
        assert!(clear_version_from(
            &mut sidecar,
            "macos-aarch64",
            "26.5.1.1"
        ));
        assert!(sidecar.builds.contains_key("amd64"));
    }

    #[test]
    fn clear_version_no_record_is_noop() {
        let mut sidecar = Sidecar::default();
        assert!(!clear_version_from(
            &mut sidecar,
            "macos-aarch64",
            "26.5.1.1"
        ));
    }
}
