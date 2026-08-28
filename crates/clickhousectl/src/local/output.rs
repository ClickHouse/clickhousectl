//! Structured output types for local commands.
//!
//! Successful output types support both JSON serialization and human-readable
//! display. Runtime failures use the redacted stable envelope below.

use crate::error::{
    Error, ManagedClientError, ManagedClientErrorKind, ManagedClientSelection, NetworkStage,
    PortKind, ProjectServerCommand, ProjectServerNotFound, ProjectServerStateMissing,
};
use serde::Serialize;
use std::fmt;
use std::io::Write;
use std::path::Path;
use tabled::{Table, Tabled, settings::Style};

/// Stable codes for local runtime failures. New codes may be added, but
/// existing spellings and meanings are part of the machine-output contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalErrorCode {
    ManagedClientServerNotFound,
    ManagedClientServerNotRunning,
    ManagedClientBinaryNotFound,
    ManagedClientProjectStateUnavailable,
    ServerNotFound,
    ServerSelectionRequired,
    ServerNotRunning,
    ServerRunning,
    InvalidServerName,
    UnsupportedArgument,
    ConfigNotFound,
    InvalidConfigName,
    InvalidVersion,
    /// The version is not installed locally. Distinct from
    /// [`Self::VersionUnavailable`], which means it could not be resolved or
    /// downloaded: `local list --remote` is no help for a local miss.
    VersionNotInstalled,
    /// The build is installed but cannot be launched: not a regular file, or
    /// carrying no execute bit. Distinct from [`Self::VersionNotInstalled`],
    /// which `local install` fixes by fetching a missing build.
    BinaryNotLaunchable,
    VersionSelectionRequired,
    VersionAlreadyInstalled,
    VersionUnavailable,
    VersionIsDefault,
    UnsupportedClientVersion,
    UnsupportedPlatform,
    PortInUse,
    StartupExit,
    StartupTimeout,
    DownloadFailed,
    NetworkError,
    DockerUnavailable,
    DockerError,
    /// The container name is held by a container clickhousectl does not
    /// manage. Distinct from [`Self::DockerError`], which covers daemon
    /// failures whose text is not rendered.
    ContainerNameConflict,
    /// A Postgres validation or state error whose text (and recovery
    /// guidance) clickhousectl composes itself, rendered verbatim.
    PostgresError,
    IoError,
    LocalError,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalErrorDetail {
    code: LocalErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
}

/// How one [`Error`] variant renders into a [`LocalErrorDetail`].
///
/// Built with either [`Mapping::parity`] — the JSON message is the error's own
/// human text, verbatim — or [`Mapping::redacted`], which substitutes a curated
/// summary for errors that interpolate foreign text.
struct Mapping {
    code: LocalErrorCode,
    command: Option<String>,
    /// Curated replacement for the human text; `None` renders `Display`.
    redacted: Option<String>,
}

impl Mapping {
    /// The JSON message is the error's `Display` text, so machine output
    /// carries exactly the detail and remediation human output prints.
    fn parity(code: LocalErrorCode) -> Self {
        Self {
            code,
            command: None,
            redacted: None,
        }
    }

