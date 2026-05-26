use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Version {0} not found")]
    VersionNotFound(String),

    #[error("No versions installed")]
    NoVersionsInstalled,

    #[error("No default version set. Run: clickhousectl local use <version>")]
    NoDefaultVersion,

    #[error("Version {0} is already installed")]
    VersionAlreadyInstalled(String),

    #[error("Unsupported platform: {os}/{arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("Failed to create directory: {0}")]
    CreateDir(PathBuf),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("No matching version found for: {0}")]
    NoMatchingVersion(String),

    #[error("Failed to execute ClickHouse: {0}")]
    Exec(String),

    #[error("Extraction failed: {0}")]
    Extract(String),

    #[error("Server '{0}' is not running")]
    ServerNotRunning(String),

    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Server '{0}' is already running")]
    ServerAlreadyRunning(String),

    #[error("{0}")]
    Cloud(String),

    /// Authentication required or invalid (missing creds, 401/403, etc).
    /// Mapped to exit code 4 (`gh` convention).
    #[error("{0}")]
    AuthRequired(String),

    /// User cancelled the operation (Ctrl-C, declined a prompt, etc).
    /// Mapped to exit code 2 (`gh` convention).
    #[error("Cancelled")]
    Cancelled,

    #[error("{0}")]
    Skills(String),

    #[error("Invalid server name '{0}': must not contain path separators or '..'")]
    InvalidServerName(String),

    #[error("--json and --foreground cannot be used together")]
    JsonForegroundConflict,

    #[error("Docker is not available: {0}")]
    DockerNotAvailable(String),

    #[error("Docker error: {0}")]
    #[allow(clippy::enum_variant_names)]
    DockerError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Process exit code following `gh` CLI conventions:
    /// `0` success, `1` error, `2` cancelled, `4` auth required.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::AuthRequired(_) => 4,
            Error::Cancelled => 2,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_required_maps_to_4() {
        assert_eq!(Error::AuthRequired("nope".into()).exit_code(), 4);
    }

    #[test]
    fn cancelled_maps_to_2() {
        assert_eq!(Error::Cancelled.exit_code(), 2);
    }

    #[test]
    fn generic_errors_map_to_1() {
        assert_eq!(Error::Cloud("boom".into()).exit_code(), 1);
        assert_eq!(Error::NoVersionsInstalled.exit_code(), 1);
        assert_eq!(Error::VersionNotFound("25.12".into()).exit_code(), 1);
    }
}
