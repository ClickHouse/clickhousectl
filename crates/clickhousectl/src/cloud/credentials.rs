use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::failure::{ApiFailure, FailureKind};
use crate::init;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CREDENTIALS_LOCK: &str = "credentials.lock";
const CREDENTIALS_FILE: &str = "credentials.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    /// The API the key pair was minted or verified against, recorded only when
    /// it is not production (#1202). Absent means production, so files written
    /// before the field existed keep resolving as they did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    /// Per-service Query API keys that versions before #1325 created and
    /// stored, keyed by service ID. Never used to authenticate: the records
    /// are kept opaque so a credential write does not drop one before
    /// [`take_legacy_service_query_key`] has told the user about it.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub service_query_keys: HashMap<String, serde_json::Value>,
}

impl Credentials {
    /// A usable management credential pair, shared by resolution and status.
    pub(crate) fn api_credentials(&self) -> Option<(&str, &str)> {
        let key = self.api_key.as_deref()?;
        let secret = self.api_secret.as_deref()?;
        (!key.is_empty() && !secret.is_empty()).then_some((key, secret))
    }

    /// The normalized API base URL the saved key pair belongs to.
    pub(crate) fn api_base_url(&self) -> String {
        self.api_url
            .as_deref()
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or_else(|| crate::cloud::client::DEFAULT_BASE_URL.to_string())
    }
}

/// Saved API keys live with the project, so commands run anywhere beneath its
/// root find them; outside any project they are in the current directory.
pub fn credentials_path() -> std::io::Result<PathBuf> {
    Ok(init::local_dir()?.join(CREDENTIALS_FILE))
}

fn ensure_local_dir() -> CloudResult<PathBuf> {
    let dir = init::local_dir()?;
    ensure_dir(&dir)?;
    Ok(dir)
}

fn ensure_dir(dir: &Path) -> CloudResult<()> {
    std::fs::create_dir_all(dir)?;
    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(gitignore, "*\n")?;
    }
    Ok(())
}

pub(crate) fn open_lock_file(path: &Path) -> std::io::Result<std::fs::File> {
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
    lock_credentials_mutation_in(&init::local_dir()?)
}

fn lock_credentials_mutation_in(dir: &Path) -> CloudResult<std::fs::File> {
    ensure_dir(dir)?;
    let path = dir.join(CREDENTIALS_LOCK);
    open_lock_file(&path).map_err(|error| {
        CloudError::new(format!(
            "failed to lock credential mutations at {}: {error}",
            path.display()
        ))
        .with_failure(ApiFailure::new(FailureKind::Io))
    })
}

/// Read the saved credentials, or `None` when no file exists. A file that
/// cannot be read (tagged `io`) or parsed is an error naming its path, never
/// "no credentials": resolution must not fall through to a lower tier (#1327).
pub(crate) fn try_load_credentials() -> CloudResult<Option<Credentials>> {
    load_credentials_from(&credentials_path()?)
}

fn load_credentials_from(path: &Path) -> CloudResult<Option<Credentials>> {
    let data = match std::fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(
                CloudError::new(format!("failed to read {}: {error}", path.display()))
                    .with_failure(ApiFailure::new(FailureKind::Io)),
            );
        }
    };
    let credentials = serde_json::from_str(&data).map_err(|error| {
        CloudError::new(format!(
            "failed to parse {}: {error}. Fix or remove the file \
             (`clickhousectl cloud auth logout --api-keys` removes it).",
            path.display()
        ))
    })?;
    Ok(Some(credentials))
}

