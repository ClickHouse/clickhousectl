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
    DockerStartupExit(String),

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

    #[error("Could not {operation} at {}: {source}. {remediation}", path.display())]
    ServerLock {
        operation: &'static str,
        path: PathBuf,
        remediation: &'static str,
        #[source]
        source: std::io::Error,
    },

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
        "Could not verify process identity for server '{name}' (PID {pid}) while attempting to {operation}: {source}. Metadata was preserved. Check that process inspection tools are available and that you have permission to inspect the process, then retry."
    )]
    ServerProcessInspection {
        name: String,
        pid: u32,
        operation: &'static str,
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
        "No removable 'default' ClickHouse server exists. Run `clickhousectl local server list`, then retry with an explicit custom server name."
    )]
    ServerNameRequiredForRemove,

    #[error("Server '{0}' is already running")]
    ServerAlreadyRunning(String),

    #[error(
        "Server '{0}' is running and cannot be removed. Run `clickhousectl local server list`, then stop it by name before retrying."
    )]
    ServerRunningCannotRemove(String),

    #[error("{message}")]
    ProjectServerScope {
        message: String,
        project: String,
        #[source]
        source: Box<Error>,
    },

    #[error("{message}")]
    ManagedClientScope {
        message: String,
        project: String,
        #[source]
        source: Box<Error>,
    },

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

    #[error("Postgres validation failed: {0}")]
    PostgresPortInUse(String),

    #[error("Postgres operation failed: {0}")]
    PostgresRuntime(String),

    #[error("{primary}\nPostgres startup rollback incomplete: {cleanup}")]
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

    pub fn with_project_server_scope(self, project: String) -> Self {
        if !matches!(
            &self,
            Error::ServerNotFound(_)
                | Error::ServerNotRunning(_)
                | Error::ServerMetadataRead { .. }
                | Error::ServerMetadataPermission { .. }
                | Error::ServerMetadataParse { .. }
                | Error::ServerMetadataWrite { .. }
                | Error::ServerNameRequiredForStop
                | Error::DefaultServerNotFoundForRemove
                | Error::ServerNameRequiredForRemove
                | Error::ServerRunningCannotRemove(_)
        ) {
            return self;
        }

        self.into_project_server_scope(project)
    }

    pub fn with_project_server_list_scope(self, project: String) -> Self {
        if matches!(&self, Error::Io(_)) {
            return self.into_project_server_scope(project);
        }

        self.with_project_server_scope(project)
    }

    fn into_project_server_scope(self, project: String) -> Self {
        let message = Self::project_server_scope_message(&self.to_string(), &project);
        Error::ProjectServerScope {
            message,
            project,
            source: Box::new(self),
        }
    }

    pub(crate) fn project_server_scope_message(message: &str, project: &str) -> String {
        format!(
            "{}\n{}\n\
             Run `clickhousectl local server list --global` to find running servers. For stopped \
             servers, change to the intended project directory and run \
             `clickhousectl local server list`.",
            message,
            project_lookup_scope(project)
        )
    }

    pub fn with_managed_client_scope(self, project: String) -> Self {
        let recovery = match &self {
            Error::ServerNotFound(_) => {
                "Run `clickhousectl local server list`; if the server belongs to a parent \
                 project, return to that project root. Start it with `clickhousectl local server \
                 start [name]`, or use direct mode with `clickhousectl local client --host \
                 localhost`."
            }
            Error::ServerNotRunning(_) => {
                "Run `clickhousectl local server list`, then `clickhousectl local server start \
                 [name]`; or use direct mode with `clickhousectl local client --host localhost`."
            }
            Error::VersionNotFound(_) => {
                "Run `clickhousectl local server list` to inspect the selected server. Restore its \
                 recorded binary with `clickhousectl local install <version>`, or use direct mode \
                 with `clickhousectl local client --host localhost --version \
                 <installed-version>`."
            }
            Error::ServerLock { .. }
            | Error::ServerMetadataRead { .. }
            | Error::ServerMetadataPermission { .. }
            | Error::ServerMetadataParse { .. }
            | Error::ServerMetadataWrite { .. }
            | Error::ServerProcessInspection { .. } => {
                "Run `clickhousectl local server list` to inspect the selected server, or use \
                 direct mode with `clickhousectl local client --host localhost`."
            }
            _ => return self,
        };

        let message = format!(
            "Managed local client: {}\n{}\n{recovery}",
            self,
            project_lookup_scope(&project)
        );
        Error::ManagedClientScope {
            message,
            project,
            source: Box::new(self),
        }
    }
}

