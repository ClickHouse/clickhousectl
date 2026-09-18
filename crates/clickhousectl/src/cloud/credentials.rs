use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::init;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

const QUERY_PROVISIONING_LOCK: &str = "query-provisioning.lock";
const CREDENTIALS_LOCK: &str = "credentials.lock";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub service_query_keys: HashMap<String, ServiceQueryKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceQueryKey {
    /// Organization in which the management API key was provisioned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    /// Management API resource ID used to delete this exact key.
    ///
    /// Records written before the cleanup metadata was introduced remain
    /// usable for queries, but cannot be safely cleaned up by service deletion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
    pub key_id: String,
    pub key_secret: String,
    /// The query endpoint the key is bound to, when the upsert echoed it.
    /// Authentication uses the key pair, so an omitted `id` still yields a
    /// usable record, but exact endpoint ownership is required for repair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
    /// Exact superseded management key IDs whose deletion must be retried.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_cleanup_api_key_ids: Vec<String>,
    pub service_name: String,
    pub created_at: DateTime<Utc>,
}

pub fn credentials_path() -> PathBuf {
    init::local_dir().join("credentials.json")
}

fn ensure_local_dir() -> CloudResult<PathBuf> {
    let dir = init::local_dir();
    std::fs::create_dir_all(&dir)?;
    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(gitignore, "*\n")?;
    }
    Ok(dir)
}

fn open_lock_file(path: &Path) -> std::io::Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

fn lock_credentials_mutation() -> CloudResult<std::fs::File> {
    let path = ensure_local_dir()?.join(CREDENTIALS_LOCK);
    open_lock_file(&path).map_err(|error| {
        CloudError::new(format!(
            "failed to lock credential mutations at {}: {error}",
            path.display()
        ))
    })
}

/// Holds the project-wide provisioning lock for its lifetime.
///
/// The lock is owned by the open file handle, not by lock-file contents. If a
/// process exits midway through provisioning, the OS releases the lock; the
/// leftover file is safely opened and locked by the next process.
pub(crate) struct QueryProvisioningLock {
    _file: std::fs::File,
}

pub(crate) async fn lock_query_provisioning() -> CloudResult<QueryProvisioningLock> {
    let lock_path = ensure_local_dir()?.join(QUERY_PROVISIONING_LOCK);
    let display_path = lock_path.clone();
    let file = tokio::task::spawn_blocking(move || open_lock_file(&lock_path))
        .await
        .map_err(|error| {
            CloudError::new(format!(
                "failed to wait for query provisioning lock {}: {error}",
                display_path.display()
            ))
        })?
        .map_err(|error| {
            CloudError::new(format!(
                "failed to lock query provisioning at {}: {error}",
                display_path.display()
            ))
        })?;
    Ok(QueryProvisioningLock { _file: file })
}