pub fn clear_credentials() -> CloudResult<()> {
    let _lock = lock_credentials_mutation()?;
    let path = credentials_path()?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn save_credentials(creds: &Credentials) -> CloudResult<()> {
    save_credentials_in(&ensure_local_dir()?, creds)
}

fn save_credentials_in(dir: &Path, creds: &Credentials) -> CloudResult<()> {
    ensure_dir(dir)?;
    let json = serde_json::to_string_pretty(creds)?;
    crate::paths::atomic_write(&dir.join(CREDENTIALS_FILE), json.as_bytes(), 0o600)?;
    Ok(())
}

/// Save a management key pair along with the API it belongs to: `api_url` is
/// the resolved base URL the key was minted or verified against, not whether
/// `--url` was passed.
pub(crate) fn set_api_credentials(
    api_key: String,
    api_secret: String,
    api_url: &str,
) -> CloudResult<()> {
    let _lock = lock_credentials_mutation()?;
    let mut creds = try_load_credentials()?.unwrap_or_default();
    creds.api_key = Some(api_key);
    creds.api_secret = Some(api_secret);
    creds.api_url = stored_api_url(api_url);
    save_credentials(&creds)
}

/// The `api_url` to record for a key used against `api_url`: `None` for
/// production, so a production key's file looks as it always has.
fn stored_api_url(api_url: &str) -> Option<String> {
    let api_url = crate::cloud::auth::normalize_api_url(api_url);
    (api_url != crate::cloud::client::DEFAULT_BASE_URL).then_some(api_url)
}

/// Remove and return the stored Query API key record of `service_id`, if any.
///
/// Versions before #1325 created a per-service key and stored it here; such a
/// record is no longer used, so the first command that touches the service
/// drops it and reports the key that still exists in the organization.
///
/// Nothing is created when there is no record to drop: the check runs on
/// every `cloud service query` and `delete`, and a stray `.clickhouse/` would
/// mark the current directory as a project root (#1444).
pub(crate) fn take_legacy_service_query_key(
    service_id: &str,
) -> CloudResult<Option<serde_json::Value>> {
    take_legacy_service_query_key_in(&init::local_dir()?, service_id)
}

fn take_legacy_service_query_key_in(
    dir: &Path,
    service_id: &str,
) -> CloudResult<Option<serde_json::Value>> {
    let path = dir.join(CREDENTIALS_FILE);
    let has_record = |creds: &Option<Credentials>| {
        creds
            .as_ref()
            .is_some_and(|creds| creds.service_query_keys.contains_key(service_id))
    };
    if !has_record(&load_credentials_from(&path)?) {
        return Ok(None);
    }
    let _lock = lock_credentials_mutation_in(dir)?;
    // Another process may have dropped the record before the lock was taken.
    let Some(mut creds) = load_credentials_from(&path)? else {
        return Ok(None);
    };
    let Some(record) = creds.service_query_keys.remove(service_id) else {
        return Ok(None);
    };
    save_credentials_in(dir, &creds)?;
    Ok(Some(record))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_non_prod_api_url_is_recorded() {
        for prod in [
            "https://api.clickhouse.cloud",
            "https://api.clickhouse.cloud/",
            "https://api.clickhouse.cloud/v1",
        ] {
            assert_eq!(stored_api_url(prod), None, "{prod}");
        }
        assert_eq!(
            stored_api_url("https://api.control-plane.clickhouse-dev.com").as_deref(),
            Some("https://api.control-plane.clickhouse-dev.com/v1")
        );
    }

    #[test]
    fn a_file_without_api_url_is_a_prod_key_and_stays_that_shape() {
        let creds: Credentials =
            serde_json::from_value(serde_json::json!({"api_key": "k", "api_secret": "s"})).unwrap();
        assert_eq!(creds.api_url, None);
        assert_eq!(creds.api_base_url(), crate::cloud::client::DEFAULT_BASE_URL);
        let written = serde_json::to_value(&creds).unwrap();
        assert!(written.get("api_url").is_none());

        let dev: Credentials = serde_json::from_value(serde_json::json!({
            "api_key": "k",
            "api_secret": "s",
            "api_url": "https://api.control-plane.clickhouse-dev.com/v1",
        }))
        .unwrap();
        assert_eq!(
            dev.api_base_url(),
            "https://api.control-plane.clickhouse-dev.com/v1"
        );
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
    fn a_stored_query_key_record_survives_a_credential_write_untouched() {
        let raw = r#"{"api_key":"k","api_secret":"s","service_query_keys":{"svc-1":
            {"organization_id":"org-1","api_key_id":"key-1","key_id":"kid","key_secret":"sec",
            "pending_cleanup_api_key_ids":["key-0"],"service_name":"demo",
            "created_at":"2026-05-11T12:00:00Z","unknown":true}}}"#;
        let mut creds: Credentials = serde_json::from_str(raw).unwrap();
        let record = creds.service_query_keys["svc-1"].clone();
        creds.api_key = Some("k2".into());

        let back: Credentials =
            serde_json::from_str(&serde_json::to_string(&creds).unwrap()).unwrap();
        assert_eq!(back.service_query_keys["svc-1"], record);
        assert_eq!(back.api_key.as_deref(), Some("k2"));
    }

    #[test]
    fn taking_a_legacy_query_key_creates_nothing_unless_a_record_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".clickhouse");
        assert!(
            take_legacy_service_query_key_in(&dir, "svc-1")
                .unwrap()
                .is_none()
        );
        assert!(!dir.exists());

        std::fs::create_dir(&dir).unwrap();
        let other = serde_json::json!({
            "api_key": "k", "api_secret": "s",
            "service_query_keys": {"svc-2": {"key_id": "kid"}}
        });
        std::fs::write(dir.join(CREDENTIALS_FILE), other.to_string()).unwrap();
        assert!(
            take_legacy_service_query_key_in(&dir, "svc-1")
                .unwrap()
                .is_none()
        );
        assert!(!dir.join(CREDENTIALS_LOCK).exists());
        assert!(!dir.join(".gitignore").exists());

        let record = take_legacy_service_query_key_in(&dir, "svc-2").unwrap();
        assert_eq!(record, Some(serde_json::json!({"key_id": "kid"})));
        let left = load_credentials_from(&dir.join(CREDENTIALS_FILE))
            .unwrap()
            .unwrap();
        assert!(left.service_query_keys.is_empty());
        assert_eq!(left.api_credentials(), Some(("k", "s")));
        assert!(
            take_legacy_service_query_key_in(&dir, "svc-2")
                .unwrap()
                .is_none()
        );
    }
}
