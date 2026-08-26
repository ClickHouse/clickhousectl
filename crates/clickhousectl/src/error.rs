use std::path::PathBuf;
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkStage {
    BuildProbe,
    VersionFallback,
    VersionList,
    MasterCheck,
    DownloadHeaders,
    DownloadBody,
    Download,
}

impl fmt::Display for NetworkStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage = match self {
            Self::BuildProbe => "build probe",
            Self::VersionFallback => "version fallback",
            Self::VersionList => "version list",
            Self::MasterCheck => "master check",
            Self::DownloadHeaders => "download headers",
            Self::DownloadBody => "download body",
            Self::Download => "download",
        };
        f.write_str(stage)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCategory {
    Timeout,
    Connection,
    Transport,
    InvalidResponse,
    Forbidden,
    NotFound,
    RateLimited,
    ClientError,
    ServerError,
    UnexpectedStatus,
}

impl NetworkCategory {
    pub(crate) const fn priority(self) -> u8 {
        match self {
            Self::RateLimited => 7,
            Self::ServerError => 6,
            Self::Timeout => 5,
            Self::Connection => 4,
            Self::Transport => 3,
            Self::InvalidResponse => 2,
            Self::ClientError | Self::UnexpectedStatus => 1,
            Self::Forbidden | Self::NotFound => 0,
        }
    }
}

impl fmt::Display for NetworkCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::Timeout => "timeout",
            Self::Connection => "connection",
            Self::Transport => "transport",
            Self::InvalidResponse => "invalid-response",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not-found",
            Self::RateLimited => "rate-limited",
            Self::ClientError => "client-error",
            Self::ServerError => "server-error",
            Self::UnexpectedStatus => "unexpected-status",
        };
        f.write_str(category)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkFailure {
    pub stage: NetworkStage,
    pub host: String,
    pub category: NetworkCategory,
    pub attempts: Option<usize>,
}

impl NetworkFailure {
    pub(crate) fn new(stage: NetworkStage, url: &str, category: NetworkCategory) -> Self {
        let host = url::Url::from_str(url)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| "unknown-host".to_string());
        Self {
            stage,
            host,
            category,
            attempts: None,
        }
    }

    pub(crate) fn after_attempts(mut self, attempts: usize) -> Self {
        self.attempts = Some(attempts);
        self
    }
}

impl fmt::Display for NetworkFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} request to {} failed ({})",
            self.stage, self.host, self.category
        )?;
        if let Some(attempts) = self.attempts {
            write!(f, " after {attempts} attempts")?;
        }
        Ok(())
    }
}

