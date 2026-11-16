use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Returns the base directory for ClickHouse CLI (~/.clickhouse/)
pub fn base_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;
    Ok(home.join(".clickhouse"))
}

/// Returns the versions directory (~/.clickhouse/versions/)
pub fn versions_dir() -> Result<PathBuf> {
    Ok(base_dir()?.join("versions"))
}

/// Rejects a version that is not a single installed-version directory name, so
/// joining it onto `versions_dir()` can never escape that directory (`..`, an
/// absolute path, a separator) or name a hidden staging directory (`.`-prefixed).
pub fn validate_installed_version(version: &str) -> Result<()> {
    let reason = if version.is_empty() {
        Some("is empty")
    } else if version.starts_with('.') {
        Some("starts with '.'")
    } else if version.contains(['/', '\\', '\0']) || Path::new(version).is_absolute() {
        Some("contains a path separator")
    } else {
        None
    };
    match reason {
        Some(reason) => Err(Error::InvalidVersion(format!(
            "'{version}' {reason}; pass an exact version from `clickhousectl local list`"
        ))),
        None => Ok(()),
    }
}

/// Returns the directory for a specific version (~/.clickhouse/versions/<version>/)
pub fn version_dir(version: &str) -> Result<PathBuf> {
    Ok(versions_dir()?.join(version))
}

/// Returns the path to the ClickHouse binary for a specific version
pub fn binary_path(version: &str) -> Result<PathBuf> {
    Ok(version_dir(version)?.join("clickhouse"))
}

/// Returns the path to the default version file (~/.clickhouse/default)
pub fn default_file() -> Result<PathBuf> {
    Ok(base_dir()?.join("default"))
}

/// Returns the custom server configs directory (~/.clickhouse/configs/)
pub fn configs_dir() -> Result<PathBuf> {
    Ok(base_dir()?.join("configs"))
}

/// Returns the global always-applied server config directory
/// (~/.clickhouse/config.d/)
pub fn global_config_d_dir() -> Result<PathBuf> {
    Ok(base_dir()?.join(GLOBAL_CONFIG_D))
}

/// The directories every `~/.clickhouse/` holds, relative to it.
const BASE_LAYOUT: [&str; 3] = ["versions", "configs", GLOBAL_CONFIG_D];

const GLOBAL_CONFIG_D: &str = "config.d";

/// Creates the base layout of `~/.clickhouse/`: `versions/`, `configs/` and
/// `config.d/`. The one place that owns that layout, so every caller that
/// creates the directory leaves it complete.
pub fn ensure_dirs() -> Result<()> {
    ensure_base_layout_in(&base_dir()?)
}

fn ensure_base_layout_in(base: &Path) -> Result<()> {
    for dir in BASE_LAYOUT {
        ensure_dir(&base.join(dir))?;
    }
    Ok(())
}

pub(crate) fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|source| Error::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

/// Replace `path` with `bytes` so readers see either the old file or the
/// complete new one, never a truncated file.
///
/// The bytes go to a temporary file in the same directory that already has
/// `mode` (Unix only) before any byte is written, and is synced before it is
/// renamed over `path`. Pass `0o600` for secrets. The parent directory must
/// exist.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> std::io::Result<()> {
    atomic_write_with(path, bytes, mode, sync_directory)
}

fn atomic_write_with(
    path: &Path,
    bytes: &[u8],
    #[cfg_attr(not(unix), allow(unused_variables))] mode: u32,
    sync_parent: impl FnOnce(&Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::io::Write;

    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
        )
    })?;
    let prefix = format!(
        ".{}.",
        path.file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default()
    );
    let mut temporary = tempfile::Builder::new().prefix(&prefix).tempfile_in(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(mode))?;
    }

    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| {
        std::io::Error::new(
            error.error.kind(),
            format!("failed to replace {}: {}", path.display(), error.error),
        )
    })?;

    // The replacement is already authoritative. Reporting a later directory
    // fsync failure as an unsuccessful save can make callers undo work whose
    // result is present in the file (for example, delete a live API key).
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

/// Returns the user-local PATH-style bin directory (~/.local/bin/)
pub fn global_bin_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;
    Ok(home.join(".local").join("bin"))
}

/// Returns the path to the global `clickhouse` symlink (~/.local/bin/clickhouse)
pub fn global_clickhouse_symlink() -> Result<PathBuf> {
    Ok(global_bin_dir()?.join("clickhouse"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_installed_version_accepts_only_a_single_visible_component() {
        for version in ["25.12.9.61", "25.8.1.1-lts", "head"] {
            validate_installed_version(version).unwrap();
        }
        for version in [
            "",
            ".",
            "..",
            ".staging-25.12",
            "../x",
            "a/b",
            "a\\b",
            "/abs",
            "a\0b",
        ] {
            let error = validate_installed_version(version).unwrap_err();
            assert!(matches!(error, Error::InvalidVersion(_)), "{version:?}");
        }
    }

    #[test]
    fn base_layout_creates_every_global_directory_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join(".clickhouse");
        ensure_base_layout_in(&base).unwrap();
        ensure_base_layout_in(&base).unwrap();
        for dir in BASE_LAYOUT {
            assert!(base.join(dir).is_dir(), "{dir}");
        }
        // The accessors name directories in that layout.
        let home_base = base_dir().unwrap();
        for accessor in [versions_dir(), configs_dir(), global_config_d_dir()] {
            let dir = accessor.unwrap();
            let relative = dir.strip_prefix(&home_base).unwrap().to_str().unwrap();
            assert!(BASE_LAYOUT.contains(&relative), "{relative}");
        }
    }

    #[test]
    fn ensure_dir_includes_path_and_not_a_directory_cause() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("not-a-directory");
        std::fs::write(&file, "content").unwrap();
        let directory = file.join("versions");

        let error = ensure_dir(&directory).unwrap_err();
        let Error::CreateDir { path, source } = &error else {
            panic!("expected create-directory error: {error}");
        };

        assert_eq!(path, &directory);
        assert_eq!(source.kind(), std::io::ErrorKind::NotADirectory);
        // The message carries the path and the io error's own text.
        let message = error.to_string();
        assert!(
            message.contains(&format!("'{}'", directory.display())),
            "{message}"
        );
        assert!(message.ends_with(&source.to_string()), "{message}");
    }

    #[test]
    fn atomic_write_succeeds_when_directory_sync_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.json");
        atomic_write_with(&path, b"new", 0o600, |_| {
            Err(std::io::Error::other("directory sync failed"))
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }
}
