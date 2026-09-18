//! Cross-process install state.
//!
//! Staging creation briefly takes the cleanup lock and then keeps only its
//! owner lock. Cleanup probes owner locks without blocking. The commit lock is
//! acquired later, after the cleanup lock is gone, so there is no lock cycle.

use crate::error::{Error, Result};
use crate::paths;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const COMMIT_LOCK: &str = ".install-commit.lock";
const STAGING_DIR: &str = ".staging";
const STAGING_CLEANUP_LOCK: &str = ".cleanup.lock";
const STAGING_OWNER_LOCK: &str = ".owner.lock";
const STAGING_PREFIX: &str = "install-";
const STAGING_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

pub(crate) struct CommitLock {
    _file: File,
}

impl CommitLock {
    pub(crate) async fn acquire(versions_dir: &Path) -> Result<Self> {
        let lock_path = versions_dir.join(COMMIT_LOCK);
        let display_path = lock_path.clone();
        tokio::task::spawn_blocking(move || Self::acquire_at(&lock_path))
            .await
            .map_err(|error| {
                Error::Exec(format!(
                    "Failed to wait for install commit lock '{}': {error}",
                    display_path.display()
                ))
            })?
    }

    fn acquire_at(lock_path: &Path) -> Result<Self> {
        let file = open_lock_file(lock_path)?;
        file.lock()?;
        Ok(Self { _file: file })
    }

    pub(crate) fn acquire_blocking(versions_dir: &Path) -> Result<Self> {
        Self::acquire_at(&versions_dir.join(COMMIT_LOCK))
    }
}

pub(crate) struct InstallStaging {
    path: PathBuf,
    payload: PathBuf,
    _owner: File,
}

impl InstallStaging {
    pub(crate) fn create(versions_dir: &Path) -> Result<Self> {
        let staging_root = versions_dir.join(STAGING_DIR);
        paths::ensure_dir(&staging_root)?;

        let cleanup_lock = open_lock_file(&staging_root.join(STAGING_CLEANUP_LOCK))?;
        cleanup_lock.lock()?;
        let stale_before = SystemTime::now()
            .checked_sub(STAGING_MAX_AGE)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        cleanup_staging_before_locked(&staging_root, stale_before);

        loop {
            let path = staging_root.join(format!("{STAGING_PREFIX}{}", Uuid::new_v4()));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    let staging = (|| {
                        let owner = open_lock_file(&path.join(STAGING_OWNER_LOCK))?;
                        owner.lock()?;
                        let payload = path.join("payload");
                        paths::ensure_dir(&payload)?;
                        Ok(Self {
                            path: path.clone(),
                            payload,
                            _owner: owner,
                        })
                    })();
                    if staging.is_err() {
                        let _ = std::fs::remove_dir_all(&path);
                    }
                    return staging;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn payload(&self) -> &Path {
        &self.payload
    }

    pub(crate) fn binary_path(&self) -> PathBuf {
        self.payload.join("clickhouse")
    }
}

impl Drop for InstallStaging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn open_lock_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

fn cleanup_staging_before_locked(staging_root: &Path, stale_before: SystemTime) {
    let Ok(entries) = std::fs::read_dir(staging_root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(id) = name.strip_prefix(STAGING_PREFIX) else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if Uuid::parse_str(id).is_err() || !file_type.is_dir() {
            continue;
        }

        let Ok(modified) = std::fs::metadata(&path).and_then(|metadata| metadata.modified()) else {
            continue;
        };
        if modified >= stale_before {
            continue;
        }

        let Ok(owner) = open_lock_file(&path.join(STAGING_OWNER_LOCK)) else {
            continue;
        };
        if owner.try_lock().is_err() {
            continue;
        }
        let _ = std::fs::remove_dir_all(path);
    }
}

#[cfg(test)]
pub(crate) fn cleanup_staging_before(versions_dir: &Path, stale_before: SystemTime) -> Result<()> {
    let staging_root = versions_dir.join(STAGING_DIR);
    paths::ensure_dir(&staging_root)?;
    let cleanup_lock = open_lock_file(&staging_root.join(STAGING_CLEANUP_LOCK))?;
    cleanup_lock.lock()?;
    cleanup_staging_before_locked(&staging_root, stale_before);
    Ok(())
}

pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