impl std::error::Error for NetworkFailure {}

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

    #[error(
        "No ClickHouse client versions are installed. Run `clickhousectl local install <version>` before making a direct connection."
    )]
    NoClientVersionInstalled,

    #[error(
        "Multiple ClickHouse client versions are installed, but no default is set. Pass `--version <version>` (see `clickhousectl local list`) or run `clickhousectl local use <version>`."
    )]
    AmbiguousClientVersion,

    #[error(
        "Default ClickHouse version '{0}' is not installed. Repair it with `clickhousectl local use <version>`, or bypass it for this direct connection with `--version <installed-version>`."
    )]
    StaleDefaultVersion(String),

    #[error(
        "ClickHouse client version '{0}' is not installed. Run `clickhousectl local install {0}`, or choose an installed version with `clickhousectl local list`."
    )]
    ClientVersionNotInstalled(String),

    #[error(
        "ClickHouse client version '{version}' does not support repeated --query values. Use ClickHouse {minimum} or newer, or send one --query value."
    )]
    RepeatedClientQueryUnsupported {
        version: String,
        minimum: &'static str,
    },

    #[error("Version {0} is already installed")]
    VersionAlreadyInstalled(String),

    #[error(
        "Version {version} is in use by running server(s): {servers}. Stop them first, or pass --force."
    )]
    VersionInUse { version: String, servers: String },

    #[error("Unsupported platform: {os}/{arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("Failed to create directory '{}': {source}", path.display())]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Download failed: {0}")]
    Download(String),

    #[error("{0}")]
    Network(#[from] NetworkFailure),

    #[error("{probe}; fallback also failed: {fallback}")]
    VersionResolutionFallback {
        probe: NetworkFailure,
        fallback: Box<Error>,
    },

    #[error("No matching version found for: {0}")]
    NoMatchingVersion(String),

    #[error(
        "build {version} is no longer available for download.\nNearest available in the {series} series: {available} (try `clickhousectl local install {series}`)"
    )]
    ExactVersionUnavailable {
        version: String,
        series: String,
        available: String,
    },

    #[error("build {0} exists, but its release channel could not be determined")]
    UnknownVersionChannel(String),

    #[error("{0}")]
    InvalidVersion(String),

    #[error("Failed to execute ClickHouse: {0}")]
    Exec(String),

    #[error("Postgres error: {0}")]
    Postgres(String),

    /// A child process whose status must be returned unchanged. This is
    /// intentionally not printed as a clickhousectl error by `run_parsed`.
    #[error("child process exited with code {0}")]
    ChildExit(i32),

    #[error("Extraction failed: {0}")]
    Extract(String),

    #[error(
        "Failed to extract archive '{}' to '{}': {source}",
        archive.display(),
        destination.display()
    )]
    ExtractArchive {
        archive: PathBuf,
        destination: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Server '{0}' is not running")]
    ServerNotRunning(String),

    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Server '{0}' is already running")]
    ServerAlreadyRunning(String),

    #[error("Server '{0}' is running; stop it first with `clickhousectl local server stop {0}`")]
    ServerRunningCannotRemove(String),

    #[error("{0}")]
    Cloud(String),

    #[error("{0}")]
    AuthRequired(String),

    #[error("Cancelled")]
    Cancelled,

    #[error("{0}")]
    Skills(String),

    #[error("Invalid server name '{0}': must not contain path separators or '..'")]
    InvalidServerName(String),

    #[error("{0}")]
    ConfigNotFound(String),

    #[error(
        "Invalid config name '{0}': must be a file in the configs dir, not a path (no '/', '\\', or '..')"
    )]
    InvalidConfigName(String),

    #[error("Docker is not available: {0}")]
    DockerNotAvailable(String),

    #[error("Docker error: {0}")]
    #[allow(clippy::enum_variant_names)]
    DockerError(String),

    #[error("{primary}\nPostgres startup rollback incomplete: {cleanup}")]
    PostgresStartupRollback {
        #[source]
        primary: Box<Error>,
        cleanup: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Process exit code: `0` success, `1` error, `3` cancelled,
    /// `4` auth required. Clap reserves `2` for usage errors.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::AuthRequired(_) => 4,
            Error::Cancelled => 3,
            Error::ChildExit(code) => *code,
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
    fn cancelled_maps_to_3() {
        assert_eq!(Error::Cancelled.exit_code(), 3);
    }

    #[test]
    fn generic_errors_map_to_1() {
        assert_eq!(Error::Cloud("boom".into()).exit_code(), 1);
        assert_eq!(Error::NoVersionsInstalled.exit_code(), 1);
        assert_eq!(Error::VersionNotFound("25.12".into()).exit_code(), 1);
        assert_eq!(
            Error::VersionInUse {
                version: "25.12".into(),
                servers: "default".into(),
            }
            .exit_code(),
            1
        );
    }

    #[test]
    fn exact_version_unavailable_error_has_an_actionable_retry() {
        let error = Error::ExactVersionUnavailable {
            version: "26.2.8.7".into(),
            series: "26.2".into(),
            available: "26.2.20.4".into(),
        };

        assert_eq!(
            error.to_string(),
            "build 26.2.8.7 is no longer available for download.\n\
             Nearest available in the 26.2 series: 26.2.20.4 \
             (try `clickhousectl local install 26.2`)"
        );
        assert_eq!(error.exit_code(), 1);
    }

    #[test]
    fn child_exit_codes_pass_through_without_changing_normal_mappings() {
        assert_eq!(Error::ChildExit(42).exit_code(), 42);
        assert_eq!(Error::ChildExit(255).exit_code(), 255);
        assert_eq!(Error::Cloud("boom".into()).exit_code(), 1);
        assert_eq!(Error::Cancelled.exit_code(), 3);
        assert_eq!(Error::AuthRequired("nope".into()).exit_code(), 4);
    }

    #[test]
    fn create_dir_error_includes_path_and_permission_cause() {
        let error = Error::CreateDir {
            path: PathBuf::from("/read-only/clickhouse/versions"),
            source: std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied by test",
            ),
        };

        assert_eq!(
            error.to_string(),
            "Failed to create directory '/read-only/clickhouse/versions': permission denied by test"
        );
    }
}
