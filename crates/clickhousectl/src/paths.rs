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

/// Ensures all necessary directories exist
pub fn ensure_dirs() -> Result<()> {
    let versions = versions_dir()?;
    create_dir_all(&versions)
}

pub(crate) fn create_dir_all(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|source| Error::CreateDir {
        path: path.to_path_buf(),
        source,
    })
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
    fn create_directory_error_preserves_not_a_directory_path_and_cause() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("not-a-directory");
        std::fs::write(&file, b"file").unwrap();
        let requested = file.join("versions");

        let error = create_dir_all(&requested).unwrap_err();
        let Error::CreateDir { path, source } = &error else {
            panic!("expected create directory error: {error}");
        };

        assert_eq!(path, &requested);
        assert_eq!(source.kind(), std::io::ErrorKind::NotADirectory);
        assert!(error.to_string().contains(&requested.display().to_string()));
        assert!(error.to_string().contains(&source.to_string()));
    }

    #[test]
    fn create_directory_error_preserves_permission_denied_cause() {
        let path = PathBuf::from("/restricted/.clickhouse/versions");
        let error = Error::CreateDir {
            path: path.clone(),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        };
        let Error::CreateDir {
            source: permission_error,
            ..
        } = &error
        else {
            unreachable!();
        };

        assert_eq!(
            permission_error.kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(error.to_string().contains(&path.display().to_string()));
        assert!(error.to_string().contains(&permission_error.to_string()));
    }
}
