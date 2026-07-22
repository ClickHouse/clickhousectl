use crate::error::Result;
use crate::paths;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SymlinkOutcome {
    Created {
        path: PathBuf,
        target: PathBuf,
    },
    Updated {
        path: PathBuf,
        target: PathBuf,
    },
    Unchanged {
        path: PathBuf,
    },
    SkippedRegularFile {
        path: PathBuf,
    },
    #[allow(dead_code)] // only constructed on non-unix
    SkippedNonUnix,
}

#[cfg(unix)]
pub fn ensure_global_symlink(version: &str) -> Result<SymlinkOutcome> {
    let link = paths::global_clickhouse_symlink()?;
    let target = paths::binary_path(version)?;
    let bin_dir = paths::global_bin_dir()?;
    ensure_symlink_at(&link, &target, &bin_dir)
}

#[cfg(not(unix))]
pub fn ensure_global_symlink(_version: &str) -> Result<SymlinkOutcome> {
    Ok(SymlinkOutcome::SkippedNonUnix)
}

#[cfg(unix)]
pub fn remove_global_symlink_for(removed_version: &str) -> Result<()> {
    let link = paths::global_clickhouse_symlink()?;
    let expected_dir = paths::version_dir(removed_version)?;
    remove_symlink_at(&link, &expected_dir)
}

#[cfg(not(unix))]
pub fn remove_global_symlink_for(_removed_version: &str) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn ensure_symlink_at(link: &Path, target: &Path, bin_dir: &Path) -> Result<SymlinkOutcome> {
    use std::os::unix::fs::symlink;

    if let Err(e) = std::fs::create_dir_all(bin_dir) {
        eprintln!("warning: could not create {}: {}", bin_dir.display(), e);
        return Ok(SymlinkOutcome::Unchanged {
            path: link.to_path_buf(),
        });
    }

    match std::fs::symlink_metadata(link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if let Ok(existing) = std::fs::read_link(link)
                && existing == target
            {
                maybe_warn_path_missing(bin_dir);
                return Ok(SymlinkOutcome::Unchanged {
                    path: link.to_path_buf(),
                });
            }
            if let Err(e) = std::fs::remove_file(link) {
                eprintln!("warning: could not update {}: {}", link.display(), e);
                return Ok(SymlinkOutcome::Unchanged {
                    path: link.to_path_buf(),
                });
            }
            if let Err(e) = symlink(target, link) {
                eprintln!("warning: could not update {}: {}", link.display(), e);
                return Ok(SymlinkOutcome::Unchanged {
                    path: link.to_path_buf(),
                });
            }
            maybe_warn_path_missing(bin_dir);
            Ok(SymlinkOutcome::Updated {
                path: link.to_path_buf(),
                target: target.to_path_buf(),
            })
        }
        Ok(_) => {
            eprintln!(
                "warning: {} exists and is not a symlink; leaving it alone. \
                 Remove it manually if you want `clickhouse` on PATH, or pass --no-global to silence this.",
                link.display()
            );
            Ok(SymlinkOutcome::SkippedRegularFile {
                path: link.to_path_buf(),
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Err(e) = symlink(target, link) {
                eprintln!("warning: could not create {}: {}", link.display(), e);
                return Ok(SymlinkOutcome::Unchanged {
                    path: link.to_path_buf(),
                });
            }
            maybe_warn_path_missing(bin_dir);
            Ok(SymlinkOutcome::Created {
                path: link.to_path_buf(),
                target: target.to_path_buf(),
            })
        }
        Err(e) => {
            eprintln!("warning: could not stat {}: {}", link.display(), e);
            Ok(SymlinkOutcome::Unchanged {
                path: link.to_path_buf(),
            })
        }
    }
}

#[cfg(unix)]
fn remove_symlink_at(link: &Path, expected_target_dir: &Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(link) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };
    if !meta.file_type().is_symlink() {
        return Ok(());
    }
    let target = match std::fs::read_link(link) {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };
    if target.starts_with(expected_target_dir) {
        let _ = std::fs::remove_file(link);
    }
    Ok(())
}