    /// The JSON message is `message`, not the error's `Display` text. For
    /// errors whose text interpolates foreign output.
    fn redacted(code: LocalErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            command: None,
            redacted: Some(message.into()),
        }
    }

    /// A safe, runnable recovery command for this failure.
    fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    fn into_detail(self, error: &Error) -> LocalErrorDetail {
        LocalErrorDetail {
            code: self.code,
            message: self.redacted.unwrap_or_else(|| error.to_string()),
            command: self.command,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalProjectScope {
    path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocalProjectScopeKind {
    ExactCurrentProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ServerProjectScope {
    kind: LocalProjectScopeKind,
    path: String,
    parent_projects_searched: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalServerSelection {
    Default,
    Named,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalManagedServer {
    selection: LocalServerSelection,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_version: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalGuidance {
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalGuidanceAction {
    ListProjectServers,
    ListGlobalServers,
    ReturnToProjectRoot,
    StopGlobalProjectServer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ProjectServerGuidance {
    action: LocalGuidanceAction,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'static str>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ManagedClientErrorDetail {
    code: LocalErrorCode,
    message: &'static str,
    project_scope: LocalProjectScope,
    server: LocalManagedServer,
    guidance: Vec<LocalGuidance>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProjectServerErrorDetail {
    code: LocalErrorCode,
    message: String,
    project_scope: ServerProjectScope,
    server: LocalProjectServer,
    guidance: Vec<ProjectServerGuidance>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProjectServerStateMissingDetail {
    code: LocalErrorCode,
    message: &'static str,
    project_scope: ServerProjectScope,
    guidance: Vec<ProjectServerGuidance>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalProjectServer {
    name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
enum LocalErrorBody {
    General(LocalErrorDetail),
    ManagedClient(ManagedClientErrorDetail),
    ProjectServer(ProjectServerErrorDetail),
    ProjectServerStateMissing(ProjectServerStateMissingDetail),
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalErrorOutput {
    error: LocalErrorBody,
}

impl LocalErrorOutput {
    /// Classify one local runtime failure.
    ///
    /// Two rules govern the `message` field, and every arm below picks one
    /// deliberately:
    ///
    /// * **Parity** ([`Mapping::parity`]) — the error's own human text is
    ///   rendered verbatim, so a JSON consumer gets exactly the detail and
    ///   remediation human mode prints. This is the default: these messages are
    ///   composed by this crate from its own fields.
    /// * **Redaction** ([`Mapping::redacted`]) — a curated summary replaces
    ///   text that interpolates foreign output (subprocess stderr, Docker
    ///   daemon or OS/serde source strings, download bodies), which can carry
    ///   paths, SQL or credentials and tells a machine consumer nothing.
    ///
    /// The match is exhaustive on purpose: a new [`Error`] variant must be
    /// classified here rather than silently collapsing to
    /// `local_error`/"Local command failed".
    fn from_error(error: &Error) -> Self {
        let mapping = match error {
            // ── structured bodies (their own DTOs, not code/message/command) ─
            Error::ManagedClient(managed) => {
                return Self {
                    error: LocalErrorBody::ManagedClient(ManagedClientErrorDetail::from_error(
                        managed,
                    )),
                };
            }
            Error::ProjectServerNotFound(not_found) => {
                return Self {
                    error: LocalErrorBody::ProjectServer(ProjectServerErrorDetail::from_error(
                        not_found,
                    )),
                };
            }
            Error::ProjectServerStateMissing(missing) => {
                return Self {
                    error: LocalErrorBody::ProjectServerStateMissing(
                        ProjectServerStateMissingDetail::from_error(missing),
                    ),
                };
            }
            // The rollback note is a cleanup detail on top of the failure that
            // actually stopped the command; classify by that primary failure.
            Error::PostgresStartupRollback { primary, .. } => {
                return Self::from_error(primary);
            }

            // ── servers ─────────────────────────────────────────────────────
            Error::ServerNotFound(_) => Mapping::parity(LocalErrorCode::ServerNotFound)
                .command("clickhousectl local server list"),
            Error::ServerStopSelectionRequired { .. }
            | Error::ServerRemoveSelectionRequired { .. } => {
                Mapping::parity(LocalErrorCode::ServerSelectionRequired)
                    .command("clickhousectl local server list")
            }
            // The same ambiguity across projects: the global list shows which
            // project root to pass to `--project`.
            Error::ServerInMultipleProjects { .. } => {
                Mapping::parity(LocalErrorCode::ServerSelectionRequired)
                    .command("clickhousectl local server list --global")
            }
            // Deliberately the list, not a `start` command: this variant is
            // also raised for Postgres servers and for global PID lookups,
            // where the name is not a `server start` argument.
            Error::ServerNotRunning(_) => Mapping::parity(LocalErrorCode::ServerNotRunning)
                .command("clickhousectl local server list"),
            Error::ServerAlreadyRunning(_) => Mapping::parity(LocalErrorCode::ServerRunning)
                .command("clickhousectl local server list"),
            // Stopping *this* server is the recovery; `server list` only
            // restates what the error already says.
            Error::ServerRunningCannotRemove(name) => {
                Mapping::parity(LocalErrorCode::ServerRunning)
                    .command(format!("clickhousectl local server stop {name}"))
            }
            Error::InvalidServerName(_) => Mapping::parity(LocalErrorCode::InvalidServerName)
                .command("clickhousectl local server list"),
            Error::UnsupportedArgument(_) => Mapping::parity(LocalErrorCode::UnsupportedArgument)
                .command("clickhousectl local server start --help"),

            // ── server configs ──────────────────────────────────────────────
            Error::ConfigNotFound(_) => Mapping::parity(LocalErrorCode::ConfigNotFound)
                .command("clickhousectl local server configs"),
            Error::InvalidConfigName(_) => Mapping::parity(LocalErrorCode::InvalidConfigName)
                .command("clickhousectl local server configs"),

            // ── versions ────────────────────────────────────────────────────
            Error::InvalidVersion(_) => Mapping::parity(LocalErrorCode::InvalidVersion)
                .command("clickhousectl local install --help"),
            // Not installed locally: the installed list, not the remote one, is
            // what resolves these.
            Error::VersionNotFound(_) | Error::StaleDefaultVersion(_) => {
                Mapping::parity(LocalErrorCode::VersionNotInstalled)
                    .command("clickhousectl local list")
            }
            Error::NoVersionsInstalled | Error::NoClientVersionInstalled => {
                Mapping::parity(LocalErrorCode::VersionNotInstalled)
                    .command("clickhousectl local install latest")
            }
            Error::ClientVersionNotInstalled(version) => {
                Mapping::parity(LocalErrorCode::VersionNotInstalled)
                    .command(format!("clickhousectl local install {version}"))
            }
            // Installed but unusable: the message is entirely self-composed
            // (path plus a closed-vocabulary problem), so it renders at parity
            // and names the repair (#471). `--force` because install counts
            // any existing path at the binary location as already installed.
            Error::BinaryNotLaunchable { version, .. } => {
                Mapping::parity(LocalErrorCode::BinaryNotLaunchable)
                    .command(format!("clickhousectl local install --force {version}"))
            }
            Error::NoDefaultVersion | Error::AmbiguousClientVersion => {
                Mapping::parity(LocalErrorCode::VersionSelectionRequired)
                    .command("clickhousectl local list")
            }
            Error::VersionAlreadyInstalled(_) => {
                Mapping::parity(LocalErrorCode::VersionAlreadyInstalled)
                    .command("clickhousectl local list")
            }
            Error::RepeatedClientQueryUnsupported { .. } => {
                Mapping::parity(LocalErrorCode::UnsupportedClientVersion)
            }
            // The blocking servers are named — including their project root —
            // because they may live outside the current project, where neither
            // `server list` nor the caller's own state can find them.
            Error::VersionInUse { .. } => Mapping::parity(LocalErrorCode::ServerRunning)
                .command("clickhousectl local server list --global"),
            Error::VersionIsDefault { .. } => Mapping::parity(LocalErrorCode::VersionIsDefault)
                .command("clickhousectl local use latest"),
            // Could not be resolved or downloaded, so the remote list is the
            // next step.
            Error::NoMatchingVersion(_)
            | Error::ExactVersionUnavailable { .. }
            | Error::UnknownVersionChannel(_)
            | Error::VersionResolutionFallback { .. } => {
                Mapping::parity(LocalErrorCode::VersionUnavailable)
                    .command("clickhousectl local list --remote")
            }
            Error::UnsupportedPlatform { .. } => {
                Mapping::parity(LocalErrorCode::UnsupportedPlatform)
            }

            // ── ports and startup ───────────────────────────────────────────
            Error::PortInUse { kind, .. } | Error::PortUnavailable(kind) => {
                Mapping::parity(LocalErrorCode::PortInUse).command(match kind {
                    PortKind::Postgres => "clickhousectl local postgres start --help",
                    PortKind::Http | PortKind::Tcp => "clickhousectl local server start --help",
                })
            }
            // `details` is the managed server's own stderr or log tail: kept in
            // human output, summarized here.
            Error::StartupExit { kind, name, .. } => Mapping::redacted(
                LocalErrorCode::StartupExit,
                format!("{kind} server '{name}' exited before becoming ready"),
            )
            .command("clickhousectl local server list"),
            Error::StartupTimeout {
                kind,
                name,
                seconds,
                ..
            } => Mapping::redacted(
                LocalErrorCode::StartupTimeout,
                format!("{kind} server '{name}' did not become ready within {seconds} seconds"),
            )
            .command("clickhousectl local server list"),

            // ── network, downloads and extraction ───────────────────────────
            Error::Network(failure)
                if matches!(
                    failure.stage,
                    NetworkStage::DownloadHeaders
                        | NetworkStage::DownloadBody
                        | NetworkStage::Download
                ) =>
            {
                Mapping::parity(LocalErrorCode::DownloadFailed)
            }
            // Version resolution probes: the remote list is the next step.
            Error::Network(_) => Mapping::parity(LocalErrorCode::VersionUnavailable)
                .command("clickhousectl local list --remote"),
            Error::Http(_) => {
                Mapping::redacted(LocalErrorCode::NetworkError, "HTTP request failed")
            }
            Error::Download(_) => {
                Mapping::redacted(LocalErrorCode::DownloadFailed, "Download failed")
            }
            Error::Extract(_) | Error::ExtractArchive { .. } => {
                Mapping::redacted(LocalErrorCode::DownloadFailed, "Extraction failed")
            }

            // ── Docker ──────────────────────────────────────────────────────
            // The unavailability text is built from a classified failure kind
            // and platform guidance; the daemon's own message is used for
            // classification only and never rendered (see `local::docker`).
            Error::DockerNotAvailable(_) => Mapping::parity(LocalErrorCode::DockerUnavailable),
            Error::DockerError(_) => {
                Mapping::redacted(LocalErrorCode::DockerError, "Docker operation failed")
            }
            // Self-composed name-conflict guidance, unlike the daemon text
            // above.
            Error::ContainerNameConflict(_) => {
                Mapping::parity(LocalErrorCode::ContainerNameConflict)
            }

            // ── filesystem and metadata ─────────────────────────────────────
            Error::Io(_)
            | Error::Json(_)
            | Error::CreateDir { .. }
            | Error::ServerMetadataPermission { .. }
            | Error::ServerMetadataRead { .. }
            | Error::ServerMetadataUtf8 { .. }
            | Error::ServerMetadataParse { .. }
            | Error::ServerMetadataWrite { .. }
            | Error::ServerLock { .. } => {
                Mapping::redacted(LocalErrorCode::IoError, "Local I/O operation failed")
            }

            // ── postgres ────────────────────────────────────────────────────
            // Self-composed validation and state guidance; the foreign-text
            // sibling `Error::Postgres` stays in the fallback below.
            Error::PostgresUsage(_) => Mapping::parity(LocalErrorCode::PostgresError),

            // ── bounded fallback ────────────────────────────────────────────
            // Subprocess text and `Postgres` (OS text from a failed psql
            // exec) are foreign output. `Cloud`, `AuthRequired` and `Skills`
            // belong to other command surfaces and are never printed through
            // this envelope; `ChildExit` passes the child's status through
            // without an error object at all.
            Error::Exec(_)
            | Error::Postgres(_)
            | Error::Cloud(_)
            | Error::AuthRequired(_)
            | Error::Skills(_)
            | Error::ChildExit(_) => {
                Mapping::redacted(LocalErrorCode::LocalError, "Local command failed")
            }
            Error::Cancelled => Mapping::parity(LocalErrorCode::LocalError),
        };
        Self {
            error: LocalErrorBody::General(mapping.into_detail(error)),
        }
    }
}

impl ManagedClientErrorDetail {
    fn from_error(error: &ManagedClientError) -> Self {
        let (code, message) = match &error.kind {
            ManagedClientErrorKind::ServerNotFound => (
                LocalErrorCode::ManagedClientServerNotFound,
                "Managed client server was not found in the current project",
            ),
            ManagedClientErrorKind::ServerNotRunning => (
                LocalErrorCode::ManagedClientServerNotRunning,
                "Managed client server is not running in the current project",
            ),
            ManagedClientErrorKind::BinaryNotFound => (
                LocalErrorCode::ManagedClientBinaryNotFound,
                "Managed client binary selected by server metadata is not installed",
            ),
            ManagedClientErrorKind::ProjectStateUnavailable(_) => (
                LocalErrorCode::ManagedClientProjectStateUnavailable,
                "Managed client project state is unavailable",
            ),
        };
        let selection = match error.selection {
            ManagedClientSelection::Default => LocalServerSelection::Default,
            ManagedClientSelection::Named => LocalServerSelection::Named,
        };
        let mut guidance = vec![LocalGuidance {
            message: "List managed servers in this exact project",
            command: Some("clickhousectl local server list"),
        }];
        match &error.kind {
            ManagedClientErrorKind::ServerNotFound => {
                guidance.push(LocalGuidance {
                    message: "Return to the project directory that owns the managed server",
                    command: None,
                });
                guidance.push(start_guidance(error.selection));
            }
            ManagedClientErrorKind::ServerNotRunning => {
                guidance.push(start_guidance(error.selection));
            }
            ManagedClientErrorKind::BinaryNotFound => {
                guidance.push(LocalGuidance {
                    message: "Install the version selected by the managed server metadata",
                    command: Some("clickhousectl local install <version>"),
                });
            }
            ManagedClientErrorKind::ProjectStateUnavailable(_) => {
                guidance.insert(
                    0,
                    LocalGuidance {
                        message: "Repair the reported project state error before retrying",
                        command: None,
                    },
                );
            }
        }
        guidance.push(LocalGuidance {
            message: "Bypass managed project lookup and connect directly",
            command: Some("clickhousectl local client --host <host> --port <port>"),
        });

        Self {
            code,
            message,
            project_scope: LocalProjectScope {
                path: error.project_dir.display().to_string(),
            },
            server: LocalManagedServer {
                selection,
                name: error.server_name.clone(),
                binary_version: error.binary_version.clone(),
            },
            guidance,
        }
    }
}

impl ProjectServerErrorDetail {
    fn from_error(error: &ProjectServerNotFound) -> Self {
        Self {
            code: LocalErrorCode::ServerNotFound,
            message: format!(
                "Server '{}' was not found in the current project",
                error.server_name
            ),
            project_scope: exact_current_project_scope(&error.project_dir),
            server: LocalProjectServer {
                name: error.server_name.clone(),
            },
            guidance: project_scope_guidance(Some(error.command)),
        }
    }
}

impl ProjectServerStateMissingDetail {
    fn from_error(error: &ProjectServerStateMissing) -> Self {
        Self {
            code: LocalErrorCode::ServerSelectionRequired,
            message: "No project-local server state was found in the current directory; no server was removed",
            project_scope: exact_current_project_scope(&error.project_dir),
            guidance: project_scope_guidance(Some(error.command)),
        }
    }
}

pub(crate) fn exact_current_project_scope(project_dir: &Path) -> ServerProjectScope {
    ServerProjectScope {
        kind: LocalProjectScopeKind::ExactCurrentProject,
        path: project_dir.display().to_string(),
        parent_projects_searched: false,
    }
}

pub(crate) fn project_scope_guidance(
    command: Option<ProjectServerCommand>,
) -> Vec<ProjectServerGuidance> {
    let mut guidance = vec![
        ProjectServerGuidance {
            action: LocalGuidanceAction::ReturnToProjectRoot,
            message: "Change to the local project root where the server was started",
            command: Some("cd <project-root>"),
        },
        ProjectServerGuidance {
            action: LocalGuidanceAction::ListProjectServers,
            message: "List servers after returning to that exact project",
            command: Some("clickhousectl local server list"),
        },
        ProjectServerGuidance {
            action: LocalGuidanceAction::ListGlobalServers,
            message: "Locate running ClickHouse servers across projects",
            command: Some("clickhousectl local server list --global"),
        },
    ];
    if command == Some(ProjectServerCommand::Stop) {
        guidance.push(ProjectServerGuidance {
            action: LocalGuidanceAction::StopGlobalProjectServer,
            message: "After confirming the project, stop the server with explicit global project selection",
            command: Some(
                "clickhousectl local server stop <name> --global --project <project-root>",
            ),
        });
    }
    guidance
}

fn start_guidance(selection: ManagedClientSelection) -> LocalGuidance {
    match selection {
        ManagedClientSelection::Default => LocalGuidance {
            message: "Start the default managed server in this project",
            command: Some("clickhousectl local server start"),
        },
        ManagedClientSelection::Named => LocalGuidance {
            message: "Start the selected named managed server in this project",
            command: Some("clickhousectl local server start <name>"),
        },
    }
}

/// Write exactly one local runtime error object to stderr. The serialized DTO
/// is allowlisted above and never includes an error source or arbitrary detail.
pub fn print_error(error: &Error) {
    let output = LocalErrorOutput::from_error(error);
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    if serde_json::to_writer_pretty(&mut stderr, &output).is_ok() {
        let _ = writeln!(stderr);
    }
}

// ── list (installed) ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct InstalledVersion {
    pub version: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListInstalledOutput {
    pub versions: Vec<InstalledVersion>,
}

#[derive(Tabled)]
struct InstalledVersionRow {
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Default")]
    default: String,
}

impl fmt::Display for ListInstalledOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.versions.is_empty() {
            writeln!(f, "No versions installed")?;
            write!(f, "Run: clickhousectl local install stable")?;
            return Ok(());
        }
        let rows: Vec<InstalledVersionRow> = self
            .versions
            .iter()
            .map(|v| InstalledVersionRow {
                version: v.version.clone(),
                default: if v.default {
                    "yes".to_string()
                } else {
                    String::new()
                },
            })
            .collect();
        let table = Table::new(rows).with(Style::markdown()).to_string();
        write!(f, "{table}")
    }
}

// ── list --remote ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct AvailableVersion {
    pub version: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListAvailableOutput {
    pub versions: Vec<AvailableVersion>,
}

#[derive(Tabled)]
struct AvailableVersionRow {
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Installed")]
    installed: String,
}

impl fmt::Display for ListAvailableOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.versions.is_empty() {
            write!(f, "No versions available")?;
            return Ok(());
        }
        let rows: Vec<AvailableVersionRow> = self
            .versions
            .iter()
            .map(|v| AvailableVersionRow {
                version: v.version.clone(),
                installed: if v.installed {
                    "yes".to_string()
                } else {
                    String::new()
                },
            })
            .collect();
        let table = Table::new(rows).with(Style::markdown()).to_string();
        writeln!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Install with: clickhousectl local install <version>")?;
        write!(
            f,
            "For exact patch versions, use: clickhousectl local install 25.12.9.61"
        )
    }
}

// ── which ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct WhichOutput {
    pub version: String,
    pub binary_path: String,
}

impl fmt::Display for WhichOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.version, self.binary_path)
    }
}

// ── install ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct InstallOutput {
    pub version: String,
    pub set_as_default: bool,
}

impl fmt::Display for InstallOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Installed version {}", self.version)?;
        if self.set_as_default {
            write!(f, " (set as default)")?;
        }
        Ok(())
    }
}

// ── use ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UseOutput {
    pub version: String,
}

impl fmt::Display for UseOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Default version set to {}", self.version)
    }
}

