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
use crate::version_manager::platform::{DownloadSource, Platform};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
fn sidecar_path() -> Result<std::path::PathBuf> {
    Ok(paths::versions_dir()?.join(".master-builds.json"))
}

fn load_sidecar() -> Sidecar {
    let Ok(path) = sidecar_path() else {
        return Sidecar::default();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Sidecar::default();
    };
    // A corrupt/old-format sidecar is treated as absent -- worst case is one
    // extra download that rewrites it.
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Load the recorded master state for this platform, if any.
fn load_record(platform: &Platform) -> Option<MasterRecord> {
    load_sidecar().builds.remove(platform.builds_path())
}

/// Persist the master state for this platform, merging into any existing
/// sidecar so other platforms' records are preserved.
pub fn record(platform: &Platform, head: &HeadInfo, version: &str) -> Result<()> {
    let mut sidecar = load_sidecar();
    sidecar.builds.insert(
        platform.builds_path().to_string(),
        MasterRecord {
            etag: head.etag.clone(),
            last_modified: head.last_modified.clone(),
            version: version.to_string(),
        },
    );
    let path = sidecar_path()?;
    let json = serde_json::to_vec_pretty(&sidecar)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// HEAD the master URL and pull the change-detection headers.
/// Best-effort: returns `None` on any network/build error, so callers fall
/// back to an unconditional download rather than failing.
pub async fn head_info(platform: &Platform) -> Option<HeadInfo> {
    let url = DownloadSource::Builds {
        version_path: "master".to_string(),
    }
    .url(platform);

    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .build()
        .ok()?;

    let resp = client.head(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let header = |name: reqwest::header::HeaderName| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };
    let etag = header(reqwest::header::ETAG)?;
    let last_modified = header(reqwest::header::LAST_MODIFIED);
    Some(HeadInfo { etag, last_modified })
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
}