#[cfg(unix)]
fn maybe_warn_path_missing(bin_dir: &Path) {
    let path_env = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return,
    };
    let bin_canon = std::fs::canonicalize(bin_dir).unwrap_or_else(|_| bin_dir.to_path_buf());
    let on_path = std::env::split_paths(&path_env).any(|entry| {
        let entry_canon = std::fs::canonicalize(&entry).unwrap_or(entry);
        entry_canon == bin_canon
    });
    if !on_path {
        eprintln!(
            "note: {} is not on $PATH. Add it (e.g. `export PATH=\"{}:$PATH\"` in your shell rc) to run `clickhouse` directly.",
            bin_dir.display(),
            bin_dir.display()
        );
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    fn version_binary(tmp: &Path, version: &str) -> PathBuf {
        let dir = tmp.join("versions").join(version);
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("clickhouse");
        std::fs::write(&bin, b"#!/bin/sh\necho fake").unwrap();
        bin
    }

    #[test]
    fn creates_symlink_when_absent() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        let link = bin_dir.join("clickhouse");
        let target = version_binary(tmp.path(), "25.12.5.44");

        let outcome = ensure_symlink_at(&link, &target, &bin_dir).unwrap();

        assert!(matches!(outcome, SymlinkOutcome::Created { .. }));
        let meta = std::fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
    }

    #[test]
    fn updates_existing_symlink() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let link = bin_dir.join("clickhouse");
        let old_target = tmp.path().join("nowhere");
        symlink(&old_target, &link).unwrap();
        let new_target = version_binary(tmp.path(), "25.12.5.44");

        let outcome = ensure_symlink_at(&link, &new_target, &bin_dir).unwrap();

        assert!(matches!(outcome, SymlinkOutcome::Updated { .. }));
        assert_eq!(std::fs::read_link(&link).unwrap(), new_target);
    }

    #[test]
    fn unchanged_when_link_already_correct() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let link = bin_dir.join("clickhouse");
        let target = version_binary(tmp.path(), "25.12.5.44");
        symlink(&target, &link).unwrap();

        let outcome = ensure_symlink_at(&link, &target, &bin_dir).unwrap();

        assert!(matches!(outcome, SymlinkOutcome::Unchanged { .. }));
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
    }

    #[test]
    fn refuses_regular_file_and_preserves_it() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let link = bin_dir.join("clickhouse");
        std::fs::write(&link, b"user-installed binary").unwrap();
        let target = version_binary(tmp.path(), "25.12.5.44");

        let outcome = ensure_symlink_at(&link, &target, &bin_dir).unwrap();

        assert!(matches!(outcome, SymlinkOutcome::SkippedRegularFile { .. }));
        let bytes = std::fs::read(&link).unwrap();
        assert_eq!(bytes, b"user-installed binary");
        let meta = std::fs::symlink_metadata(&link).unwrap();
        assert!(!meta.file_type().is_symlink());
    }

    #[test]
    fn creates_bin_dir_if_missing() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join("nested").join("does-not-exist").join("bin");
        assert!(!bin_dir.exists());
        let link = bin_dir.join("clickhouse");
        let target = version_binary(tmp.path(), "25.12.5.44");

        let outcome = ensure_symlink_at(&link, &target, &bin_dir).unwrap();

        assert!(matches!(outcome, SymlinkOutcome::Created { .. }));
        assert!(bin_dir.is_dir());
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
    }

    #[test]
    fn remove_only_when_target_matches_version_dir() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let link = bin_dir.join("clickhouse");
        let version_dir = tmp.path().join("versions").join("25.12.5.44");
        std::fs::create_dir_all(&version_dir).unwrap();
        let target = version_dir.join("clickhouse");
        std::fs::write(&target, b"x").unwrap();
        symlink(&target, &link).unwrap();

        remove_symlink_at(&link, &version_dir).unwrap();

        assert!(std::fs::symlink_metadata(&link).is_err());
    }

    #[test]
    fn remove_preserves_link_pointing_elsewhere() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let link = bin_dir.join("clickhouse");
        let elsewhere = tmp.path().join("opt").join("custom").join("clickhouse");
        std::fs::create_dir_all(elsewhere.parent().unwrap()).unwrap();
        std::fs::write(&elsewhere, b"x").unwrap();
        symlink(&elsewhere, &link).unwrap();
        let version_dir = tmp.path().join("versions").join("25.12.5.44");

        remove_symlink_at(&link, &version_dir).unwrap();

        assert!(std::fs::symlink_metadata(&link).is_ok());
        assert_eq!(std::fs::read_link(&link).unwrap(), elsewhere);
    }

    #[test]
    fn remove_noop_when_link_missing() {
        let tmp = TempDir::new().unwrap();
        let link = tmp.path().join(".local").join("bin").join("clickhouse");
        let version_dir = tmp.path().join("versions").join("25.12.5.44");

        remove_symlink_at(&link, &version_dir).unwrap();
        assert!(std::fs::symlink_metadata(&link).is_err());
    }

    #[test]
    fn remove_noop_when_path_is_regular_file() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let link = bin_dir.join("clickhouse");
        std::fs::write(&link, b"user-installed").unwrap();
        let version_dir = tmp.path().join("versions").join("25.12.5.44");

        remove_symlink_at(&link, &version_dir).unwrap();

        let bytes = std::fs::read(&link).unwrap();
        assert_eq!(bytes, b"user-installed");
    }
}