// ── remove ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct RemoveOutput {
    pub version: String,
    /// `true` when the removed version was the one named by
    /// `~/.clickhouse/default`: that marker was deleted, and the global
    /// `~/.local/bin/clickhouse` symlink was removed if it still pointed into
    /// this version. Only reachable with `--force`; see
    /// [`crate::error::Error::VersionIsDefault`].
    pub was_default: bool,
}

impl fmt::Display for RemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Removed version {}", self.version)?;
        if self.was_default {
            write!(
                f,
                "\nCleared the default version marker (~/.clickhouse/default) and the global \
                 `clickhouse` symlink (~/.local/bin/clickhouse).\n\
                 Set a new default with: clickhousectl local use latest"
            )?;
        }
        Ok(())
    }
}

// ── init ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct InitOutput {
    /// Every project-local path this invocation created or manages, e.g.
    /// `.clickhouse/`, and (when newly created) `clickhouse/` and `postgres/`.
    pub paths: Vec<String>,
    /// Human-output detail only: the project dir already existed before this
    /// run. JSON consumers can tell from `paths`, so it is not serialized.
    #[serde(skip)]
    pub already_initialized: bool,
}

impl fmt::Display for InitOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dir = self
            .paths
            .first()
            .map(String::as_str)
            .unwrap_or(".clickhouse/");
        if self.already_initialized {
            write!(f, "Already initialized at {dir}")?;
        } else {
            write!(f, "Initialized ClickHouse project in {dir}")?;
        }
        for path in self.paths.iter().skip(1) {
            write!(f, "\nCreated project scaffold in {path}")?;
        }
        Ok(())
    }
}

// ── server configs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerConfigsOutput {
    pub dir: String,
    pub configs: Vec<String>,
}

impl fmt::Display for ServerConfigsOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.configs.is_empty() {
            writeln!(f, "No config files in {}", self.dir)?;
            write!(
                f,
                "Drop a ClickHouse config file there, then start with: \
                 clickhousectl local server start --config <NAME>"
            )?;
            return Ok(());
        }
        writeln!(f, "Config files in {}:", self.dir)?;
        for name in &self.configs {
            writeln!(f, "  {name}")?;
        }
        write!(
            f,
            "Use with: clickhousectl local server start --config <NAME>"
        )
    }
}

// ── server start ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerStartOutput {
    pub name: String,
    pub pid: u32,
    pub http_port: u16,
    pub tcp_port: u16,
    pub version: String,
}

impl fmt::Display for ServerStartOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Server '{}' started in background (PID: {})",
            self.name, self.pid
        )?;
        writeln!(f, "  HTTP port: {}", self.http_port)?;
        writeln!(f, "  TCP port:  {}", self.tcp_port)?;
        write!(f, "  Version:   {}", self.version)
    }
}