fn project_lookup_scope(project: &str) -> String {
    format!(
        "Project directory used for lookup: {project:?}\n\
         Only this exact directory's `.clickhouse` is searched; parent `.clickhouse` directories \
         are not searched."
    )
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
    fn project_server_scope_wraps_only_lookup_and_state_errors() {
        let project = "/tmp/project".to_string();
        let error =
            Error::ServerNotFound("default".into()).with_project_server_scope(project.clone());

        assert!(matches!(
            error,
            Error::ProjectServerScope {
                ref project,
                ref source,
                ..
            } if project == "/tmp/project" && matches!(source.as_ref(), Error::ServerNotFound(_))
        ));
        assert_eq!(
            Error::InvalidServerName("../private".into())
                .with_project_server_scope(project)
                .to_string(),
            "Invalid server name '../private': must not contain path separators or '..'"
        );
    }

    #[test]
    fn server_list_scopes_io_without_widening_other_project_commands() {
        let project = "/tmp/project".to_string();
        let unscoped = Error::Io(std::io::Error::other("read failed"))
            .with_project_server_scope(project.clone());
        assert!(matches!(unscoped, Error::Io(_)));

        let scoped =
            Error::Io(std::io::Error::other("read failed")).with_project_server_list_scope(project);
        assert!(matches!(
            scoped,
            Error::ProjectServerScope {
                ref project,
                ref source,
                ..
            } if project == "/tmp/project"
                && matches!(source.as_ref(), Error::Io(source) if source.to_string() == "read failed")
        ));
    }

    #[test]
    fn managed_client_scope_wraps_only_selected_managed_state_errors() {
        let project = "/tmp/project".to_string();
        let error =
            Error::VersionNotFound("25.12.9.61".into()).with_managed_client_scope(project.clone());

        assert!(matches!(
            error,
            Error::ManagedClientScope {
                ref project,
                ref source,
                ..
            } if project == "/tmp/project" && matches!(source.as_ref(), Error::VersionNotFound(_))
        ));

        for error in [
            Error::ServerLock {
                operation: "open server lifecycle lock file",
                path: "/tmp/project/.clickhouse/servers/.locks/default.lock".into(),
                remediation: "Check access and retry.",
                source: std::io::Error::other("lock failed"),
            },
            Error::ServerProcessInspection {
                name: "default".into(),
                pid: 12345,
                operation: "read the recorded process working directory",
                source: std::io::Error::other("inspection failed"),
            },
        ] {
            assert!(matches!(
                error.with_managed_client_scope(project.clone()),
                Error::ManagedClientScope { .. }
            ));
        }

        assert_eq!(
            Error::ClientVersionNotInstalled("25.12.9.61".into())
                .with_managed_client_scope(project)
                .to_string(),
            "ClickHouse client version 25.12.9.61 is not installed. Run `clickhousectl local install 25.12.9.61`, or choose an exact version from `clickhousectl local list`."
        );
    }

    #[test]
    fn typed_local_boundaries_preserve_human_error_text() {
        assert_eq!(
            Error::PortInUse("HTTP port 8123 is already in use".into()).to_string(),
            "Failed to execute ClickHouse: HTTP port 8123 is already in use"
        );
        assert_eq!(
            Error::PostgresPortInUse("explicit Postgres port 5432 is already in use".into())
                .to_string(),
            "Postgres validation failed: explicit Postgres port 5432 is already in use"
        );
        for error in [
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
            Error::DockerStartupExit("postgres exited".into()).to_string(),
            "Docker error: postgres exited"
        );
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