pub fn load_credentials() -> Option<Credentials> {
    let path = credentials_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn try_load_credentials() -> CloudResult<Option<Credentials>> {
    let path = credentials_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(CloudError::new(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    let credentials = serde_json::from_str(&data)
        .map_err(|error| CloudError::new(format!("failed to parse {}: {error}", path.display())))?;
    Ok(Some(credentials))
}

pub fn clear_credentials() -> CloudResult<()> {
    let _lock = lock_credentials_mutation()?;
    let path = credentials_path();
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn save_credentials(creds: &Credentials) -> CloudResult<()> {
    ensure_local_dir()?;
    let path = credentials_path();
    save_credentials_to(&path, creds, sync_directory)
}

fn save_credentials_to(
    path: &Path,
    creds: &Credentials,
    sync_parent: impl FnOnce(&Path) -> std::io::Result<()>,
) -> CloudResult<()> {
    let dir = path
        .parent()
        .ok_or_else(|| CloudError::new(format!("{} has no parent directory", path.display())))?;
    let json = serde_json::to_string_pretty(creds)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".credentials.json.")
        .tempfile_in(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    temporary.write_all(json.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| {
        CloudError::new(format!(
            "failed to replace {}: {}",
            path.display(),
            error.error
        ))
    })?;

    // The replacement is already authoritative. Reporting a later directory
    // fsync failure as an unsuccessful save can make callers delete a live key
    // whose credentials are present in this file.
    let _ = sync_parent(dir);

    Ok(())
}

#[cfg(unix)]
fn sync_directory(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

pub fn try_get_service_query_key(service_id: &str) -> CloudResult<Option<ServiceQueryKey>> {
    Ok(try_load_credentials()?
        .and_then(|credentials| credentials.service_query_keys.get(service_id).cloned()))
}

pub(crate) fn set_service_query_key(
    service_id: &str,
    key: ServiceQueryKey,
    _lock: &QueryProvisioningLock,
) -> CloudResult<()> {
    let _mutation_lock = lock_credentials_mutation()?;
    let mut creds = try_load_credentials()?.unwrap_or_default();
    creds.service_query_keys.insert(service_id.to_string(), key);
    save_credentials(&creds)
}

pub(crate) fn set_api_credentials(api_key: String, api_secret: String) -> CloudResult<()> {
    let _lock = lock_credentials_mutation()?;
    let mut creds = try_load_credentials()?.unwrap_or_default();
    creds.api_key = Some(api_key);
    creds.api_secret = Some(api_secret);
    save_credentials(&creds)
}

pub fn remove_service_query_key(service_id: &str) -> CloudResult<()> {
    let _lock = lock_credentials_mutation()?;
    let Some(mut creds) = try_load_credentials()? else {
        return Ok(());
    };
    if creds.service_query_keys.remove(service_id).is_some() {
        save_credentials(&creds)?;
    }
    Ok(())
}

/// Drop `cleaned_api_key_ids` from the pending-retirement list of the stored
/// query key for `service_id`, only while the record still names management
/// key `api_key_id` as its active key.
///
/// A retirement is retried on a later run (#527), and the record may have
/// been replaced or removed by a concurrent repair or service deletion in
/// the meantime: a list belonging to another record is not ours to edit, and
/// a removed record must not be resurrected. Holding the provisioning lock
/// keeps a repair from writing between the compare and the update. Returns
/// whether the record was updated.
pub(crate) fn remove_pending_cleanup_if_api_key_matches(
    service_id: &str,
    api_key_id: &str,
    cleaned_api_key_ids: &[String],
    _lock: &QueryProvisioningLock,
) -> CloudResult<bool> {
    let _mutation_lock = lock_credentials_mutation()?;
    let Some(mut creds) = try_load_credentials()? else {
        return Ok(false);
    };
    if !retain_uncleaned_pending_if_api_key_matches(
        &mut creds,
        service_id,
        api_key_id,
        cleaned_api_key_ids,
    ) {
        return Ok(false);
    }
    save_credentials(&creds)?;
    Ok(true)
}

/// The compare-and-update behind [`remove_pending_cleanup_if_api_key_matches`].
fn retain_uncleaned_pending_if_api_key_matches(
    creds: &mut Credentials,
    service_id: &str,
    api_key_id: &str,
    cleaned_api_key_ids: &[String],
) -> bool {
    let Some(stored) = creds.service_query_keys.get_mut(service_id) else {
        return false;
    };
    if stored.api_key_id.as_deref() != Some(api_key_id) {
        return false;
    }
    stored
        .pending_cleanup_api_key_ids
        .retain(|pending| !cleaned_api_key_ids.contains(pending));
    true
}

/// Record `pending_api_key_id` as awaiting deletion on the record for
/// `service_id`, but only while that record still names `api_key_id` as its
/// active key: a rolled-back repair's replacement key whose delete failed
/// belongs on the record the repair started from, not on a concurrent
/// repair's fresh one (#658). The ID is appended once. Returns whether the
/// record was updated.
pub(crate) fn add_pending_cleanup_if_api_key_matches(
    service_id: &str,
    api_key_id: &str,
    pending_api_key_id: &str,
    _lock: &QueryProvisioningLock,
) -> CloudResult<bool> {
    let _mutation_lock = lock_credentials_mutation()?;
    let Some(mut creds) = try_load_credentials()? else {
        return Ok(false);
    };
    if !push_pending_if_api_key_matches(&mut creds, service_id, api_key_id, pending_api_key_id) {
        return Ok(false);
    }
    save_credentials(&creds)?;
    Ok(true)
}

/// The compare-and-append behind [`add_pending_cleanup_if_api_key_matches`].
/// The active key itself is never listed as pending: a record that marks its
/// own key for deletion is refused by repair, so it must not be produced here.
fn push_pending_if_api_key_matches(
    creds: &mut Credentials,
    service_id: &str,
    api_key_id: &str,
    pending_api_key_id: &str,
) -> bool {
    let Some(stored) = creds.service_query_keys.get_mut(service_id) else {
        return false;
    };
    if stored.api_key_id.as_deref() != Some(api_key_id) || pending_api_key_id == api_key_id {
        return false;
    }
    if !stored
        .pending_cleanup_api_key_ids
        .iter()
        .any(|pending| pending == pending_api_key_id)
    {
        stored
            .pending_cleanup_api_key_ids
            .push(pending_api_key_id.to_string());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_credentials_succeed_when_directory_sync_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("credentials.json");
        let creds = Credentials {
            api_key: Some("key".into()),
            api_secret: Some("secret".into()),
            ..Credentials::default()
        };

        save_credentials_to(&path, &creds, |_| {
            Err(std::io::Error::other("directory sync failed"))
        })
        .unwrap();

        let stored: Credentials = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(stored.api_key.as_deref(), Some("key"));
        assert_eq!(stored.api_secret.as_deref(), Some("secret"));
    }

    #[test]
    fn legacy_credentials_round_trip() {
        // Legacy creds files have only api_key/api_secret as bare strings.
        let raw = r#"{"api_key":"k","api_secret":"s"}"#;
        let creds: Credentials = serde_json::from_str(raw).unwrap();
        assert_eq!(creds.api_key.as_deref(), Some("k"));
        assert_eq!(creds.api_secret.as_deref(), Some("s"));
        assert!(creds.service_query_keys.is_empty());

        let written = serde_json::to_string(&creds).unwrap();
        assert!(written.contains("\"api_key\":\"k\""));
        assert!(!written.contains("service_query_keys"));
    }

    #[test]
    fn service_query_keys_round_trip() {
        let mut creds = Credentials::default();
        creds.service_query_keys.insert(
            "svc-1".into(),
            ServiceQueryKey {
                organization_id: Some("org-1".into()),
                api_key_id: Some("api-key-uuid".into()),
                key_id: "kid".into(),
                key_secret: "sec".into(),
                endpoint_id: Some("ep".into()),
                pending_cleanup_api_key_ids: vec![],
                service_name: "demo".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-05-11T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            },
        );

        let s = serde_json::to_string(&creds).unwrap();
        assert!(s.contains("\"organization_id\":\"org-1\""));
        assert!(s.contains("\"api_key_id\":\"api-key-uuid\""));
        let back: Credentials = serde_json::from_str(&s).unwrap();
        let key = back.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.organization_id.as_deref(), Some("org-1"));
        assert_eq!(key.api_key_id.as_deref(), Some("api-key-uuid"));
        assert_eq!(key.key_id, "kid");
        assert_eq!(key.key_secret, "sec");
        assert_eq!(key.endpoint_id.as_deref(), Some("ep"));
        assert!(key.pending_cleanup_api_key_ids.is_empty());
        assert_eq!(key.service_name, "demo");
    }

    #[test]
    fn stored_key_without_an_endpoint_id_round_trips_with_the_field_omitted() {
        let mut creds = Credentials::default();
        creds.service_query_keys.insert(
            "svc-1".into(),
            ServiceQueryKey {
                organization_id: Some("org-1".into()),
                api_key_id: Some("api-key-uuid".into()),
                key_id: "kid".into(),
                key_secret: "sec".into(),
                endpoint_id: None,
                pending_cleanup_api_key_ids: vec![],
                service_name: "demo".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-05-11T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            },
        );

        let s = serde_json::to_string(&creds).unwrap();
        assert!(!s.contains("endpoint_id"), "absent means omitted: {s}");
        let back: Credentials = serde_json::from_str(&s).unwrap();
        let key = back.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.key_id, "kid");
        assert_eq!(key.key_secret, "sec");
        assert_eq!(key.endpoint_id, None);
        assert!(key.pending_cleanup_api_key_ids.is_empty());
    }

    fn stored_key(api_key_id: Option<&str>) -> ServiceQueryKey {
        ServiceQueryKey {
            organization_id: Some("org-1".into()),
            api_key_id: api_key_id.map(str::to_string),
            key_id: "kid".into(),
            key_secret: "sec".into(),
            endpoint_id: Some("ep".into()),
            pending_cleanup_api_key_ids: vec![],
            service_name: "demo".into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn existing_query_keys_without_cleanup_metadata_still_deserialize() {
        // Existing files contain query credentials and an endpoint ID, but
        // not the ownership metadata added for exact cloud-side cleanup.
        let raw = r#"{"service_query_keys":{"svc-1":{"key_id":"kid","key_secret":"sec",
            "endpoint_id":"ep","service_name":"demo","created_at":"2026-05-11T12:00:00Z"}}}"#;
        let creds: Credentials = serde_json::from_str(raw).unwrap();
        let key = creds.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.organization_id, None);
        assert_eq!(key.api_key_id, None);
        assert_eq!(key.endpoint_id.as_deref(), Some("ep"));
        assert!(key.pending_cleanup_api_key_ids.is_empty());
        assert_eq!(key.key_id, "kid");

        let written = serde_json::to_string(&creds).unwrap();
        assert!(!written.contains("organization_id"));
        assert!(!written.contains("api_key_id"));
    }

    #[test]
    fn a_record_with_pending_retirements_round_trips_and_a_legacy_one_reads_empty() {
        let mut creds = Credentials::default();
        let mut key = stored_key(Some("active-key"));
        key.pending_cleanup_api_key_ids = vec!["retired-1".into(), "retired-2".into()];
        creds.service_query_keys.insert("svc-1".into(), key);

        let written = serde_json::to_string(&creds).unwrap();
        assert!(
            written.contains(r#""pending_cleanup_api_key_ids":["retired-1","retired-2"]"#),
            "{written}"
        );
        let back: Credentials = serde_json::from_str(&written).unwrap();
        assert_eq!(
            back.service_query_keys["svc-1"].pending_cleanup_api_key_ids,
            ["retired-1", "retired-2"]
        );

        // A record written before retirements were tracked has no list at
        // all and must read as "nothing pending", never fail to parse.
        let legacy = r#"{"service_query_keys":{"svc-1":{"organization_id":"org-1",
            "api_key_id":"active-key","key_id":"kid","key_secret":"sec","endpoint_id":"ep",
            "service_name":"demo","created_at":"2026-05-11T12:00:00Z"}}}"#;
        let creds: Credentials = serde_json::from_str(legacy).unwrap();
        assert!(
            creds.service_query_keys["svc-1"]
                .pending_cleanup_api_key_ids
                .is_empty()
        );
        // And an empty list is omitted again on the way out.
        assert!(
            !serde_json::to_string(&creds)
                .unwrap()
                .contains("pending_cleanup")
        );
    }

    #[test]
    fn pending_retirements_are_removed_only_from_the_record_that_still_names_the_active_key() {
        let mut creds = Credentials::default();
        let mut key = stored_key(Some("active-key"));
        key.pending_cleanup_api_key_ids = vec!["retired-1".into(), "retired-2".into()];
        creds.service_query_keys.insert("svc-1".into(), key);

        // Only the cleaned id goes; an id whose deletion failed stays.
        assert!(retain_uncleaned_pending_if_api_key_matches(
            &mut creds,
            "svc-1",
            "active-key",
            &["retired-1".to_string()],
        ));
        assert_eq!(
            creds.service_query_keys["svc-1"].pending_cleanup_api_key_ids,
            ["retired-2"]
        );

        // A concurrent repair replaced the active key: the list belongs to
        // the new record and is left alone.
        assert!(!retain_uncleaned_pending_if_api_key_matches(
            &mut creds,
            "svc-1",
            "previous-active-key",
            &["retired-2".to_string()],
        ));
        assert_eq!(
            creds.service_query_keys["svc-1"].pending_cleanup_api_key_ids,
            ["retired-2"]
        );

        // A removed record is not resurrected.
        let mut empty = Credentials::default();
        assert!(!retain_uncleaned_pending_if_api_key_matches(
            &mut empty,
            "svc-1",
            "active-key",
            &["retired-2".to_string()],
        ));
        assert!(empty.service_query_keys.is_empty());
    }

    #[test]
    fn pending_cleanup_is_appended_only_while_the_active_key_matches() {
        let mut creds = Credentials::default();
        creds.service_query_keys.insert(
            "svc-1".into(),
            ServiceQueryKey {
                organization_id: Some("org-1".into()),
                api_key_id: Some("active-key".into()),
                key_id: "kid".into(),
                key_secret: "sec".into(),
                endpoint_id: Some("ep".into()),
                pending_cleanup_api_key_ids: vec!["retired-1".into()],
                service_name: "demo".into(),
                created_at: Utc::now(),
            },
        );

        // The rolled-back key joins the list once, after what was there.
        assert!(push_pending_if_api_key_matches(
            &mut creds,
            "svc-1",
            "active-key",
            "rolled-back",
        ));
        assert!(push_pending_if_api_key_matches(
            &mut creds,
            "svc-1",
            "active-key",
            "rolled-back",
        ));
        assert_eq!(
            creds.service_query_keys["svc-1"].pending_cleanup_api_key_ids,
            ["retired-1", "rolled-back"]
        );

        // Never the active key itself: repair refuses such a record.
        assert!(!push_pending_if_api_key_matches(
            &mut creds,
            "svc-1",
            "active-key",
            "active-key",
        ));
        assert_eq!(
            creds.service_query_keys["svc-1"].pending_cleanup_api_key_ids,
            ["retired-1", "rolled-back"]
        );

        // A concurrent repair replaced the active key: not this run's record.
        assert!(!push_pending_if_api_key_matches(
            &mut creds,
            "svc-1",
            "previous-active-key",
            "other",
        ));
        assert!(!push_pending_if_api_key_matches(
            &mut creds,
            "svc-missing",
            "active-key",
            "other",
        ));
        assert_eq!(
            creds.service_query_keys["svc-1"].pending_cleanup_api_key_ids,
            ["retired-1", "rolled-back"]
        );
    }
}