// ── server list ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerListEntry {
    pub name: String,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// "clickhouse" or "postgres".
    pub engine: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerListOutput {
    pub servers: Vec<ServerListEntry>,
    pub total_servers: usize,
    pub total_running_servers: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project_scope: Option<ServerProjectScope>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) guidance: Vec<ProjectServerGuidance>,
}

#[derive(Tabled)]
struct ServerListRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "PID")]
    pid: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "HTTP Port")]
    http_port: String,
    #[tabled(rename = "TCP Port")]
    tcp_port: String,
}

#[derive(Tabled)]
struct ServerListRowWithEngine {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Engine")]
    engine: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "ID")]
    pid_or_container: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "HTTP Port")]
    http_port: String,
    #[tabled(rename = "TCP Port")]
    tcp_port: String,
}

#[derive(Tabled)]
struct ServerListRowGlobal {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "PID")]
    pid: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "HTTP Port")]
    http_port: String,
    #[tabled(rename = "TCP Port")]
    tcp_port: String,
    #[tabled(rename = "Project")]
    project: String,
}

impl fmt::Display for ServerListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.servers.is_empty() {
            if let Some(scope) = &self.project_scope {
                writeln!(f, "No servers found in project '{}'.", scope.path)?;
                writeln!(
                    f,
                    "Project-local server list uses the exact current working directory; parent `.clickhouse` directories are not searched."
                )?;
                return write!(
                    f,
                    "Return to the local project root where the server was started and run `clickhousectl local server list`, or use `clickhousectl local server list --global` to locate running servers in other projects."
                );
            }
            write!(f, "No servers")?;
            return Ok(());
        }

        let has_project = self.servers.iter().any(|e| e.project.is_some());
        let has_postgres = self.servers.iter().any(|e| e.engine == "postgres");

        if !has_project && has_postgres {
            // Show an engine-aware table that combines PID (ClickHouse) and
            // container short-id (Postgres) into a single "ID" column.
            let rows: Vec<ServerListRowWithEngine> = self
                .servers
                .iter()
                .map(|e| {
                    let id = if e.engine == "postgres" {
                        e.container_id
                            .as_deref()
                            .map(|s| s.chars().take(12).collect::<String>())
                            .unwrap_or_default()
                    } else {
                        e.pid.map(|p| p.to_string()).unwrap_or_default()
                    };
                    ServerListRowWithEngine {
                        name: e.name.clone(),
                        engine: e.engine.clone(),
                        status: if e.running {
                            "running".into()
                        } else {
                            "stopped".into()
                        },
                        pid_or_container: id,
                        version: e.version.clone().unwrap_or_default(),
                        http_port: e.http_port.map(|p| p.to_string()).unwrap_or_default(),
                        tcp_port: e.tcp_port.map(|p| p.to_string()).unwrap_or_default(),
                    }
                })
                .collect();
            let table = Table::new(rows).with(Style::markdown()).to_string();
            writeln!(f, "{table}")?;
            return write!(
                f,
                "\n{} server{}, {} running",
                self.total_servers,
                if self.total_servers == 1 { "" } else { "s" },
                self.total_running_servers
            );
        }

        if has_project {
            let rows: Vec<ServerListRowGlobal> = self
                .servers
                .iter()
                .map(|e| ServerListRowGlobal {
                    name: e.name.clone(),
                    status: if e.running {
                        "running".to_string()
                    } else {
                        "stopped".to_string()
                    },
                    pid: e.pid.map(|p| p.to_string()).unwrap_or_default(),
                    version: e.version.clone().unwrap_or_default(),
                    http_port: e.http_port.map(|p| p.to_string()).unwrap_or_default(),
                    tcp_port: e.tcp_port.map(|p| p.to_string()).unwrap_or_default(),
                    project: e.project.clone().unwrap_or_default(),
                })
                .collect();
            let table = Table::new(rows).with(Style::markdown()).to_string();
            writeln!(f, "{table}")?;
        } else {
            let rows: Vec<ServerListRow> = self
                .servers
                .iter()
                .map(|e| ServerListRow {
                    name: e.name.clone(),
                    status: if e.running {
                        "running".to_string()
                    } else {
                        "stopped".to_string()
                    },
                    pid: e.pid.map(|p| p.to_string()).unwrap_or_default(),
                    version: e.version.clone().unwrap_or_default(),
                    http_port: e.http_port.map(|p| p.to_string()).unwrap_or_default(),
                    tcp_port: e.tcp_port.map(|p| p.to_string()).unwrap_or_default(),
                })
                .collect();
            let table = Table::new(rows).with(Style::markdown()).to_string();
            writeln!(f, "{table}")?;
        }

        write!(
            f,
            "\n{} server{}, {} running",
            self.total_servers,
            if self.total_servers == 1 { "" } else { "s" },
            self.total_running_servers
        )
    }
}

// ── postgres start ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PostgresStartOutput {
    pub name: String,
    pub container_id: String,
    pub image: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl fmt::Display for PostgresStartOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let short = self.container_id.chars().take(12).collect::<String>();
        writeln!(f, "Postgres '{}' running (container: {})", self.name, short)?;
        writeln!(f, "  Image:    {}", self.image)?;
        writeln!(f, "  Port:     {}", self.port)?;
        writeln!(f, "  User:     {}", self.user)?;
        writeln!(f, "  Password: {}", self.password)?;
        writeln!(f, "  Database: {}", self.database)?;
        write!(
            f,
            "  Connect:  clickhousectl local postgres client --name {}",
            self.name
        )
    }
}

// ── postgres dotenv ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PostgresDotenvOutput {
    pub file: String,
    pub server: String,
    pub vars: Vec<DotenvVar>,
}

impl fmt::Display for PostgresDotenvOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Wrote to {} (postgres '{}')", self.file, self.server)?;
        for var in &self.vars {
            writeln!(f, "  {}={}", var.key, var.value)?;
        }
        Ok(())
    }
}

// ── server stop ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerSelection {
    Explicit,
    Implicit,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerStopOutput {
    pub name: String,
    /// True when the server existed but was already stopped (idempotent noop).
    pub already_stopped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<ServerSelection>,
}

impl fmt::Display for ServerStopOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.already_stopped {
            write!(f, "Server '{}' is already stopped", self.name)?;
        } else {
            write!(f, "Server '{}' stopped", self.name)?;
        }
        if self.selection == Some(ServerSelection::Implicit) {
            write!(f, " (selected automatically)")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerStopNoopOutput {
    pub stopped: bool,
    pub selection: ServerSelection,
    pub reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project_scope: Option<ServerProjectScope>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) guidance: Vec<ProjectServerGuidance>,
}

impl fmt::Display for ServerStopNoopOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No ClickHouse servers found; nothing to stop")?;
        if let Some(scope) = &self.project_scope {
            writeln!(f)?;
            writeln!(
                f,
                "No `.clickhouse` directory existed under project '{}' when the command started.",
                scope.path
            )?;
            writeln!(
                f,
                "Project-local server stop uses the exact current working directory; parent `.clickhouse` directories are not searched."
            )?;
            write!(
                f,
                "The `.clickhouse` directory typically lives in the local project root where the server was started. Return there and run `clickhousectl local server list`, or use `clickhousectl local server list --global` to locate running servers in other projects."
            )?;
        }
        Ok(())
    }
}

// ── server stop-all ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerStopEntry {
    pub name: String,
    /// "clickhouse" or "postgres".
    pub engine: String,
    /// Postgres image version, used to distinguish same-name major versions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub stopped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerStopAllOutput {
    pub servers: Vec<ServerStopEntry>,
}

impl fmt::Display for ServerStopAllOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.servers.is_empty() {
            write!(f, "No running servers")?;
            return Ok(());
        }
        for s in &self.servers {
            let engine = match s.version.as_deref() {
                Some(version) => format!("{}, {}", s.engine, version),
                None => s.engine.clone(),
            };
            if s.stopped {
                writeln!(f, "Stopping '{}' ({})... stopped", s.name, engine)?;
            } else {
                writeln!(
                    f,
                    "Stopping '{}' ({})... error: {}",
                    s.name,
                    engine,
                    s.error.as_deref().unwrap_or("unknown")
                )?;
            }
        }
        write!(f, "Done")
    }
}

// ── server remove ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerRemoveOutput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<ServerSelection>,
}

impl fmt::Display for ServerRemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Server '{}' removed", self.name)?;
        if self.selection == Some(ServerSelection::Implicit) {
            write!(f, " (selected automatically)")?;
        }
        Ok(())
    }
}

// ── server dotenv ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerDotenvOutput {
    pub file: String,
    pub server: String,
    pub vars: Vec<DotenvVar>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DotenvVar {
    pub key: String,
    pub value: String,
}

impl fmt::Display for ServerDotenvOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Wrote to {} (server '{}')", self.file, self.server)?;
        for var in &self.vars {
            writeln!(f, "  {}={}", var.key, var.value)?;
        }
        Ok(())
    }
}

// ── helper ──────────────────────────────────────────────────────────────────

