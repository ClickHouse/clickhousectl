use crate::error::{Error, Result};
use std::path::PathBuf;

/// Returns the base directory for clickhousectl (~/.clickhousectl/)
pub fn base_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;
    Ok(home.join(".clickhousectl"))
}

/// Returns the versions directory (~/.clickhousectl/versions/)
pub fn versions_dir() -> Result<PathBuf> {
    Ok(base_dir()?.join("versions"))
}

/// Returns the directory for a specific version (~/.clickhousectl/versions/<version>/)
pub fn version_dir(version: &str) -> Result<PathBuf> {
    Ok(versions_dir()?.join(version))
}

/// Returns the path to the ClickHouse binary for a specific version
pub fn binary_path(version: &str) -> Result<PathBuf> {
    Ok(version_dir(version)?.join("clickhouse"))
}

/// Returns the path to the default version file (~/.clickhousectl/default)
pub fn default_file() -> Result<PathBuf> {
    Ok(base_dir()?.join("default"))
}

/// Ensures all necessary directories exist
pub fn ensure_dirs() -> Result<()> {
    let versions = versions_dir()?;
    std::fs::create_dir_all(&versions).map_err(|_| Error::CreateDir(versions))?;
    Ok(())
}
