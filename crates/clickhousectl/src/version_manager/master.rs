//! Change detection for the floating `latest`/`master` build.
//!
//! `builds.clickhouse.com/master/...` is a single, stable URL whose *content*
//! changes as master moves. Multiple builds can share a version string, so
//! reuse requires remote validation of the installed artifact's HTTP ETag.
//!
//! Downloads record their response headers only after committing the binary.
//! Later requests send that ETag with `If-None-Match`; a 304 avoids transferring
//! the body and detecting the version again.

use crate::error::Result;
use crate::paths;
use crate::version_manager::atomic::{CommitLock, sync_directory};
use crate::version_manager::platform::Platform;
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

/// Change-detection headers from the response that supplied the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInfo {
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
    info: &ArtifactInfo,
    version: &str,
) -> Result<()> {
    let mut sidecar = load_sidecar_at(&versions_dir.join(".master-builds.json"));
    sidecar.builds.insert(
        platform.builds_path().to_string(),
        MasterRecord {
            etag: info.etag.clone(),
            last_modified: info.last_modified.clone(),
            version: version.to_string(),
        },
    );
    write_sidecar_atomic(versions_dir, scratch_dir, &sidecar)
}

/// Accept one HTTP entity-tag, never a wildcard or list of validators.
pub(crate) fn usable_etag(etag: &str) -> bool {
    let opaque = etag.strip_prefix("W/").unwrap_or(etag);
    opaque.len() >= 2
        && opaque.starts_with('"')
        && opaque.ends_with('"')
        && opaque.as_bytes()[1..opaque.len() - 1]
            .iter()
            .all(|byte| *byte == 0x21 || (0x23..=0x7e).contains(byte))
}

impl ArtifactInfo {
    pub(crate) fn from_headers(headers: &reqwest::header::HeaderMap) -> Option<Self> {
        let etag = headers.get(reqwest::header::ETAG)?.to_str().ok()?;
        if !usable_etag(etag) {
            return None;
        }
        Some(Self {
            etag: etag.to_string(),
            last_modified: headers
                .get(reqwest::header::LAST_MODIFIED)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
        })
    }
}

/// Read the sidecar and binary together under the install commit lock.
pub(crate) fn usable_record(_lock: &CommitLock, platform: &Platform) -> Option<MasterRecord> {
    let record = load_record(platform)?;
    if usable_etag(&record.etag) && paths::binary_path(&record.version).ok()?.is_file() {
        Some(record)
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
    fn validators_must_be_single_entity_tags() {
        for etag in ["\"abc-1\"", "W/\"abc-1\"", "\"\""] {
            assert!(usable_etag(etag), "{etag}");
        }
        for etag in ["", "*", "abc", "\"a\", \"b\"", "\"bad\nvalue\""] {
            assert!(!usable_etag(etag), "{etag}");
        }
    }

    #[test]
    fn response_metadata_requires_a_usable_etag() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::LAST_MODIFIED,
            "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap(),
        );
        assert!(ArtifactInfo::from_headers(&headers).is_none());
        headers.insert(reqwest::header::ETAG, "*".parse().unwrap());
        assert!(ArtifactInfo::from_headers(&headers).is_none());
        headers.insert(reqwest::header::ETAG, "\"abc\"".parse().unwrap());
        let info = ArtifactInfo::from_headers(&headers).unwrap();
        assert_eq!(info.etag, "\"abc\"");
        assert_eq!(
            info.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
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