/// Print output as JSON or human-readable text.
pub fn print_output(output: &(impl Serialize + fmt::Display), json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(output).expect("JSON serialization failed")
        );
    } else {
        println!("{}", output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::BinaryLaunchProblem;

    fn error_json(error: &Error) -> serde_json::Value {
        serde_json::to_value(LocalErrorOutput::from_error(error)).unwrap()
    }

    #[test]
    fn local_error_codes_cover_the_stable_vocabulary() {
        let cases = [
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::ServerNotFound,
                    project_dir: "/project".into(),
                    selection: ManagedClientSelection::Default,
                    server_name: "default".into(),
                    binary_version: None,
                }),
                "managed_client_server_not_found",
            ),
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::ServerNotRunning,
                    project_dir: "/project".into(),
                    selection: ManagedClientSelection::Named,
                    server_name: "dev".into(),
                    binary_version: None,
                }),
                "managed_client_server_not_running",
            ),
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::BinaryNotFound,
                    project_dir: "/project".into(),
                    selection: ManagedClientSelection::Named,
                    server_name: "dev".into(),
                    binary_version: Some("25.12.9.61".into()),
                }),
                "managed_client_binary_not_found",
            ),
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::ProjectStateUnavailable(Box::new(
                        Error::ServerLock {
                            operation: "open the server metadata lock file",
                            path: "/project/.clickhouse/servers/.metadata.lock".into(),
                            remediation: "Check access, then retry.",
                            source: std::io::Error::other("lock failed"),
                        },
                    )),
                    project_dir: "/project".into(),
                    selection: ManagedClientSelection::Default,
                    server_name: "default".into(),
                    binary_version: None,
                }),
                "managed_client_project_state_unavailable",
            ),
            (
                Error::ProjectServerNotFound(ProjectServerNotFound {
                    command: ProjectServerCommand::Stop,
                    project_dir: "/project".into(),
                    server_name: "default".into(),
                }),
                "server_not_found",
            ),
            (
                Error::ProjectServerStateMissing(ProjectServerStateMissing {
                    command: ProjectServerCommand::Remove,
                    project_dir: "/project".into(),
                }),
                "server_selection_required",
            ),
            (Error::ServerNotFound("default".into()), "server_not_found"),
            (
                Error::ServerStopSelectionRequired { available: 2 },
                "server_selection_required",
            ),
            (
                Error::ServerInMultipleProjects {
                    name: "dev".into(),
                    projects: "/a, /b".into(),
                },
                "server_selection_required",
            ),
            (
                Error::ServerNotRunning("default".into()),
                "server_not_running",
            ),
            (
                Error::ServerAlreadyRunning("default".into()),
                "server_running",
            ),
            (
                Error::VersionInUse {
                    version: "25.12.9.61".into(),
                    servers: "default".into(),
                },
                "server_running",
            ),
            (
                Error::VersionIsDefault {
                    version: "25.12.9.61".into(),
                },
                "version_is_default",
            ),
            (
                Error::InvalidVersion("unsafe input".into()),
                "invalid_version",
            ),
            (
                Error::VersionNotFound("25.12.9.61".into()),
                "version_not_installed",
            ),
            (
                Error::StaleDefaultVersion("25.12.9.61".into()),
                "version_not_installed",
            ),
            (Error::NoVersionsInstalled, "version_not_installed"),
            (
                Error::ClientVersionNotInstalled("25.12.9.61".into()),
                "version_not_installed",
            ),
            (Error::NoDefaultVersion, "version_selection_required"),
            (Error::AmbiguousClientVersion, "version_selection_required"),
            (
                Error::VersionAlreadyInstalled("25.12.9.61".into()),
                "version_already_installed",
            ),
            (
                Error::RepeatedClientQueryUnsupported {
                    version: "24.1.1.1".into(),
                    minimum: "24.2",
                },
                "unsupported_client_version",
            ),
            (
                Error::NoMatchingVersion("99.99".into()),
                "version_unavailable",
            ),
            (
                Error::ExactVersionUnavailable {
                    version: "26.2.8.7".into(),
                    series: "26.2".into(),
                    available: "26.2.20.4".into(),
                },
                "version_unavailable",
            ),
            (
                Error::UnsupportedPlatform {
                    os: "plan9".into(),
                    arch: "sparc".into(),
                },
                "unsupported_platform",
            ),
            (
                Error::ConfigNotFound("config 'x' not found in /configs (available: none)".into()),
                "config_not_found",
            ),
            (
                Error::InvalidConfigName("../etc/passwd".into()),
                "invalid_config_name",
            ),
            (
                Error::InvalidServerName("../escape".into()),
                "invalid_server_name",
            ),
            (
                Error::UnsupportedArgument("--config cannot be passed through".into()),
                "unsupported_argument",
            ),
            (
                Error::UnsupportedArgument(
                    "--http-port 0 is not allowed; pick a specific port or omit the flag".into(),
                ),
                "unsupported_argument",
            ),
            (
                Error::DockerNotAvailable("Docker socket was not found.\nStart Docker.".into()),
                "docker_unavailable",
            ),
            (
                Error::DockerError("raw daemon details".into()),
                "docker_error",
            ),
            (
                Error::ContainerNameConflict("chctl-pg-dev-17".into()),
                "container_name_conflict",
            ),
            (
                Error::PostgresUsage("--port 0 is not allowed".into()),
                "postgres_error",
            ),
            (
                Error::ServerRunningCannotRemove("dev".into()),
                "server_running",
            ),
            (
                Error::PortInUse {
                    kind: PortKind::Http,
                    port: 8123,
                },
                "port_in_use",
            ),
            (
                Error::StartupExit {
                    kind: crate::error::StartupKind::ClickHouse,
                    name: "default".into(),
                    details: "raw startup details".into(),
                },
                "startup_exit",
            ),
            (
                Error::StartupTimeout {
                    kind: crate::error::StartupKind::Postgres,
                    name: "default".into(),
                    seconds: 60,
                    details: "raw timeout details".into(),
                },
                "startup_timeout",
            ),
            (
                Error::Download("raw download details".into()),
                "download_failed",
            ),
            (
                Error::Io(std::io::Error::other("raw I/O details")),
                "io_error",
            ),
            (Error::Exec("raw fallback details".into()), "local_error"),
            (
                Error::BinaryNotLaunchable {
                    version: "25.12.9.61".into(),
                    problem: BinaryLaunchProblem::NotExecutable,
                    path: "/home/u/.clickhouse/versions/25.12.9.61/clickhouse".into(),
                },
                "binary_not_launchable",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error_json(&error)["error"]["code"], expected);
        }
    }

    /// An installed-but-unlaunchable build is a different failure from a
    /// missing one, and its recovery is the reinstall of *that* version (#471).
    #[test]
    fn unlaunchable_binary_json_error_names_the_problem_and_the_reinstall() {
        let json = error_json(&Error::BinaryNotLaunchable {
            version: "25.12.9.61".into(),
            problem: BinaryLaunchProblem::NotExecutable,
            path: "/home/u/.clickhouse/versions/25.12.9.61/clickhouse".into(),
        });
        let message = json["error"]["message"].as_str().expect("message");
        assert!(message.contains("not executable"), "{message}");
        assert!(
            message.contains("/home/u/.clickhouse/versions/25.12.9.61/clickhouse"),
            "{message}"
        );
        assert_eq!(
            json["error"]["command"],
            "clickhousectl local install --force 25.12.9.61"
        );
    }

    #[test]
    fn version_is_default_json_error_explains_both_the_refusal_and_the_way_forward() {
        let json = error_json(&Error::VersionIsDefault {
            version: "25.12.9.61".into(),
        });
        let message = json["error"]["message"].as_str().expect("message");

        for required in [
            "current default",
            "~/.clickhouse/default",
            "~/.local/bin/clickhouse",
            "--force",
        ] {
            assert!(
                message.contains(required),
                "missing {required:?}: {message}"
            );
        }
        assert_eq!(
            json["error"]["command"], "clickhousectl local use latest",
            "the JSON error must name the recovery command"
        );
    }

    #[test]
    fn structured_fallback_and_wrapped_errors_never_serialize_raw_details() {
        let sensitive =
            "SELECT * FROM private_table; password=hunter2; /Users/al/secret; container=abc";
        let fallback = serde_json::to_string(&LocalErrorOutput::from_error(&Error::Exec(
            sensitive.to_string(),
        )))
        .unwrap();
        assert_eq!(
            fallback,
            r#"{"error":{"code":"local_error","message":"Local command failed"}}"#
        );
        assert!(!fallback.contains(sensitive));

        let wrapped = Error::PostgresStartupRollback {
            primary: Box::new(Error::StartupExit {
                kind: crate::error::StartupKind::Postgres,
                name: "default".into(),
                details: sensitive.into(),
            }),
            cleanup: sensitive.into(),
        };
        let wrapped = serde_json::to_string(&LocalErrorOutput::from_error(&wrapped)).unwrap();
        assert_eq!(
            wrapped,
            r#"{"error":{"code":"startup_exit","message":"Postgres server 'default' exited before becoming ready","command":"clickhousectl local server list"}}"#
        );
        assert!(!wrapped.contains("hunter2"));
    }

    #[test]
    fn running_server_remove_json_error_points_at_stopping_that_server() {
        assert_eq!(
            serde_json::to_string(&LocalErrorOutput::from_error(
                &Error::ServerRunningCannotRemove("dev".into())
            ))
            .unwrap(),
            r#"{"error":{"code":"server_running","message":"Server 'dev' is running; stop it first with `clickhousectl local server stop dev`","command":"clickhousectl local server stop dev"}}"#
        );
    }

    #[test]
    fn missing_config_json_error_keeps_the_full_human_detail() {
        let error = Error::ConfigNotFound(
            "config 'does-not-exist' not found in /home/dev/.clickhouse/configs \
             (available: analytics.xml)"
                .to_string(),
        );

        assert_eq!(
            serde_json::to_value(LocalErrorOutput::from_error(&error)).unwrap(),
            serde_json::json!({
                "error": {
                    "code": "config_not_found",
                    "message": "config 'does-not-exist' not found in /home/dev/.clickhouse/configs (available: analytics.xml)",
                    "command": "clickhousectl local server configs"
                }
            })
        );
    }

    #[test]
    fn a_version_that_is_not_installed_is_not_reported_as_unavailable_for_download() {
        assert_eq!(
            serde_json::to_value(LocalErrorOutput::from_error(&Error::VersionNotFound(
                "25.12.9.61".into()
            )))
            .unwrap(),
            serde_json::json!({
                "error": {
                    "code": "version_not_installed",
                    "message": "Version 25.12.9.61 not found",
                    "command": "clickhousectl local list"
                }
            })
        );

        // A version that cannot be resolved remotely keeps the remote hint.
        let unresolvable = error_json(&Error::NoMatchingVersion("99.99".into()));
        assert_eq!(unresolvable["error"]["code"], "version_unavailable");
        assert_eq!(
            unresolvable["error"]["command"],
            "clickhousectl local list --remote"
        );
    }

    /// Errors this crate composes itself render their human text verbatim, so
    /// `--json` never carries less than the `Error: ...` line does.
    #[test]
    fn self_composed_errors_serialize_their_human_message_verbatim() {
        let cases = [
            Error::ServerNotFound("dev".into()),
            Error::ServerNotRunning("dev".into()),
            Error::ServerAlreadyRunning("dev".into()),
            Error::ServerRunningCannotRemove("dev".into()),
            Error::ServerStopSelectionRequired { available: 2 },
            Error::ServerRemoveSelectionRequired { available: 1 },
            Error::ServerInMultipleProjects {
                name: "dev".into(),
                projects: "/projects/a, /projects/b".into(),
            },
            Error::InvalidServerName("../escape".into()),
            Error::UnsupportedArgument("--config cannot be passed through".into()),
            Error::UnsupportedArgument(
                "--tcp-port 0 is not allowed; pick a specific port or omit the flag".into(),
            ),
            Error::ConfigNotFound("config 'x' not found in /configs (available: y.xml)".into()),
            Error::InvalidConfigName("../etc/passwd".into()),
            Error::VersionNotFound("25.12.9.61".into()),
            Error::NoVersionsInstalled,
            Error::NoDefaultVersion,
            Error::NoClientVersionInstalled,
            Error::AmbiguousClientVersion,
            Error::StaleDefaultVersion("25.12.9.61".into()),
            Error::ClientVersionNotInstalled("25.12.9.61".into()),
            Error::BinaryNotLaunchable {
                version: "25.12.9.61".into(),
                problem: BinaryLaunchProblem::NotAFile,
                path: "/home/u/.clickhouse/versions/25.12.9.61/clickhouse".into(),
            },
            Error::RepeatedClientQueryUnsupported {
                version: "24.1.1.1".into(),
                minimum: "24.2",
            },
            Error::VersionAlreadyInstalled("25.12.9.61".into()),
            Error::VersionInUse {
                version: "25.12.9.61".into(),
                servers: "dev (/project, pid 42)".into(),
            },
            Error::VersionIsDefault {
                version: "25.12.9.61".into(),
            },
            Error::NoMatchingVersion("99.99".into()),
            Error::ExactVersionUnavailable {
                version: "26.2.8.7".into(),
                series: "26.2".into(),
                available: "26.2.20.4".into(),
            },
            Error::UnknownVersionChannel("26.2.8.7".into()),
            Error::InvalidVersion("Invalid version 'nope'".into()),
            Error::UnsupportedPlatform {
                os: "plan9".into(),
                arch: "sparc".into(),
            },
            Error::PortInUse {
                kind: PortKind::Postgres,
                port: 5432,
            },
            Error::PortUnavailable(PortKind::Http),
            Error::DockerNotAvailable("Docker socket was not found.\nStart Docker Desktop.".into()),
            Error::ContainerNameConflict("chctl-pg-dev-17".into()),
            Error::PostgresUsage(
                "multiple postgres instances named 'dev' (17, 18); pass --version to select one"
                    .into(),
            ),
        ];

        for error in cases {
            let json = error_json(&error);
            assert_eq!(
                json["error"]["message"],
                serde_json::Value::String(error.to_string()),
                "JSON message must match human output for {error:?}"
            );
        }
    }

    /// The complement of the parity rule: text that interpolates foreign output
    /// (subprocess, Docker daemon, OS/serde sources) stays summarized.
    #[test]
    fn errors_carrying_foreign_output_stay_summarized() {
        let secret = "password=hunter2; /Users/al/secret-project";
        let cases = [
            (Error::Exec(secret.into()), "Local command failed"),
            (Error::DockerError(secret.into()), "Docker operation failed"),
            (Error::Postgres(secret.into()), "Local command failed"),
            (Error::Download(secret.into()), "Download failed"),
            (Error::Extract(secret.into()), "Extraction failed"),
            (
                Error::Io(std::io::Error::other(secret)),
                "Local I/O operation failed",
            ),
            (
                Error::ServerMetadataWrite {
                    path: secret.into(),
                    source: std::io::Error::other(secret),
                },
                "Local I/O operation failed",
            ),
            (
                Error::ServerLock {
                    operation: "open the server metadata lock file",
                    path: secret.into(),
                    remediation: "Check access, then retry.",
                    source: std::io::Error::other(secret),
                },
                "Local I/O operation failed",
            ),
            (
                Error::StartupTimeout {
                    kind: crate::error::StartupKind::ClickHouse,
                    name: "default".into(),
                    seconds: 30,
                    details: secret.into(),
                },
                "ClickHouse server 'default' did not become ready within 30 seconds",
            ),
        ];

        for (error, expected) in cases {
            let serialized = serde_json::to_string(&LocalErrorOutput::from_error(&error)).unwrap();
            assert_eq!(
                error_json(&error)["error"]["message"],
                expected,
                "unexpected summary for {error:?}"
            );
            assert!(
                !serialized.contains("hunter2") && !serialized.contains("secret-project"),
                "leaked foreign output: {serialized}"
            );
        }
    }

    // ── JSON serialization tests ────────────────────────────────────────

    #[test]
    fn list_installed_json_with_versions() {
        let output = ListInstalledOutput {
            versions: vec![
                InstalledVersion {
                    version: "25.12.5.44".to_string(),
                    default: true,
                },
                InstalledVersion {
                    version: "25.11.3.22".to_string(),
                    default: false,
                },
            ],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["versions"][0]["version"], "25.12.5.44");
        assert_eq!(json["versions"][0]["default"], true);
        assert_eq!(json["versions"][1]["version"], "25.11.3.22");
        assert_eq!(json["versions"][1]["default"], false);
        assert_eq!(json["versions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn list_installed_json_empty() {
        let output = ListInstalledOutput { versions: vec![] };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();
        assert_eq!(json["versions"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn list_available_json_with_versions() {
        let output = ListAvailableOutput {
            versions: vec![
                AvailableVersion {
                    version: "25.12".to_string(),
                    installed: true,
                },
                AvailableVersion {
                    version: "25.11".to_string(),
                    installed: false,
                },
            ],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["versions"][0]["version"], "25.12");
        assert_eq!(json["versions"][0]["installed"], true);
        assert_eq!(json["versions"][1]["installed"], false);
    }

    #[test]
    fn which_json() {
        let output = WhichOutput {
            version: "25.12.5.44".to_string(),
            binary_path: "/home/user/.clickhouse/versions/25.12.5.44/clickhouse".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["version"], "25.12.5.44");
        assert_eq!(
            json["binary_path"],
            "/home/user/.clickhouse/versions/25.12.5.44/clickhouse"
        );
    }

    #[test]
    fn install_json() {
        let output = InstallOutput {
            version: "25.12.5.44".to_string(),
            set_as_default: true,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["version"], "25.12.5.44");
        assert_eq!(json["set_as_default"], true);
    }

    #[test]
    fn use_json() {
        let output = UseOutput {
            version: "25.12.5.44".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["version"], "25.12.5.44");
    }

    #[test]
    fn remove_json() {
        let output = RemoveOutput {
            version: "25.12.5.44".to_string(),
            was_default: false,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["version"], "25.12.5.44");
        assert_eq!(json["was_default"], false);
    }

    #[test]
    fn remove_json_reports_a_cleared_default() {
        let output = RemoveOutput {
            version: "25.12.5.44".to_string(),
            was_default: true,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["version"], "25.12.5.44");
        assert_eq!(json["was_default"], true);
    }

    #[test]
    fn remove_human_output_warns_when_the_default_was_cleared() {
        let plain = RemoveOutput {
            version: "25.12.5.44".to_string(),
            was_default: false,
        }
        .to_string();
        assert_eq!(plain, "Removed version 25.12.5.44");

        let cleared = RemoveOutput {
            version: "25.12.5.44".to_string(),
            was_default: true,
        }
        .to_string();
        assert!(
            cleared.starts_with("Removed version 25.12.5.44\n"),
            "{cleared}"
        );
        for required in [
            "~/.clickhouse/default",
            "~/.local/bin/clickhouse",
            "clickhousectl local use latest",
        ] {
            assert!(
                cleared.contains(required),
                "missing {required:?}: {cleared}"
            );
        }
    }

    #[test]
    fn init_json_first_run_reports_all_created_paths() {
        let output = InitOutput {
            paths: vec![
                ".clickhouse/".to_string(),
                "clickhouse/".to_string(),
                "postgres/".to_string(),
            ],
            already_initialized: false,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(
            json["paths"],
            serde_json::json!([".clickhouse/", "clickhouse/", "postgres/"])
        );
        // Human-only detail must stay out of the JSON payload.
        assert!(json.get("already_initialized").is_none());
    }

    #[test]
    fn init_json_idempotent_run_only_reports_clickhouse_dir() {
        let output = InitOutput {
            paths: vec![".clickhouse/".to_string()],
            already_initialized: true,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["paths"], serde_json::json!([".clickhouse/"]));
    }

    #[test]
    fn server_start_json() {
        let output = ServerStartOutput {
            name: "default".to_string(),
            pid: 12345,
            http_port: 8123,
            tcp_port: 9000,
            version: "25.12.5.44".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["name"], "default");
        assert_eq!(json["pid"], 12345);
        assert_eq!(json["http_port"], 8123);
        assert_eq!(json["tcp_port"], 9000);
        assert_eq!(json["version"], "25.12.5.44");
    }

    #[test]
    fn server_list_json_with_entries() {
        let output = ServerListOutput {
            servers: vec![
                ServerListEntry {
                    name: "default".to_string(),
                    running: true,
                    pid: Some(12345),
                    version: Some("25.12.5.44".to_string()),
                    http_port: Some(8123),
                    tcp_port: Some(9000),
                    project: None,
                    engine: "clickhouse".to_string(),
                    container_id: None,
                },
                ServerListEntry {
                    name: "test".to_string(),
                    running: false,
                    pid: None,
                    version: None,
                    http_port: None,
                    tcp_port: None,
                    project: None,
                    engine: "clickhouse".to_string(),
                    container_id: None,
                },
            ],
            total_servers: 2,
            total_running_servers: 1,
            project_scope: Some(exact_current_project_scope(Path::new("/project"))),
            guidance: Vec::new(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"].as_array().unwrap().len(), 2);
        assert_eq!(json["servers"][0]["name"], "default");
        assert_eq!(json["servers"][0]["running"], true);
        assert_eq!(json["servers"][0]["pid"], 12345);
        assert_eq!(json["servers"][1]["name"], "test");
        assert_eq!(json["servers"][1]["running"], false);
        // Stopped server should not have pid/version/ports in JSON
        assert!(json["servers"][1].get("pid").is_none());
        assert!(json["servers"][1].get("version").is_none());
        assert_eq!(json["total_servers"], 2);
        assert_eq!(json["total_running_servers"], 1);
        assert_eq!(json["project_scope"]["path"], "/project");
        assert_eq!(json["project_scope"]["parent_projects_searched"], false);
    }

    #[test]
    fn server_list_json_empty() {
        let output = ServerListOutput {
            servers: vec![],
            total_servers: 0,
            total_running_servers: 0,
            project_scope: Some(exact_current_project_scope(Path::new("/project"))),
            guidance: project_scope_guidance(None),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"].as_array().unwrap().len(), 0);
        assert_eq!(json["total_servers"], 0);
        assert_eq!(json["total_running_servers"], 0);
        assert_eq!(json["project_scope"]["kind"], "exact_current_project");
        assert_eq!(json["guidance"][2]["action"], "list_global_servers");
    }

    #[test]
    fn server_stop_json() {
        let output = ServerStopOutput {
            name: "default".to_string(),
            already_stopped: false,
            selection: Some(ServerSelection::Explicit),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["name"], "default");
        assert_eq!(json["already_stopped"], false);
        assert_eq!(json["selection"], "explicit");
    }

    #[test]
    fn server_stop_already_stopped() {
        let output = ServerStopOutput {
            name: "default".to_string(),
            already_stopped: true,
            selection: Some(ServerSelection::Implicit),
        };
        assert_eq!(
            output.to_string(),
            "Server 'default' is already stopped (selected automatically)"
        );

        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();
        assert_eq!(json["already_stopped"], true);
    }

    #[test]
    fn server_stop_all_json() {
        let output = ServerStopAllOutput {
            servers: vec![
                ServerStopEntry {
                    name: "default".to_string(),
                    engine: "clickhouse".to_string(),
                    version: None,
                    stopped: true,
                    error: None,
                },
                ServerStopEntry {
                    name: "default".to_string(),
                    engine: "postgres".to_string(),
                    version: Some("postgres:18".to_string()),
                    stopped: false,
                    error: Some("container not found".to_string()),
                },
            ],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"][0]["name"], "default");
        assert_eq!(json["servers"][0]["engine"], "clickhouse");
        assert_eq!(json["servers"][0]["stopped"], true);
        assert!(json["servers"][0].get("version").is_none());
        assert!(json["servers"][0].get("error").is_none());
        assert_eq!(json["servers"][1]["name"], "default");
        assert_eq!(json["servers"][1]["engine"], "postgres");
        assert_eq!(json["servers"][1]["version"], "postgres:18");
        assert_eq!(json["servers"][1]["stopped"], false);
        assert_eq!(json["servers"][1]["error"], "container not found");
    }

    #[test]
    fn server_stop_all_json_empty() {
        let output = ServerStopAllOutput { servers: vec![] };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn server_configs_json() {
        let output = ServerConfigsOutput {
            dir: "/home/user/.clickhouse/configs".to_string(),
            configs: vec!["dev.xml".to_string(), "prod.yaml".to_string()],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();
        assert_eq!(json["dir"], "/home/user/.clickhouse/configs");
        assert_eq!(json["configs"][0], "dev.xml");
        assert_eq!(json["configs"][1], "prod.yaml");
        assert_eq!(json["configs"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn server_configs_display_empty() {
        let output = ServerConfigsOutput {
            dir: "/home/user/.clickhouse/configs".to_string(),
            configs: vec![],
        };
        let text = output.to_string();
        assert!(text.contains("No config files"));
        assert!(text.contains("--config <NAME>"));
    }

    #[test]
    fn server_configs_display_with_entries() {
        let output = ServerConfigsOutput {
            dir: "/home/user/.clickhouse/configs".to_string(),
            configs: vec!["dev.xml".to_string()],
        };
        let text = output.to_string();
        assert!(text.contains("dev.xml"));
        assert!(text.contains("Use with:"));
    }

    #[test]
    fn server_remove_json() {
        let output = ServerRemoveOutput {
            name: "test".to_string(),
            selection: Some(ServerSelection::Explicit),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["name"], "test");
        assert_eq!(json["selection"], "explicit");
    }

    // ── Display (human-readable) tests ──────────────────────────────────

    #[test]
    fn list_installed_display_with_versions() {
        let output = ListInstalledOutput {
            versions: vec![
                InstalledVersion {
                    version: "25.12.5.44".to_string(),
                    default: true,
                },
                InstalledVersion {
                    version: "25.11.3.22".to_string(),
                    default: false,
                },
            ],
        };
        let text = output.to_string();
        assert!(text.contains("Version"));
        assert!(text.contains("Default"));
        assert!(text.contains("25.12.5.44"));
        assert!(text.contains("yes"));
        assert!(text.contains("25.11.3.22"));
    }

    #[test]
    fn list_installed_display_empty() {
        let output = ListInstalledOutput { versions: vec![] };
        let text = output.to_string();
        assert!(text.contains("No versions installed"));
        assert!(text.contains("Run: clickhousectl local install stable"));
    }

    #[test]
    fn list_available_display_with_versions() {
        let output = ListAvailableOutput {
            versions: vec![
                AvailableVersion {
                    version: "25.12".to_string(),
                    installed: true,
                },
                AvailableVersion {
                    version: "25.11".to_string(),
                    installed: false,
                },
            ],
        };
        let text = output.to_string();
        assert!(text.contains("Version"));
        assert!(text.contains("Installed"));
        assert!(text.contains("25.12"));
        assert!(text.contains("yes"));
        assert!(text.contains("25.11"));
        assert!(text.contains("Install with: clickhousectl local install <version>"));
    }

    #[test]
    fn list_available_display_empty() {
        let output = ListAvailableOutput { versions: vec![] };
        assert_eq!(output.to_string(), "No versions available");
    }

    #[test]
    fn which_display() {
        let output = WhichOutput {
            version: "25.12.5.44".to_string(),
            binary_path: "/home/user/.clickhouse/versions/25.12.5.44/clickhouse".to_string(),
        };
        assert_eq!(
            output.to_string(),
            "25.12.5.44 (/home/user/.clickhouse/versions/25.12.5.44/clickhouse)"
        );
    }

    #[test]
    fn install_display() {
        let output = InstallOutput {
            version: "25.12.5.44".to_string(),
            set_as_default: false,
        };
        assert_eq!(output.to_string(), "Installed version 25.12.5.44");

        let output_default = InstallOutput {
            version: "25.12.5.44".to_string(),
            set_as_default: true,
        };
        assert_eq!(
            output_default.to_string(),
            "Installed version 25.12.5.44 (set as default)"
        );
    }

    #[test]
    fn use_display() {
        let output = UseOutput {
            version: "25.12.5.44".to_string(),
        };
        assert_eq!(output.to_string(), "Default version set to 25.12.5.44");
    }

    #[test]
    fn remove_display() {
        let output = RemoveOutput {
            version: "25.12.5.44".to_string(),
            was_default: false,
        };
        assert_eq!(output.to_string(), "Removed version 25.12.5.44");
    }

    #[test]
    fn init_display_idempotent() {
        let output = InitOutput {
            paths: vec![".clickhouse/".to_string()],
            already_initialized: true,
        };
        assert_eq!(output.to_string(), "Already initialized at .clickhouse/");
    }

    #[test]
    fn init_display_first_run_lists_created_scaffolds() {
        let output = InitOutput {
            paths: vec![
                ".clickhouse/".to_string(),
                "clickhouse/".to_string(),
                "postgres/".to_string(),
            ],
            already_initialized: false,
        };
        assert_eq!(
            output.to_string(),
            "Initialized ClickHouse project in .clickhouse/\n\
             Created project scaffold in clickhouse/\n\
             Created project scaffold in postgres/"
        );
    }

    #[test]
    fn server_start_display() {
        let output = ServerStartOutput {
            name: "default".to_string(),
            pid: 12345,
            http_port: 8123,
            tcp_port: 9000,
            version: "25.12.5.44".to_string(),
        };
        let text = output.to_string();
        assert!(text.contains("Server 'default' started in background (PID: 12345)"));
        assert!(text.contains("  HTTP port: 8123"));
        assert!(text.contains("  TCP port:  9000"));
        assert!(text.contains("  Version:   25.12.5.44"));
    }

    #[test]
    fn server_list_display_with_entries() {
        let output = ServerListOutput {
            servers: vec![
                ServerListEntry {
                    name: "default".to_string(),
                    running: true,
                    pid: Some(12345),
                    version: Some("25.12.5.44".to_string()),
                    http_port: Some(8123),
                    tcp_port: Some(9000),
                    project: None,
                    engine: "clickhouse".to_string(),
                    container_id: None,
                },
                ServerListEntry {
                    name: "test".to_string(),
                    running: false,
                    pid: None,
                    version: None,
                    http_port: None,
                    tcp_port: None,
                    project: None,
                    engine: "clickhouse".to_string(),
                    container_id: None,
                },
            ],
            total_servers: 2,
            total_running_servers: 1,
            project_scope: None,
            guidance: Vec::new(),
        };
        let text = output.to_string();
        assert!(text.contains("Name"));
        assert!(text.contains("Status"));
        assert!(text.contains("PID"));
        assert!(text.contains("HTTP Port"));
        assert!(text.contains("TCP Port"));
        assert!(text.contains("default"));
        assert!(text.contains("running"));
        assert!(text.contains("12345"));
        assert!(text.contains("25.12.5.44"));
        assert!(text.contains("8123"));
        assert!(text.contains("9000"));
        assert!(text.contains("test"));
        assert!(text.contains("stopped"));
        assert!(text.contains("2 servers, 1 running"));
    }

    #[test]
    fn server_list_display_empty() {
        let output = ServerListOutput {
            servers: vec![],
            total_servers: 0,
            total_running_servers: 0,
            project_scope: None,
            guidance: Vec::new(),
        };
        assert_eq!(output.to_string(), "No servers");
    }

    #[test]
    fn server_list_display_empty_project_explains_exact_scope() {
        let output = ServerListOutput {
            servers: vec![],
            total_servers: 0,
            total_running_servers: 0,
            project_scope: Some(exact_current_project_scope(Path::new("/project"))),
            guidance: project_scope_guidance(None),
        };
        let text = output.to_string();
        assert!(text.contains("No servers found in project '/project'"));
        assert!(text.contains("exact current working directory"));
        assert!(text.contains("parent `.clickhouse` directories are not searched"));
        assert!(text.contains("clickhousectl local server list --global"));
    }

    #[test]
    fn server_list_display_single() {
        let output = ServerListOutput {
            servers: vec![ServerListEntry {
                name: "default".to_string(),
                running: true,
                pid: Some(100),
                version: Some("25.12.5.44".to_string()),
                http_port: Some(8123),
                tcp_port: Some(9000),
                project: None,
                engine: "clickhouse".to_string(),
                container_id: None,
            }],
            total_servers: 1,
            total_running_servers: 1,
            project_scope: None,
            guidance: Vec::new(),
        };
        let text = output.to_string();
        assert!(text.contains("1 server, 1 running"));
    }

    #[test]
    fn server_stop_display() {
        let output = ServerStopOutput {
            name: "default".to_string(),
            already_stopped: false,
            selection: Some(ServerSelection::Explicit),
        };
        assert_eq!(output.to_string(), "Server 'default' stopped");
    }

    #[test]
    fn server_stop_all_display() {
        let output = ServerStopAllOutput {
            servers: vec![
                ServerStopEntry {
                    name: "default".to_string(),
                    engine: "clickhouse".to_string(),
                    version: None,
                    stopped: true,
                    error: None,
                },
                ServerStopEntry {
                    name: "default".to_string(),
                    engine: "postgres".to_string(),
                    version: Some("postgres:18".to_string()),
                    stopped: false,
                    error: Some("container not found".to_string()),
                },
            ],
        };
        let text = output.to_string();
        assert!(text.contains("Stopping 'default' (clickhouse)... stopped"));
        assert!(
            text.contains(
                "Stopping 'default' (postgres, postgres:18)... error: container not found"
            )
        );
        assert!(text.contains("Done"));
    }

    #[test]
    fn server_stop_all_display_empty() {
        let output = ServerStopAllOutput { servers: vec![] };
        assert_eq!(output.to_string(), "No running servers");
    }

    #[test]
    fn server_remove_display() {
        let output = ServerRemoveOutput {
            name: "test".to_string(),
            selection: Some(ServerSelection::Explicit),
        };
        assert_eq!(output.to_string(), "Server 'test' removed");
    }

    // ── print_output helper tests ───────────────────────────────────────

    #[test]
    fn json_keys_use_snake_case() {
        // Verify all JSON field names are snake_case (not camelCase)
        let output = ServerStartOutput {
            name: "default".to_string(),
            pid: 1,
            http_port: 8123,
            tcp_port: 9000,
            version: "25.12".to_string(),
        };
        let json_str = serde_json::to_string(&output).unwrap();
        assert!(json_str.contains("\"http_port\""));
        assert!(json_str.contains("\"tcp_port\""));
        assert!(!json_str.contains("\"httpPort\""));
        assert!(!json_str.contains("\"tcpPort\""));
    }

    #[test]
    fn install_output_roundtrip() {
        let output = InstallOutput {
            version: "25.12.5.44".to_string(),
            set_as_default: true,
        };
        let json_str = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["version"], "25.12.5.44");
        assert_eq!(parsed["set_as_default"], true);
    }
}
