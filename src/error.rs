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

    #[error("{0}")]
    Skills(String),

    #[error("--json and --foreground cannot be used together")]
    JsonForegroundConflict,
}

pub type Result<T> = std::result::Result<T, Error>;
