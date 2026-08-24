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

    #[error(
        "No ClickHouse client version selected for the direct connection. Pass `--version <installed-version>`, or set a default with `clickhousectl local use <version>` (see `clickhousectl local list`)."
    )]
    DirectClientVersionRequired,

    #[error(
        "ClickHouse client version {0} is not installed. Run `clickhousectl local install {0}`, or choose an exact version from `clickhousectl local list`."
    )]
    ClientVersionNotInstalled(String),

    #[error(
        "Default ClickHouse version {0} is not installed. Repair it with `clickhousectl local use <version>`, or pass `--version <installed-version>` for this direct connection."
    )]
    StaleClientDefault(String),

    #[error("Version {0} is already installed")]
    VersionAlreadyInstalled(String),

    #[error(
        "Version {version} is in use by running server(s): {servers}. Stop them first, or pass --force."
    )]
    VersionInUse { version: String, servers: String },

    #[error("Unsupported platform: {os}/{arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("Failed to create directory {}: {source}", path.display())]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Download failed: {0}")]
    Download(String),

    #[error("{0}")]
    VersionNetwork(#[from] crate::version_manager::network::NetworkFailure),

    #[error("{fallback}; initial probe failure: {probe}")]
    VersionFallback {
        probe: crate::version_manager::network::NetworkFailure,
        fallback: Box<Error>,
    },

    #[error("{failure} after {attempts} attempts")]
    VersionNetworkRetryExhausted {
        failure: crate::version_manager::network::NetworkFailure,
        attempts: usize,
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

    #[error("Failed to execute ClickHouse: {0}")]
    PortInUse(String),

    #[error("Failed to execute ClickHouse: {0}")]
    StartupExit(String),

    #[error("Failed to execute ClickHouse: {0}")]
    StartupTimeout(String),

    #[error("Docker error: {0}")]
    DockerStartupTimeout(String),

    /// A child process whose status must be returned unchanged. This is
    /// intentionally not printed as a clickhousectl error by `run_parsed`.
    #[error("child process exited with code {0}")]
    ChildExit(i32),

    #[error("Extraction failed: {0}")]
    Extract(String),

    #[error(
        "Failed to extract archive {} to {}: {source}",
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

    #[error(
        "Could not read metadata for server '{name}' at .clickhouse/servers/{name}.json: {source}. Check that the file is readable and retry."
    )]
    ServerMetadataRead {
        name: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "Permission denied accessing metadata for server '{name}' at .clickhouse/servers/{name}.json. Restore access to the file and its parent directory, then retry."
    )]
    ServerMetadataPermission {
        name: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "Metadata for server '{name}' at .clickhouse/servers/{name}.json is invalid: {source}. Repair or remove the metadata file, then retry."
    )]
    ServerMetadataParse {
        name: String,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "Could not update metadata for server '{name}' at .clickhouse/servers/{name}.json: {source}. Check write access to .clickhouse/servers and retry."
    )]
    ServerMetadataWrite {
        name: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "No server name was provided and multiple non-default ClickHouse servers exist. Pass a name or run `clickhousectl local server stop-all`; use `clickhousectl local server list` to see available servers."
    )]
    ServerNameRequiredForStop,

    #[error(
        "No removable 'default' ClickHouse server exists, and no custom ClickHouse servers are available. Run `clickhousectl local server list` to inspect local server state."
    )]
    DefaultServerNotFoundForRemove,

    #[error(
        "No removable 'default' ClickHouse server exists. Run `clickhousectl local server list`, then pass a custom server name explicitly with `clickhousectl local server remove <name>`."
    )]
    ServerNameRequiredForRemove,

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

    #[error("Postgres validation failed: {0}")]
    PostgresValidation(String),

    #[error("Postgres operation failed: {0}")]
    PostgresRuntime(String),

    #[error("{primary}\nPostgres startup rollback diagnostics: {cleanup}")]
    PostgresStartupRollback {
        #[source]
        primary: Box<Error>,
        cleanup: String,
    },

    #[error("Docker error: {0}")]
    DockerDownload(String),
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
    fn typed_local_boundaries_preserve_human_error_text() {
        for error in [
            Error::PortInUse("HTTP port 8123 is already in use".into()),
            Error::StartupExit("server exited".into()),
            Error::StartupTimeout("server timed out".into()),
        ] {
            assert!(
                error
                    .to_string()
                    .starts_with("Failed to execute ClickHouse: ")
            );
        }
        assert_eq!(
            Error::DockerStartupTimeout("postgres timed out".into()).to_string(),
            "Docker error: postgres timed out"
        );
        assert_eq!(
            Error::DockerDownload("pull failed".into()).to_string(),
            "Docker error: pull failed"
        );
    }
}
